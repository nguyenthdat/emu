//! Asynchronous GDB Remote Serial Protocol (RSP) client.
//!
//! Defined in accordance with RFC 9003, plan.md §Structure, and contracts/cli.md §9.

use super::packet::{
    GdbStopReply, decode_hex_bytes, encode_hex_bytes, frame_packet, is_ack, is_nak,
    parse_error_reply, parse_stop_reply, unframe_packet,
};
use super::registers::{Arm64Registers, decode_register_value, encode_register_value};
use crate::protocols::constants::{
    DEFAULT_GDB_TIMEOUT, GDB_ACK, GDB_BREAK_BYTE, GDB_MAX_NAK_RETRIES, GDB_PACKET_END,
    GDB_PACKET_START, MAX_GDB_PACKET_SIZE,
};
use anyhow::{Context, Result, bail};
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

/// Asynchronous GDB RSP client.
///
/// Features:
/// - Bounded byte-level packet frames and deadlines
/// - Strict ASCII/byte hex parsing without UTF-8 string slicing panics
/// - ACK/NAK state machine with bounded same-frame retransmission upon NAK
/// - Resilient stop reply consumption on `halt` and `step`
/// - Rapid `cont` returning immediately upon ACK acknowledgment
/// - Indexed register reading (`p`) and writing (`P`) preserving other registers
/// - Memory reading (`m`) and writing (`M`)
/// - Software (`Z0`/`z0`) and hardware (`Z1`/`z1`) breakpoints with `E..` error validation
/// - Invariant: Zero packets (no `D`, `c`, or `k`) sent on drop / observer disconnect
pub struct GdbClient<R, W> {
    reader: R,
    writer: W,
    timeout: Duration,
    max_nak_retries: usize,
    no_ack_mode: bool,
}

impl<R: AsyncRead + Unpin + Send, W: AsyncWrite + Unpin + Send> GdbClient<R, W> {
    /// Creates a new GDB RSP client over asynchronous reader and writer streams.
    pub fn new(reader: R, writer: W) -> Self {
        Self {
            reader,
            writer,
            timeout: DEFAULT_GDB_TIMEOUT,
            max_nak_retries: GDB_MAX_NAK_RETRIES,
            no_ack_mode: false,
        }
    }

    /// Configures the I/O timeout duration for packet exchanges and acknowledgments.
    pub fn set_timeout(&mut self, timeout: Duration) {
        self.timeout = timeout;
    }

    /// Sets whether the remote gdbstub operates in no-ack mode (`QStartNoAckMode`).
    pub fn set_no_ack_mode(&mut self, no_ack: bool) {
        self.no_ack_mode = no_ack;
    }

    /// Sends an RSP packet payload, awaits remote ACK (`+`), and reads the unescaped response payload.
    ///
    /// If remote replies with NAK (`-`), retransmits the same framed byte sequence up to
    /// `max_nak_retries` without re-executing or re-generating the underlying command.
    pub async fn send_packet(&mut self, payload: &[u8]) -> Result<Vec<u8>> {
        let framed = frame_packet(payload);

        let mut initial_response_byte: Option<u8> = None;
        if !self.no_ack_mode {
            let mut acked = false;
            for attempt in 0..=self.max_nak_retries {
                self.writer
                    .write_all(&framed)
                    .await
                    .context("Failed to write GDB RSP packet to transport")?;
                self.writer
                    .flush()
                    .await
                    .context("Failed to flush GDB writer")?;

                let mut ack_buf = [0u8; 1];
                let read_res =
                    tokio::time::timeout(self.timeout, self.reader.read_exact(&mut ack_buf))
                        .await
                        .context("Timed out awaiting GDB RSP ACK/NAK")?;

                read_res.context("Failed to read GDB RSP ACK byte")?;

                let b = ack_buf[0];
                if is_ack(b) {
                    acked = true;
                    break;
                } else if is_nak(b) {
                    if attempt == self.max_nak_retries {
                        bail!(
                            "GDB remote returned NAK (-) after {} retransmission attempts",
                            self.max_nak_retries
                        );
                    }
                    continue;
                } else if b == GDB_PACKET_START {
                    // Some stubs omit ACK and send response packet starting immediately with '$'
                    initial_response_byte = Some(b);
                    acked = true;
                    break;
                }
            }

            if !acked {
                bail!("Failed to obtain positive acknowledgment (+) from GDB remote");
            }
        } else {
            self.writer.write_all(&framed).await?;
            self.writer.flush().await?;
        }

        self.read_response_packet(initial_response_byte).await
    }

