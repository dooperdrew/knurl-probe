//! Sending the report through the bedrock relay (DooperDrew/bedrock-relay),
//! which opens an issue in this repository. No I/O here: the request is built
//! and the answer read as plain data, and the Windows side moves the bytes
//! ([`Transport`]). Tested on any system.
//!
//! The relay takes a `data` report with the report as its attachment
//! (bedrock-relay ADR-0002): it always opens a new issue, never a "seen again"
//! comment, so every report is kept.

use knurl_report::Report;
use serde::Deserialize;

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

/// The attachment's file name. Short on purpose: the relay hides long runs of
/// letters and digits (they look like keys), which a dated name would be.
pub const ATTACHMENT_NAME: &str = "knurl-report.json";

/// The relay's limits on what this app sends (bedrock-relay README).
pub const MAX_ATTACHMENT: usize = 40_000;
pub const MAX_MESSAGE: usize = 2_000;

/// Why a report could not be prepared.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NotSendable {
    /// The report is longer than the relay takes; saving it still works.
    TooLong { length: usize },
}

/// The exact request body: what "See what will be sent" shows and what
/// [`send`] posts, byte for byte.
///
/// # Errors
/// [`NotSendable::TooLong`] when the report does not fit in the relay's limit.
pub fn request_body(report: &Report, version: &str) -> Result<String, NotSendable> {
    let content = report.to_json();
    let length = content.chars().count();
    if length > MAX_ATTACHMENT {
        return Err(NotSendable::TooLong { length });
    }
    let message: String = format!("Headphone report: {}", model_name(report))
        .chars()
        .take(MAX_MESSAGE)
        .collect();
    let body = serde_json::json!({
        "kind": "data",
        "message": message,
        "attachment": { "name": ATTACHMENT_NAME, "content": content },
        "version": version,
        "platform": report.os,
        "user": true,
    });
    // A json! value of strings and a bool always serialises.
    Ok(serde_json::to_string_pretty(&body).unwrap_or_default())
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
                limit = MAX_ATTACHMENT
            ));
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
    fn the_body_is_a_data_report_with_the_report_attached() {
        let r = report();
        let body = request_body(&r, "0.1.0").expect("body");
        let v: serde_json::Value = serde_json::from_str(&body).expect("json");
        assert_eq!(v["kind"], "data");
        assert_eq!(v["message"], "Headphone report: WH-1000XM6");
        assert_eq!(v["attachment"]["name"], ATTACHMENT_NAME);
        assert_eq!(v["attachment"]["content"], r.to_json());
        assert_eq!(v["version"], "0.1.0");
        assert_eq!(v["platform"], "Windows 10.0.26100");
        assert_eq!(v["user"], true);
        assert_eq!(
            v.as_object().map(serde_json::Map::len),
            Some(6),
            "nothing else leaves"
        );
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
        r.problems = vec!["x".repeat(MAX_ATTACHMENT)];
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
