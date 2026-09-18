//! QEMU Machine Protocol (QMP) line-delimited JSON framing and wire codec.
//!
//! Defined in accordance with RFC 9003 and plan.md §Structure.

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufRead, AsyncBufReadExt};

/// Reusable bounded JSON-lines reader.
///
/// Buffers fragmented stream reads, enforces a strict upper bound on frame size,
/// rejects incomplete frames on premature stream EOF, and preserves internal buffer
/// state across cancellation points.
pub struct JsonLineReader<R> {
    reader: R,
    buf: Vec<u8>,
    max_line_length: usize,
}

impl<R> JsonLineReader<R> {
    /// Creates a new `JsonLineReader` with the given maximum line length in bytes.
    pub fn new(reader: R, max_line_length: usize) -> Self {
        Self {
            reader,
            buf: Vec::new(),
            max_line_length,
        }
    }

    /// Returns a reference to the underlying reader.
    pub fn reader(&self) -> &R {
        &self.reader
    }

    /// Returns a mutable reference to the underlying reader.
    pub fn reader_mut(&mut self) -> &mut R {
        &mut self.reader
    }

    /// Consumes this reader, returning the wrapped inner reader.
    pub fn into_inner(self) -> R {
        self.reader
    }
}

impl<R: AsyncBufRead + Unpin> JsonLineReader<R> {
    /// Reads the next bounded line from the stream.
    ///
    /// Returns:
    /// - `Ok(Some(line))` on successful read of a complete line (stripped of `\r` and `\n`).
    /// - `Ok(None)` on clean EOF when no bytes have been read for the current frame.
    /// - `Err` if the line exceeds `max_line_length` or if the stream terminates with an
    ///   incomplete frame before a newline delimiter.
    pub async fn read_line_raw(&mut self) -> Result<Option<Vec<u8>>> {
        loop {
            let available = self
                .reader
                .fill_buf()
                .await
                .context("Failed to fill buffer from input stream")?;

            if available.is_empty() {
                if self.buf.is_empty() {
                    return Ok(None);
                } else {
                    bail!(
                        "Unexpected EOF: stream terminated with incomplete frame ({} bytes unconsumed)",
                        self.buf.len()
                    );
                }
            }

            if let Some(pos) = available.iter().position(|&b| b == b'\n') {
                let chunk = &available[..pos];
                let total_len = self.buf.len() + chunk.len();
                if total_len > self.max_line_length {
                    bail!(
                        "Line length ({} bytes) exceeds maximum limit of {} bytes",
                        total_len,
                        self.max_line_length
                    );
                }
                self.buf.extend_from_slice(chunk);
                self.reader.consume(pos + 1);

                // Strip trailing \r if present
                if self.buf.last() == Some(&b'\r') {
                    self.buf.pop();
                }

                let line = std::mem::take(&mut self.buf);
                return Ok(Some(line));
            } else {
                let total_len = self.buf.len() + available.len();
                if total_len > self.max_line_length {
                    bail!(
                        "Line length (exceeding {} bytes) exceeds maximum limit of {} bytes",
                        total_len,
                        self.max_line_length
                    );
                }
                self.buf.extend_from_slice(available);
                let len = available.len();
                self.reader.consume(len);
            }
        }
    }
}

/// Reads a single bounded line from an async buffered reader up to `max` bytes.
///
/// Returns `Ok(Some(bytes))` on complete line, `Ok(None)` on clean EOF, or `Err` on
/// frame length overflow or trailing incomplete frame at EOF.
pub async fn read_bounded_line<R: AsyncBufRead + Unpin>(
    reader: &mut R,
    max: usize,
) -> Result<Option<Vec<u8>>> {
    let mut line_reader = JsonLineReader::new(reader, max);
    line_reader.read_line_raw().await
}

/// High-level parsed QMP wire message.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum QmpMessage {
    Greeting {
        #[serde(rename = "QMP")]
        qmp: QmpGreetingData,
    },
    Event {
        event: String,
        #[serde(default)]
        data: Option<serde_json::Value>,
        #[serde(default)]
        timestamp: Option<serde_json::Value>,
    },
    Return {
        #[serde(rename = "return")]
        return_value: serde_json::Value,
        #[serde(default)]
        id: Option<String>,
    },
    Error {
        error: QmpErrorData,
        #[serde(default)]
        id: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QmpGreetingData {
    pub version: serde_json::Value,
    #[serde(default)]
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QmpErrorData {
    pub class: String,
    pub desc: String,
}

/// Encodes a QMP command into line-delimited JSON bytes.
pub fn encode_command(
    execute: &str,
    arguments: Option<serde_json::Value>,
    id: Option<&str>,
) -> Result<Vec<u8>> {
    let mut obj = serde_json::Map::new();
    obj.insert(
        "execute".to_string(),
        serde_json::Value::String(execute.to_string()),
    );
    if let Some(args) = arguments {
        obj.insert("arguments".to_string(), args);
    }
    if let Some(id_str) = id {
        obj.insert(
            "id".to_string(),
            serde_json::Value::String(id_str.to_string()),
        );
    }
    let mut json_bytes = serde_json::to_vec(&serde_json::Value::Object(obj))
        .context("Failed to serialize QMP command")?;
    json_bytes.push(b'\n');
    Ok(json_bytes)
}

/// Parses a single line of raw UTF-8 text into a QmpMessage.
pub fn parse_line(line: &str) -> Result<QmpMessage> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        bail!("Empty QMP line");
    }
    serde_json::from_str::<QmpMessage>(trimmed)
        .with_context(|| format!("Failed to parse QMP message from '{trimmed}'"))
}

