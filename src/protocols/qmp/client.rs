//! Asynchronous QMP client operating over stream abstraction.
//!
//! Defined in accordance with RFC 9003 and plan.md §Structure.

use super::codec::{
    JsonLineReader, QmpErrorData, QmpGreetingData, QmpMessage, encode_command, parse_bytes,
};
use super::commands::{self, QmpStatusResponse};
use super::events::{QmpEventSender, QmpLifecycleEvent};
use crate::protocols::constants::{
    DEFAULT_QMP_EVENT_CAPACITY, DEFAULT_QMP_TIMEOUT, MAX_JSONL_FRAME_SIZE,
};
use anyhow::{Context, Result, bail};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::sync::{Mutex, broadcast, oneshot};
use tokio::task::JoinHandle;

type PendingMap =
    Arc<Mutex<HashMap<String, oneshot::Sender<Result<serde_json::Value, QmpErrorData>>>>>;

/// Asynchronous QMP client operating over generic stream halves.
///
/// Features:
/// - Bounded line reading (max 1 MiB) with rejection of oversize and incomplete frames
/// - Automatic initial handshake with `qmp_capabilities`
/// - Monotonic request ID correlation for request/response pairing
/// - Continuous background reader task that delivers unsolicited lifecycle events even when
///   no command is running
/// - Strict deadline handling without replaying mutating commands on timeout
/// - Clean owned shutdown
pub struct QmpClient<W> {
    writer: W,
    event_tx: QmpEventSender,
    greeting: Option<QmpGreetingData>,
    pending: PendingMap,
    next_id: Arc<AtomicU64>,
    reader_handle: Option<JoinHandle<()>>,
    timeout: Duration,
}

