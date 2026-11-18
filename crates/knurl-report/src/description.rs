//! The description of one headphone model: `headphones/<id>.toml`. It says
//! how the model answers on Sony's control service, and how far that is
//! checked. Knurl supports the models described here; a new model is added
//! by adding its file, drafted from a report ([`crate::compare::draft`]).
//!
//! Same concept as Keyfold Probe's keyboard descriptions: a description never
//! claims more than its reports show (`unverified` until a report agrees).

use serde::{Deserialize, Serialize};

/// Version of the description layout.
pub const DESCRIPTION_FORMAT: u32 = 1;

/// How far a description has been checked.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Status {
    /// Written from documents or other tools: no knurl-probe report backs it.
    Unverified,
    /// One knurl-probe report agrees with it.
    Reported,
    /// Two or more reports, from different people, agree.
    Verified,
}

impl Status {
    pub fn id(self) -> &'static str {
        match self {
            Status::Unverified => "unverified",
            Status::Reported => "reported",
            Status::Verified => "verified",
        }
    }
}

/// Who checked the description, and when.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Verification {
    pub status: Status,
    /// Number of knurl-probe reports behind it.
    #[serde(default)]
    pub reports: u32,
    /// Date of the last check, `YYYY-MM-DD`.
    #[serde(default)]
    pub last: String,
    /// Where the facts come from.
    #[serde(default)]
    pub source: String,
}

/// The Sony control service the model answers on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ServiceId {
    MdrV1,
    MdrV2,
}

impl ServiceId {
    pub fn id(self) -> &'static str {
        match self {
            ServiceId::MdrV1 => "mdr-v1",
            ServiceId::MdrV2 => "mdr-v2",
        }
    }
}

/// One headphone model.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Description {
    /// [`DESCRIPTION_FORMAT`].
    pub format: u32,
    /// Same as the file name: lowercase letters, digits, dashes.
    pub id: String,
    /// Name for people.
    pub name: String,
    /// The model name the headphones answer with (and Windows' default
    /// device name), e.g. `WH-1000XM3`.
    pub model: String,
    pub service: ServiceId,
    pub verification: Verification,
    /// Protocol version from the first answer, e.g. `0x4010`.
    #[serde(default)]
    pub protocol_version: Option<u16>,
    /// Firmware versions seen in reports.
    #[serde(default)]
    pub firmware: Vec<String>,
    /// Feature codes the headphones declare.
    #[serde(default)]
    pub functions: Vec<u8>,
    /// Anything a person should know.
    #[serde(default)]
    pub note: String,
}

impl Description {
    /// Reads and checks a description file.
    ///
    /// # Errors
    /// Not TOML, a newer format, or an inconsistent description.
    pub fn from_toml(text: &str) -> Result<Description, String> {
        let d: Description = toml::from_str(text).map_err(|e| e.to_string())?;
        d.validate()?;
        Ok(d)
    }

    /// Checks what a file cannot say by its shape alone.
    ///
    /// # Errors
    /// What is wrong, in words.
    pub fn validate(&self) -> Result<(), String> {
        if self.format > DESCRIPTION_FORMAT {
            return Err(format!(
                "description format {} is newer than this tool ({DESCRIPTION_FORMAT})",
                self.format
            ));
        }
        let id_ok = !self.id.is_empty()
            && self
                .id
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
        if !id_ok {
            return Err(format!(
                "id \"{}\" must be lowercase letters, digits and dashes",
                self.id
            ));
        }
        if self.name.trim().is_empty() {
            return Err("name is empty".into());
        }
        if self.model.trim().is_empty() {
            return Err("model is empty".into());
        }
        let needed = match self.verification.status {
            Status::Unverified => 0,
            Status::Reported => 1,
            Status::Verified => 2,
        };
        if self.verification.reports < needed {
            return Err(format!(
                "verification: a {} description needs reports >= {needed}",
                self.verification.status.id()
            ));
        }
        for (i, code) in self.functions.iter().enumerate() {
            if self.functions[..i].contains(code) {
                return Err(format!("functions: 0x{code:02X} is listed twice"));
            }
        }
        for (i, f) in self.firmware.iter().enumerate() {
            if f.trim().is_empty() {
                return Err("firmware: an empty version".into());
            }
            if self.firmware[..i].contains(f) {
                return Err(format!("firmware: \"{f}\" is listed twice"));
            }
        }
        Ok(())
    }

