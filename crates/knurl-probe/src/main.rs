//! `knurl-probe-cli` (package `knurl-probe`): finds Sony headphones paired with Windows and records how
//! they talk, to add new models to Knurl.
//!
//! On Windows, all read-only towards Windows settings:
//! * `run`: check one pair of headphones and write a report (JSON);
//! * `devices`: list paired Bluetooth devices exposing a Sony MDR RFCOMM service;
//! * `audio`: list the Windows audio endpoints with their form factor and
//!   default roles (to tell the stereo and Hands-Free endpoints apart);
//! * `capture`: open the MDR channel and log every byte in both directions to a
//!   JSON-lines file, optionally sending frames typed at the prompt.
//!
//! On any system, with files only (`files`): `known`, `check`, `draft`,
//! `validate` for the model descriptions in `headphones/`.

#[cfg(windows)]
mod audio;
#[cfg(windows)]
use knurl_probe::bluetooth;
#[cfg(windows)]
mod capture;
mod files;
#[cfg(windows)]
mod report;
// The capture log is written only by `capture`.
#[cfg(windows)]
mod log;

use clap::{Parser, Subcommand};
pub use knurl_probe::ServiceKind;

#[derive(Parser)]
#[command(
    name = "knurl-probe-cli",
    version,
    about = "Knurl Probe on the command line: find Sony headphones and record how they talk"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// List paired devices that expose a Sony MDR RFCOMM service.
    Devices {
        /// Also list every RFCOMM service each paired device reports in a live
        /// SDP query (diagnostic, when no MDR service is found).
        #[arg(long)]
        all_services: bool,
    },
    /// Try to open each MDR service with Winsock (AF_BTH) and close it again.
    /// Sends no data to the headphones.
    RfcommCheck {
        /// Part of the paired device name, e.g. "WH-1000XM3".
        #[arg(long)]
        device: String,
    },
    /// Run the same connection thread as the app and print what it reports
    /// (read-only requests only).
    Watch {
        /// How long to watch, in seconds.
        #[arg(long, default_value_t = 30)]
        seconds: u64,
    },
    /// List Windows audio endpoints (render and capture).
    Audio {
        /// Include disabled, unplugged and not-present endpoints.
        #[arg(long)]
        all: bool,
    },
    /// Check one pair of headphones read-only and write a report to attach to
    /// a "New headphone model" issue (no Bluetooth address, no personal names).
    Run {
        /// Part of the device name; needed only when several are paired.
        #[arg(long)]
        device: Option<String>,
        /// Folder for the report. Default: reports/
        #[arg(long)]
        out: Option<std::path::PathBuf>,
    },
    /// List the headphone descriptions that ship with the probe.
    Known,
    /// Compare a report with a description. Exit code 1 when they differ.
    Check {
        /// The report (JSON) written by `run`.
        report: std::path::PathBuf,
        /// A description file. Default: the shipped description of the
        /// report's model.
        #[arg(long)]
        against: Option<std::path::PathBuf>,
    },
    /// Draft a description of a new model from a report, to review by hand.
    Draft {
        /// The report (JSON) written by `run`.
        report: std::path::PathBuf,
        /// File name without .toml: lowercase letters, digits, dashes,
        /// e.g. sony-wh-1000xm6.
        #[arg(long)]
        id: String,
        /// Name for people, e.g. "Sony WH-1000XM6".
        #[arg(long)]
        name: String,
        /// Write the description here. Default: print it.
        #[arg(long)]
        out: Option<std::path::PathBuf>,
    },
    /// Check description files. Exit code 1 when one is wrong.
    Validate {
        #[arg(required = true)]
        files: Vec<std::path::PathBuf>,
    },
    /// Connect to the headphones and record the MDR traffic.
    Capture {
        /// Part of the device name to connect to, e.g. "WH-1000XM4".
        #[arg(long)]
        device: String,
        /// Which MDR service to open. Default: the first one found (v1 first).
        #[arg(long, value_enum)]
        service: Option<ServiceKind>,
        /// Output file (JSON lines). Default: captures/raw/<time>-<device>.jsonl
        #[arg(long)]
        out: Option<std::path::PathBuf>,
        /// Do not acknowledge frames received from the headphones.
        #[arg(long)]
        no_ack: bool,
        /// Hex payload sent as a DATA_MDR frame right after connecting
        /// (e.g. "00 00"). Repeat the flag to send several.
        #[arg(long = "send-on-connect")]
        send_on_connect: Vec<String>,
        /// Run the guided test session (prints instructions and writes notes).
        #[arg(long)]
        guided: bool,
    },
}

