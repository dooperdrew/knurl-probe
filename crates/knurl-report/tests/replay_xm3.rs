//! The XM3 capture of 2026-11-07 (frames only, no address) replayed through
//! the v1 collector: the report must read what the headphones answered and
//! say that Knurl supports them.

use knurl_report::{
    Collector, ReportInput, Service, ServiceResult, Services, Verdict, build, sanitize_device_name,
};

const CAPTURE: &str = include_str!("../../knurl-protocol/tests/fixtures/xm3-2026-11-07-guided.txt");

/// The frames the headphones sent, with their time.
fn received() -> Vec<(u64, Vec<u8>)> {
    CAPTURE
        .lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .filter_map(|l| {
            let mut parts = l.split_whitespace();
            let t: u64 = parts.next()?.parse().ok()?;
            let dir = parts.next()?;
            let wire = parts
                .map(|b| u8::from_str_radix(b, 16).expect("hex byte"))
                .collect();
            (dir == "rx").then_some((t, wire))
        })
        .collect()
}

#[test]
fn the_xm3_capture_becomes_a_supported_report() {
    let mut c = Collector::new(Service::V1);
    assert!(!c.start(0).is_empty());
    let mut done_at = None;
    for (t, wire) in received() {
        if c.is_done(t) {
            done_at = Some(t);
            break;
        }
        c.poll(t);
        c.receive(&wire, t);
    }
    // Idle once the equalizer reply (the last start-up answer) is in.
    assert_eq!(done_at, Some(5914));
    let outcome = c.finish();
    assert!(outcome.problems.is_empty(), "{:?}", outcome.problems);

    let a = &outcome.answers;
    assert_eq!(a.model.as_deref(), Some("WH-1000XM3"));
    assert_eq!(a.firmware.as_deref(), Some("4.5.2"));
    assert_eq!(a.protocol_version.as_deref(), Some("0x4010"));
    assert_eq!(a.battery_percent, Some(100));
    assert_eq!(a.charging, Some("no"));
    assert_eq!(a.noise.as_deref(), Some("noise cancelling"));
    assert_eq!(a.equalizer_bands, Some([0; 6]));
    let functions = a.functions.as_ref().expect("functions");
    assert_eq!(functions.len(), 19);
    assert_eq!(functions[0].code, "0x11");
    assert_eq!(functions[0].name, Some("battery level"));

    let report = build(ReportInput {
        probe_version: "0.1.0",
        os: "Windows 10.0.26100",
        windows_name: "WH-1000XM3",
        services: Services {
            v1: ServiceResult::Opened,
            v2: ServiceResult::NotOffered,
        },
        outcomes: vec![outcome],
        problems: Vec::new(),
    });
    assert_eq!(report.verdict, Verdict::Supported);
    assert_eq!(report.device_name, sanitize_device_name("WH-1000XM3"));
    // Every frame of the conversation is there, in both directions.
    assert!(report.frames.iter().any(|f| f.dir == "tx"));
    assert!(report.frames.iter().any(|f| f.dir == "rx"));
    assert_eq!(
        report.frames[0].hex, "3E 0C 00 00 00 00 02 00 00 0E 3C",
        "the protocol question comes first"
    );
    let json = report.to_json();
    assert!(json.contains("\"verdict\": \"supported\""));
    assert!(json.contains("\"schema\": 1"));
}
