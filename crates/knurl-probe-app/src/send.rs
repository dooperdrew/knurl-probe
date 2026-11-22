//! Sending the report through the bedrock relay (DooperDrew/bedrock-relay),
//! which opens an issue in this repository. No I/O here: the request is built
//! and the answer read as plain data, and the Windows side moves the bytes
//! ([`Transport`]). Tested on any system.
//!
//! The relay takes a `probe` report (bedrock-relay ADR-0003): the report as a
//! JSON object behind the envelope every probe shares (`probe`, `schema`,
//! `version`, `date`, `steps`). It always opens a new issue, never a "seen
//! again" comment, so every report is kept, and labels it `probe` and
//! `probe:knurl-probe`.

use knurl_report::{REPORT_FORMAT, Report, ServiceResult};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::t;
use crate::view::model_name;

/// Where reports go and the project's public key (public by design: it can
/// only send reports). Edited in `relay.json`, read at build time.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct RelayConfig {
    pub endpoint: String,
    pub key: String,
}

const RELAY_JSON: &str = include_str!("../relay.json");

/// The committed relay settings.
///
/// # Errors
/// `relay.json` is not valid (a test makes sure it is).
pub fn relay_config() -> Result<RelayConfig, String> {
    serde_json::from_str(RELAY_JSON).map_err(|e| format!("relay.json: {e}"))
}

/// The probe's name in the envelope: the relay labels the issue
/// `probe:knurl-probe`.
pub const PROBE: &str = "knurl-probe";

/// The relay's limits on what this app sends (bedrock-relay ADR-0003): the
/// report object as JSON with 2-space indentation, in UTF-16 units as the
/// relay counts them, and the whole request in bytes.
pub const MAX_REPORT: usize = 45_000;
pub const MAX_REQUEST: usize = 64 * 1024;
pub const MAX_MESSAGE: usize = 2_000;

/// Why a report could not be prepared.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NotSendable {
    /// The report is longer than the relay takes; saving it still works.
    TooLong { length: usize },
    /// A report saved before the time of the check was recorded: the
    /// envelope needs it. The app only sends reports it has just made, which
    /// always have it.
    NoTime,
}

/// The envelope's steps: the two services, then each start-up question of
/// the v1 service. Outcomes are words of at most 40 characters: a service is
/// `opened`, `not-offered`, `not-tried` or `failed`; a question `arrived`,
/// `silent` (the service opened, no answer came) or `not-reached` (the
/// service did not open).
pub fn steps(report: &Report) -> Vec<Value> {
    let service = |id: &str, result: &ServiceResult| {
        let outcome = match result {
            ServiceResult::Opened => "opened",
            ServiceResult::NotOffered => "not-offered",
            ServiceResult::NotTried => "not-tried",
            ServiceResult::Failed { .. } => "failed",
        };
        json!({ "id": id, "outcome": outcome })
    };
    let v1_opened = report.services.v1 == ServiceResult::Opened;
    let a = &report.answers;
    let questions = [
        ("protocol-version", a.protocol_version.is_some()),
        ("model", a.model.is_some()),
        ("firmware", a.firmware.is_some()),
        ("functions", a.functions.is_some()),
        ("battery", a.battery_percent.is_some()),
        ("noise", a.noise.is_some()),
        ("equalizer", a.equalizer_preset.is_some()),
    ];
    let mut out = vec![
        service("service-v1", &report.services.v1),
        service("service-v2", &report.services.v2),
    ];
    out.extend(questions.iter().map(|(id, arrived)| {
        let outcome = match (arrived, v1_opened) {
            (true, _) => "arrived",
            (false, true) => "silent",
            (false, false) => "not-reached",
        };
        json!({ "id": id, "outcome": outcome })
    }));
    out
}

/// What the relay gets as `report`: the envelope, then the report exactly as
/// it is saved to a file, under `headphones`.
pub fn probe_report(report: &Report, version: &str) -> Result<Value, NotSendable> {
    if report.checked_at.is_empty() {
        return Err(NotSendable::NoTime);
    }
    // A Report holds strings, numbers and lists only: it always converts.
    let headphones = serde_json::to_value(report).unwrap_or_default();
    Ok(json!({
        "probe": PROBE,
        "schema": REPORT_FORMAT,
        "version": version,
        "date": report.checked_at,
        "steps": steps(report),
        "headphones": headphones,
    }))
}

