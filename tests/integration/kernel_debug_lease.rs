//! Integration tests for GDB RSP kernel pause, registers, memory, step,
//! and exclusive KernelDebugLease concurrency rules.
//!
//! Defined in accordance with FR-027, FR-028, FR-029, SC-004, SC-006, and D-05.

use emu::models::research::{DebugLeaseState, ResearchGuestId};
use emu::protocols::gdb::client::GdbClient;
use emu::protocols::gdb::lease::{KernelDebugLease, KernelDebugLeaseRegistry};
use emu::protocols::gdb::packet::frame_packet;
use emu::protocols::gdb::registers::Arm64Registers;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[tokio::test]
async fn test_gdb_client_duplex_comprehensive_operations() {
    let (client_io, mut server_io) = tokio::io::duplex(4096);
    let (client_reader, client_writer) = tokio::io::split(client_io);
    let mut client = GdbClient::new(client_reader, client_writer);

    // Mock GDB RSP Server simulating QEMU gdbstub
    let server_task = tokio::spawn(async move {
        // 1. Handle read_registers ('g') with NAK retransmission test
        let mut buf = [0u8; 256];
        let n = server_io.read(&mut buf).await.unwrap();
        assert_eq!(&buf[..n], b"$g#67");

        // Send NAK (-) on first attempt to force retransmission of same frame
        server_io.write_all(b"-").await.unwrap();
        let n2 = server_io.read(&mut buf).await.unwrap();
        assert_eq!(
            &buf[..n2],
            b"$g#67",
            "Retransmitted frame must be identical"
        );

        // Now send ACK (+) and register response
        server_io.write_all(b"+").await.unwrap();
        let mut x = [0u64; 31];
        x[0] = 0xdead_beef_cafe_babe;
        let mock_regs = Arm64Registers {
            x,
            pc: 0xffff_fe00_0700_4000,
            sp: 0xffff_fe00_0700_3ff0,
            pstate: 0,
        };
        let reg_hex = mock_regs.to_gdb_hex();
        let framed_regs = frame_packet(reg_hex.as_bytes());
        server_io.write_all(&framed_regs).await.unwrap();

        // Read client ACK (+)
        let mut ack = [0u8; 1];
        server_io.read_exact(&mut ack).await.unwrap();
        assert_eq!(ack[0], b'+');

        // 2. Handle indexed register write ('P0=...')
        let n = server_io.read(&mut buf).await.unwrap();
        assert!(
            buf[..n].starts_with(b"$P0="),
            "Expected P0 register write command"
        );
        server_io.write_all(b"+").await.unwrap();
        server_io.write_all(&frame_packet(b"OK")).await.unwrap();
        server_io.read_exact(&mut ack).await.unwrap();

        // 3. Handle read_memory ('m1000,4')
        let n = server_io.read(&mut buf).await.unwrap();
        assert_eq!(&buf[..n], b"$m1000,4#8e");
        server_io.write_all(b"+").await.unwrap();
        server_io
            .write_all(&frame_packet(b"11223344"))
            .await
            .unwrap();
        server_io.read_exact(&mut ack).await.unwrap();

        // 4. Handle write_memory ('M1000,4:55667788')
        let n = server_io.read(&mut buf).await.unwrap();
        assert!(
            buf[..n].starts_with(b"$M1000,4:"),
            "Expected M write memory command"
        );
        server_io.write_all(b"+").await.unwrap();
        server_io.write_all(&frame_packet(b"OK")).await.unwrap();
        server_io.read_exact(&mut ack).await.unwrap();

        // 5. Handle memory write failure ('Mffff,4:...') returning error E01
        let _ = server_io.read(&mut buf).await.unwrap();
        server_io.write_all(b"+").await.unwrap();
        server_io.write_all(&frame_packet(b"E01")).await.unwrap();
        server_io.read_exact(&mut ack).await.unwrap();

        // 6. Handle software breakpoint set ('Z0,2000,4')
        let n = server_io.read(&mut buf).await.unwrap();
        assert_eq!(&buf[..n], b"$Z0,2000,4#d8");
        server_io.write_all(b"+").await.unwrap();
        server_io.write_all(&frame_packet(b"OK")).await.unwrap();
        server_io.read_exact(&mut ack).await.unwrap();

        // 7. Handle software breakpoint remove ('z0,2000,4')
        let n = server_io.read(&mut buf).await.unwrap();
        assert_eq!(&buf[..n], b"$z0,2000,4#f8");
        server_io.write_all(b"+").await.unwrap();
        server_io.write_all(&frame_packet(b"OK")).await.unwrap();
        server_io.read_exact(&mut ack).await.unwrap();

        // 8. Handle continue ('c'): server sends ACK (+); client returns without waiting for stop
        let n = server_io.read(&mut buf).await.unwrap();
        assert_eq!(&buf[..n], b"$c#63");
        server_io.write_all(b"+").await.unwrap();

        // 9. Handle halt (\x03 break byte): server drains stop reply T02
        let n = server_io.read(&mut buf).await.unwrap();
        assert_eq!(&buf[..n], &[0x03], "Expected out-of-band break byte (0x03)");
        server_io
            .write_all(&frame_packet(b"T02thread:01;core:0;"))
            .await
            .unwrap();
        server_io.read_exact(&mut ack).await.unwrap();

        // 10. Handle step ('s'): server sends ACK (+) followed by stop reply T05
        let n = server_io.read(&mut buf).await.unwrap();
        assert_eq!(&buf[..n], b"$s#73");
        server_io.write_all(b"+").await.unwrap();
        server_io
            .write_all(&frame_packet(b"T05thread:01;"))
            .await
            .unwrap();
        server_io.read_exact(&mut ack).await.unwrap();
    });

    // 1. Read registers (exercising NAK retransmission)
    let regs = client
        .read_registers()
        .await
        .expect("read_registers failed");
    assert_eq!(regs.pc, 0xffff_fe00_0700_4000);
    assert_eq!(regs.sp, 0xffff_fe00_0700_3ff0);
    assert_eq!(regs.x[0], 0xdead_beef_cafe_babe);

    // 2. Write indexed register (P)
    client
        .write_register(0, 0x1234_5678_9abc_def0)
        .await
        .expect("write_register failed");

    // 3. Read memory
    let mem = client
        .read_memory(0x1000, 4)
        .await
        .expect("read_memory failed");
    assert_eq!(mem, vec![0x11, 0x22, 0x33, 0x44]);

    // 4. Write memory
    client
        .write_memory(0x1000, &[0x55, 0x66, 0x77, 0x88])
        .await
        .expect("write_memory failed");

    // 5. Memory write error (E01) rejection
    let write_err = client.write_memory(0xffff, &[0x00, 0x00]).await;
    assert!(write_err.is_err(), "Write to invalid memory must fail");
    assert!(
        write_err.unwrap_err().to_string().contains("E01"),
        "Error message must contain E01"
    );

    // 6. Set software breakpoint
    client
        .set_breakpoint(0x2000)
        .await
        .expect("set_breakpoint failed");

    // 7. Remove software breakpoint
    client
        .remove_breakpoint(0x2000)
        .await
        .expect("remove_breakpoint failed");

    // 8. Continue returns immediately on ACK
    client.cont().await.expect("cont failed");

    // 9. Halt drains stop reply T02
    let halt_reply = client.halt().await.expect("halt failed");
    assert!(halt_reply.is_stopped());
    assert_eq!(halt_reply.signal(), Some(2));

    // 10. Step consumes stop reply T05
    let step_reply = client.step().await.expect("step failed");
    assert!(step_reply.is_stopped());
    assert_eq!(step_reply.signal(), Some(5));

    server_task.await.unwrap();
}