impl<W: AsyncWrite + Unpin + Send + 'static> QmpClient<W> {
    /// Connects to a QMP stream, completes the initial greeting/capabilities handshake,
    /// and spawns the continuous background reader.
    pub async fn new(
        reader: impl AsyncRead + Unpin + Send + 'static,
        mut writer: W,
    ) -> Result<Self> {
        let buf_reader = BufReader::new(Box::new(reader) as Box<dyn AsyncRead + Unpin + Send>);
        let mut line_reader = JsonLineReader::new(buf_reader, MAX_JSONL_FRAME_SIZE);

        // 1. Read initial greeting banner
        let greeting_raw = line_reader
            .read_line_raw()
            .await
            .context("Failed to read QMP greeting banner from stream")?
            .ok_or_else(|| anyhow::anyhow!("Stream closed before QMP greeting was received"))?;

        let greeting_msg =
            parse_bytes(&greeting_raw).context("Failed to parse initial QMP greeting message")?;

        let greeting = match greeting_msg {
            QmpMessage::Greeting { qmp } => qmp,
            other => bail!("Expected QMP Greeting banner, received: {other:?}"),
        };

        // 2. Perform capabilities negotiation handshake with unique ID
        let handshake_id = "cap_init";
        let cap_cmd = commands::qmp_capabilities(Some(handshake_id))?;
        writer
            .write_all(&cap_cmd)
            .await
            .context("Failed to write qmp_capabilities handshake command")?;
        writer
            .flush()
            .await
            .context("Failed to flush capabilities command")?;

        let mut initial_events = Vec::new();
        loop {
            let resp_raw = line_reader
                .read_line_raw()
                .await
                .context("Failed to read response to qmp_capabilities")?
                .ok_or_else(|| {
                    anyhow::anyhow!("Stream closed while awaiting qmp_capabilities response")
                })?;

            let resp_msg =
                parse_bytes(&resp_raw).context("Failed to parse qmp_capabilities response")?;

            match resp_msg {
                QmpMessage::Return { .. } => break,
                QmpMessage::Error { error, .. } => {
                    bail!(
                        "qmp_capabilities handshake rejected by remote: [{}] {}",
                        error.class,
                        error.desc
                    );
                }
                QmpMessage::Event { event, data, .. } => {
                    initial_events.push(QmpLifecycleEvent::from_event_name_and_data(
                        &event,
                        data.as_ref(),
                    ));
                }
                QmpMessage::Greeting { .. } => {
                    bail!("Unexpected secondary QMP greeting banner received during handshake");
                }
            }
        }

        // 3. Set up event broadcasting channel and pending request registry
        let (event_tx, _) = broadcast::channel(DEFAULT_QMP_EVENT_CAPACITY);
        for ev in initial_events {
            let _ = event_tx.send(ev);
        }

        let pending: PendingMap = Arc::new(Mutex::new(HashMap::new()));
        let pending_clone = Arc::clone(&pending);
        let event_tx_clone = event_tx.clone();

        // 4. Spawn continuous background reader task
        let reader_handle = tokio::spawn(async move {
            while let Ok(Some(raw_line)) = line_reader.read_line_raw().await {
                if let Ok(msg) = parse_bytes(&raw_line) {
                    match msg {
                        QmpMessage::Event { event, data, .. } => {
                            let parsed =
                                QmpLifecycleEvent::from_event_name_and_data(&event, data.as_ref());
                            let _ = event_tx_clone.send(parsed);
                        }
                        QmpMessage::Return { return_value, id } => {
                            let mut map = pending_clone.lock().await;
                            if let Some(id_str) = id {
                                if let Some(tx) = map.remove(&id_str) {
                                    let _ = tx.send(Ok(return_value));
                                }
                            } else if map.len() == 1 {
                                let key = map.keys().next().cloned().unwrap();
                                if let Some(tx) = map.remove(&key) {
                                    let _ = tx.send(Ok(return_value));
                                }
                            }
                        }
                        QmpMessage::Error { error, id } => {
                            let mut map = pending_clone.lock().await;
                            if let Some(id_str) = id {
                                if let Some(tx) = map.remove(&id_str) {
                                    let _ = tx.send(Err(error));
                                }
                            } else if map.len() == 1 {
                                let key = map.keys().next().cloned().unwrap();
                                if let Some(tx) = map.remove(&key) {
                                    let _ = tx.send(Err(error));
                                }
                            }
                        }
                        QmpMessage::Greeting { .. } => {}
                    }
                }
            }

            // If stream reader loop exits, drain and fail all pending requests
            let mut map = pending_clone.lock().await;
            for (_, tx) in map.drain() {
                let _ = tx.send(Err(QmpErrorData {
                    class: "ConnectionClosed".to_string(),
                    desc: "QMP stream connection closed".to_string(),
                }));
            }
        });

        Ok(Self {
            writer,
            event_tx,
            greeting: Some(greeting),
            pending,
            next_id: Arc::new(AtomicU64::new(1)),
            reader_handle: Some(reader_handle),
            timeout: DEFAULT_QMP_TIMEOUT,
        })
    }

    /// Sets the command timeout duration.
    pub fn set_timeout(&mut self, timeout: Duration) {
        self.timeout = timeout;
    }

    /// Subscribes to the continuous stream of unsolicited QMP lifecycle events.
    pub fn subscribe_events(&self) -> broadcast::Receiver<QmpLifecycleEvent> {
        self.event_tx.subscribe()
    }

    /// Returns the QMP greeting metadata received during handshake.
    pub fn greeting(&self) -> Option<&QmpGreetingData> {
        self.greeting.as_ref()
    }

    /// Executes a typed QMP command with monotonic request ID correlation.
    ///
    /// Invariant: If a command times out, it is strictly NOT replayed to prevent
    /// unintended duplicate mutations.
    pub async fn execute(
        &mut self,
        execute: &str,
        arguments: Option<serde_json::Value>,
    ) -> Result<serde_json::Value> {
        let req_num = self.next_id.fetch_add(1, Ordering::SeqCst);
        let req_id = format!("req-{req_num}");

        let (tx, rx) = oneshot::channel();
        {
            let mut map = self.pending.lock().await;
            map.insert(req_id.clone(), tx);
        }

        let cmd = encode_command(execute, arguments, Some(&req_id))?;
        self.writer
            .write_all(&cmd)
            .await
            .context("Failed to write QMP command to socket")?;
        self.writer
            .flush()
            .await
            .context("Failed to flush QMP writer")?;

        match tokio::time::timeout(self.timeout, rx).await {
            Ok(Ok(Ok(val))) => Ok(val),
            Ok(Ok(Err(err))) => {
                bail!(
                    "QMP command '{execute}' failed: [{}] {}",
                    err.class,
                    err.desc
                )
            }
            Ok(Err(_canceled)) => {
                bail!("QMP connection lost while awaiting response for '{execute}'")
            }
            Err(_elapsed) => {
                // Remove pending request; DO NOT replay mutating commands
                let mut map = self.pending.lock().await;
                map.remove(&req_id);
                bail!(
                    "QMP command '{execute}' timed out after {:?}; mutating command not replayed",
                    self.timeout
                )
            }
        }
    }

    /// Executes a raw QMP command byte buffer.
    pub async fn execute_raw(&mut self, cmd_bytes: &[u8]) -> Result<serde_json::Value> {
        if let Ok(serde_json::Value::Object(map)) =
            serde_json::from_slice::<serde_json::Value>(cmd_bytes)
        {
            if let Some(serde_json::Value::String(exec_name)) = map.get("execute") {
                let args = map.get("arguments").cloned();
                return self.execute(exec_name, args).await;
            }
        }

        let req_num = self.next_id.fetch_add(1, Ordering::SeqCst);
        let req_id = format!("raw-{req_num}");
        let (tx, rx) = oneshot::channel();
        {
            let mut map = self.pending.lock().await;
            map.insert(req_id.clone(), tx);
        }

        self.writer.write_all(cmd_bytes).await?;
        self.writer.flush().await?;

        match tokio::time::timeout(self.timeout, rx).await {
            Ok(Ok(Ok(val))) => Ok(val),
            Ok(Ok(Err(err))) => bail!("QMP raw command failed: [{}] {}", err.class, err.desc),
            Ok(Err(_canceled)) => bail!("QMP connection lost while awaiting raw command response"),
            Err(_) => {
                let mut map = self.pending.lock().await;
                map.remove(&req_id);
                bail!("QMP raw command timed out after {:?}", self.timeout)
            }
        }
    }

    /// Queries the current virtual CPU runstate via `query-status`.
    ///
    /// Note: `running == true` indicates only that the vCPU execution loop is active,
    /// NOT proof of guest OS boot completion or root access readiness.
    pub async fn query_status(&mut self) -> Result<QmpStatusResponse> {
        let val = self.execute("query-status", None).await?;
        serde_json::from_value::<QmpStatusResponse>(val)
            .context("Failed to parse QmpStatusResponse from return payload")
    }

    /// Pauses virtual CPU execution (`stop`).
    pub async fn stop(&mut self) -> Result<()> {
        let _ = self.execute("stop", None).await?;
        Ok(())
    }

    /// Resumes virtual CPU execution (`cont`).
    pub async fn cont(&mut self) -> Result<()> {
        let _ = self.execute("cont", None).await?;
        Ok(())
    }

    /// Requests guest ACPI powerdown (`system_powerdown`).
    pub async fn system_powerdown(&mut self) -> Result<()> {
        let _ = self.execute("system_powerdown", None).await?;
        Ok(())
    }

    /// Requests guest ACPI system reset (`system_reset`).
    pub async fn system_reset(&mut self) -> Result<()> {
        let _ = self.execute("system_reset", None).await?;
        Ok(())
    }

    /// Cleanly terminates the hypervisor process (`quit`).
    pub async fn quit(&mut self) -> Result<()> {
        let _ = self.execute("quit", None).await?;
        Ok(())
    }

    /// Shuts down the QMP client and terminates the background reader task.
    pub async fn shutdown(mut self) -> Result<()> {
        if let Some(handle) = self.reader_handle.take() {
            handle.abort();
            let _ = handle.await;
        }
        self.writer
            .shutdown()
            .await
            .context("Failed to gracefully shutdown QMP writer stream")?;
        Ok(())
    }
}

impl<W> Drop for QmpClient<W> {
    fn drop(&mut self) {
        if let Some(handle) = self.reader_handle.take() {
            handle.abort();
        }
    }
}