/// The exact request body: what "See what will be sent" shows and what
/// [`send`] posts, byte for byte.
///
/// # Errors
/// [`NotSendable::TooLong`] when the report does not fit in the relay's
/// limits, [`NotSendable::NoTime`] for a report without the time of its check.
pub fn request_body(report: &Report, version: &str) -> Result<String, NotSendable> {
    let object = probe_report(report, version)?;
    // serde_json's pretty form is 2-space indentation, as the relay measures.
    let length = serde_json::to_string_pretty(&object)
        .unwrap_or_default()
        .encode_utf16()
        .count();
    if length > MAX_REPORT {
        return Err(NotSendable::TooLong { length });
    }
    let message: String = format!("Headphone report: {}", model_name(report))
        .chars()
        .take(MAX_MESSAGE)
        .collect();
    let body = json!({
        "kind": "probe",
        "message": message,
        "report": object,
        "version": version,
        "platform": report.os,
        "user": true,
    });
    // A json! value of strings, numbers, lists and a bool always serialises.
    let body = serde_json::to_string_pretty(&body).unwrap_or_default();
    if body.len() > MAX_REQUEST {
        return Err(NotSendable::TooLong { length });
    }
    Ok(body)
}

/// Moves one request: POST `body` to `url` with the project key, and give back
/// the HTTP status and the answer's body, or why nothing came back.
pub trait Transport {
    /// # Errors
    /// No answer at all (no network, timeout, TLS), in plain words.
    fn post(&self, url: &str, key: &str, body: &str) -> Result<(u16, String), String>;
}

/// How a send ended.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// The relay opened this issue.
    Sent { issue: u64 },
    /// Not sent, with the reason in plain words.
    NotSent(String),
}

#[derive(Deserialize)]
struct Accepted {
    issue: u64,
}

#[derive(Deserialize)]
struct Refused {
    error: String,
}

/// Reads the relay's answer (bedrock-relay README, API).
pub fn interpret(status: u16, body: &str) -> Outcome {
    if status == 202 {
        return match serde_json::from_str::<Accepted>(body) {
            Ok(a) => Outcome::Sent { issue: a.issue },
            Err(_) => Outcome::NotSent(t!(
                "the server answered {status} without an issue number: it is unclear whether the report arrived",
                status = status
            )),
        };
    }
    let error = serde_json::from_str::<Refused>(body)
        .map(|r| r.error)
        .unwrap_or_default();
    let reason = match (status, error.as_str()) {
        (400, e) if e.starts_with("kind must be") => {
            t!("the server does not accept reports from Knurl Probe yet").to_string()
        }
        (403, "disabled") => t!("reports to Knurl Probe are switched off for now").to_string(),
        (400 | 403, e) => t!("the server refused the report ({reason})", reason = e),
        (401, _) => t!("the server does not know this app's key").to_string(),
        (413, _) => t!("the report is too large for the server").to_string(),
        (429, _) => t!("too many reports were sent in the last hour: try again later").to_string(),
        (502, _) => t!("the server could not reach GitHub: try again later").to_string(),
        (s, "") => t!("the server answered HTTP {status}", status = s),
        (s, e) => t!(
            "the server answered HTTP {status} ({reason})",
            status = s,
            reason = e
        ),
    };
    Outcome::NotSent(reason)
}

/// Builds the request, sends it and reads the answer.
pub fn send(
    transport: &dyn Transport,
    config: &RelayConfig,
    report: &Report,
    version: &str,
) -> Outcome {
    let body = match request_body(report, version) {
        Ok(body) => body,
        Err(NotSendable::TooLong { length }) => {
            return Outcome::NotSent(t!(
                "the report is {length} characters, more than the {limit} the server takes: save it to a file and attach it to an issue instead",
                length = length,
                limit = MAX_REPORT
            ));
        }
        Err(NotSendable::NoTime) => {
            return Outcome::NotSent(
                t!("this report does not say when the check ran: run the check again").to_string(),
            );
        }
    };
    match transport.post(&config.endpoint, &config.key, &body) {
        Ok((status, answer)) => interpret(status, &answer),
        Err(reason) => Outcome::NotSent(t!(
            "the server could not be reached: {reason}",
            reason = reason
        )),
    }
}

