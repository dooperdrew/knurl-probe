//! The XM3 capture of 2026-11-07 (frames only, no address) replayed through
//! the v1 collector: the report must read what the headphones answered and
//! say that Knurl supports them.

use knurl_report::compare::{compare, draft};
use knurl_report::description::{ServiceId, Status, find_model};
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

/// The report the probe writes from the replayed capture.
fn xm3_report() -> knurl_report::Report {
    let mut c = Collector::new(Service::V1);
    c.start(0);
    for (t, wire) in received() {
        if c.is_done(t) {
            break;
        }
        c.poll(t);
        c.receive(&wire, t);
    }
    build(ReportInput {
        tool: "knurl-probe 0.1.0",
        date: "2026-11-07",
        checked_at: "2026-11-07T10:00:00Z",
        os: "Windows 10.0.26100",
        windows_name: "WH-1000XM3",
        services: Services {
            v1: ServiceResult::Opened,
            v2: ServiceResult::NotOffered,
        },
        outcomes: vec![c.finish()],
        problems: Vec::new(),
    })
}

#[test]
fn the_shipped_xm3_description_agrees_with_the_capture() {
    let report = xm3_report();
    let d = find_model("WH-1000XM3").expect("XM3 description");
    assert_eq!(compare(&report, &d), vec![]);
    // A report read back from its JSON compares the same way.
    let again = knurl_report::Report::from_json(&report.to_json()).expect("json");
    assert_eq!(compare(&again, &d), vec![]);
}

#[test]
fn a_draft_from_the_capture_is_the_shipped_description() {
    let report = xm3_report();
    let shipped = find_model("WH-1000XM3").expect("XM3 description");
    let drafted = draft(&report, "sony-wh-1000xm3", "Sony WH-1000XM3").expect("draft");
    assert_eq!(drafted.model, shipped.model);
    assert_eq!(drafted.service, shipped.service);
    assert_eq!(drafted.protocol_version, shipped.protocol_version);
    assert_eq!(drafted.firmware, shipped.firmware);
    assert_eq!(drafted.functions, shipped.functions);
    assert_eq!(drafted.verification.status, Status::Reported);
    assert_eq!(drafted.verification.last, "2026-11-07");
    assert_eq!(drafted.verification.source, "knurl-probe 0.1.0 report");
    assert_eq!(compare(&report, &drafted), vec![]);
}

#[test]
fn differences_with_a_description_are_named() {
    let report = xm3_report();
    let mut d = find_model("WH-1000XM3").expect("XM3 description");
    d.firmware = vec!["4.1.0".into()];
    d.functions.retain(|&c| c != 0x39);
    d.functions.push(0x99);
    d.protocol_version = Some(0x4000);
    d.service = ServiceId::MdrV2;
    let found = compare(&report, &d);
    let what: Vec<&str> = found.iter().map(|m| m.what.as_str()).collect();
    assert_eq!(
        what,
        ["service", "protocol_version", "functions", "firmware"]
    );
    let functions = &found[2];
    assert_eq!(functions.expected, "also 0x99");
    assert_eq!(functions.found, "also 0x39");
    // Another model stops the comparison at the model name.
    let xm4 = find_model("WH-1000XM4").expect("XM4 description");
    let found = compare(&report, &xm4);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].what, "model");
    assert_eq!(found[0].found, "WH-1000XM3");
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
    assert_eq!(a.charging.as_deref(), Some("no"));
    assert_eq!(a.noise.as_deref(), Some("noise cancelling"));
    assert_eq!(a.equalizer_bands, Some([0; 6]));
    let functions = a.functions.as_ref().expect("functions");
    assert_eq!(functions.len(), 19);
    assert_eq!(functions[0].code, "0x11");
    assert_eq!(functions[0].name.as_deref(), Some("battery level"));

    let report = build(ReportInput {
        tool: "knurl-probe 0.1.0",
        date: "2026-11-07",
        checked_at: "2026-11-07T10:00:00Z",
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
    assert!(json.contains("\"format\": 1"));
}