    /// The description as TOML, codes in hexadecimal as people write them.
    pub fn to_toml(&self) -> String {
        let q = |s: &str| toml_string(s);
        let mut out = String::new();
        out.push_str(&format!("format = {}\n", self.format));
        out.push_str(&format!("id = {}\n", q(&self.id)));
        out.push_str(&format!("name = {}\n", q(&self.name)));
        out.push_str(&format!("model = {}\n", q(&self.model)));
        out.push_str(&format!("service = {}\n", q(self.service.id())));
        if let Some(v) = self.protocol_version {
            out.push_str(&format!("protocol_version = 0x{v:04X}\n"));
        }
        if !self.firmware.is_empty() {
            let list: Vec<String> = self.firmware.iter().map(|f| q(f)).collect();
            out.push_str(&format!("firmware = [{}]\n", list.join(", ")));
        }
        if !self.functions.is_empty() {
            let list: Vec<String> = self
                .functions
                .iter()
                .map(|c| format!("0x{c:02X}"))
                .collect();
            out.push_str(&format!("functions = [{}]\n", list.join(", ")));
        }
        if !self.note.is_empty() {
            out.push_str(&format!("note = {}\n", q(&self.note)));
        }
        out.push_str("\n[verification]\n");
        out.push_str(&format!("status = {}\n", q(self.verification.status.id())));
        out.push_str(&format!("reports = {}\n", self.verification.reports));
        if !self.verification.last.is_empty() {
            out.push_str(&format!("last = {}\n", q(&self.verification.last)));
        }
        if !self.verification.source.is_empty() {
            out.push_str(&format!("source = {}\n", q(&self.verification.source)));
        }
        out
    }
}

/// A TOML basic string, with quotes and backslashes escaped.
fn toml_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\u{:04X}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// The descriptions that ship with the probe, as (file name, text).
pub const KNOWN: [(&str, &str); 2] = [
    (
        "sony-wh-1000xm3.toml",
        include_str!("../../../headphones/sony-wh-1000xm3.toml"),
    ),
    (
        "sony-wh-1000xm4.toml",
        include_str!("../../../headphones/sony-wh-1000xm4.toml"),
    ),
];

/// The shipped descriptions, parsed. A file that does not parse is left out
/// here; the `known_descriptions_are_valid` test makes sure there is none.
pub fn known() -> Vec<Description> {
    KNOWN
        .iter()
        .filter_map(|(_, text)| Description::from_toml(text).ok())
        .collect()
}

/// The shipped description of `model` (case-insensitive), if any.
pub fn find_model(model: &str) -> Option<Description> {
    let wanted = model.trim();
    if wanted.is_empty() {
        return None;
    }
    known()
        .into_iter()
        .find(|d| d.model.eq_ignore_ascii_case(wanted))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Description {
        Description {
            format: 1,
            id: "sony-wh-1000xm9".into(),
            name: "Sony WH-1000XM9 \"test\"".into(),
            model: "WH-1000XM9".into(),
            service: ServiceId::MdrV2,
            verification: Verification {
                status: Status::Reported,
                reports: 1,
                last: "2026-11-18".into(),
                source: "knurl-probe 0.1.0 report".into(),
            },
            protocol_version: Some(0x4010),
            firmware: vec!["1.0.0".into()],
            functions: vec![0x11, 0xC1],
            note: String::new(),
        }
    }

    #[test]
    fn known_descriptions_are_valid_and_named_after_their_file() {
        for (file, text) in KNOWN {
            let d = Description::from_toml(text).unwrap_or_else(|e| panic!("{file}: {e}"));
            assert_eq!(format!("{}.toml", d.id), file);
        }
        assert_eq!(known().len(), KNOWN.len());
        assert_eq!(
            find_model("wh-1000xm3").map(|d| d.id),
            Some("sony-wh-1000xm3".into())
        );
        assert!(find_model("WH-1000XM5").is_none());
        assert!(find_model("").is_none());
    }

    #[test]
    fn a_description_reads_back_what_it_wrote_in_hex() {
        let d = sample();
        let text = d.to_toml();
        assert!(text.contains("protocol_version = 0x4010"));
        assert!(text.contains("functions = [0x11, 0xC1]"));
        assert_eq!(Description::from_toml(&text), Ok(d));
    }

    #[test]
    fn mistakes_are_named() {
        let check = |f: &dyn Fn(&mut Description), words: &str| {
            let mut d = sample();
            f(&mut d);
            let err = d.validate().expect_err("a mistake");
            assert!(err.contains(words), "{err}");
        };
        check(&|d| d.id = "Sony XM9".into(), "lowercase");
        check(&|d| d.model = " ".into(), "model is empty");
        check(&|d| d.verification.reports = 0, "needs reports >= 1");
        check(
            &|d| {
                d.verification.status = Status::Verified;
                d.verification.reports = 1;
            },
            "needs reports >= 2",
        );
        check(&|d| d.functions = vec![0x11, 0x11], "listed twice");
        check(&|d| d.firmware = vec![String::new()], "empty version");
        check(&|d| d.format = 2, "newer");
        assert!(Description::from_toml("format = 1\nid = \"x\"").is_err());
    }
}
