//! Deterministic integration tests for QMP and GDB RSP protocol engines,
//! stream codecs, continuous background readers, and KernelDebugLease exclusion.
//!
//! Defined in accordance with RFC 9003, contracts/cli.md §9, and plan.md §Structure.

use emu::models::research::{DebugLeaseState, ResearchGuestId};
use emu::protocols::constants::MAX_JSONL_FRAME_SIZE;
use emu::protocols::gdb::client::GdbClient;
use emu::protocols::gdb::lease::KernelDebugLeaseRegistry;
use emu::protocols::gdb::packet::{
    decode_hex_bytes, escape_payload, frame_packet, unescape_payload, unframe_packet,
};
use emu::protocols::gdb::registers::Arm64Registers;
use emu::protocols::qmp::client::QmpClient;
use emu::protocols::qmp::codec::{JsonLineReader, read_bounded_line};
use emu::protocols::qmp::events::QmpLifecycleEvent;
use std::io::Cursor;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

// ============================================================================
// QMP Codec & Stream Framing Tests
// ============================================================================

#[tokio::test]
async fn test_qmp_bounded_line_reader_fragmented_delivery() {
    let (mut tx, rx) = tokio::io::duplex(1024);
    let mut reader = JsonLineReader::new(tokio::io::BufReader::new(rx), MAX_JSONL_FRAME_SIZE);

    tokio::spawn(async move {
        // Fragmented frame chunks arriving over time
        tx.write_all(b"{\"Q").await.unwrap();
        tokio::time::sleep(Duration::from_millis(5)).await;
        tx.write_all(b"MP\": ").await.unwrap();
        tokio::time::sleep(Duration::from_millis(5)).await;
        tx.write_all(b"{\"version\": {}}}\n").await.unwrap();

        // Second complete frame
        tx.write_all(b"{\"event\": \"STOP\"}\r\n").await.unwrap();
    });

    let l1 = reader.read_line_raw().await.expect("Failed to read l1");
    assert_eq!(
        l1,
        Some(b"{\"QMP\": {\"version\": {}}}".to_vec()),
        "Fragmented chunks must assemble into complete frame"
    );

    let l2 = reader.read_line_raw().await.expect("Failed to read l2");
    assert_eq!(
        l2,
        Some(b"{\"event\": \"STOP\"}".to_vec()),
        "Trailing CRLF must be handled properly"
    );
}

#[tokio::test]
async fn test_qmp_bounded_line_reader_rejects_oversize_frame() {
    let (mut tx, rx) = tokio::io::duplex(2048);
    let mut reader = JsonLineReader::new(tokio::io::BufReader::new(rx), 32); // Restrict to 32 bytes max

    tokio::spawn(async move {
        // Send a frame longer than 32 bytes without newline
        tx.write_all(b"{\"long_key_exceeding_the_configured_buffer_capacity\": 12345}\n")
            .await
            .unwrap();
    });

    let res = reader.read_line_raw().await;
    assert!(res.is_err(), "Oversize frame must be rejected");
    assert!(
        res.unwrap_err().to_string().contains("exceeds maximum"),
        "Error message must specify size limit overflow"
    );
}

#[tokio::test]
async fn test_qmp_bounded_line_reader_rejects_incomplete_frame_at_eof() {
    let (mut tx, rx) = tokio::io::duplex(1024);
    let mut reader = JsonLineReader::new(tokio::io::BufReader::new(rx), MAX_JSONL_FRAME_SIZE);

    tokio::spawn(async move {
        tx.write_all(b"{\"incomplete\": true").await.unwrap();
        // Drop tx without sending newline
    });

    let res = reader.read_line_raw().await;
    assert!(res.is_err(), "Premature EOF must be rejected as an error");
    assert!(
        res.unwrap_err().to_string().contains("incomplete frame"),
        "Error message must indicate incomplete frame at EOF"
    );
}

