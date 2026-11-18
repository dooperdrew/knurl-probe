//! `knurl-probe run`: checks one pair of headphones read-only and writes
//! the report (crate `knurl-report`) to a file, with a short summary. The
//! Bluetooth address is used to connect and never written anywhere.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::{RecvTimeoutError, channel};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use knurl_device::bluetooth::{MDR_V1, MDR_V2};
use knurl_device::winsock::{RfcommSocket, connect_rfcomm};
use knurl_report::{Collector, Outcome, ReportInput, Service, ServiceResult, Services};

use crate::ServiceKind;
use crate::bluetooth::find_services;

const POLL: Duration = Duration::from_millis(50);

pub struct Options {
    /// Part of the device name; needed only when several are paired.
    pub device: Option<String>,
    /// Folder for the report. Default: `reports/`.
    pub out_dir: Option<PathBuf>,
}

/// A pair of headphones with the Sony services Windows lists for it.
struct Candidate {
    name: String,
    address: u64,
    v1: bool,
    v2: bool,
}

fn candidates() -> Result<Vec<Candidate>, Box<dyn std::error::Error>> {
    let mut list: Vec<Candidate> = Vec::new();
    for found in find_services()? {
        let entry = match list.iter_mut().position(|c| c.address == found.address) {
            Some(i) => &mut list[i],
            None => {
                list.push(Candidate {
                    name: found.device_name.clone(),
                    address: found.address,
                    v1: false,
                    v2: false,
                });
                list.last_mut().expect("just pushed")
            }
        };
        match found.kind {
            ServiceKind::V1 => entry.v1 = true,
            ServiceKind::V2 => entry.v2 = true,
        }
    }
    Ok(list)
}

/// Runs one collector on an open socket until it is done.
fn converse(socket: RfcommSocket, service: Service) -> Outcome {
    let socket = Arc::new(socket);
    let (tx, rx) = channel::<Result<Vec<u8>, String>>();
    let reader = {
        let socket = socket.clone();
        std::thread::spawn(move || {
            let mut buf = vec![0u8; 1024];
            loop {
                match socket.recv(&mut buf) {
                    Ok(0) => {
                        let _ = tx.send(Err("the headphones closed the connection".into()));
                        return;
                    }
                    Ok(n) => {
                        if tx.send(Ok(buf[..n].to_vec())).is_err() {
                            return;
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(Err(e));
                        return;
                    }
                }
            }
        })
    };
    let origin = Instant::now();
    let now = || origin.elapsed().as_millis() as u64;
    let mut collector = Collector::new(service);
    let mut problem = None;
    let mut out = collector.start(now());
    while problem.is_none() && !collector.is_done(now()) {
        if !out.is_empty()
            && let Err(e) = socket.send_all(&out)
        {
            problem = Some(format!("could not write to the headphones: {e}"));
            break;
        }
        out = match rx.recv_timeout(POLL) {
            Ok(Ok(bytes)) => collector.receive(&bytes, now()),
            Ok(Err(e)) => {
                problem = Some(e);
                Vec::new()
            }
            Err(RecvTimeoutError::Timeout) => Vec::new(),
            Err(RecvTimeoutError::Disconnected) => {
                problem = Some("the reader stopped".into());
                Vec::new()
            }
        };
        out.extend(collector.poll(now()));
    }
    // The last ACKs, so the headphones do not repeat their final answer.
    if problem.is_none() && !out.is_empty() {
        let _ = socket.send_all(&out);
    }
    // Ends the reader's recv.
    socket.shutdown();
    let _ = reader.join();
    let mut outcome = collector.finish();
    outcome.problems.extend(problem);
    outcome
}

/// "Windows 10.0.26100.2314" from `ver`, or a fixed text when it fails.
fn os_version() -> String {
    std::process::Command::new("cmd")
        .args(["/C", "ver"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|s| {
            let start = s.find("Version ")? + "Version ".len();
            let end = start + s[start..].find(']')?;
            Some(format!("Windows {}", s[start..end].trim()))
        })
        .unwrap_or_else(|| "Windows (version not read)".into())
}

pub fn run(opts: Options) -> Result<(), Box<dyn std::error::Error>> {
    let mut list = candidates()?;
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

    let mut outcomes = Vec::new();
    let mut problems = Vec::new();
    let mut try_service = |offered: bool, uuid, service, label: &str| -> ServiceResult {
        if !offered {
            return ServiceResult::NotOffered;
        }
        match connect_rfcomm(target.address, uuid) {
            Ok(socket) => {
                outcomes.push(converse(socket, service));
                ServiceResult::Opened
            }
            Err(reason) => {
                problems.push(format!("could not open {label}: {reason}"));
                ServiceResult::Failed { reason }
            }
        }
    };
    let v1 = try_service(target.v1, MDR_V1, Service::V1, "MDR v1");
    let v2 = try_service(target.v2, MDR_V2, Service::V2, "MDR v2");

    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let report = knurl_report::build(ReportInput {
        tool: concat!("knurl-probe ", env!("CARGO_PKG_VERSION")),
        date: &knurl_report::date_from_unix(secs),
        os: &os_version(),
        windows_name: &target.name,
        services: Services { v1, v2 },
        outcomes,
        problems,
    });
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
