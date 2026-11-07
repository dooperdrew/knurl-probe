//! Frames from our own WH-1000XM3 (firmware 4.5.2), captured with
//! `xmc-probe capture --guided` over Winsock on 2026-11-07. Only the wire
//! bytes are kept: the capture file itself holds the Bluetooth address and
//! stays out of the repository. See `docs/04-risultati-xm3.md`.

use xmc_protocol::{DataType, Frame, FrameReader, ReadEvent, ack_for};

fn bytes(s: &str) -> Vec<u8> {
    s.split_whitespace()
        .map(|b| u8::from_str_radix(b, 16).expect("valid hex in fixture"))
        .collect()
}

fn decode(s: &str) -> Frame {
    Frame::decode(&bytes(s)).expect("captured frame must decode")
}

/// Every frame the headphones sent, plus our requests, in capture order.
const CAPTURED: &[&str] = &[
    // host: GET_PROTOCOL_INFO, reply, device ACK
    "3E 0C 00 00 00 00 02 00 00 0E 3C",
    "3E 0C 00 00 00 00 04 01 00 40 10 61 3C",
    // reply: model name
    "3E 0C 01 00 00 00 0D 05 01 0A 57 48 2D 31 30 30 30 58 4D 33 8F 3C",
    // reply: firmware
    "3E 0C 00 00 00 00 08 05 02 05 34 2E 35 2E 32 17 3C",
    // host: GET NC/ASM, device ACK, reply
    "3E 0C 01 00 00 00 02 66 02 77 3C",
    "3E 01 00 00 00 00 00 01 3C",
    "3E 0C 01 00 00 00 08 67 02 01 02 02 01 00 00 84 3C",
    // host: GET EQ, device ACK, reply
    "3E 0C 00 00 00 00 02 56 01 65 3C",
    "3E 01 01 00 00 00 00 02 3C",
    "3E 0C 00 00 00 00 0A 57 01 00 06 0A 0A 0A 0A 0A 0A B0 3C",
    // unsolicited NC/ASM notification
    "3E 0C 01 00 00 00 08 69 02 01 02 00 01 00 14 98 3C",
];

#[test]
fn every_captured_frame_decodes_and_reencodes_identically() {
    for s in CAPTURED {
        assert_eq!(decode(s).encode(), bytes(s), "re-encoding {s}");
    }
}

#[test]
fn protocol_info_matches_published_xm3_capture() {
    let f = decode("3E 0C 00 00 00 00 04 01 00 40 10 61 3C");
    assert_eq!(f.payload, vec![0x01, 0x00, 0x40, 0x10]);
}

#[test]
fn model_and_firmware() {
    let model = decode(CAPTURED[2]);
    assert_eq!(&model.payload[..3], &[0x05, 0x01, 0x0A]);
    assert_eq!(&model.payload[3..], b"WH-1000XM3");

    let fw = decode(CAPTURED[3]);
    assert_eq!(&fw.payload[..3], &[0x05, 0x02, 0x05]);
    assert_eq!(&fw.payload[3..], b"4.5.2");
}

#[test]
fn battery_reply_reads_full_and_not_charging() {
    let f = decode("3E 0C 00 00 00 00 04 11 00 64 00 85 3C");
    assert_eq!(f.payload, vec![0x11, 0x00, 0x64, 0x00]);
}

#[test]
fn support_list_has_19_entries() {
    let f = decode(
        "3E 0C 01 00 00 00 16 07 00 13 11 C1 12 13 14 30 71 62 81 42 41 51 A1 E1 E2 D1 D2 F4 39 D4 3C",
    );
    assert_eq!(&f.payload[..2], &[0x07, 0x00]);
    assert_eq!(f.payload[2] as usize, f.payload.len() - 3);
}

#[test]
fn host_acks_match_what_the_device_accepted() {
    // The device did not repeat these replies after receiving these ACKs.
    let reply = decode(CAPTURED[6]);
    assert_eq!(
        ack_for(&reply).encode(),
        bytes("3E 01 00 00 00 00 00 01 3C")
    );
}

#[test]
fn burst_of_repeated_notifications_is_split_into_frames() {
    // At 76 s the XM3 sent the same notification 11 times within ~15 ms.
    let one = bytes(CAPTURED[10]);
    let stream: Vec<u8> = one.iter().copied().cycle().take(one.len() * 11).collect();
    let mut reader = FrameReader::new();
    let frames: Vec<Frame> = reader
        .push(&stream)
        .into_iter()
        .map(|ev| match ev {
            ReadEvent::Frame { frame, .. } => frame,
            other => panic!("unexpected event {other:?}"),
        })
        .collect();
    assert_eq!(frames.len(), 11);
    assert!(frames.iter().all(|f| f.data_type == DataType::DataMdr));
}