#[tokio::test]
async fn test_qmp_free_fn_read_bounded_line() {
    let data = b"line 1\nline 2\n";
    let mut cursor = Cursor::new(data);

    let r1 = read_bounded_line(&mut cursor, 64).await.unwrap();
    assert_eq!(r1, Some(b"line 1".to_vec()));

    let r2 = read_bounded_line(&mut cursor, 64).await.unwrap();
    assert_eq!(r2, Some(b"line 2".to_vec()));

    let r3 = read_bounded_line(&mut cursor, 64).await.unwrap();
    assert_eq!(r3, None, "Clean EOF must return Ok(None)");
}

// ============================================================================
// QMP Client Peer Exchange, Correlation & Unsolicited Events
// ============================================================================

#[tokio::test]
async fn test_qmp_client_handshake_and_correlated_requests() {
    let (client_io, mut server_io) = tokio::io::duplex(4096);
    let (client_reader, client_writer) = tokio::io::split(client_io);

    // Simulated QEMU QMP Server
    let server_task = tokio::spawn(async move {
        // 1. Send initial QMP Greeting banner
        let greeting =
            b"{\"QMP\": {\"version\": {\"qemu\": {\"major\": 8, \"minor\": 2, \"micro\": 0}, \"package\": \"\"}, \"capabilities\": []}}\n";
        server_io.write_all(greeting).await.unwrap();

        // 2. Read qmp_capabilities command
        let mut buf = vec![0u8; 512];
        let n = server_io.read(&mut buf).await.unwrap();
        let cmd_str = std::str::from_utf8(&buf[..n]).unwrap();
        assert!(cmd_str.contains("\"execute\":\"qmp_capabilities\""));
        assert!(cmd_str.contains("\"id\":\"cap_init\""));

        // 3. Return capabilities success response
        server_io
            .write_all(b"{\"return\": {}, \"id\": \"cap_init\"}\n")
            .await
            .unwrap();

        // 4. Read query-status command
        let n = server_io.read(&mut buf).await.unwrap();
        let cmd_str = std::str::from_utf8(&buf[..n]).unwrap();
        assert!(cmd_str.contains("\"execute\":\"query-status\""));
        assert!(cmd_str.contains("\"id\":\"req-1\""));

        // 5. Return query-status response
        let status_resp =
            b"{\"return\": {\"running\": true, \"singlestep\": false, \"status\": \"running\"}, \"id\": \"req-1\"}\n";
        server_io.write_all(status_resp).await.unwrap();

        // 6. Read stop command
        let n = server_io.read(&mut buf).await.unwrap();
        let cmd_str = std::str::from_utf8(&buf[..n]).unwrap();
        assert!(cmd_str.contains("\"execute\":\"stop\""));
        assert!(cmd_str.contains("\"id\":\"req-2\""));

        // 7. Return stop response
        server_io
            .write_all(b"{\"return\": {}, \"id\": \"req-2\"}\n")
            .await
            .unwrap();
    });

    let mut client = QmpClient::new(client_reader, client_writer)
        .await
        .expect("QMP handshake failed");

    // Verify greeting was recorded
    assert!(client.greeting().is_some());

    // Execute query_status
    let status = client.query_status().await.expect("query_status failed");
    assert!(status.running);
    assert!(status.is_vcpu_running());
    assert!(!status.is_paused());
    assert_eq!(status.status, "running");

    // Execute stop
    client.stop().await.expect("stop command failed");

    server_task.await.unwrap();
}

