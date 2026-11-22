//! The read-only check of one pair of headphones, shared by `knurl-probe-cli run`
//! and the app. The Bluetooth address is used to connect and kept private:
//! [`Headphones`] never shows it, and the report never holds it.

use std::sync::Arc;
use std::sync::mpsc::{RecvTimeoutError, channel};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use knurl_device::bluetooth::{MDR_V1, MDR_V2};
use knurl_device::winsock::{RfcommSocket, connect_rfcomm};
use knurl_report::{Collector, Outcome, Report, ReportInput, Service, ServiceResult, Services};

use crate::bluetooth::{find_services, paired_classic_devices};
use crate::{CheckStep, ServiceKind, TOOL};

const POLL: Duration = Duration::from_millis(50);

/// Whether Windows lists a Sony service for a pair of headphones.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Offer {
    /// The WinRT lookup listed it.
    Listed,
    /// Not listed, but the lookup cannot be trusted: on our WH-1000XM3
    /// (2026-11-07) WinRT listed no MDR service at all, while Winsock opened
    /// MDR v1. The check opens it with Winsock to find out.
    Unknown,
    /// The lookup listed the other service only.
    No,
}

/// A pair of headphones with the Sony services Windows lists for it.
pub struct Headphones {
    /// The name Windows shows (the user may have renamed it: it is shown on
    /// this PC only, and the report keeps it only if it is a Sony model name).
    pub name: String,
    address: u64,
    v1: Offer,
    v2: Offer,
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

/// The paired headphones to check, one entry per pair: those for which
/// WinRT lists a Sony control service, and the paired classic devices with a
/// Sony model name, whose services are found out by opening them (WinRT does
/// not list MDR v1 on the WH-1000XM3). Headphones renamed in Windows and not
/// listed by WinRT are not found. The headphones must be on.
///
/// # Errors
/// Both Windows Bluetooth lookups failed.
pub fn find_headphones() -> Result<Vec<Headphones>, String> {
    let services = find_services().map_err(|e| e.message().to_string());
    let paired = paired_classic_devices()
        .map_err(|e| e.message().to_string())
        .map(|devices| {
            devices
                .iter()
                .filter_map(|d| {
                    Some((d.BluetoothAddress().ok()?, d.Name().ok()?.to_string_lossy()))
                })
                .collect::<Vec<_>>()
        });
    let (services, paired) = match (services, paired) {
        (Err(a), Err(b)) => return Err(format!("{a}; {b}")),
        (s, p) => (s.unwrap_or_default(), p.unwrap_or_default()),
    };
    let listed = services
        .iter()
        .map(|f| (f.address, f.device_name.clone(), f.kind))
        .collect::<Vec<_>>();
    Ok(merge(&listed, &paired))
}

/// One entry per address: the services WinRT listed, then the Sony-named
/// paired devices it did not list, with both services to be found out.
fn merge(listed: &[(u64, String, ServiceKind)], paired: &[(u64, String)]) -> Vec<Headphones> {
    let mut list: Vec<Headphones> = Vec::new();
    for (address, name, kind) in listed {
        let index = match list.iter().position(|c| c.address == *address) {
            Some(i) => i,
            None => {
                list.push(Headphones {
                    name: name.clone(),
                    address: *address,
                    v1: Offer::No,
                    v2: Offer::No,
                });
                list.len() - 1
            }
        };
        match kind {
            ServiceKind::V1 => list[index].v1 = Offer::Listed,
            ServiceKind::V2 => list[index].v2 = Offer::Listed,
        }
    }
    for (address, name) in paired {
        let sony = knurl_report::sanitize_device_name(name) == name.trim();
        if sony && !list.iter().any(|c| c.address == *address) {
            list.push(Headphones {
                name: name.clone(),
                address: *address,
                v1: Offer::Unknown,
                v2: Offer::Unknown,
            });
        }
    }
    list
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

/// "Windows 10.0.26100.2314", read from the registry, or a fixed text when
/// it cannot be read. No child process: `cmd /C ver` flashed a console from
/// the app and printed "Versione" or "Versión" on other Windows languages.
pub fn os_version() -> String {
    use windows::Win32::System::Registry::{
        HKEY_LOCAL_MACHINE, RRF_RT_REG_DWORD, RRF_RT_REG_SZ, RegGetValueW,
    };
    use windows::core::{HSTRING, PCWSTR};
    const KEY: &str = r"SOFTWARE\Microsoft\Windows NT\CurrentVersion";
    let key = HSTRING::from(KEY);
    let dword = |name: &str| -> Option<u32> {
        let name = HSTRING::from(name);
        let mut value = 0u32;
        let mut size = std::mem::size_of::<u32>() as u32;
        // SAFETY: the key and value names are valid NUL-terminated strings
        // for the call; `value` and `size` describe a writable u32.
        let status = unsafe {
            RegGetValueW(
                HKEY_LOCAL_MACHINE,
                PCWSTR(key.as_ptr()),
                PCWSTR(name.as_ptr()),
                RRF_RT_REG_DWORD,
                None,
                Some((&raw mut value).cast()),
                Some(&raw mut size),
            )
        };
        status.is_ok().then_some(value)
    };
    let text = |name: &str| -> Option<String> {
        let name = HSTRING::from(name);
        let mut buf = [0u16; 64];
        let mut size = std::mem::size_of_val(&buf) as u32;
        // SAFETY: as above; `buf` and `size` describe a writable buffer of
        // `size` bytes, and RegGetValueW NUL-terminates what it writes.
        let status = unsafe {
            RegGetValueW(
                HKEY_LOCAL_MACHINE,
                PCWSTR(key.as_ptr()),
                PCWSTR(name.as_ptr()),
                RRF_RT_REG_SZ,
                None,
                Some(buf.as_mut_ptr().cast()),
                Some(&raw mut size),
            )
        };
        if status.is_err() {
            return None;
        }
        let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
        Some(String::from_utf16_lossy(&buf[..len]))
    };
    format_version(
        dword("CurrentMajorVersionNumber"),
        dword("CurrentMinorVersionNumber"),
        text("CurrentBuildNumber").as_deref(),
        dword("UBR"),
    )
}

/// "Windows 10.0.26100.2314" from the registry values; the update number
/// (UBR) may be missing. Anything else missing: a fixed text.
fn format_version(
    major: Option<u32>,
    minor: Option<u32>,
    build: Option<&str>,
    ubr: Option<u32>,
) -> String {
    let build = build
        .map(str::trim)
        .filter(|b| !b.is_empty() && b.bytes().all(|c| c.is_ascii_digit()));
    match (major, minor, build) {
        (Some(major), Some(minor), Some(build)) => match ubr {
            Some(ubr) => format!("Windows {major}.{minor}.{build}.{ubr}"),
            None => format!("Windows {major}.{minor}.{build}"),
        },
        _ => "Windows (version not read)".into(),
    }
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
    let v1_answered = std::cell::Cell::new(false);
    let mut try_service = |offer: Offer, uuid, service, label: &str| -> ServiceResult {
        if offer == Offer::No {
            return ServiceResult::NotOffered;
        }
        match connect_rfcomm(target.address, uuid) {
            Ok(socket) => {
                let outcome = converse(socket, service);
                if service == Service::V1 {
                    v1_answered.set(!outcome.answers.is_empty());
                }
                outcomes.push(outcome);
                ServiceResult::Opened
            }
            // Not listed and not there: a model without this service.
            Err(_) if offer == Offer::Unknown => ServiceResult::NotOffered,
            Err(reason) => {
                problems.push(format!("could not open {label}: {reason}"));
                ServiceResult::Failed { reason }
            }
        }
    };
    progress(CheckStep::Opening);
    if target.v1 != Offer::No {
        progress(CheckStep::Asking);
    }
    let v1 = try_service(target.v1, MDR_V1, Service::V1, "MDR v1");
    // Found by name only and v1 answered: v1 is enough, v2 is not tried.
    let v2 = if target.v2 == Offer::Unknown && v1_answered.get() {
        ServiceResult::NotTried
    } else {
        if target.v2 != Offer::No {
            progress(CheckStep::Listening);
        }
        try_service(target.v2, MDR_V2, Service::V2, "MDR v2")
    };
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_windows_version_is_built_from_the_registry_values() {
        assert_eq!(
            format_version(Some(10), Some(0), Some("26100"), Some(2314)),
            "Windows 10.0.26100.2314"
        );
        assert_eq!(
            format_version(Some(10), Some(0), Some("19045"), None),
            "Windows 10.0.19045"
        );
        for (major, build) in [
            (None, Some("19045")),
            (Some(10), None),
            (Some(10), Some("x1")),
        ] {
            assert_eq!(
                format_version(major, Some(0), build, Some(1)),
                "Windows (version not read)"
            );
        }
    }

    #[test]
    fn the_windows_version_of_this_pc_is_read() {
        assert!(
            os_version().starts_with("Windows 10.0."),
            "{}",
            os_version()
        );
    }

    #[test]
    fn sony_named_devices_winrt_missed_are_kept_to_be_opened() {
        // As on our WH-1000XM3: WinRT lists nothing, the device is paired.
        let list = merge(&[], &[(1, "WH-1000XM3".into()), (2, "Keyboard".into())]);
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "WH-1000XM3");
        assert_eq!((list[0].v1, list[0].v2), (Offer::Unknown, Offer::Unknown));
    }

    #[test]
    fn listed_services_win_over_the_name_and_unknown_names_are_left_out() {
        let listed = [
            (1, "WH-1000XM4".to_string(), ServiceKind::V1),
            (3, "My cans".to_string(), ServiceKind::V2),
        ];
        let paired = [(1, "WH-1000XM4".into()), (4, "My other cans".into())];
        let list = merge(&listed, &paired);
        assert_eq!(list.len(), 2);
        assert_eq!((list[0].v1, list[0].v2), (Offer::Listed, Offer::No));
        // Renamed, but WinRT listed its service: kept.
        assert_eq!((list[1].v1, list[1].v2), (Offer::No, Offer::Listed));
    }
}
