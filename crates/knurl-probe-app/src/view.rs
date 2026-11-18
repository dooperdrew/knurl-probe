//! What each screen shows, from the [`Flow`]: plain words for the result,
//! the rows of the check, and the values handed to the Slint window.

use knurl_probe::CheckStep;
use knurl_report::{RENAMED, Report, ServiceResult, Verdict};
use slint::{ModelRc, SharedString, VecModel};

use crate::flow::{Flow, Page, Search};
use crate::ui::{Fact, MainWindow, StepRow};

/// The result screen in plain words.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResultView {
    /// 0 supported, 1 new model, 2 not decoded, 3 nothing read.
    pub kind: i32,
    pub title: String,
    pub text: String,
    /// "What was read", label and value.
    pub facts: Vec<(String, String)>,
    pub problems: Vec<String>,
}

/// The model's name for sentences: what the headphones answered, else the
/// Windows name when it is a Sony model name, else "headphones".
pub fn model_name(report: &Report) -> String {
    report
        .answers
        .model
        .clone()
        .or_else(|| (report.device_name != RENAMED).then(|| report.device_name.clone()))
        .unwrap_or_else(|| "headphones".to_string())
}

fn service_line(report: &Report) -> Option<String> {
    let opened = |s: &ServiceResult| *s == ServiceResult::Opened;
    match (opened(&report.services.v1), opened(&report.services.v2)) {
        (true, true) => Some("MDR v1 and MDR v2".into()),
        (true, false) => Some("MDR v1".into()),
        (false, true) => Some("MDR v2 (newer)".into()),
        (false, false) => None,
    }
}

pub fn result_view(report: &Report) -> ResultView {
    let model = model_name(report);
    let (kind, title, text) = match report.verdict {
        Verdict::Supported => (
            0,
            format!("Knurl already supports your {model}"),
            "Sending the report still helps: it confirms the model with your firmware.".to_string(),
        ),
        Verdict::NewModel => (
            1,
            "New model: thank you".to_string(),
            format!("Knurl does not know the {model} yet. Send the report and it can be added."),
        ),
        Verdict::NotDecoded => (
            2,
            "Your headphones use the newer Sony service".to_string(),
            "They answered on MDR v2, which Knurl cannot read yet. The report recorded what they sent: sending it helps to add them.".to_string(),
        ),
        Verdict::NothingRead => (
            3,
            "Nothing could be read".to_string(),
            "Check that the headphones are on and connected to this PC, then check again."
                .to_string(),
        ),
    };
    let a = &report.answers;
    let mut facts = Vec::new();
    if let Some(m) = &a.model {
        facts.push(("Model".into(), m.clone()));
    }
    if let Some(f) = &a.firmware {
        facts.push(("Firmware".into(), f.clone()));
    }
    if let Some(p) = &a.protocol_version {
        facts.push(("Protocol".into(), p.clone()));
    }
    if let Some(list) = &a.functions {
        facts.push(("Features declared".into(), list.len().to_string()));
    }
    if let Some(b) = a.battery_percent {
        facts.push(("Battery".into(), format!("{b} %")));
    }
    if let Some(s) = service_line(report) {
        facts.push(("Control service".into(), s));
    }
    ResultView {
        kind,
        title,
        text,
        facts,
        problems: report.problems.clone(),
    }
}

/// The step labels, in order.
const STEPS: [(CheckStep, &str); 4] = [
    (CheckStep::Opening, "Opening the control channel"),
    (
        CheckStep::Asking,
        "Asking for the model, firmware and features",
    ),
    (CheckStep::Listening, "Listening on the newer Sony service"),
    (CheckStep::Finishing, "Putting the report together"),
];

/// Each step's label and state: 0 waiting, 1 running, 2 done, 3 not offered
/// (skipped: a later step has started).
pub fn step_rows(started: &[CheckStep]) -> Vec<(&'static str, i32)> {
    let last = started.last().copied();
    let position = |s: CheckStep| STEPS.iter().position(|(x, _)| *x == s);
    let last_pos = last.and_then(position);
    STEPS
        .iter()
        .enumerate()
        .map(|(i, (step, label))| {
            let state = if Some(*step) == last {
                1
            } else if started.contains(step) {
                2
            } else if last_pos.is_some_and(|l| i < l) {
                3
            } else {
                0
            };
            (*label, state)
        })
        .collect()
}

fn strings(items: impl IntoIterator<Item = String>) -> ModelRc<SharedString> {
    ModelRc::new(VecModel::from(
        items
            .into_iter()
            .map(SharedString::from)
            .collect::<Vec<_>>(),
    ))
}

