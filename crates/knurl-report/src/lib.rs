//! The report knurl-probe sends about one pair of headphones: what Windows
//! offers, what the headphones answer, and the frames exchanged, with no
//! personal data. No I/O: the Windows side ([`Collector`] feeds) and the app
//! are elsewhere, so everything here is tested on any machine.
//!
//! Privacy rules, enforced here so no caller can forget them:
//! - no Bluetooth address and no PC or user name ever enter a [`Report`];
//! - the device name from Windows is kept only when it looks like a Sony
//!   model name ([`sanitize_device_name`]): people rename their headphones,
//!   often after themselves;
//! - the frames are the MDR traffic only (model, firmware, settings), which
//!   carries none of the above.

use std::fmt::Write as _;

use knurl_protocol::client::{Client, ClientEvent, DeviceState};
use knurl_protocol::message::{Charging, EQ_PRESETS, NcAsmMode, function_name};
use knurl_protocol::session::{Config, Event, Session};
use knurl_protocol::{FrameReader, ReadEvent};
use serde::{Deserialize, Serialize};

pub mod compare;
pub mod description;

/// Version of the report's layout; raised when a field changes meaning.
pub const REPORT_FORMAT: u32 = 1;

/// How long a check may take before the report is written with what came.
pub const TIMEOUT_MS: u64 = 15_000;

/// After the request sent on a v2 service, how long to keep listening.
pub const V2_LISTEN_MS: u64 = 4_000;

/// The request every MDR conversation starts with: "which protocol do you
/// speak". Read-only; the same first frame as the XM3 capture.
const PROTOCOL_INFO: [u8; 2] = [0x00, 0x00];

/// What one Sony service gave.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum ServiceResult {
    /// Windows lists no such service for these headphones.
    NotOffered,
    /// Offered, not tried (another service was enough).
    NotTried,
    /// Opened and checked.
    Opened,
    /// Offered, but it could not be opened.
    Failed { reason: String },
}

/// The two Sony control services.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Services {
    /// MDR v1: WH-1000XM3 and WH-1000XM4.
    pub v1: ServiceResult,
    /// MDR v2: newer models, per public sources.
    pub v2: ServiceResult,
}

/// A function the headphones declare, by code and, when known, by name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Function {
    pub code: String,
    pub name: Option<String>,
}

/// What the headphones answered on the v1 service.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Answers {
    pub protocol_version: Option<String>,
    pub model: Option<String>,
    pub firmware: Option<String>,
    pub functions: Option<Vec<Function>>,
    pub battery_percent: Option<u8>,
    pub charging: Option<String>,
    pub noise: Option<String>,
    pub equalizer_preset: Option<String>,
    pub equalizer_bands: Option<[i8; 6]>,
}

impl Answers {
    fn from_state(state: &DeviceState) -> Answers {
        Answers {
            protocol_version: state.protocol_version.map(|v| format!("0x{v:04X}")),
            model: state.model.clone(),
            firmware: state.firmware.clone(),
            functions: state.functions.as_ref().map(|codes| {
                codes
                    .iter()
                    .map(|&c| Function {
                        code: format!("0x{c:02X}"),
                        name: function_name(c).map(str::to_string),
                    })
                    .collect()
            }),
            battery_percent: state.battery.map(|b| b.level),
            charging: state.battery.map(|b| {
                match b.charging {
                    Charging::Yes => "yes",
                    Charging::No => "no",
                    Charging::Unknown => "unknown",
                }
                .to_string()
            }),
            noise: state.nc_asm.map(|m| match m {
                NcAsmMode::Off => "off".to_string(),
                NcAsmMode::NoiseCancelling => "noise cancelling".to_string(),
                NcAsmMode::WindReduction => "wind reduction".to_string(),
                NcAsmMode::Ambient {
                    level,
                    focus_on_voice,
                } => format!(
                    "ambient {level}{}",
                    if focus_on_voice { ", voice focus" } else { "" }
                ),
            }),
            equalizer_preset: state.equalizer.map(|e| {
                EQ_PRESETS
                    .iter()
                    .find(|(code, _)| *code == e.preset)
                    .map_or_else(|| format!("0x{:02X}", e.preset), |(_, n)| (*n).to_string())
            }),
            equalizer_bands: state.equalizer.map(|e| {
                let b = e.bands;
                [e.clear_bass, b[0], b[1], b[2], b[3], b[4]]
            }),
        }
    }