    /// Reads a framed `$payload#xx` response packet from the stream.
    async fn read_response_packet(&mut self, initial_byte: Option<u8>) -> Result<Vec<u8>> {
        let mut resp_buf = Vec::with_capacity(128);
        let mut byte_buf = [0u8; 1];
        let mut in_packet = false;
        let mut checksum_remaining = 0;

        if let Some(b) = initial_byte {
            if b == GDB_PACKET_START {
                in_packet = true;
                resp_buf.push(b);
            }
        }

        loop {
            let read_res = tokio::time::timeout(self.timeout, self.reader.read(&mut byte_buf))
                .await
                .context("Timed out reading GDB RSP response packet")?;

            let n = read_res.context("Failed to read byte from GDB stream")?;
            if n == 0 {
                bail!("GDB stream closed unexpectedly while awaiting response packet");
            }

            let b = byte_buf[0];

            if !in_packet {
                if b == GDB_PACKET_START {
                    in_packet = true;
                    resp_buf.push(b);
                }
                // Discard any leading ACKs or garbage before '$'
            } else {
                if resp_buf.len() >= MAX_GDB_PACKET_SIZE {
                    bail!(
                        "GDB RSP response packet exceeded maximum size limit of {MAX_GDB_PACKET_SIZE} bytes"
                    );
                }
                resp_buf.push(b);

                if b == GDB_PACKET_END {
                    checksum_remaining = 2;
                } else if checksum_remaining > 0 {
                    checksum_remaining -= 1;
                    if checksum_remaining == 0 {
                        break;
                    }
                }
            }
        }

        // Send positive ACK back to remote
        if !self.no_ack_mode {
            self.writer
                .write_all(&[GDB_ACK])
                .await
                .context("Failed to write ACK response to GDB remote")?;
            self.writer
                .flush()
                .await
                .context("Failed to flush ACK byte")?;
        }

        unframe_packet(&resp_buf)
    }

    /// Halts virtual CPU execution by sending out-of-band break (0x03) and drains stop reply.
    ///
    /// Drains and parses the real target stop reply packet (e.g. `T02...` or `S02`).
    pub async fn halt(&mut self) -> Result<GdbStopReply> {
        self.writer
            .write_all(&[GDB_BREAK_BYTE])
            .await
            .context("Failed to send GDB break byte (0x03)")?;
        self.writer
            .flush()
            .await
            .context("Failed to flush GDB break byte")?;

        let resp = self.read_response_packet(None).await?;
        parse_stop_reply(&resp)
    }

    /// Resumes virtual CPU execution (`c`).
    ///
    /// Invariant: Returns immediately upon positive acknowledgment (`+`) without
    /// blocking waiting for a future stop reply, since execution may run indefinitely.
    pub async fn cont(&mut self) -> Result<()> {
        let framed = frame_packet(b"c");
        self.writer
            .write_all(&framed)
            .await
            .context("Failed to send GDB continue command 'c'")?;
        self.writer
            .flush()
            .await
            .context("Failed to flush continue command")?;

        if !self.no_ack_mode {
            let mut ack_buf = [0u8; 1];
            let read_res = tokio::time::timeout(self.timeout, self.reader.read_exact(&mut ack_buf))
                .await
                .context("Timed out awaiting ACK for continue ('c')")?;

            read_res.context("Failed to read ACK for continue")?;
            if !is_ack(ack_buf[0]) {
                bail!(
                    "GDB remote rejected continue command with 0x{:02x}",
                    ack_buf[0]
                );
            }
        }
        Ok(())
    }

    /// Single-steps virtual CPU instruction execution (`s`), consuming the real stop reply.
    pub async fn step(&mut self) -> Result<GdbStopReply> {
        let resp = self.send_packet(b"s").await?;
        parse_stop_reply(&resp)
    }

    /// Reads all general purpose ARM64 registers ('g').
    pub async fn read_registers(&mut self) -> Result<Arm64Registers> {
        let resp = self.send_packet(b"g").await?;
        if let Some(err) = parse_error_reply(&resp) {
            bail!("GDB remote returned error E{err:02x} reading ARM64 registers");
        }
        Arm64Registers::from_gdb_hex_bytes(&resp)
    }