#[tokio::test]
async fn test_qmp_client_unsolicited_events_while_idle() {
    let (client_io, mut server_io) = tokio::io::duplex(4096);
    let (client_reader, client_writer) = tokio::io::split(client_io);

    let server_task = tokio::spawn(async move {
        // Handshake
        server_io
            .write_all(b"{\"QMP\": {\"version\": {\"qemu\": {\"major\": 8, \"minor\": 2, \"micro\": 0}, \"package\": \"\"}, \"capabilities\": []}}\n")
            .await
            .unwrap();

        let mut buf = [0u8; 256];
        let _ = server_io.read(&mut buf).await.unwrap();
        server_io
            .write_all(b"{\"return\": {}, \"id\": \"cap_init\"}\n")
            .await
            .unwrap();

        // While client issues NO commands (is completely idle), server sends unsolicited events:
        tokio::time::sleep(Duration::from_millis(10)).await;
        server_io
            .write_all(b"{\"event\": \"STOP\"}\n")
            .await
            .unwrap();

        tokio::time::sleep(Duration::from_millis(10)).await;
        server_io
            .write_all(b"{\"event\": \"RESUME\"}\n")
            .await
            .unwrap();

        tokio::time::sleep(Duration::from_millis(10)).await;
        server_io
            .write_all(b"{\"event\": \"SHUTDOWN\", \"data\": {\"guest\": true, \"reason\": \"guest-shutdown\"}}\n")
            .await
            .unwrap();
    });

    let client = QmpClient::new(client_reader, client_writer)
        .await
        .expect("Handshake failed");

    let mut event_rx = client.subscribe_events();

    // Client receives events in order while completely idle
    let e1 = event_rx.recv().await.expect("Failed to receive event 1");
    assert_eq!(e1, QmpLifecycleEvent::Stop);
    assert!(e1.is_stop());

    let e2 = event_rx.recv().await.expect("Failed to receive event 2");
    assert_eq!(e2, QmpLifecycleEvent::Resume);
    assert!(e2.is_resume());

    let e3 = event_rx.recv().await.expect("Failed to receive event 3");
    match e3 {
        QmpLifecycleEvent::Shutdown { guest, reason } => {
            assert!(guest);
            assert_eq!(reason, Some("guest-shutdown".to_string()));
        }
        _ => panic!("Expected Shutdown event"),
    }

    server_task.await.unwrap();
}

#[tokio::test]
async fn test_qmp_command_timeout_never_replays_mutation() {
    let (client_io, mut server_io) = tokio::io::duplex(2048);
    let (client_reader, client_writer) = tokio::io::split(client_io);

    let server_task = tokio::spawn(async move {
        // Handshake
        server_io
            .write_all(b"{\"QMP\": {\"version\": {\"qemu\": {\"major\": 8, \"minor\": 2, \"micro\": 0}, \"package\": \"\"}, \"capabilities\": []}}\n")
            .await
            .unwrap();
        let mut buf = [0u8; 256];
        let _ = server_io.read(&mut buf).await.unwrap();
        server_io
            .write_all(b"{\"return\": {}, \"id\": \"cap_init\"}\n")
            .await
            .unwrap();

        // Server receives command but deliberately does not reply within deadline
        let n = server_io.read(&mut buf).await.unwrap();
        assert!(n > 0, "Server received mutating command");

        // Wait to verify client does NOT send a duplicate replay
        tokio::time::sleep(Duration::from_millis(100)).await;
        // Check no additional bytes are sent
        let mut check_buf = [0u8; 32];
        let read_more =
            tokio::time::timeout(Duration::from_millis(50), server_io.read(&mut check_buf)).await;
        // Either timeout or EOF; no new command data
        if let Ok(Ok(more_bytes)) = read_more {
            assert_eq!(more_bytes, 0, "Client must NOT replay mutating command");
        }
    });

    let mut client = QmpClient::new(client_reader, client_writer)
        .await
        .expect("Handshake failed");

    // Set short timeout (30ms)
    client.set_timeout(Duration::from_millis(30));

    let stop_res = client.stop().await;
    assert!(
        stop_res.is_err(),
        "Command must fail upon deadline expiration"
    );
    let err_msg = stop_res.unwrap_err().to_string();
    assert!(
        err_msg.contains("timed out") && err_msg.contains("not replayed"),
        "Error must document timeout without replay"
    );

    server_task.await.unwrap();
}

// ============================================================================
// GDB RSP Codec & Transport Peer Tests
// ============================================================================

