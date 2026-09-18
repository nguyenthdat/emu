//! Unit tests for GDB RSP packet framing, checksums, escaping, and ARM64 register serialization.
//!
//! Defined in accordance with RFC 9003 and plan.md §Structure.

use emu::protocols::gdb::packet::{
    GdbStopReply, calculate_checksum, decode_hex_bytes, encode_hex_bytes, escape_payload,
    frame_packet, parse_error_reply, parse_stop_reply, unescape_payload, unframe_packet,
};
use emu::protocols::gdb::registers::{
    Arm64Registers, decode_register_value, encode_register_value,
};

#[test]
fn test_gdb_rsp_checksum_calculation() {
    // Checksum for "g" is 0x67
    assert_eq!(calculate_checksum(b"g"), 0x67);

    // Checksum for "OK" is 'O'(0x4f) + 'K'(0x4b) = 0x9a
    assert_eq!(calculate_checksum(b"OK"), 0x9a);

    // Checksum for "c" is 'c'(0x63)
    assert_eq!(calculate_checksum(b"c"), 0x63);

    // Checksum for "s" is 's'(0x73)
    assert_eq!(calculate_checksum(b"s"), 0x73);
}

#[test]
fn test_gdb_rsp_escape_and_unescape() {
    let raw = b"test$with#special*chars}inside";
    let escaped = escape_payload(raw);
    assert!(escaped.len() > raw.len());
    // None of the 4 special characters may appear unescaped
    assert!(!escaped.contains(&b'$'));
    assert!(!escaped.contains(&b'#'));
    assert!(!escaped.contains(&b'*'));

    let unescaped = unescape_payload(&escaped).expect("Failed to unescape");
    assert_eq!(unescaped, raw);
}

#[test]
fn test_gdb_rsp_frame_and_unframe_normal() {
    let payload = b"qSupported:multiprocess+;swbreak+";
    let framed = frame_packet(payload);

    assert_eq!(framed[0], b'$');
    assert_eq!(framed[framed.len() - 3], b'#');

    let unframed = unframe_packet(&framed).expect("Failed to unframe valid packet");
    assert_eq!(unframed, payload);
}

#[test]
fn test_gdb_rsp_frame_and_unframe_with_escaped_characters() {
    let payload = b"mem:0x1000$val#42";
    let framed = frame_packet(payload);
    let unframed = unframe_packet(&framed).expect("Failed to unframe escaped packet");
    assert_eq!(unframed, payload);
}

#[test]
fn test_gdb_rsp_corrupted_checksum_rejected() {
    let mut framed = frame_packet(b"c");
    let last = framed.len() - 1;
    framed[last] = if framed[last] == b'0' { b'1' } else { b'0' };

    let res = unframe_packet(&framed);
    assert!(res.is_err(), "Corrupted checksum must be rejected");
}

#[test]
fn test_gdb_rsp_truncated_packet_rejected() {
    assert!(unframe_packet(b"$").is_err());
    assert!(unframe_packet(b"$#").is_err());
    assert!(unframe_packet(b"$c#").is_err());
    assert!(unframe_packet(b"c#63").is_err());
}

#[test]
fn test_byte_hex_decoder_strict_ascii() {
    let valid_hex = b"0123456789abcdefABCDEF";
    let decoded = decode_hex_bytes(valid_hex).expect("Valid hex decoding failed");
    assert_eq!(decoded.len(), 11);
    assert_eq!(decoded[0], 0x01);
    assert_eq!(decoded[1], 0x23);

    let re_encoded = encode_hex_bytes(&decoded);
    assert_eq!(re_encoded.len(), 22);

    // Rejection on odd length
    assert!(decode_hex_bytes(b"123").is_err());

    // Rejection on non-hex ASCII character without panic
    assert!(decode_hex_bytes(b"1z").is_err());
    assert!(decode_hex_bytes(&[0xff, 0xfe]).is_err());
}