    /// Writes all general purpose ARM64 registers ('G').
    pub async fn write_registers(&mut self, regs: &Arm64Registers) -> Result<()> {
        let hex_bytes = regs.to_gdb_hex_bytes();
        let mut cmd = Vec::with_capacity(1 + hex_bytes.len());
        cmd.push(b'G');
        cmd.extend_from_slice(&hex_bytes);

        let resp = self.send_packet(&cmd).await?;
        check_ok_or_error(&resp, "write_registers ('G')")
    }

    /// Reads a single ARM64 register by architectural index ('p<reg:x>').
    pub async fn read_register(&mut self, reg_num: u32) -> Result<u64> {
        let cmd = format!("p{reg_num:x}");
        let resp = self.send_packet(cmd.as_bytes()).await?;
        if let Some(err) = parse_error_reply(&resp) {
            bail!("GDB remote returned error E{err:02x} reading register {reg_num}");
        }
        decode_register_value(&resp)
    }

    /// Writes a single ARM64 register by architectural index ('P<reg:x>=<val:x>').
    ///
    /// Preferred over 'G' writes to avoid overwriting uninspected registers.
    pub async fn write_register(&mut self, reg_num: u32, value: u64) -> Result<()> {
        let val_hex = encode_register_value(value);
        let cmd = format!("P{reg_num:x}={val_hex}");
        let resp = self.send_packet(cmd.as_bytes()).await?;
        check_ok_or_error(&resp, &format!("write_register(index={reg_num})"))
    }

    /// Reads memory from target virtual address ('m<addr:x>,<len:x>').
    pub async fn read_memory(&mut self, addr: u64, len: usize) -> Result<Vec<u8>> {
        let cmd = format!("m{addr:x},{len:x}");
        let resp = self.send_packet(cmd.as_bytes()).await?;
        if let Some(err) = parse_error_reply(&resp) {
            bail!("GDB remote returned error E{err:02x} reading memory at 0x{addr:x}");
        }
        decode_hex_bytes(&resp)
    }

    /// Writes memory to target virtual address ('M<addr:x>,<len:x>:<hex>').
    pub async fn write_memory(&mut self, addr: u64, data: &[u8]) -> Result<()> {
        let hex_data = encode_hex_bytes(data);
        let hex_str = String::from_utf8(hex_data).unwrap_or_default();
        let cmd = format!("M{addr:x},{:x}:{hex_str}", data.len());
        let resp = self.send_packet(cmd.as_bytes()).await?;
        check_ok_or_error(&resp, &format!("write_memory(addr=0x{addr:x})"))
    }

    /// Sets a software breakpoint at the given address ('Z0,<addr:x>,4').
    pub async fn set_breakpoint(&mut self, addr: u64) -> Result<()> {
        let cmd = format!("Z0,{addr:x},4");
        let resp = self.send_packet(cmd.as_bytes()).await?;
        check_ok_or_error(&resp, &format!("set_breakpoint(addr=0x{addr:x})"))
    }

    /// Removes a software breakpoint at the given address ('z0,<addr:x>,4').
    pub async fn remove_breakpoint(&mut self, addr: u64) -> Result<()> {
        let cmd = format!("z0,{addr:x},4");
        let resp = self.send_packet(cmd.as_bytes()).await?;
        check_ok_or_error(&resp, &format!("remove_breakpoint(addr=0x{addr:x})"))
    }

    /// Sets a hardware breakpoint at the given address ('Z1,<addr:x>,4').
    pub async fn set_hardware_breakpoint(&mut self, addr: u64) -> Result<()> {
        let cmd = format!("Z1,{addr:x},4");
        let resp = self.send_packet(cmd.as_bytes()).await?;
        check_ok_or_error(&resp, &format!("set_hardware_breakpoint(addr=0x{addr:x})"))
    }

    /// Removes a hardware breakpoint at the given address ('z1,<addr:x>,4').
    pub async fn remove_hardware_breakpoint(&mut self, addr: u64) -> Result<()> {
        let cmd = format!("z1,{addr:x},4");
        let resp = self.send_packet(cmd.as_bytes()).await?;
        check_ok_or_error(
            &resp,
            &format!("remove_hardware_breakpoint(addr=0x{addr:x})"),
        )
    }
}

/// Validates whether a response payload is `OK` or a recognized error (`E..`).
fn check_ok_or_error(resp: &[u8], op_name: &str) -> Result<()> {
    if resp == b"OK" {
        Ok(())
    } else if let Some(code) = parse_error_reply(resp) {
        bail!("GDB remote returned error E{code:02x} during {op_name}")
    } else {
        bail!(
            "GDB remote returned unexpected response '{}' during {op_name}",
            String::from_utf8_lossy(resp)
        )
    }
}
