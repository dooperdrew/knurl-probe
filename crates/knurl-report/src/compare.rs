//! A report against a description: do the headphones answer as the
//! description says? And a first description drafted from a report.

use crate::description::{DESCRIPTION_FORMAT, Description, ServiceId, Status, Verification};
use crate::{Report, ServiceResult};

/// One point where report and description disagree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mismatch {
    /// `model`, `service`, `protocol_version`, `functions` or `firmware`.
    pub what: String,
    /// What the description says.
    pub expected: String,
    /// What the report found.
    pub found: String,
}

/// `0x4010` or `0x11` as written in a report.
fn parse_hex(text: &str) -> Option<u32> {
    let digits = text
        .trim()
        .strip_prefix("0x")
        .or_else(|| text.trim().strip_prefix("0X"))?;
    u32::from_str_radix(digits, 16).ok()
}

/// The service the report opened and got answers from: v1 when the answers
/// were decoded, v2 when only the newer service answered.
fn opened_service(report: &Report) -> Option<ServiceId> {
    if report.services.v1 == ServiceResult::Opened && !report.answers.is_empty() {
        Some(ServiceId::MdrV1)
    } else if report.services.v2 == ServiceResult::Opened {
        Some(ServiceId::MdrV2)
    } else if report.services.v1 == ServiceResult::Opened {
        Some(ServiceId::MdrV1)
    } else {
        None
    }
}

/// The feature codes in a report, or `None` when it has none (or one is not
/// a byte in hexadecimal).
fn report_functions(report: &Report) -> Option<Vec<u8>> {
    report
        .answers
        .functions
        .as_ref()?
        .iter()
        .map(|f| parse_hex(&f.code).and_then(|c| u8::try_from(c).ok()))
        .collect()
}

fn hex_list(codes: &[u8]) -> String {
    if codes.is_empty() {
        return "none".into();
    }
    codes
        .iter()
        .map(|c| format!("0x{c:02X}"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Compares a report with a description. An empty list means they agree on
/// everything the report read; what the report could not read (a v2
/// conversation, a missing answer) is not compared.
#[must_use]
pub fn compare(report: &Report, d: &Description) -> Vec<Mismatch> {
    let mut out = Vec::new();
    let a = &report.answers;
    if let Some(model) = &a.model
        && !model.eq_ignore_ascii_case(&d.model)
    {
        out.push(Mismatch {
            what: "model".into(),
            expected: d.model.clone(),
            found: model.clone(),
        });
        return out;
    }
    match opened_service(report) {
        Some(s) if s != d.service => out.push(Mismatch {
            what: "service".into(),
            expected: d.service.id().into(),
            found: s.id().into(),
        }),
        Some(_) => {}
        None => out.push(Mismatch {
            what: "service".into(),
            expected: d.service.id().into(),
            found: "no service opened".into(),
        }),
    }
    if let (Some(want), Some(text)) = (d.protocol_version, &a.protocol_version)
        && parse_hex(text) != Some(u32::from(want))
    {
        out.push(Mismatch {
            what: "protocol_version".into(),
            expected: format!("0x{want:04X}"),
            found: text.clone(),
        });
    }
    if a.functions.is_some() {
        match report_functions(report) {
            Some(found) => {
                let missing: Vec<u8> = d
                    .functions
                    .iter()
                    .copied()
                    .filter(|c| !found.contains(c))
                    .collect();
                let extra: Vec<u8> = found
                    .iter()
                    .copied()
                    .filter(|c| !d.functions.contains(c))
                    .collect();
                if !missing.is_empty() || !extra.is_empty() {
                    out.push(Mismatch {
                        what: "functions".into(),
                        expected: format!("also {}", hex_list(&missing)),
                        found: format!("also {}", hex_list(&extra)),
                    });
                }
            }
            None => out.push(Mismatch {
                what: "functions".into(),
                expected: hex_list(&d.functions),
                found: "codes that are not bytes in hexadecimal".into(),
            }),
        }
    }
    if let Some(f) = &a.firmware
        && !d.firmware.contains(f)
    {
        out.push(Mismatch {
            what: "firmware".into(),
            expected: if d.firmware.is_empty() {
                "none listed".into()
            } else {
                d.firmware.join(", ")
            },
            found: f.clone(),
        });
    }
    out
}

/// A first description from one report, to be reviewed by a person before it
/// is used.
///
/// # Errors
/// The report holds nothing a description can say: no service opened, or
/// no model name.
pub fn draft(report: &Report, id: &str, name: &str) -> Result<Description, String> {
    let service = opened_service(report)
        .ok_or("the report opened no Sony service: there is nothing to describe")?;
    let model = report
        .answers
        .model
        .clone()
        .or_else(|| (report.device_name != crate::RENAMED).then(|| report.device_name.clone()))
        .ok_or("the report holds no model name: give the headphones their Sony name in Windows and check again")?;
    let functions = report_functions(report).unwrap_or_default();
    let d = Description {
        format: DESCRIPTION_FORMAT,
        id: id.to_owned(),
        name: name.to_owned(),
        model,
        service,
        verification: Verification {
            status: Status::Reported,
            reports: 1,
            last: report.date.clone(),
            source: format!("{} report", report.tool),
        },
        protocol_version: report
            .answers
            .protocol_version
            .as_deref()
            .and_then(parse_hex)
            .and_then(|v| u16::try_from(v).ok()),
        firmware: report.answers.firmware.iter().cloned().collect(),
        functions,
        note: String::new(),
    };
    d.validate()?;
    Ok(d)
}