/// The issue the relay opened.
pub fn issue_url(issue: u64) -> String {
    format!("https://github.com/DooperDrew/knurl-probe/issues/{issue}")
}

/// Host, port, path and HTTPS-or-not of an address, for the Windows transport.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Target {
    pub host: String,
    pub port: u16,
    pub path: String,
    pub secure: bool,
}

/// # Errors
/// Not an http(s) address, a bad port or no host.
pub fn target(url: &str) -> Result<Target, String> {
    let (secure, rest) = if let Some(rest) = url.strip_prefix("https://") {
        (true, rest)
    } else if let Some(rest) = url.strip_prefix("http://") {
        (false, rest)
    } else {
        return Err(format!("not an http(s) address: {url}"));
    };
    let (authority, path) = rest.find('/').map_or((rest, "/"), |i| rest.split_at(i));
    let (host, port) = match authority.rsplit_once(':') {
        Some((host, port)) => (
            host,
            port.parse::<u16>()
                .map_err(|_| format!("bad port in {url}"))?,
        ),
        None => (authority, if secure { 443 } else { 80 }),
    };
    if host.is_empty() {
        return Err(format!("no host in {url}"));
    }
    Ok(Target {
        host: host.to_string(),
        port,
        path: path.to_string(),
        secure,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use knurl_report::{Answers, REPORT_FORMAT, ServiceResult, Services, Verdict};
    use std::cell::RefCell;

    fn report() -> Report {
        Report {
            format: REPORT_FORMAT,
            tool: "knurl-probe 0.1.0".into(),
            date: "2026-11-19".into(),
            checked_at: "2026-11-19T10:00:00Z".into(),
            os: "Windows 10.0.26100".into(),
            device_name: "WH-1000XM6".into(),
            services: Services {
                v1: ServiceResult::Opened,
                v2: ServiceResult::NotOffered,
            },
            answers: Answers {
                model: Some("WH-1000XM6".into()),
                ..Answers::default()
            },
            verdict: Verdict::NewModel,
            frames: Vec::new(),
            problems: Vec::new(),
        }
    }

    /// Keeps what was posted and answers as told.
    struct Fake {
        answer: Result<(u16, String), String>,
        posted: RefCell<Vec<(String, String, String)>>,
    }

    impl Fake {
        fn answering(answer: Result<(u16, &str), &str>) -> Fake {
            Fake {
                answer: answer
                    .map(|(s, b)| (s, b.to_string()))
                    .map_err(str::to_string),
                posted: RefCell::new(Vec::new()),
            }
        }
    }

    impl Transport for Fake {
        fn post(&self, url: &str, key: &str, body: &str) -> Result<(u16, String), String> {
            self.posted
                .borrow_mut()
                .push((url.into(), key.into(), body.into()));
            self.answer.clone()
        }
    }

    #[test]
    fn the_committed_relay_settings_are_read() {
        let c = relay_config().expect("relay.json");
        assert_eq!(
            c.endpoint,
            "https://bedrock-relay.dooperdrew.workers.dev/v1/report"
        );
        assert!(c.key.starts_with("pk_knurlprobe_app_"));
        let t = target(&c.endpoint).expect("target");
        assert_eq!(
            (t.host.as_str(), t.port, t.path.as_str(), t.secure),
            (
                "bedrock-relay.dooperdrew.workers.dev",
                443,
                "/v1/report",
                true
            )
        );
    }

    #[test]
    fn the_body_is_a_probe_report_with_the_envelope() {
        let r = report();
        let body = request_body(&r, "0.1.0").expect("body");
        let v: serde_json::Value = serde_json::from_str(&body).expect("json");
        assert_eq!(v["kind"], "probe");
        assert_eq!(v["message"], "Headphone report: WH-1000XM6");
        assert_eq!(v["version"], "0.1.0");
        assert_eq!(v["platform"], "Windows 10.0.26100");
        assert_eq!(v["user"], true);
        assert_eq!(
            v.as_object().map(serde_json::Map::len),
            Some(6),
            "nothing else leaves"
        );
        let report_value = &v["report"];
        assert_eq!(report_value["probe"], "knurl-probe");
        assert_eq!(report_value["schema"], REPORT_FORMAT);
        assert_eq!(report_value["version"], "0.1.0");
        assert_eq!(report_value["date"], "2026-11-19T10:00:00Z");
        // The report inside is exactly the one saved to a file.
        let inside = serde_json::to_string_pretty(&report_value["headphones"]).expect("json");
        assert_eq!(Report::from_json(&inside), Ok(r.clone()));
        assert_eq!(inside, r.to_json());
        assert_eq!(
            report_value.as_object().map(serde_json::Map::len),
            Some(6),
            "the envelope and the report only"
        );
    }

    /// The relay's envelope check (bedrock-relay `validateProbe`), so a body
    /// this app builds is never refused for its shape.
    fn relay_accepts(v: &serde_json::Value) -> Result<(), String> {
        let r = v.as_object().ok_or("report must be a JSON object")?;
        let probe = r["probe"].as_str().ok_or("probe")?;
        let name_ok = (1..=40).contains(&probe.len())
            && probe
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
            && !probe.starts_with('-')
            && !probe.ends_with('-');
        if !name_ok {
            return Err("probe".into());
        }
        r["schema"].as_u64().ok_or("schema")?;
        let version = r["version"].as_str().ok_or("version")?;
        if version.trim().is_empty() || version.len() > 50 || version.contains('\n') {
            return Err("version".into());
        }
        let date = r["date"].as_str().ok_or("date")?;
        let b = date.as_bytes();
        let date_ok = date.len() == 20
            && b[4] == b'-'
            && b[7] == b'-'
            && b[10] == b'T'
            && b[13] == b':'
            && b[16] == b':'
            && b[19] == b'Z';
        if !date_ok {
            return Err(format!("date {date}"));
        }
        for (i, step) in r["steps"].as_array().ok_or("steps")?.iter().enumerate() {
            let outcome = step["outcome"].as_str().ok_or(format!("steps[{i}]"))?;
            if outcome.trim().is_empty() || outcome.len() > 40 {
                return Err(format!("steps[{i}].outcome"));
            }
        }
        let length = serde_json::to_string_pretty(v)
            .map_err(|e| e.to_string())?
            .encode_utf16()
            .count();
        if length > MAX_REPORT {
            return Err(format!("{length} characters"));
        }
        Ok(())
    }

    #[test]
    fn the_envelope_passes_the_relays_check() {
        let mut r = report();
        assert_eq!(
            relay_accepts(&probe_report(&r, "0.1.0").expect("report")),
            Ok(())
        );
        r.services.v1 = ServiceResult::Failed {
            reason: "refused".into(),
        };
        r.answers = Answers::default();
        assert_eq!(
            relay_accepts(&probe_report(&r, "0.1.0").expect("report")),
            Ok(())
        );
    }

    #[test]
    fn steps_say_what_each_service_and_question_gave() {
        let outcomes = |r: &Report| -> Vec<(String, String)> {
            steps(r)
                .iter()
                .map(|s| {
                    (
                        s["id"].as_str().unwrap_or_default().to_string(),
                        s["outcome"].as_str().unwrap_or_default().to_string(),
                    )
                })
                .collect()
        };
        let mut r = report();
        let got = outcomes(&r);
        assert_eq!(got.len(), 9);
        assert_eq!(got[0], ("service-v1".into(), "opened".into()));
        assert_eq!(got[1], ("service-v2".into(), "not-offered".into()));
        assert_eq!(got[3], ("model".into(), "arrived".into()));
        assert_eq!(got[4], ("firmware".into(), "silent".into()));

        r.services.v1 = ServiceResult::Failed {
            reason: "refused".into(),
        };
        r.services.v2 = ServiceResult::NotTried;
        r.answers = Answers::default();
        let got = outcomes(&r);
        assert_eq!(got[0].1, "failed");
        assert_eq!(got[1].1, "not-tried");
        assert!(got[2..].iter().all(|(_, o)| o == "not-reached"), "{got:?}");
    }

    #[test]
    fn a_report_without_the_time_of_its_check_is_not_sent() {
        let mut r = report();
        r.checked_at = String::new();
        assert_eq!(request_body(&r, "0.1.0"), Err(NotSendable::NoTime));
        let fake = Fake::answering(Ok((202, r#"{"issue":1}"#)));
        let config = relay_config().expect("relay.json");
        let Outcome::NotSent(reason) = send(&fake, &config, &r, "0.1.0") else {
            panic!("a report without a time must not count as sent");
        };
        assert!(reason.contains("run the check again"), "{reason}");
        assert!(fake.posted.borrow().is_empty(), "nothing went out");
    }

    #[test]
    fn a_renamed_device_is_not_named_in_the_message() {
        let mut r = report();
        r.answers.model = None;
        r.device_name = knurl_report::RENAMED.into();
        let body = request_body(&r, "0.1.0").expect("body");
        assert!(body.contains("\"Headphone report: headphones\""));
    }

    #[test]
    fn what_is_shown_is_exactly_what_is_sent() {
        let r = report();
        let fake = Fake::answering(Ok((202, r#"{"action":"opened","issue":12}"#)));
        let config = relay_config().expect("relay.json");
        assert_eq!(
            send(&fake, &config, &r, "0.1.0"),
            Outcome::Sent { issue: 12 }
        );
        let posted = fake.posted.borrow();
        assert_eq!(posted.len(), 1);
        assert_eq!(posted[0].0, config.endpoint);
        assert_eq!(posted[0].1, config.key);
        assert_eq!(posted[0].2, request_body(&r, "0.1.0").expect("body"));
    }

    #[test]
    fn a_report_over_the_limit_is_not_sent() {
        let mut r = report();
        r.problems = vec!["x".repeat(MAX_REPORT)];
        assert!(matches!(
            request_body(&r, "0.1.0"),
            Err(NotSendable::TooLong { .. })
        ));
        let fake = Fake::answering(Ok((202, r#"{"issue":1}"#)));
        let config = relay_config().expect("relay.json");
        let Outcome::NotSent(reason) = send(&fake, &config, &r, "0.1.0") else {
            panic!("a long report must not count as sent");
        };
        assert!(reason.contains("save it to a file"), "{reason}");
        assert!(fake.posted.borrow().is_empty(), "nothing went out");
    }

    #[test]
    fn every_answer_reads_as_sent_or_not_sent() {
        let not_sent = |status: u16, body: &str| match interpret(status, body) {
            Outcome::NotSent(reason) => reason,
            Outcome::Sent { .. } => panic!("{status} {body} is not a success"),
        };
        assert_eq!(
            interpret(202, r#"{"action":"opened","issue":7}"#),
            Outcome::Sent { issue: 7 }
        );
        assert!(not_sent(202, "{}").contains("unclear"));
        assert!(
            not_sent(
                400,
                r#"{"error":"kind must be one of crash, error, bug, feedback"}"#
            )
            .contains("does not accept reports from Knurl Probe yet")
        );
        assert!(
            not_sent(400, r#"{"error":"message is required"}"#).contains("message is required")
        );
        assert!(not_sent(401, r#"{"error":"unknown key"}"#).contains("key"));
        assert!(not_sent(403, r#"{"error":"disabled"}"#).contains("switched off"));
        assert!(not_sent(403, r#"{"error":"origin"}"#).contains("origin"));
        assert!(not_sent(413, "").contains("too large"));
        assert!(not_sent(429, r#"{"error":"rate limit"}"#).contains("try again later"));
        assert!(not_sent(502, r#"{"error":"upstream"}"#).contains("GitHub"));
        assert_eq!(not_sent(500, "oops"), "the server answered HTTP 500");
        assert_eq!(
            not_sent(200, r#"{"issue":3}"#),
            "the server answered HTTP 200"
        );
    }

    #[test]
    fn no_answer_is_not_sent() {
        let fake = Fake::answering(Err("timeout"));
        let config = relay_config().expect("relay.json");
        assert_eq!(
            send(&fake, &config, &report(), "0.1.0"),
            Outcome::NotSent("the server could not be reached: timeout".into())
        );
    }

    #[test]
    fn addresses_are_parsed_or_refused() {
        assert_eq!(
            target("http://127.0.0.1:8787/v1/report"),
            Ok(Target {
                host: "127.0.0.1".into(),
                port: 8787,
                path: "/v1/report".into(),
                secure: false
            })
        );
        assert!(target("ftp://x").is_err());
        assert!(target("https://:443/").is_err());
        assert!(target("https://host:99999/").is_err());
        assert_eq!(
            issue_url(5),
            "https://github.com/DooperDrew/knurl-probe/issues/5"
        );
    }
}