#[test]
fn test_arm64_registers_roundtrip() {
    let mut regs = Arm64Registers::default();
    regs.x[0] = 0x1122334455667788;
    regs.x[1] = 0xaabbccddeeff0011;
    regs.x[30] = 0x8877665544332211;
    regs.sp = 0xffff_fe00_0700_3ff0;
    regs.pc = 0xffff_fe00_0700_4000;
    regs.pstate = 0x60000000;

    let hex_payload = regs.to_gdb_hex();
    let deserialized =
        Arm64Registers::from_gdb_hex(&hex_payload).expect("Failed to parse registers from hex");

    assert_eq!(regs.x[0], deserialized.x[0]);
    assert_eq!(regs.x[1], deserialized.x[1]);
    assert_eq!(regs.x[30], deserialized.x[30]);
    assert_eq!(regs.sp, deserialized.sp);
    assert_eq!(regs.pc, deserialized.pc);
    assert_eq!(regs.pstate, deserialized.pstate);

    let map = regs.to_map();
    assert_eq!(map.get("x0").unwrap(), "0x1122334455667788");
    assert_eq!(map.get("sp").unwrap(), "0xfffffe0007003ff0");
    assert_eq!(map.get("pc").unwrap(), "0xfffffe0007004000");
    assert_eq!(map.get("pstate").unwrap(), "0x60000000");

    assert_eq!(regs.get_register(0), Some(0x1122334455667788));
    assert_eq!(regs.get_register(31), Some(0xffff_fe00_0700_3ff0));
    assert_eq!(regs.get_register(32), Some(0xffff_fe00_0700_4000));
    assert_eq!(regs.get_register(33), Some(0x60000000));
    assert_eq!(regs.get_register(34), None);
}

#[test]
fn test_single_register_indexed_encode_decode() {
    let val: u64 = 0x1234_5678_9abc_def0;
    let hex = encode_register_value(val);
    assert_eq!(hex.len(), 16);

    let decoded = decode_register_value(hex.as_bytes()).expect("decode register value failed");
    assert_eq!(decoded, val);
}

#[test]
fn test_stop_reply_parsing() {
    // S05 (SIGTRAP)
    let s_reply = parse_stop_reply(b"S05").expect("Failed to parse S05");
    match s_reply {
        GdbStopReply::Signal(sig) => assert_eq!(sig, 5),
        _ => panic!("Expected Signal(5)"),
    }

    // T02 (SIGINT) with thread info
    let t_reply = parse_stop_reply(b"T02thread:01;core:0;").expect("Failed to parse T02");
    match t_reply {
        GdbStopReply::WatchpointOrSignal { signal, details } => {
            assert_eq!(signal, 2);
            assert_eq!(details.len(), 2);
            assert_eq!(details[0], ("thread".to_string(), "01".to_string()));
            assert_eq!(details[1], ("core".to_string(), "0".to_string()));
        }
        _ => panic!("Expected WatchpointOrSignal"),
    }

    // W00 (Normal exit code 0)
    let w_reply = parse_stop_reply(b"W00").expect("Failed to parse W00");
    match w_reply {
        GdbStopReply::Exited(code) => assert_eq!(code, 0),
        _ => panic!("Expected Exited(0)"),
    }

    // X09 (Terminated by SIGKILL 9)
    let x_reply = parse_stop_reply(b"X09").expect("Failed to parse X09");
    match x_reply {
        GdbStopReply::Terminated(sig) => assert_eq!(sig, 9),
        _ => panic!("Expected Terminated(9)"),
    }
}

#[test]
fn test_error_reply_parsing() {
    assert_eq!(parse_error_reply(b"E01"), Some(1));
    assert_eq!(parse_error_reply(b"E22"), Some(0x22));
    assert_eq!(parse_error_reply(b"OK"), None);
    assert_eq!(parse_error_reply(b"T05"), None);
}