#[test]
fn test_gdb_rsp_strict_byte_hex_and_escaping_invariants() {
    let special_payload = b"reg$pc#sp*val}end";
    let escaped = escape_payload(special_payload);
    assert!(!escaped.contains(&b'$'));
    assert!(!escaped.contains(&b'#'));
    assert!(!escaped.contains(&b'*'));

    let unescaped = unescape_payload(&escaped).unwrap();
    assert_eq!(unescaped, special_payload);

    let framed = frame_packet(special_payload);
    let unframed = unframe_packet(&framed).unwrap();
    assert_eq!(unframed, special_payload);

    // Byte hex parser safety
    assert!(decode_hex_bytes(b"").unwrap().is_empty());
    assert_eq!(decode_hex_bytes(b"00ff").unwrap(), vec![0x00, 0xff]);
    assert!(decode_hex_bytes(b"nonhex").is_err());
    assert!(decode_hex_bytes(b"123").is_err()); // odd length
}

#[tokio::test]
async fn test_gdb_client_no_ack_mode() {
    let (client_io, mut server_io) = tokio::io::duplex(2048);
    let (client_reader, client_writer) = tokio::io::split(client_io);
    let mut client = GdbClient::new(client_reader, client_writer);
    client.set_no_ack_mode(true);

    let server_task = tokio::spawn(async move {
        let mut buf = [0u8; 128];
        // Read $g#67 without sending ACK (+)
        let n = server_io.read(&mut buf).await.unwrap();
        assert_eq!(&buf[..n], b"$g#67");

        // Send register packet directly without ACK
        let regs = Arm64Registers::default();
        let framed = frame_packet(regs.to_gdb_hex().as_bytes());
        server_io.write_all(&framed).await.unwrap();
        // Do NOT wait for client ACK
    });

    let regs = client
        .read_registers()
        .await
        .expect("read_registers failed in no-ack mode");
    assert_eq!(regs.pc, 0);

    server_task.await.unwrap();
}

#[tokio::test]
async fn test_gdb_client_drop_never_sends_detach_or_kill() {
    let (client_io, mut server_io) = tokio::io::duplex(2048);
    let (client_reader, client_writer) = tokio::io::split(client_io);

    let server_task = tokio::spawn(async move {
        let mut buf = [0u8; 64];
        let n = server_io.read(&mut buf).await.unwrap();
        assert_eq!(&buf[..n], b"$OK#9a");
        server_io.write_all(b"+").await.unwrap();
        server_io.write_all(&frame_packet(b"OK")).await.unwrap();
        let mut ack = [0u8; 1];
        server_io.read_exact(&mut ack).await.unwrap();

        // Now client will drop. Server reads remaining bytes until EOF.
        let mut remaining = Vec::new();
        let _ = server_io.read_to_end(&mut remaining).await.unwrap();

        // Assert strictly ZERO packets (no 'D', 'c', or 'k') sent on drop
        assert!(
            remaining.is_empty(),
            "Supervisor retains GDB; client drop must NEVER transmit D, c, or k packets: got {:?}",
            String::from_utf8_lossy(&remaining)
        );
    });

    {
        let mut client = GdbClient::new(client_reader, client_writer);
        let resp = client.send_packet(b"OK").await.unwrap();
        assert_eq!(resp, b"OK");
        // client drops here
    }

    server_task.await.unwrap();
}

// ============================================================================
// KernelDebugLease Concurrency & Guard State Machine Tests
// ============================================================================

#[test]
fn test_kernel_debug_lease_guard_auto_disconnect_on_drop() {
    let registry = Arc::new(KernelDebugLeaseRegistry::new());
    let guest_id = ResearchGuestId::new();

    {
        let guard = registry
            .acquire(guest_id, "transient-debugger", None)
            .expect("Acquire failed");
        assert!(guard.lease().is_active());
        // Guard drops here without explicit release
    }

    // FR-029: Lease auto-transitions to DisconnectedPaused on guard drop!
    let recorded = registry.get_lease(&guest_id).expect("Lease must exist");
    assert_eq!(
        recorded.lease_state,
        DebugLeaseState::DisconnectedPaused,
        "Dropped active guard must transition to DisconnectedPaused, never released"
    );

    // QMP cont remains blocked while disconnected_paused
    assert!(registry.check_qmp_cont_allowed(&guest_id).is_err());

    // Explicit release frees the slot
    registry.release(&guest_id);
    assert!(registry.check_qmp_cont_allowed(&guest_id).is_ok());
}