/// Parses a raw byte slice into a QmpMessage.
pub fn parse_bytes(bytes: &[u8]) -> Result<QmpMessage> {
    let trimmed = match (
        bytes.iter().position(|&b| !b.is_ascii_whitespace()),
        bytes.iter().rposition(|&b| !b.is_ascii_whitespace()),
    ) {
        (Some(start), Some(end)) => &bytes[start..=end],
        _ => bail!("Empty QMP bytes"),
    };
    serde_json::from_slice::<QmpMessage>(trimmed)
        .with_context(|| "Failed to parse QMP message from byte slice")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocols::constants::MAX_JSONL_FRAME_SIZE;
    use std::io::Cursor;
    #[tokio::test]
    async fn test_bounded_line_reader_clean_lines() {
        let data = b"{\"hello\": 1}\n{\"world\": 2}\r\n";
        let cursor = Cursor::new(data);
        let mut reader = JsonLineReader::new(cursor, MAX_JSONL_FRAME_SIZE);

        let l1 = reader.read_line_raw().await.unwrap();
        assert_eq!(l1, Some(b"{\"hello\": 1}".to_vec()));

        let l2 = reader.read_line_raw().await.unwrap();
        assert_eq!(l2, Some(b"{\"world\": 2}".to_vec()));

        let l3 = reader.read_line_raw().await.unwrap();
        assert_eq!(l3, None);
    }

    #[tokio::test]
    async fn test_bounded_line_reader_incomplete_frame_fails() {
        let data = b"{\"incomplete\": true";
        let cursor = Cursor::new(data);
        let mut reader = JsonLineReader::new(cursor, MAX_JSONL_FRAME_SIZE);

        let res = reader.read_line_raw().await;
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("incomplete frame"));
    }

    #[tokio::test]
    async fn test_bounded_line_reader_oversize_fails() {
        let data = b"12345678901234567890\n";
        let cursor = Cursor::new(data);
        let mut reader = JsonLineReader::new(cursor, 10);

        let res = reader.read_line_raw().await;
        assert!(res.is_err());
        assert!(
            res.unwrap_err()
                .to_string()
                .contains("exceeds maximum limit")
        );
    }

    #[tokio::test]
    async fn test_free_fn_read_bounded_line() {
        let data = b"test line\n";
        let mut cursor = Cursor::new(data);
        let line = read_bounded_line(&mut cursor, 64).await.unwrap();
        assert_eq!(line, Some(b"test line".to_vec()));
    }

    #[test]
    fn test_parse_greeting() {
        let raw = r#"{"QMP": {"version": {"qemu": {"micro": 0, "minor": 2, "major": 8}, "package": ""}, "capabilities": []}}"#;
        let msg = parse_line(raw).unwrap();
        match msg {
            QmpMessage::Greeting { qmp } => {
                assert!(qmp.capabilities.is_empty());
            }
            _ => panic!("Expected Greeting"),
        }
    }

    #[test]
    fn test_parse_return_with_id() {
        let raw = r#"{"return": {"running": true, "singlestep": false, "status": "running"}, "id": "req-1"}"#;
        let msg = parse_line(raw).unwrap();
        match msg {
            QmpMessage::Return { return_value, id } => {
                assert_eq!(return_value["running"], true);
                assert_eq!(return_value["status"], "running");
                assert_eq!(id, Some("req-1".to_string()));
            }
            _ => panic!("Expected Return"),
        }
    }

    #[test]
    fn test_parse_error_with_id() {
        let raw = r#"{"error": {"class": "CommandNotFound", "desc": "The command has not been found"}, "id": "req-2"}"#;
        let msg = parse_line(raw).unwrap();
        match msg {
            QmpMessage::Error { error, id } => {
                assert_eq!(error.class, "CommandNotFound");
                assert_eq!(id, Some("req-2".to_string()));
            }
            _ => panic!("Expected Error"),
        }
    }

    #[test]
    fn test_parse_event() {
        let raw = r#"{"event": "SHUTDOWN", "data": {"guest": true, "reason": "guest-shutdown"}, "timestamp": {"seconds": 1690000000, "microseconds": 0}}"#;
        let msg = parse_line(raw).unwrap();
        match msg {
            QmpMessage::Event { event, .. } => {
                assert_eq!(event, "SHUTDOWN");
            }
            _ => panic!("Expected Event"),
        }
    }
}