    /// True when nothing at all was answered.
    pub fn is_empty(&self) -> bool {
        *self == Answers::default()
    }
}

/// One frame on the wire, as in the test fixtures: time, direction, bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrameRecord {
    pub t_ms: u64,
    pub dir: String,
    pub hex: String,
}

/// What the report says, in one word, for the app's result screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    /// A description of this model ships with the probe (`headphones/`):
    /// Knurl supports it.
    Supported,
    /// The headphones answered, and Knurl does not know this model yet.
    NewModel,
    /// A service opened, but the answers are not decoded yet (MDR v2).
    NotDecoded,
    /// Nothing could be read.
    NothingRead,
}

/// Everything sent about one pair of headphones.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Report {
    /// [`REPORT_FORMAT`].
    pub format: u32,
    /// The tool and its version, e.g. "knurl-probe 0.1.0".
    pub tool: String,
    /// The day of the check, `YYYY-MM-DD` (UTC).
    pub date: String,
    /// For example "Windows 10.0.26100".
    pub os: String,
    /// The Windows device name if it is a Sony model name, else a fixed
    /// placeholder ([`sanitize_device_name`]).
    pub device_name: String,
    pub services: Services,
    /// Decoded answers on the v1 service; empty for v2.
    pub answers: Answers,
    pub verdict: Verdict,
    /// Every frame of both services, in order.
    pub frames: Vec<FrameRecord>,
    /// What went wrong, in plain words.
    pub problems: Vec<String>,
}

impl Report {
    /// The report as indented JSON, as previewed and as sent.
    pub fn to_json(&self) -> String {
        // A Report holds strings, numbers and lists only: serialising it
        // cannot fail.
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    /// Reads a report written by [`Report::to_json`].
    ///
    /// # Errors
    /// Not JSON, not a report, or a newer format than this tool reads.
    pub fn from_json(text: &str) -> Result<Report, String> {
        let report: Report = serde_json::from_str(text).map_err(|e| e.to_string())?;
        if report.format > REPORT_FORMAT {
            return Err(format!(
                "report format {} is newer than this tool ({REPORT_FORMAT})",
                report.format
            ));
        }
        Ok(report)
    }
}

/// Placeholder for a device name that is not a Sony model name.
pub const RENAMED: &str = "(renamed by the user: not included)";

/// Prefixes of Sony's own device names (headphones, earbuds, gaming).
const SONY_PREFIXES: [&str; 8] = [
    "WH-", "WF-", "WI-", "MDR-", "ULT ", "INZONE", "LINKBUDS", "SRS-",
];

/// The Windows device name, kept only if it looks like the model name Sony
/// gives the headphones (`WH-1000XM4`, `LE_WF-1000XM5`, `LinkBuds S`):
/// letters, digits, spaces and dashes, at most 24 characters, starting with
/// a Sony prefix. Anything else may be a name the owner chose, often their
/// own, and becomes [`RENAMED`]. The `LE_` prefix of the low-energy entry is
/// dropped.
pub fn sanitize_device_name(name: &str) -> String {
    let trimmed = name.trim();
    let model = trimmed.strip_prefix("LE_").unwrap_or(trimmed);
    let plain = !model.is_empty()
        && model.chars().count() <= 24
        && model
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == ' ' || c == '-');
    let upper = model.to_ascii_uppercase();
    if plain && SONY_PREFIXES.iter().any(|p| upper.starts_with(p)) {
        model.to_string()
    } else {
        RENAMED.to_string()
    }
}

fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 3);
    for (i, b) in bytes.iter().enumerate() {
        if i > 0 {
            out.push(' ');
        }
        let _ = write!(out, "{b:02X}");
    }
    out
}

/// Which service a [`Collector`] talks to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Service {
    V1,
    V2,
}

/// The conversation with one service, without I/O. The caller opens the
/// socket, writes every byte returned by [`Collector::start`],
/// [`Collector::receive`] and [`Collector::poll`], and stops when
/// [`Collector::is_done`]. Only read requests are sent.
#[derive(Debug)]
pub struct Collector {
    service: Service,
    client: Client,
    session: Session,
    rx: FrameReader,
    frames: Vec<FrameRecord>,
    started_at: u64,
    v2_sent_at: Option<u64>,
    problems: Vec<String>,
}

impl Collector {
    pub fn new(service: Service) -> Collector {
        Collector {
            service,
            client: Client::new(Config::default()),
            session: Session::new(Config::default()),
            rx: FrameReader::new(),
            frames: Vec::new(),
            started_at: 0,
            v2_sent_at: None,
            problems: Vec::new(),
        }
    }

    fn tx(&mut self, wire: &[u8], now: u64) {
        self.frames.push(FrameRecord {
            t_ms: now.saturating_sub(self.started_at),
            dir: "tx".into(),
            hex: hex(wire),
        });
    }

    fn client_out(&mut self, events: Vec<ClientEvent>, now: u64) -> Vec<u8> {
        let mut out = Vec::new();
        for event in events {
            match event {
                ClientEvent::Send(wire) => {
                    self.tx(&wire, now);
                    out.extend(wire);
                }
                ClientEvent::RequestFailed { opcode, outcome } => self
                    .problems
                    .push(format!("request 0x{opcode:02X} ended with {outcome:?}")),
                ClientEvent::ProtocolError(e) => self.problems.push(e),
                // A question about a change: none is made, so none comes.
                ClientEvent::StateChanged | ClientEvent::Alert { .. } => {}
            }
        }
        out
    }

    fn session_out(&mut self, events: Vec<Event>, now: u64) -> Vec<u8> {
        let mut out = Vec::new();
        for event in events {
            if let Event::Send(wire) = event {
                self.tx(&wire, now);
                out.extend(wire);
            }
        }
        out
    }

    /// The first bytes to write.
    pub fn start(&mut self, now: u64) -> Vec<u8> {
        self.started_at = now;
        match self.service {
            Service::V1 => {
                let events = self.client.start(now);
                self.client_out(events, now)
            }
            Service::V2 => {
                self.v2_sent_at = Some(now);
                match self.session.send(PROTOCOL_INFO.to_vec(), now) {
                    Ok(wire) => {
                        self.tx(&wire, now);
                        wire
                    }
                    Err(_) => Vec::new(),
                }
            }
        }
    }

    /// Bytes read from the socket; returns what to write back (ACKs, the
    /// next request).
    pub fn receive(&mut self, bytes: &[u8], now: u64) -> Vec<u8> {
        for event in self.rx.push(bytes) {
            let wire = match event {
                ReadEvent::Frame { wire, .. }
                | ReadEvent::Invalid { wire, .. }
                | ReadEvent::Discarded(wire) => wire,
            };
            self.frames.push(FrameRecord {
                t_ms: now.saturating_sub(self.started_at),
                dir: "rx".into(),
                hex: hex(&wire),
            });
        }
        match self.service {
            Service::V1 => {
                let events = self.client.receive(bytes, now);
                self.client_out(events, now)
            }
            Service::V2 => {
                let events = self.session.receive(bytes, now);
                self.session_out(events, now)
            }
        }
    }

