//! GDB Remote Serial Protocol (RSP) packet framing, checksum calculation, and escaping.
//!
//! Defined in accordance with RFC 9003 and plan.md §Structure.

use crate::protocols::constants::{
    GDB_ACK, GDB_ESCAPE_CHAR, GDB_ESCAPE_XOR, GDB_NAK, GDB_PACKET_END, GDB_PACKET_START,
};
use anyhow::{Result, bail};

/// Calculates the GDB RSP checksum (sum of bytes modulo 256).
pub fn calculate_checksum(data: &[u8]) -> u8 {
    let mut sum: u8 = 0;
    for &b in data {
        sum = sum.wrapping_add(b);
    }
    sum
}

/// Escapes special RSP characters (`#`, `$`, `*`, `}`) in packet data.
pub fn escape_payload(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len());
    for &b in data {
        match b {
            b'#' | b'$' | b'*' | GDB_ESCAPE_CHAR => {
                out.push(GDB_ESCAPE_CHAR);
                out.push(b ^ GDB_ESCAPE_XOR);
            }
            other => out.push(other),
        }
    }
    out
}

/// Unescapes an escaped RSP byte slice.
pub fn unescape_payload(data: &[u8]) -> Result<Vec<u8>> {
    let mut out = Vec::with_capacity(data.len());
    let mut i = 0;
    while i < data.len() {
        let b = data[i];
        if b == GDB_ESCAPE_CHAR {
            i += 1;
            if i >= data.len() {
                bail!("Incomplete escape sequence at end of RSP packet");
            }
            out.push(data[i] ^ GDB_ESCAPE_XOR);
        } else {
            out.push(b);
        }
        i += 1;
    }
    Ok(out)
}

/// Frames a raw payload into a standard GDB RSP packet: `$payload#xx`.
///
/// Automatically escapes special characters in payload before computing checksum.
pub fn frame_packet(payload: &[u8]) -> Vec<u8> {
    let escaped = escape_payload(payload);
    let checksum = calculate_checksum(&escaped);
    let mut framed = Vec::with_capacity(escaped.len() + 4);
    framed.push(GDB_PACKET_START);
    framed.extend_from_slice(&escaped);
    framed.push(GDB_PACKET_END);
    framed.extend_from_slice(&encode_hex_byte(checksum));
    framed
}

/// Parses and verifies a framed packet: `$payload#xx`. Returns the unescaped payload.
pub fn unframe_packet(packet: &[u8]) -> Result<Vec<u8>> {
    if packet.len() < 4 {
        bail!(
            "Packet too short ({} bytes) to be valid GDB RSP framing",
            packet.len()
        );
    }
    if packet[0] != GDB_PACKET_START {
        bail!("Packet does not start with '$', found 0x{:02x}", packet[0]);
    }

    let hash_idx = packet
        .iter()
        .rposition(|&b| b == GDB_PACKET_END)
        .ok_or_else(|| anyhow::anyhow!("Packet missing checksum separator '#'"))?;

    let wire_payload = &packet[1..hash_idx];
    let checksum_slice = &packet[hash_idx + 1..];
    if checksum_slice.len() != 2 {
        bail!(
            "Invalid checksum length ({} bytes), expected 2 hex digits",
            checksum_slice.len()
        );
    }

    let expected_checksum = decode_hex_byte(checksum_slice[0], checksum_slice[1])?;
    let actual_checksum = calculate_checksum(wire_payload);

    if actual_checksum != expected_checksum {
        bail!(
            "Checksum mismatch: expected 0x{expected_checksum:02x}, calculated 0x{actual_checksum:02x}"
        );
    }

    unescape_payload(wire_payload)
}

/// Converts a single ASCII hex character into its 4-bit numeric value.
///
/// Returns an error if the character is not a valid hexadecimal digit.
/// Never panics on non-ASCII input.
#[inline]
pub fn hex_nibble(b: u8) -> Result<u8> {
    match b {
        b'0'..=b'9' => Ok(b - b'0'),
        b'a'..=b'f' => Ok(b - b'a' + 10),
        b'A'..=b'F' => Ok(b - b'A' + 10),
        _ => bail!("Invalid hex character: 0x{b:02x} ('{}')", b.escape_ascii()),
    }
}

/// Decodes two hex ASCII bytes into a single u8 byte.
#[inline]
pub fn decode_hex_byte(hi: u8, lo: u8) -> Result<u8> {
    let h = hex_nibble(hi)?;
    let l = hex_nibble(lo)?;
    Ok((h << 4) | l)
}

/// Encodes a single u8 byte into two lowercase ASCII hex bytes.
#[inline]
pub fn encode_hex_byte(b: u8) -> [u8; 2] {
    const HEX_CHARS: &[u8; 16] = b"0123456789abcdef";
    [HEX_CHARS[(b >> 4) as usize], HEX_CHARS[(b & 0x0f) as usize]]
}

