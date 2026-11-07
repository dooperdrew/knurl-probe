//! Replays the guided capture of our WH-1000XM3 (2026-11-07) through
//! [`Session`]: same requests at the same times, same bytes from the
//! headphones. The session must produce byte for byte what was sent on the
//! wire, pair each reply with its request, and flag the repeats.

use xmc_protocol::message::{Charging, Message, NcAsmMode};
use xmc_protocol::session::{Config, Event, Outcome, Session};

const CAPTURE: &str = include_str!("fixtures/xm3-2026-11-07-guided.txt");

struct Line {
    t: u64,
    tx: bool,
    wire: Vec<u8>,
}

fn lines() -> Vec<Line> {
    CAPTURE
        .lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .map(|l| {
            let mut parts = l.split_whitespace();
            let t = parts.next().expect("time").parse().expect("number");
            let tx = parts.next().expect("direction") == "tx";
            let wire = parts
                .map(|b| u8::from_str_radix(b, 16).expect("hex"))
                .collect();
            Line { t, tx, wire }
        })
        .collect()
}

#[test]
fn replay_matches_the_wire_and_decodes_every_reply() {
    let mut session = Session::new(Config::default());
    let mut sent: Vec<Vec<u8>> = Vec::new();
    let mut events: Vec<Event> = Vec::new();

    for line in lines() {
        for ev in session.poll(line.t) {
            if let Event::Send(w) = &ev {
                sent.push(w.clone());
            }
            events.push(ev);
        }
        if line.tx {
            // Host data frames are the requests; host ACKs are produced by the
            // session itself and checked through `sent`.
            if line.wire[1] == 0x0C {
                let payload = line.wire[7..line.wire.len() - 2].to_vec();
                let wire = session
                    .send(payload, line.t)
                    .expect("one request at a time");
                sent.push(wire);
            }
        } else {
            for ev in session.receive(&line.wire, line.t) {
                if let Event::Send(w) = &ev {
                    sent.push(w.clone());
                }
                events.push(ev);
            }
        }
    }

    let captured_tx: Vec<Vec<u8>> = lines()
        .into_iter()
        .filter(|l| l.tx)
        .map(|l| l.wire)
        .collect();
    assert_eq!(
        sent, captured_tx,
        "session output differs from the captured host bytes"
    );

    let replies: Vec<Message> = events
        .iter()
        .filter_map(|e| match e {
            Event::Received {
                reply: true,
                message,
                ..
            } => Some(message.clone().expect("every reply decodes")),
            _ => None,
        })
        .collect();
    assert_eq!(
        replies,
        vec![
            Message::ProtocolInfo { version: 0x4010 },
            Message::ModelName("WH-1000XM3".into()),
            Message::Firmware("4.5.2".into()),
            Message::SupportedFunctions(vec![
                0x11, 0xC1, 0x12, 0x13, 0x14, 0x30, 0x71, 0x62, 0x81, 0x42, 0x41, 0x51, 0xA1, 0xE1,
                0xE2, 0xD1, 0xD2, 0xF4, 0x39
            ]),
            Message::Battery {
                level: 100,
                charging: Charging::No,
                notification: false
            },
            Message::NcAsm {
                mode: NcAsmMode::NoiseCancelling,
                notification: false
            },
            Message::Equalizer {
                preset: 0x00,
                clear_bass: 0,
                bands: [0; 5],
                notification: false
            },
        ]
    );

    let done: Vec<(u8, Outcome)> = events
        .iter()
        .filter_map(|e| match e {
            Event::RequestDone { opcode, outcome } => Some((*opcode, *outcome)),
            _ => None,
        })
        .collect();
    assert_eq!(
        done,
        [0x00, 0x04, 0x04, 0x06, 0x10, 0x66, 0x56].map(|op| (op, Outcome::Answered))
    );

    let notifications: Vec<Message> = events
        .iter()
        .filter_map(|e| match e {
            Event::Received {
                reply: false,
                message,
                ..
            } => Some(message.clone().expect("notification decodes")),
            _ => None,
        })
        .collect();
    assert_eq!(
        notifications,
        vec![Message::NcAsm {
            mode: NcAsmMode::Ambient {
                level: 20,
                focus_on_voice: false
            },
            notification: true
        }]
    );

    let duplicates = events.iter().filter(|e| **e == Event::Duplicate).count();
    assert_eq!(duplicates, 1 + 10, "one EQ repeat and ten NC repeats");
}

#[test]
fn client_replay_builds_the_device_state() {
    use xmc_protocol::client::{Battery, Client, ClientEvent, DeviceState, Equalizer};

    let mut client = Client::new(Config::default());
    let mut sent: Vec<Vec<u8>> = Vec::new();
    let mut collect = |events: Vec<ClientEvent>| {
        for ev in events {
            match ev {
                ClientEvent::Send(w) => sent.push(w),
                ClientEvent::StateChanged => {}
                other => panic!("unexpected client event {other:?}"),
            }
        }
    };

    // The client paces its own requests (next one as soon as the previous is
    // answered); the capture answered them in the same order.
    collect(client.start(0));
    for line in lines().into_iter().filter(|l| !l.tx) {
        collect(client.poll(line.t));
        collect(client.receive(&line.wire, line.t));
    }

    let captured_tx: Vec<Vec<u8>> = lines()
        .into_iter()
        .filter(|l| l.tx)
        .map(|l| l.wire)
        .collect();
    // 60 s after the start-up battery request (sent at 3205 ms) the client
    // asks for the battery again, at the next poll (75996 ms). The capture
    // tool did not; everything else is byte-identical.
    let refresh = sent.remove(15);
    assert_eq!(&refresh[7..9], &[0x10, 0x00], "periodic battery request");
    assert_eq!(sent, captured_tx);

    assert_eq!(
        client.state(),
        &DeviceState {
            protocol_version: Some(0x4010),
            model: Some("WH-1000XM3".into()),
            firmware: Some("4.5.2".into()),
            functions: Some(vec![
                0x11, 0xC1, 0x12, 0x13, 0x14, 0x30, 0x71, 0x62, 0x81, 0x42, 0x41, 0x51, 0xA1, 0xE1,
                0xE2, 0xD1, 0xD2, 0xF4, 0x39
            ]),
            battery: Some(Battery {
                level: 100,
                charging: Charging::No
            }),
            // The last NC/ASM message was the notification: ambient, level 20.
            nc_asm: Some(NcAsmMode::Ambient {
                level: 20,
                focus_on_voice: false
            }),
            equalizer: Some(Equalizer {
                preset: 0,
                clear_bass: 0,
                bands: [0; 5]
            }),
        }
    );
}