    /// Call regularly (every 50 ms): resends and timeouts.
    pub fn poll(&mut self, now: u64) -> Vec<u8> {
        match self.service {
            Service::V1 => {
                let events = self.client.poll(now);
                self.client_out(events, now)
            }
            Service::V2 => {
                let events = self.session.poll(now);
                self.session_out(events, now)
            }
        }
    }

    /// True when the check is over: v1 once every start-up request is
    /// answered or failed, v2 after listening for [`V2_LISTEN_MS`], both at
    /// the latest after [`TIMEOUT_MS`].
    pub fn is_done(&self, now: u64) -> bool {
        let elapsed = now.saturating_sub(self.started_at);
        if elapsed >= TIMEOUT_MS {
            return true;
        }
        match self.service {
            Service::V1 => self.client.is_idle(),
            Service::V2 => self
                .v2_sent_at
                .is_some_and(|t| now.saturating_sub(t) >= V2_LISTEN_MS),
        }
    }

    /// What this conversation gave: the answers (v1), the frames and the
    /// problems, with a timeout noted when the check did not finish.
    pub fn finish(mut self) -> Outcome {
        let finished = match self.service {
            Service::V1 => self.client.is_idle(),
            Service::V2 => true,
        };
        if !finished {
            self.problems.push(format!(
                "the headphones did not answer everything within {} seconds",
                TIMEOUT_MS / 1000
            ));
        }
        Outcome {
            service: self.service,
            answers: match self.service {
                Service::V1 => Answers::from_state(self.client.state()),
                Service::V2 => Answers::default(),
            },
            frames: self.frames,
            problems: self.problems,
        }
    }
}

/// What one [`Collector`] gave.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    pub service: Service,
    pub answers: Answers,
    pub frames: Vec<FrameRecord>,
    pub problems: Vec<String>,
}

/// Builds the report from what Windows offered and what each service gave.
pub struct ReportInput<'a> {
    /// The tool and its version, e.g. "knurl-probe 0.1.0".
    pub tool: &'a str,
    /// `YYYY-MM-DD`, see [`date_from_unix`].
    pub date: &'a str,
    pub os: &'a str,
    pub windows_name: &'a str,
    pub services: Services,
    pub outcomes: Vec<Outcome>,
    pub problems: Vec<String>,
}

pub fn build(input: ReportInput<'_>) -> Report {
    let mut answers = Answers::default();
    let mut frames = Vec::new();
    let mut problems = input.problems;
    let mut v2_answered = false;
    for outcome in input.outcomes {
        if outcome.service == Service::V1 {
            answers = outcome.answers;
        } else if outcome.frames.iter().any(|f| f.dir == "rx") {
            v2_answered = true;
        }
        frames.extend(outcome.frames);
        problems.extend(outcome.problems);
    }
    let model = answers.model.clone().unwrap_or_default();
    let verdict = if description::find_model(&model).is_some() {
        Verdict::Supported
    } else if !answers.is_empty() {
        Verdict::NewModel
    } else if v2_answered {
        Verdict::NotDecoded
    } else {
        Verdict::NothingRead
    };
    Report {
        format: REPORT_FORMAT,
        tool: input.tool.to_string(),
        date: input.date.to_string(),
        os: input.os.to_string(),
        device_name: sanitize_device_name(input.windows_name),
        services: input.services,
        answers,
        verdict,
        frames,
        problems,
    }
}

/// `YYYY-MM-DD` (UTC) of a time in seconds since 1970, without a date crate
/// (days to civil date, Howard Hinnant's algorithm).
pub fn date_from_unix(secs: u64) -> String {
    let days = (secs / 86_400) as i64;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}")
}

/// The report's file name: the model (or "headphones"), then the time in
/// seconds, so two checks never overwrite each other. Only letters, digits
/// and dashes.
pub fn file_name(report: &Report, unix_secs: u64) -> String {
    let model = report
        .answers
        .model
        .clone()
        .or_else(|| (report.device_name != RENAMED).then(|| report.device_name.clone()))
        .unwrap_or_else(|| "headphones".to_string());
    let safe: String = model
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' {
                c
            } else {
                '-'
            }
        })
        .collect();
    format!("knurl-report-{safe}-{unix_secs}.json")
}

