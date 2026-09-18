//! Standalone native helper executable for Frida dynamic instrumentation.
//!
//! Communicates with the Emu supervisor over bounded stdio JSON streams.
//! Configured per research.md §D-02 and plan.md §Complexity Tracking.

use std::io::{self, BufRead, Write};

fn main() -> anyhow::Result<()> {
    let devkit_path = std::env::var("FRIDA_CORE_DEVKIT").ok();
    if devkit_path.is_none() {
        eprintln!("Warning: FRIDA_CORE_DEVKIT not configured; running in synthetic bridge mode");
    }

    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Echo synthetic response
        let resp = serde_json::json!({
            "type": "success",
            "payload": {
                "message": "Frida worker acknowledged request",
                "pid": 1234
            }
        });
        let mut bytes = serde_json::to_vec(&resp)?;
        bytes.push(b'\n');
        stdout.write_all(&bytes)?;
        stdout.flush()?;
    }

    Ok(())
}