/// Decodes an even-length slice of ASCII hex bytes into binary bytes.
///
/// Avoids UTF-8 string conversions and string slicing to guarantee panic-free parsing.
pub fn decode_hex_bytes(bytes: &[u8]) -> Result<Vec<u8>> {
    if (bytes.len() & 1) != 0 {
        bail!("Hex byte sequence length must be even, got {}", bytes.len());
    }
    let mut out = Vec::with_capacity(bytes.len() / 2);
    let mut i = 0;
    while i < bytes.len() {
        out.push(decode_hex_byte(bytes[i], bytes[i + 1])?);
        i += 2;
    }
    Ok(out)
}

/// Encodes binary bytes into an ASCII hex byte vector.
pub fn encode_hex_bytes(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes.len() * 2);
    for &b in bytes {
        let hex = encode_hex_byte(b);
        out.push(hex[0]);
        out.push(hex[1]);
    }
    out
}

/// Parses an RSP error code from a response payload (`E..`).
///
/// For example, `E01` or `E22` yields `Some(code)`.
pub fn parse_error_reply(payload: &[u8]) -> Option<u8> {
    if payload.len() >= 3 && payload[0] == b'E' {
        decode_hex_byte(payload[1], payload[2]).ok()
    } else {
        None
    }
}

/// Represents a parsed GDB RSP target stop reply packet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GdbStopReply {
    /// Signal stop with target status details (`T<sig><details>`).
    WatchpointOrSignal {
        signal: u8,
        details: Vec<(String, String)>,
    },
    /// Simple signal stop (`S<sig>`).
    Signal(u8),
    /// Process normal exit (`W<status>`).
    Exited(u32),
    /// Process terminated by signal (`X<sig>`).
    Terminated(u8),
    /// Other stop reply format.
    Other(Vec<u8>),
}

impl GdbStopReply {
    /// Returns `true` if this reply indicates the target is halted by signal or watchpoint.
    pub fn is_stopped(&self) -> bool {
        matches!(self, Self::WatchpointOrSignal { .. } | Self::Signal(_))
    }

    /// Returns the signal number if the target was halted by a signal.
    pub fn signal(&self) -> Option<u8> {
        match self {
            Self::WatchpointOrSignal { signal, .. } => Some(*signal),
            Self::Signal(sig) => Some(*sig),
            Self::Terminated(sig) => Some(*sig),
            _ => None,
        }
    }
}

/// Parses a raw RSP payload into a typed `GdbStopReply`.
pub fn parse_stop_reply(payload: &[u8]) -> Result<GdbStopReply> {
    if payload.is_empty() {
        bail!("Empty GDB stop reply payload");
    }

    match payload[0] {
        b'S' => {
            if payload.len() < 3 {
                bail!(
                    "Truncated 'S' stop reply: {}",
                    String::from_utf8_lossy(payload)
                );
            }
            let sig = decode_hex_byte(payload[1], payload[2])?;
            Ok(GdbStopReply::Signal(sig))
        }
        b'T' => {
            if payload.len() < 3 {
                bail!(
                    "Truncated 'T' stop reply: {}",
                    String::from_utf8_lossy(payload)
                );
            }
            let sig = decode_hex_byte(payload[1], payload[2])?;
            let mut details = Vec::new();

            if payload.len() > 3 {
                let text = std::str::from_utf8(&payload[3..]).unwrap_or("");
                for item in text.split(';') {
                    let trimmed = item.trim();
                    if trimmed.is_empty() {
                        continue;
                    }
                    if let Some((k, v)) = trimmed.split_once(':') {
                        details.push((k.to_string(), v.to_string()));
                    }
                }
            }
            Ok(GdbStopReply::WatchpointOrSignal {
                signal: sig,
                details,
            })
        }
        b'W' => {
            let s = std::str::from_utf8(&payload[1..]).unwrap_or("0");
            let code = u32::from_str_radix(s.trim(), 16).unwrap_or(0);
            Ok(GdbStopReply::Exited(code))
        }
        b'X' => {
            if payload.len() >= 3 {
                let sig = decode_hex_byte(payload[1], payload[2])?;
                Ok(GdbStopReply::Terminated(sig))
            } else {
                Ok(GdbStopReply::Terminated(0))
            }
        }
        _ => Ok(GdbStopReply::Other(payload.to_vec())),
    }
}

/// Validates whether a response is positive acknowledgment (`+`) or negative (`-`).
pub fn is_ack(b: u8) -> bool {
    b == GDB_ACK
}

pub fn is_nak(b: u8) -> bool {
    b == GDB_NAK
}
