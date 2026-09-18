//! ARM64 register set mapping and serialization for GDB RSP.
//!
//! Defined in accordance with Apple Silicon arm64 register architecture (RFC 9003, plan.md §Structure).

use super::packet::{decode_hex_bytes, encode_hex_bytes};
use crate::protocols::constants::{ARM64_GDB_REGISTER_BYTES, ARM64_GP_REGISTER_COUNT};
use anyhow::{Context, Result, bail};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Arm64Registers {
    pub x: [u64; ARM64_GP_REGISTER_COUNT],
    pub sp: u64,
    pub pc: u64,
    pub pstate: u32,
}

impl Arm64Registers {
    /// Parses ARM64 register state from raw GDB RSP hex payload bytes ('g' packet response).
    ///
    /// Avoids UTF-8 string slicing to guarantee panic-free parsing on arbitrary byte streams.
    pub fn from_gdb_hex_bytes(hex_bytes: &[u8]) -> Result<Self> {
        let clean_bytes = match (
            hex_bytes.iter().position(|&b| !b.is_ascii_whitespace()),
            hex_bytes.iter().rposition(|&b| !b.is_ascii_whitespace()),
        ) {
            (Some(s), Some(e)) => &hex_bytes[s..=e],
            _ => bail!("Empty register hex payload"),
        };

        let bytes = decode_hex_bytes(clean_bytes)
            .context("Failed to decode hexadecimal register buffer")?;

        if bytes.len() < ARM64_GDB_REGISTER_BYTES {
            bail!(
                "Register payload too short for ARM64: got {} bytes, expected >= {}",
                bytes.len(),
                ARM64_GDB_REGISTER_BYTES
            );
        }

        let mut x = [0u64; ARM64_GP_REGISTER_COUNT];
        for (i, reg) in x.iter_mut().enumerate() {
            let offset = i * 8;
            *reg = u64::from_le_bytes(
                bytes[offset..offset + 8]
                    .try_into()
                    .context("Invalid register byte slice")?,
            );
        }

        let sp = u64::from_le_bytes(
            bytes[248..256]
                .try_into()
                .context("Invalid SP byte slice")?,
        );
        let pc = u64::from_le_bytes(
            bytes[256..264]
                .try_into()
                .context("Invalid PC byte slice")?,
        );
        let pstate = u32::from_le_bytes(
            bytes[264..268]
                .try_into()
                .context("Invalid PSTATE byte slice")?,
        );

        Ok(Self { x, sp, pc, pstate })
    }

    /// Parses ARM64 register state from GDB RSP hex string ('g' packet response).
    pub fn from_gdb_hex(hex_str: &str) -> Result<Self> {
        Self::from_gdb_hex_bytes(hex_str.as_bytes())
    }

    /// Serializes registers to raw GDB RSP hex bytes for 'G' packet.
    pub fn to_gdb_hex_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(ARM64_GDB_REGISTER_BYTES);
        for reg in &self.x {
            bytes.extend_from_slice(&reg.to_le_bytes());
        }
        bytes.extend_from_slice(&self.sp.to_le_bytes());
        bytes.extend_from_slice(&self.pc.to_le_bytes());
        bytes.extend_from_slice(&self.pstate.to_le_bytes());
        encode_hex_bytes(&bytes)
    }

    /// Serializes registers to GDB RSP hex string for 'G' packet.
    pub fn to_gdb_hex(&self) -> String {
        String::from_utf8(self.to_gdb_hex_bytes()).unwrap_or_default()
    }

    /// Retrieves register value by ARM64 architectural index:
    /// - 0..=30: x0..x30
    /// - 31: sp
    /// - 32: pc
    /// - 33: pstate (as u64)
    pub fn get_register(&self, idx: usize) -> Option<u64> {
        match idx {
            0..=30 => Some(self.x[idx]),
            31 => Some(self.sp),
            32 => Some(self.pc),
            33 => Some(self.pstate as u64),
            _ => None,
        }
    }

    /// Updates register value by ARM64 architectural index.
    pub fn set_register(&mut self, idx: usize, val: u64) -> Result<()> {
        match idx {
            0..=30 => {
                self.x[idx] = val;
                Ok(())
            }
            31 => {
                self.sp = val;
                Ok(())
            }
            32 => {
                self.pc = val;
                Ok(())
            }
            33 => {
                self.pstate = val as u32;
                Ok(())
            }
            _ => bail!("Register index {idx} out of range for ARM64 (max 33)"),
        }
    }

    /// Converts registers to a human-readable map of register names to hex values.
    pub fn to_map(&self) -> BTreeMap<String, String> {
        let mut map = BTreeMap::new();
        for (i, val) in self.x.iter().enumerate() {
            map.insert(format!("x{i}"), format!("0x{val:016x}"));
        }
        map.insert("sp".to_string(), format!("0x{:016x}", self.sp));
        map.insert("pc".to_string(), format!("0x{:016x}", self.pc));
        map.insert("pstate".to_string(), format!("0x{:08x}", self.pstate));
        map
    }
}

/// Encodes an 8-byte 64-bit value into a 16-character little-endian ASCII hex string for RSP `P` packet.
pub fn encode_register_value(val: u64) -> String {
    let bytes = val.to_le_bytes();
    let hex_bytes = encode_hex_bytes(&bytes);
    String::from_utf8(hex_bytes).unwrap_or_default()
}

/// Decodes an 8-byte little-endian hex byte slice from RSP `p` packet response into a u64.
pub fn decode_register_value(hex_bytes: &[u8]) -> Result<u64> {
    let raw_bytes = decode_hex_bytes(hex_bytes).context("Invalid register value hex string")?;
    if raw_bytes.len() == 8 {
        Ok(u64::from_le_bytes(raw_bytes.try_into().unwrap()))
    } else if raw_bytes.len() == 4 {
        Ok(u32::from_le_bytes(raw_bytes.try_into().unwrap()) as u64)
    } else {
        bail!(
            "Unexpected register value byte length {}, expected 8 or 4 bytes",
            raw_bytes.len()
        )
    }
}