/// A few lines in plain English for the command line: what was found and
/// what to do with the file.
pub fn summary(report: &Report) -> Vec<String> {
    let a = &report.answers;
    let name = a
        .model
        .clone()
        .unwrap_or_else(|| report.device_name.clone());
    let mut lines = vec![match report.verdict {
        Verdict::Supported => format!("{name}: Knurl already supports this model."),
        Verdict::NewModel => format!("{name}: a model Knurl does not know yet. Thank you!"),
        Verdict::NotDecoded => format!(
            "{name}: the headphones answered on the newer Sony service (MDR v2), which Knurl cannot read yet. Thank you!"
        ),
        Verdict::NothingRead => format!("{name}: nothing could be read from these headphones."),
    }];
    if let Some(f) = &a.firmware {
        lines.push(format!("Firmware: {f}"));
    }
    if let Some(functions) = &a.functions {
        lines.push(format!("Features declared: {}", functions.len()));
    }
    for problem in &report.problems {
        lines.push(format!("Problem: {problem}"));
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sony_names_are_kept_and_other_names_are_not() {
        for (name, kept) in [
            ("WH-1000XM4", "WH-1000XM4"),
            ("LE_WH-1000XM3", "WH-1000XM3"),
            ("WF-1000XM5", "WF-1000XM5"),
            ("LinkBuds S", "LinkBuds S"),
            ("ULT WEAR", "ULT WEAR"),
            ("INZONE H9", "INZONE H9"),
            ("  WH-CH720N ", "WH-CH720N"),
        ] {
            assert_eq!(sanitize_device_name(name), kept, "{name}");
        }
        for name in [
            "Alex's headphones",
            "Cuffie di Marco",
            "WH-1000XM4 (Giulia)",
            "WH-1000XM4 of a very long owner name",
            "",
            "LE_",
            "AirPods Pro",
        ] {
            assert_eq!(sanitize_device_name(name), RENAMED, "{name:?}");
        }
    }

    #[test]
    fn dates_from_unix_time() {
        assert_eq!(date_from_unix(0), "1970-01-01");
        // 2026-11-18 00:00:00 UTC and one second before.
        assert_eq!(date_from_unix(1_794_960_000), "2026-11-18");
        assert_eq!(date_from_unix(1_794_959_999), "2026-11-17");
        // A leap day.
        assert_eq!(date_from_unix(1_709_164_800), "2024-02-29");
    }

    #[test]
    fn a_report_reads_back_what_it_wrote() {
        let report = build(ReportInput {
            tool: "knurl-probe 0.1.0",
            date: "2026-11-18",
            os: "Windows 10.0.26100",
            windows_name: "WH-1000XM4",
            services: Services {
                v1: ServiceResult::Failed {
                    reason: "refused".into(),
                },
                v2: ServiceResult::NotOffered,
            },
            outcomes: Vec::new(),
            problems: vec!["could not open MDR v1".into()],
        });
        assert_eq!(Report::from_json(&report.to_json()), Ok(report.clone()));
        let newer = report.to_json().replace("\"format\": 1", "\"format\": 99");
        assert!(
            Report::from_json(&newer)
                .expect_err("newer format")
                .contains("newer")
        );
        assert!(Report::from_json("not json").is_err());
    }

    #[test]
    fn hex_is_spaced_upper_case() {
        assert_eq!(hex(&[0x3E, 0x0C, 0x00]), "3E 0C 00");
        assert_eq!(hex(&[]), "");
    }

    #[test]
    fn the_verdict_follows_what_was_read() {
        let base = |outcomes| ReportInput {
            tool: "knurl-probe 0.1.0",
            date: "2026-11-18",
            os: "Windows 10.0.26100",
            windows_name: "My phones",
            services: Services {
                v1: ServiceResult::NotOffered,
                v2: ServiceResult::Opened,
            },
            outcomes,
            problems: Vec::new(),
        };
        let v2 = |rx: bool| Outcome {
            service: Service::V2,
            answers: Answers::default(),
            frames: if rx {
                vec![FrameRecord {
                    t_ms: 10,
                    dir: "rx".into(),
                    hex: "3E 01".into(),
                }]
            } else {
                Vec::new()
            },
            problems: Vec::new(),
        };
        assert_eq!(build(base(vec![v2(true)])).verdict, Verdict::NotDecoded);
        assert_eq!(build(base(vec![v2(false)])).verdict, Verdict::NothingRead);
        let new_model = Outcome {
            service: Service::V1,
            answers: Answers {
                model: Some("WH-1000XM5".into()),
                ..Answers::default()
            },
            frames: Vec::new(),
            problems: Vec::new(),
        };
        let report = build(base(vec![new_model]));
        assert_eq!(report.verdict, Verdict::NewModel);
        // A renamed device never reaches the report.
        assert_eq!(report.device_name, RENAMED);
        assert!(!report.to_json().contains("My phones"));
    }

    #[test]
    fn v2_sends_only_the_protocol_question_then_listens() {
        let mut c = Collector::new(Service::V2);
        let first = c.start(1000);
        // DATA_MDR, sequence 0, payload 00 00.
        assert_eq!(
            first,
            vec![
                0x3E, 0x0C, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x0E, 0x3C
            ]
        );
        assert!(!c.is_done(1000 + V2_LISTEN_MS - 1));
        assert!(c.is_done(1000 + V2_LISTEN_MS));
        let outcome = c.finish();
        assert_eq!(outcome.frames.len(), 1);
        assert_eq!(outcome.frames[0].dir, "tx");
        assert_eq!(outcome.frames[0].t_ms, 0);
    }

    #[test]
    fn a_silent_v1_service_times_out_with_a_problem() {
        let mut c = Collector::new(Service::V1);
        assert!(!c.start(0).is_empty());
        let mut t = 0;
        while !c.is_done(t) {
            t += 50;
            c.poll(t);
        }
        assert_eq!(t, TIMEOUT_MS);
        let outcome = c.finish();
        assert!(outcome.answers.is_empty());
        assert!(
            outcome
                .problems
                .iter()
                .any(|p| p.contains("did not answer everything"))
        );
    }

    #[test]
    fn file_names_and_summaries() {
        let mut report = build(ReportInput {
            tool: "knurl-probe 0.1.0",
            date: "2026-11-18",
            os: "Windows",
            windows_name: "Alex's phones",
            services: Services {
                v1: ServiceResult::Failed {
                    reason: "refused".into(),
                },
                v2: ServiceResult::NotOffered,
            },
            outcomes: Vec::new(),
            problems: vec!["could not open MDR v1: refused".into()],
        });
        assert_eq!(report.verdict, Verdict::NothingRead);
        // A renamed device never names the file either.
        assert_eq!(file_name(&report, 42), "knurl-report-headphones-42.json");
        let lines = summary(&report);
        assert!(lines[0].contains("nothing could be read"));
        assert!(
            lines
                .iter()
                .any(|l| l == "Problem: could not open MDR v1: refused")
        );

        report.answers.model = Some("WH-1000XM5".into());
        report.answers.firmware = Some("2.1.0".into());
        report.verdict = Verdict::NewModel;
        assert_eq!(file_name(&report, 7), "knurl-report-WH-1000XM5-7.json");
        let lines = summary(&report);
        assert_eq!(
            lines[0],
            "WH-1000XM5: a model Knurl does not know yet. Thank you!"
        );
        assert!(lines.contains(&"Firmware: 2.1.0".to_string()));
    }
}
