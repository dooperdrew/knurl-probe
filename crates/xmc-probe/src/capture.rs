//! Opens the MDR RFCOMM channel and records the traffic.
//!
//! Prompt commands while connected:
//! * `send <hex>`  send a DATA_MDR frame with that payload (sequence auto-alternates)
//! * `raw <hex>`   send bytes exactly as typed (no framing)
//! * `note <text>` write a marker into the log (e.g. "pressed NC button")
//! * `quit`

use crate::bluetooth;
use crate::log::{CaptureLog, hex};
use crate::{ServiceKind, parse_hex};
use std::io::{BufRead, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use xmc_device::winsock::{RfcommSocket, connect_rfcomm};
use xmc_protocol::{DataType, Frame, FrameReader, ReadEvent, ack_for};

pub struct Options {
    pub device: String,
    pub service: Option<ServiceKind>,
    pub out: Option<PathBuf>,
    pub auto_ack: bool,
    pub send_on_connect: Vec<String>,
    pub guided: bool,
}

/// Writes frames to the socket and remembers the next outgoing sequence bit.
struct Sender {
    socket: Arc<RfcommSocket>,
    next_seq: u8,
}

impl Sender {
    fn send_bytes(&mut self, bytes: &[u8]) -> Result<(), String> {
        self.socket.send_all(bytes)
    }

    fn send_mdr(&mut self, payload: Vec<u8>) -> Result<(Frame, Vec<u8>), String> {
        let frame = Frame::new(DataType::DataMdr, self.next_seq, payload);
        let wire = frame.encode();
        self.send_bytes(&wire)?;
        self.next_seq ^= 1;
        Ok((frame, wire))
    }
}

/// The paired classic device whose name contains `opts.device`.
fn pick_device(opts: &Options) -> Result<(String, u64), Box<dyn std::error::Error>> {
    let wanted = opts.device.to_lowercase();
    let mut matches = Vec::new();
    for device in bluetooth::paired_classic_devices()? {
        let name = device.Name()?.to_string_lossy();
        if name.to_lowercase().contains(&wanted) {
            matches.push((name, device.BluetoothAddress()?));
        }
    }
    match matches.len() {
        0 => Err(format!(
            "no paired classic Bluetooth device matches {:?}; pair the headphones (not only the LE_ entry)",
            opts.device
        )
        .into()),
        1 => Ok(matches.remove(0)),
        _ => {
            let names: Vec<&str> = matches.iter().map(|(n, _)| n.as_str()).collect();
            Err(format!("{:?} matches several devices: {names:?}; be more specific", opts.device).into())
        }
    }
}

/// Connects with Winsock to the requested MDR service, or to v1 then v2.
fn open_mdr(
    address: u64,
    wanted: Option<ServiceKind>,
) -> Result<(ServiceKind, RfcommSocket), Box<dyn std::error::Error>> {
    let kinds: Vec<ServiceKind> = match wanted {
        Some(k) => vec![k],
        None => vec![ServiceKind::V1, ServiceKind::V2],
    };
    let mut errors = Vec::new();
    for kind in kinds {
        match connect_rfcomm(address, bluetooth::uuid_of(kind)) {
            Ok(socket) => return Ok((kind, socket)),
            Err(e) => errors.push(format!("{kind:?}: {e}")),
        }
    }
    Err(format!("could not open an MDR service ({})", errors.join("; ")).into())
}

fn default_out(device: &str) -> PathBuf {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let safe: String = device
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    PathBuf::from("captures")
        .join("raw")
        .join(format!("{secs}-{safe}.jsonl"))
}

const GUIDE: &[&str] = &[
    "Leave the headphones idle for 10 seconds (record what they send on their own).",
    "Press the NC/AMBIENT button 3 times, waiting 3 seconds between presses.",
    "Take the headphones off for 5 seconds, then put them back on.",
    "Start music on the PC for 10 seconds, then pause it.",
    "Cover the right earcup with your hand for 3 seconds (Quick Attention), then release.",
    "Say a few words out loud (Speak-to-Chat, XM4 only), then stay silent for 10 seconds.",
    "Plug in the USB charging cable for 10 seconds, then unplug it.",
];

pub fn run(opts: Options) -> Result<(), Box<dyn std::error::Error>> {
    let (device_name, address) = pick_device(&opts)?;
    let out = opts
        .out
        .clone()
        .unwrap_or_else(|| default_out(&device_name));

    println!(
        "Connecting to {device_name} ({})...",
        bluetooth::format_address(address)
    );
    let (kind, socket) = open_mdr(address, opts.service)?;
    let socket = Arc::new(socket);
    let log = Arc::new(Mutex::new(CaptureLog::create(&out)?));
    lock(&log).event(&format!(
        "connected device={device_name} service={kind:?} transport=winsock auto_ack={}",
        opts.auto_ack
    ));
    println!("Connected via MDR {kind:?}.");
    println!("Connected. Logging to {}", out.display());

    let sender = Arc::new(Mutex::new(Sender {
        socket: socket.clone(),
        next_seq: 0,
    }));
    let stop = Arc::new(AtomicBool::new(false));

    let rx = {
        let (log, sender, stop, socket) =
            (log.clone(), sender.clone(), stop.clone(), socket.clone());
        let auto_ack = opts.auto_ack;
        std::thread::spawn(move || {
            let mut frames = FrameReader::new();
            let mut buf = vec![0u8; 1024];
            while !stop.load(Ordering::Relaxed) {
                let n = match socket.recv(&mut buf) {
                    Ok(0) if stop.load(Ordering::Relaxed) => break,
                    Ok(0) => {
                        lock(&log).event("remote closed the connection");
                        println!("\n[headphones closed the connection]");
                        break;
                    }
                    Ok(n) => n,
                    Err(e) => {
                        if !stop.load(Ordering::Relaxed) {
                            lock(&log).event(&format!("read error: {e}"));
                            println!("\n[read error: {e}]");
                        }
                        break;
                    }
                };
                // Chunk boundaries show whether frames arrived together (e.g.
                // a burst of repeats after a stall) or one by one.
                lock(&log).event(&format!("recv {n} bytes"));
                for ev in frames.push(&buf[..n]) {
                    match ev {
                        ReadEvent::Frame { frame, wire } => {
                            lock(&log).frame("rx", &wire, &frame);
                            println!(
                                "\n<- {:?} seq={} payload=[{}]",
                                frame.data_type,
                                frame.seq,
                                hex(&frame.payload)
                            );
                            if frame.data_type == DataType::Ack {
                                // The next command must use the sequence number the device acknowledged with.
                                sender.lock().unwrap_or_else(|p| p.into_inner()).next_seq =
                                    frame.seq & 1;
                            } else if auto_ack {
                                let ack = ack_for(&frame);
                                let ack_wire = ack.encode();
                                let res = sender
                                    .lock()
                                    .unwrap_or_else(|p| p.into_inner())
                                    .send_bytes(&ack_wire);
                                match res {
                                    Ok(()) => lock(&log).frame("tx", &ack_wire, &ack),
                                    Err(e) => lock(&log).event(&format!("ack send failed: {e}")),
                                }
                            }
                        }
                        ReadEvent::Invalid { wire, error } => {
                            lock(&log).invalid(&wire, &error);
                            println!("\n<- INVALID ({error}): {}", hex(&wire));
                        }
                        ReadEvent::Discarded(bytes) => {
                            lock(&log).discarded(&bytes);
                            println!("\n<- junk: {}", hex(&bytes));
                        }
                    }
                }
            }
        })
    };

    let send_payload = |payload: Vec<u8>| -> Result<(), String> {
        let res = sender
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .send_mdr(payload);
        match res {
            Ok((frame, wire)) => {
                lock(&log).frame("tx", &wire, &frame);
                println!(
                    "-> DataMdr seq={} payload=[{}]",
                    frame.seq,
                    hex(&frame.payload)
                );
                Ok(())
            }
            Err(e) => {
                lock(&log).event(&format!("send failed: {e}"));
                Err(format!("send failed: {e}"))
            }
        }
    };

    for p in &opts.send_on_connect {
        send_payload(parse_hex(p)?)?;
        // One request at a time: give the device time to reply and acknowledge.
        std::thread::sleep(std::time::Duration::from_millis(800));
    }

    let stdin = std::io::stdin();
    let mut lines = stdin.lock().lines();
    let prompt = |text: &str| {
        print!("{text}");
        let _ = std::io::stdout().flush();
    };

    if opts.guided {
        println!(
            "\nGuided session: {} steps. Press Enter after each step (or type `skip`).",
            GUIDE.len()
        );
        for (i, step) in GUIDE.iter().enumerate() {
            lock(&log).note(&format!("step {} start: {step}", i + 1));
            prompt(&format!("\n[{}/{}] {step}\n> ", i + 1, GUIDE.len()));
            let answer = lines.next().transpose()?.unwrap_or_default();
            lock(&log).note(&format!("step {} end: {}", i + 1, answer.trim()));
        }
        println!("\nGuided steps done. You can keep using the prompt; `quit` to finish.");
    }

    println!("Commands: send <hex> | raw <hex> | note <text> | quit");
    loop {
        prompt("> ");
        let Some(line) = lines.next().transpose()? else {
            break;
        };
        let line = line.trim();
        let (cmd, arg) = line.split_once(' ').unwrap_or((line, ""));
        let result = match cmd {
            "" => Ok(()),
            "quit" | "exit" => break,
            "note" => {
                lock(&log).note(arg);
                Ok(())
            }
            "send" => parse_hex(arg).and_then(send_payload),
            "raw" => parse_hex(arg).and_then(|bytes| {
                let res = sender
                    .lock()
                    .unwrap_or_else(|p| p.into_inner())
                    .send_bytes(&bytes);
                res.map_err(|e| format!("send failed: {e}")).map(|()| {
                    lock(&log).event(&format!("raw tx: {}", hex(&bytes)));
                })
            }),
            other => Err(format!("unknown command {other:?}")),
        };
        if let Err(e) = result {
            println!("error: {e}");
        }
    }

    stop.store(true, Ordering::Relaxed);
    lock(&log).event("closing");
    // Shutting the socket down unblocks the pending read in the receive thread.
    socket.shutdown();
    let _ = rx.join();
    println!("Capture saved to {}", out.display());
    Ok(())
}

/// Locks the shared log, recovering from a poisoned mutex (a panic in one
/// thread must not lose the rest of the capture).
fn lock(l: &Mutex<CaptureLog>) -> std::sync::MutexGuard<'_, CaptureLog> {
    l.lock().unwrap_or_else(|p| p.into_inner())
}
