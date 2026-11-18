//! The read-only check of one pair of headphones, shared by `knurl-probe run`
//! and the app. The Bluetooth address is used to connect and kept private:
//! [`Headphones`] never shows it, and the report never holds it.

use std::sync::Arc;
use std::sync::mpsc::{RecvTimeoutError, channel};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use knurl_device::bluetooth::{MDR_V1, MDR_V2};
use knurl_device::winsock::{RfcommSocket, connect_rfcomm};
use knurl_report::{Collector, Outcome, Report, ReportInput, Service, ServiceResult, Services};

use crate::bluetooth::find_services;
use crate::{CheckStep, ServiceKind, TOOL};

const POLL: Duration = Duration::from_millis(50);

/// A pair of headphones with the Sony services Windows lists for it.
pub struct Headphones {
    /// The name Windows shows (the user may have renamed it: it is shown on
    /// this PC only, and the report keeps it only if it is a Sony model name).
    pub name: String,
    address: u64,
    v1: bool,
    v2: bool,
}

impl std::fmt::Debug for Headphones {
    // No address, even in debug output.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Headphones")
            .field("name", &self.name)
            .field("v1", &self.v1)
            .field("v2", &self.v2)
            .finish()
    }
}

/// The paired headphones that expose a Sony control service, one entry per
/// pair. The headphones must be on: Windows asks them live.
///
/// # Errors
/// The Windows Bluetooth lookup failed.
pub fn find_headphones() -> Result<Vec<Headphones>, String> {
    let mut list: Vec<Headphones> = Vec::new();
    for found in find_services().map_err(|e| e.message().to_string())? {
        let index = match list.iter().position(|c| c.address == found.address) {
            Some(i) => i,
            None => {
                list.push(Headphones {
                    name: found.device_name.clone(),
                    address: found.address,
                    v1: false,
                    v2: false,
                });
                list.len() - 1
            }
        };
        match found.kind {
            ServiceKind::V1 => list[index].v1 = true,
            ServiceKind::V2 => list[index].v2 = true,
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
pub fn os_version() -> String {
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

/// Seconds since 1970, for the report's date and file name.
pub fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// Checks `target` read-only on every Sony service Windows offers for it and
/// builds the report. `progress` is told each step as it starts. What fails
/// is in the report's `problems`, never hidden.
pub fn run(target: &Headphones, mut progress: impl FnMut(CheckStep)) -> Report {
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
    progress(CheckStep::Opening);
    if target.v1 {
        progress(CheckStep::Asking);
    }
    let v1 = try_service(target.v1, MDR_V1, Service::V1, "MDR v1");
    if target.v2 {
        progress(CheckStep::Listening);
    }
    let v2 = try_service(target.v2, MDR_V2, Service::V2, "MDR v2");
    progress(CheckStep::Finishing);
    knurl_report::build(ReportInput {
        tool: TOOL,
        date: &knurl_report::date_from_unix(now_secs()),
        os: &os_version(),
        windows_name: &target.name,
        services: Services { v1, v2 },
        outcomes,
        problems,
    })
}
