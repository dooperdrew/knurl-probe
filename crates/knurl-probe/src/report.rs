//! `knurl-probe-cli run`: checks one pair of headphones read-only and writes
//! the report (crate `knurl-report`) to a file, with a short summary. The
//! Bluetooth address is used to connect and never written anywhere.

use std::path::PathBuf;

use knurl_probe::check;

pub struct Options {
    /// Part of the device name; needed only when several are paired.
    pub device: Option<String>,
    /// Folder for the report. Default: `reports/`.
    pub out_dir: Option<PathBuf>,
}

pub fn run(opts: Options) -> Result<(), Box<dyn std::error::Error>> {
    let mut list = check::find_headphones()?;
    if let Some(needle) = &opts.device {
        let needle = needle.to_lowercase();
        list.retain(|c| c.name.to_lowercase().contains(&needle));
    }
    let target = match list.len() {
        0 => {
            return Err("no paired headphones with a Sony control service found. \
                Turn them on, check they are paired in Windows Bluetooth settings, \
                and try again"
                .into());
        }
        1 => list.remove(0),
        _ => {
            let names: Vec<&str> = list.iter().map(|c| c.name.as_str()).collect();
            return Err(format!(
                "several headphones found ({}): choose one with --device",
                names.join(", ")
            )
            .into());
        }
    };
    println!("Checking {} (read-only)...", target.name);
    let report = check::run(&target, |_| {});
    let secs = check::now_secs();
    let dir = opts.out_dir.unwrap_or_else(|| PathBuf::from("reports"));
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(knurl_report::file_name(&report, secs));
    std::fs::write(&path, report.to_json())?;

    println!();
    for line in knurl_report::summary(&report) {
        println!("{line}");
    }
    // Already described: a confirmation, or exactly the differences worth
    // reporting.
    if let Some(d) = report
        .answers
        .model
        .as_deref()
        .and_then(knurl_report::description::find_model)
    {
        let found = knurl_report::compare::compare(&report, &d);
        if found.is_empty() {
            println!(
                "It agrees with the description {} ({}).",
                d.id,
                d.verification.status.id()
            );
        } else {
            println!("It differs from the description {}:", d.id);
            for line in crate::files::mismatch_lines(&found) {
                println!("{line}");
            }
        }
    }
    println!();
    println!("Report written to {}", path.display());
    println!("It holds no Bluetooth address and no personal names. Attach it to a");
    println!(
        "\"New headphone model\" issue: https://github.com/DooperDrew/knurl-probe/issues/new/choose"
    );
    Ok(())
}