#[test]
fn test_kernel_debug_lease_registry_and_concurrency_invariants() {
    let registry = Arc::new(KernelDebugLeaseRegistry::new());
    let guest_id = ResearchGuestId::new();

    // 1. Acquire lease
    let mut guard = registry
        .acquire(guest_id, "session-test-client", None)
        .expect("Failed to acquire lease");

    assert!(guard.lease().is_active());
    assert_eq!(guard.lease().lease_state, DebugLeaseState::Active);
    assert!(guard.lease().qmp_cont_blocked);

    // 2. Second acquire on same guest while active must conflict
    let conflict_res = registry.acquire(guest_id, "concurrent-client", None);
    assert!(
        conflict_res.is_err(),
        "Duplicate lease acquisition must be rejected"
    );
    assert!(
        conflict_res
            .unwrap_err()
            .to_string()
            .contains("DEBUG_LEASE_CONFLICT")
    );

    // 3. QMP cont must be blocked while lease is active
    let cont_res = registry.check_qmp_cont_allowed(&guest_id);
    assert!(cont_res.is_err());
    assert!(
        cont_res
            .unwrap_err()
            .to_string()
            .contains("DEBUG_LEASE_CONFLICT")
    );

    // 4. Heartbeat update
    let old_heartbeat = guard.lease().heartbeat_at.clone();
    guard.heartbeat().unwrap();
    assert!(guard.lease().heartbeat_at >= old_heartbeat);

    // 5. Disconnect paused (FR-029: preserving paused state on client disconnect)
    guard.disconnect_paused().unwrap();
    assert!(guard.lease().is_disconnected_paused());
    assert_eq!(
        guard.lease().lease_state,
        DebugLeaseState::DisconnectedPaused
    );

    // QMP cont remains blocked even in disconnected_paused state
    assert!(registry.check_qmp_cont_allowed(&guest_id).is_err());

    // 6. Explicit release
    let released_lease = guard.release().expect("Failed to release guard");
    assert_eq!(released_lease.lease_state, DebugLeaseState::Released);

    // QMP cont is now allowed
    assert!(registry.check_qmp_cont_allowed(&guest_id).is_ok());

    // New lease can now be acquired for the guest
    let guard2 = registry
        .acquire(guest_id, "second-session-client", None)
        .expect("Re-acquisition after release must succeed");
    assert!(guard2.lease().is_active());
}

#[test]
fn test_kernel_debug_lease_canonical_json_schema_shape() {
    let guest_id = ResearchGuestId::new();
    let lease = KernelDebugLease::with_endpoint(
        guest_id,
        "test-client-identity",
        format!("/tmp/emu-{guest_id}/gdb.sock"),
    );

    let json_val = serde_json::to_value(&lease).expect("Failed to serialize KernelDebugLease");

    // Assert all canonical JSON Schema required fields are present
    assert!(json_val.get("lease_id").is_some());
    assert!(json_val.get("guest_id").is_some());
    assert!(json_val.get("session_id").is_some());
    assert!(json_val.get("client_identity").is_some());
    assert_eq!(json_val.get("lease_state").unwrap(), "active");
    assert!(json_val.get("acquired_at").is_some());
    assert!(json_val.get("heartbeat_at").is_some());
    assert_eq!(json_val.get("qmp_cont_blocked").unwrap(), true);
    assert!(json_val.get("gdb_endpoint_path").is_some());

    // Verify roundtrip deserialization with deny_unknown_fields
    let deserialized: KernelDebugLease =
        serde_json::from_value(json_val).expect("Roundtrip deserialization failed");
    assert_eq!(lease, deserialized);
}