/// Hands the whole state to the window.
pub fn apply(ui: &MainWindow, flow: &Flow) {
    ui.set_page(match flow.page {
        Page::Welcome => 0,
        Page::Choose => 1,
        Page::Checking => 2,
        Page::Result => 3,
        Page::Send => 4,
    });
    let (state, devices, error) = match &flow.search {
        Search::Searching => (0, Vec::new(), String::new()),
        Search::Found(names) => (1, names.clone(), String::new()),
        Search::NoneFound => (2, Vec::new(), String::new()),
        Search::Failed(reason) => (3, Vec::new(), reason.clone()),
    };
    ui.set_search_state(state);
    ui.set_devices(strings(devices));
    ui.set_search_error(error.into());

    ui.set_device_name(
        flow.chosen
            .as_ref()
            .map_or_else(String::new, |(_, n)| n.clone())
            .into(),
    );
    let rows: Vec<StepRow> = step_rows(&flow.steps)
        .into_iter()
        .map(|(label, state)| StepRow {
            label: label.into(),
            state,
        })
        .collect();
    ui.set_steps(ModelRc::new(VecModel::from(rows)));

    if let Some(report) = &flow.report {
        let r = result_view(report);
        ui.set_result_kind(r.kind);
        ui.set_result_title(r.title.into());
        ui.set_result_text(r.text.into());
        let facts: Vec<Fact> = r
            .facts
            .into_iter()
            .map(|(label, value)| Fact {
                label: label.into(),
                value: value.into(),
            })
            .collect();
        ui.set_facts(ModelRc::new(VecModel::from(facts)));
        ui.set_problems(strings(r.problems));
        ui.set_data(report.to_json().into());
    }
    ui.set_show_data(flow.show_data);
    let (saved, failed) = match &flow.saved {
        Some(Ok(path)) => (path.clone(), String::new()),
        Some(Err(reason)) => (String::new(), reason.clone()),
        None => (String::new(), String::new()),
    };
    ui.set_saved_path(saved.into());
    ui.set_save_error(failed.into());
    ui.set_version(env!("CARGO_PKG_VERSION").into());
}

#[cfg(test)]
mod tests {
    use super::*;
    use knurl_report::{Answers, Function, REPORT_FORMAT, Services};

    fn report(verdict: Verdict) -> Report {
        Report {
            format: REPORT_FORMAT,
            tool: "knurl-probe 0.1.0".into(),
            date: "2026-11-18".into(),
            os: "Windows 10.0.26100".into(),
            device_name: RENAMED.into(),
            services: Services {
                v1: ServiceResult::Opened,
                v2: ServiceResult::NotOffered,
            },
            answers: Answers::default(),
            verdict,
            frames: Vec::new(),
            problems: Vec::new(),
        }
    }

    #[test]
    fn each_verdict_reads_in_plain_words() {
        let mut r = report(Verdict::Supported);
        r.answers.model = Some("WH-1000XM3".into());
        assert_eq!(
            result_view(&r).title,
            "Knurl already supports your WH-1000XM3"
        );
        let mut r = report(Verdict::NewModel);
        r.answers.model = Some("WH-1000XM6".into());
        let v = result_view(&r);
        assert_eq!((v.kind, v.title.as_str()), (1, "New model: thank you"));
        assert!(v.text.contains("WH-1000XM6"));
        assert_eq!(result_view(&report(Verdict::NotDecoded)).kind, 2);
        let v = result_view(&report(Verdict::NothingRead));
        assert_eq!((v.kind, v.title.as_str()), (3, "Nothing could be read"));
    }

    #[test]
    fn a_renamed_device_is_not_named() {
        let r = report(Verdict::NotDecoded);
        assert_eq!(model_name(&r), "headphones");
        let mut r = report(Verdict::NotDecoded);
        r.device_name = "WF-1000XM5".into();
        assert_eq!(model_name(&r), "WF-1000XM5");
    }

    #[test]
    fn what_was_read_lists_only_what_was_answered() {
        let mut r = report(Verdict::NewModel);
        assert_eq!(
            result_view(&r).facts,
            [("Control service".to_string(), "MDR v1".to_string())]
        );
        r.answers = Answers {
            model: Some("WH-1000XM6".into()),
            firmware: Some("1.2.0".into()),
            protocol_version: Some("0x4010".into()),
            functions: Some(vec![Function {
                code: "0x11".into(),
                name: None,
            }]),
            battery_percent: Some(80),
            ..Answers::default()
        };
        r.services.v2 = ServiceResult::Opened;
        r.problems = vec!["could not open MDR v2: timeout".into()];
        let v = result_view(&r);
        let labels: Vec<&str> = v.facts.iter().map(|(l, _)| l.as_str()).collect();
        assert_eq!(
            labels,
            [
                "Model",
                "Firmware",
                "Protocol",
                "Features declared",
                "Battery",
                "Control service"
            ]
        );
        assert_eq!(v.facts[3].1, "1");
        assert_eq!(v.facts[4].1, "80 %");
        assert_eq!(v.facts[5].1, "MDR v1 and MDR v2");
        assert_eq!(v.problems.len(), 1);
    }

    #[test]
    fn step_rows_show_running_done_skipped_and_waiting() {
        let states = |s: &[CheckStep]| -> Vec<i32> { step_rows(s).iter().map(|r| r.1).collect() };
        assert_eq!(states(&[]), [0, 0, 0, 0]);
        assert_eq!(states(&[CheckStep::Opening]), [1, 0, 0, 0]);
        assert_eq!(
            states(&[CheckStep::Opening, CheckStep::Asking]),
            [2, 1, 0, 0]
        );
        // No MDR v2: listening is skipped when finishing starts.
        assert_eq!(
            states(&[CheckStep::Opening, CheckStep::Asking, CheckStep::Finishing]),
            [2, 2, 3, 1]
        );
        // Only MDR v2.
        assert_eq!(
            states(&[CheckStep::Opening, CheckStep::Listening]),
            [2, 3, 1, 0]
        );
    }
}
