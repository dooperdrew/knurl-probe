//! Framing test vectors from public sources, until our own XM3/XM4 captures exist.
//!
//! Device frames: real WH-1000XM3 fw 4.5.2 captures published in
//! mos9527/SonyHeadphonesClient (`tests/WH-1000XM3-4.5.2/`, MIT licence).
//! Host frames: Gadgetbridge `SonyProtocolImplV2Test.java` (only the bytes are
//! used as facts; no code is copied). See `docs/02-riferimenti-protocollo.md`.

use knurl_protocol::{DataType, Frame, ack_for};

fn bytes(s: &str) -> Vec<u8> {
    s.split_whitespace()
        .map(|b| u8::from_str_radix(b, 16).expect("valid hex in test vector"))
        .collect()
}

fn decode(s: &str) -> Frame {
    Frame::decode(&bytes(s)).expect("reference frame must decode")
}

#[test]
fn xm3_device_ack() {
    let f = decode("3e 01 01 00 00 00 00 02 3c");
    assert_eq!(f.data_type, DataType::Ack);
    assert_eq!(f.seq, 1);
    assert!(f.payload.is_empty());
}

#[test]
fn xm3_protocol_info_reply() {
    let f = decode("3e 0c 00 00 00 00 04 01 00 40 10 61 3c");
    assert_eq!(f.data_type, DataType::DataMdr);
    assert_eq!(f.payload, vec![0x01, 0x00, 0x40, 0x10]);
}

#[test]
fn xm3_device_info_model_name() {
    let f = decode("3e 0c 00 00 00 00 0d 05 01 0a 57 48 2d 31 30 30 30 58 4d 33 8e 3c");
    assert_eq!(&f.payload[..3], &[0x05, 0x01, 0x0A]);
    assert_eq!(&f.payload[3..], b"WH-1000XM3");
}

#[test]
fn xm3_device_info_firmware() {
    let f = decode("3e 0c 01 00 00 00 08 05 02 05 34 2e 35 2e 32 18 3c");
    assert_eq!(f.seq, 1);
    assert_eq!(&f.payload[3..], b"4.5.2");
}

#[test]
fn xm3_ncasm_reply() {
    let f = decode("3e 0c 00 00 00 00 08 67 02 01 02 02 01 00 00 83 3c");
    assert_eq!(f.payload[0], 0x67);
    assert_eq!(f.payload.len(), 8);
}

#[test]
fn reference_frames_reencode_identically() {
    for s in [
        "3e 01 01 00 00 00 00 02 3c",
        "3e 0c 00 00 00 00 04 01 00 40 10 61 3c",
        "3e 0c 01 00 00 00 08 05 02 05 34 2e 35 2e 32 18 3c",
        "3e 0c 00 00 00 00 02 f6 0d 11 3c",
        "3e 0c 01 00 00 00 04 58 00 00 00 69 3c",
    ] {
        assert_eq!(decode(s).encode(), bytes(s), "re-encoding {s}");
    }
}

#[test]
fn host_init_and_acks_match_expected_bytes() {
    let init = Frame::new(DataType::DataMdr, 0, vec![0x00, 0x00]);
    assert_eq!(init.encode(), bytes("3e 0c 00 00 00 00 02 00 00 0e 3c"));

    let rx0 = Frame::new(DataType::DataMdr, 0, vec![]);
    let rx1 = Frame::new(DataType::DataMdr, 1, vec![]);
    assert_eq!(ack_for(&rx0).encode(), bytes("3e 01 01 00 00 00 00 02 3c"));
    assert_eq!(ack_for(&rx1).encode(), bytes("3e 01 00 00 00 00 00 01 3c"));
}

#[test]
fn escaping_example() {
    let f = Frame::new(DataType::DataMdr, 0, vec![0x58, 0x01, 0x3C]);
    assert_eq!(f.encode(), bytes("3e 0c 00 00 00 00 03 58 01 3d 2c a4 3c"));
}
