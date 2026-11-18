//! The commands that work on files only, on any system: `known`, `check`,
//! `draft` and `validate`. They read reports and descriptions; they never
//! talk to headphones.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use knurl_report::Report;
use knurl_report::compare::{Mismatch, compare, draft};
use knurl_report::description::{Description, KNOWN, known};

type Error = Box<dyn std::error::Error>;

fn read(path: &Path) -> Result<String, Error> {
    std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()).into())
}

fn read_report(path: &Path) -> Result<Report, Error> {
    Report::from_json(&read(path)?).map_err(|e| format!("{}: {e}", path.display()).into())
}

fn read_description(path: &Path) -> Result<Description, Error> {
    Description::from_toml(&read(path)?).map_err(|e| format!("{}: {e}", path.display()).into())
}

/// Lines for a list of differences, empty when there are none.
pub fn mismatch_lines(found: &[Mismatch]) -> Vec<String> {
    found
        .iter()
        .map(|m| {
            format!(
                "  {}: the description says {}, the report found {}",
                m.what, m.expected, m.found
            )
        })
        .collect()
}

/// `known`: the descriptions that ship with the probe.
pub fn known_models() -> ExitCode {
    let models = known();
    for d in &models {
        println!(
            "{:<18} {:<10} {}  ({})",
            d.id,
            d.verification.status.id(),
            d.name,
            d.service.id()
        );
    }
    if models.len() == KNOWN.len() {
        ExitCode::SUCCESS
    } else {
        // Cannot happen with the files the tests check; said, never hidden.
        eprintln!("error: a shipped description does not read");
        ExitCode::FAILURE
    }
}

/// `check`: a report against a description file, or against the shipped
/// description of the report's model. Differences end with exit code 1.
pub fn check(report: &Path, against: Option<&Path>) -> Result<ExitCode, Error> {
    let report = read_report(report)?;
    let description = match against {
        Some(path) => read_description(path)?,
        None => {
            let model = report
                .answers
                .model
                .as_deref()
                .ok_or("the report holds no model name: give a description with --against")?;
            knurl_report::description::find_model(model).ok_or_else(|| {
                format!("no description of {model} ships with the probe: draft one with `draft`")
            })?
        }
    };
    let found = compare(&report, &description);
    if found.is_empty() {
        println!(
            "The report agrees with {} ({}).",
            description.id,
            description.verification.status.id()
        );
        Ok(ExitCode::SUCCESS)
    } else {
        println!(
            "The report differs from {} in {} point(s):",
            description.id,
            found.len()
        );
        for line in mismatch_lines(&found) {
            println!("{line}");
        }
        Ok(ExitCode::from(1))
    }
}

/// `draft`: a first description from a report, printed or written to a file.
pub fn draft_file(
    report: &Path,
    id: &str,
    name: &str,
    out: Option<&PathBuf>,
) -> Result<ExitCode, Error> {
    let report = read_report(report)?;
    let d = draft(&report, id, name)?;
    let text = d.to_toml();
    match out {
        Some(path) => {
            std::fs::write(path, &text).map_err(|e| format!("{}: {e}", path.display()))?;
            println!(
                "Description written to {}. Review it before opening a pull request.",
                path.display()
            );
        }
        None => print!("{text}"),
    }
    Ok(ExitCode::SUCCESS)
}

/// `validate`: checks description files; any error ends with exit code 1.
pub fn validate(files: &[PathBuf]) -> ExitCode {
    let mut failed = false;
    for path in files {
        match read_description(path) {
            Ok(d) => {
                let named = path.file_stem().and_then(|s| s.to_str()) == Some(d.id.as_str());
                if named {
                    println!("{}: ok ({})", path.display(), d.verification.status.id());
                } else {
                    failed = true;
                    println!(
                        "{}: the file must be named {}.toml after its id",
                        path.display(),
                        d.id
                    );
                }
            }
            Err(e) => {
                failed = true;
                println!("{e}");
            }
        }
    }
    if failed {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("knurl-probe-files-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        dir.join(name)
    }

    #[test]
    fn validate_accepts_the_shipped_files_and_names_bad_ones() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../headphones");
        let shipped: Vec<PathBuf> = KNOWN.iter().map(|(f, _)| root.join(f)).collect();
        assert_eq!(validate(&shipped), ExitCode::SUCCESS);

        let wrong_name = scratch("other-name.toml");
        std::fs::write(&wrong_name, KNOWN[0].1).expect("write");
        assert_eq!(validate(&[wrong_name]), ExitCode::from(1));

        let broken = scratch("broken.toml");
        std::fs::write(&broken, "format = 1\n").expect("write");
        assert_eq!(validate(&[broken]), ExitCode::from(1));

        assert_eq!(validate(&[scratch("missing.toml")]), ExitCode::from(1));
    }

    #[test]
    fn check_fails_on_a_report_that_is_not_json() {
        let path = scratch("not-a-report.json");
        std::fs::write(&path, "hello").expect("write");
        assert!(check(&path, None).is_err());
    }

    #[test]
    fn mismatches_read_as_sentences() {
        let lines = mismatch_lines(&[Mismatch {
            what: "firmware".into(),
            expected: "4.5.2".into(),
            found: "4.6.0".into(),
        }]);
        assert_eq!(
            lines,
            ["  firmware: the description says 4.5.2, the report found 4.6.0"]
        );
    }
}