fn main() -> std::process::ExitCode {
    let cli = Cli::parse();
    // The file commands run on any system.
    let files = match &cli.command {
        Command::Known => Some(Ok(files::known_models())),
        Command::Check { report, against } => Some(files::check(report, against.as_deref())),
        Command::Draft {
            report,
            id,
            name,
            out,
        } => Some(files::draft_file(report, id, name, out.as_ref())),
        Command::Validate { files } => Some(Ok(files::validate(files))),
        _ => None,
    };
    if let Some(result) = files {
        return result.unwrap_or_else(|e| {
            eprintln!("error: {e}");
            std::process::ExitCode::FAILURE
        });
    }
    match run(cli) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            std::process::ExitCode::FAILURE
        }
    }
}

#[cfg(windows)]
fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    match cli.command {
        Command::Devices { all_services } => {
            bluetooth::print_devices()?;
            if all_services {
                println!();
                bluetooth::print_all_rfcomm()?;
            }
            Ok(())
        }
        Command::RfcommCheck { device } => rfcomm_check(&device),
        Command::Watch { seconds } => {
            let worker = knurl_device::Worker::spawn();
            let start = std::time::Instant::now();
            while start.elapsed() < std::time::Duration::from_secs(seconds) {
                for event in worker.try_events() {
                    println!("[{:>6} ms] {event:?}", start.elapsed().as_millis());
                }
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            Ok(())
        }
        Command::Audio { all } => audio::print_endpoints(all),
        Command::Run { device, out } => report::run(report::Options {
            device,
            out_dir: out,
        }),
        Command::Capture {
            device,
            service,
            out,
            no_ack,
            send_on_connect,
            guided,
        } => capture::run(capture::Options {
            device,
            service,
            out,
            auto_ack: !no_ack,
            send_on_connect,
            guided,
        }),
        Command::Known
        | Command::Check { .. }
        | Command::Draft { .. }
        | Command::Validate { .. } => {
            unreachable!("file commands are handled in main")
        }
    }
}

/// Reports, for each MDR service, whether a Winsock RFCOMM connection opens.
#[cfg(windows)]
fn rfcomm_check(name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let needle = name.to_lowercase();
    let mut matched = false;
    for device in bluetooth::paired_classic_devices()? {
        let device_name = device.Name()?.to_string_lossy();
        if !device_name.to_lowercase().contains(&needle) {
            continue;
        }
        matched = true;
        let address = device.BluetoothAddress()?;
        println!("{device_name}");
        for kind in [ServiceKind::V1, ServiceKind::V2] {
            let label = format!("{kind:?}").to_lowercase();
            match knurl_device::winsock::connect_rfcomm(address, bluetooth::uuid_of(kind)) {
                Ok(_socket) => println!("  MDR {label}: connection opened (closed again)"),
                Err(e) => println!("  MDR {label}: {e}"),
            }
        }
    }
    if !matched {
        return Err(format!("no paired classic device matches \"{name}\"").into());
    }
    Ok(())
}

#[cfg(not(windows))]
fn run(_cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    Err("this command needs Windows; known, check, draft and validate run anywhere".into())
}

/// Parses "00 0a FF" / "000aff" / "0x00,0x0a" into bytes.
pub fn parse_hex(s: &str) -> Result<Vec<u8>, String> {
    let cleaned: String = s
        .replace("0x", "")
        .replace("0X", "")
        .chars()
        .filter(|c| !matches!(c, ' ' | ',' | ':' | '-'))
        .collect();
    if !cleaned.len().is_multiple_of(2) {
        return Err(format!("odd number of hex digits in {s:?}"));
    }
    (0..cleaned.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&cleaned[i..i + 2], 16)
                .map_err(|_| format!("invalid hex byte {:?} in {s:?}", &cleaned[i..i + 2]))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::parse_hex;

    #[test]
    fn parses_common_hex_spellings() {
        assert_eq!(parse_hex("00 0a FF"), Ok(vec![0x00, 0x0A, 0xFF]));
        assert_eq!(parse_hex("000aff"), Ok(vec![0x00, 0x0A, 0xFF]));
        assert_eq!(parse_hex("0x00,0x0a"), Ok(vec![0x00, 0x0A]));
        assert_eq!(parse_hex(""), Ok(vec![]));
        assert!(parse_hex("0").is_err());
        assert!(parse_hex("zz").is_err());
    }
}
