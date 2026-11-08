//! Background thread: find the headphones, connect, run the client, and
//! reconnect after a failure. The UI only reads [`DeviceEvent`]s.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use xmc_protocol::client::{Client, ClientEvent, DeviceState};
use xmc_protocol::message::Request;
use xmc_protocol::session::{Config, Outcome};

use crate::bluetooth::{self, MDR_V1, Paired};
use crate::winsock::{RfcommSocket, connect_rfcomm};

/// Wait before searching or connecting again after a failure.
const RETRY_DELAY: Duration = Duration::from_secs(10);
/// How often the client is polled when no data arrives.
const POLL_INTERVAL: Duration = Duration::from_millis(50);

/// What the worker reports. Every state the UI shows comes from one of these.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeviceEvent {
    /// No paired WH-1000XM3 / WH-1000XM4 was found in Windows.
    NoHeadphones,
    /// Windows refused or failed the Bluetooth lookup.
    BluetoothError(String),
    Connecting {
        name: String,
    },
    Connected {
        name: String,
    },
    /// The headphones' state changed.
    State(DeviceState),
    /// A request got no ACK or no reply.
    RequestFailed {
        opcode: u8,
        outcome: Outcome,
    },
    /// The headphones sent something undecodable.
    ProtocolError(String),
    /// The headphones ask about the last change. When `asks` is true they
    /// wait for `Request::AnswerAlert`.
    Alert {
        message: u8,
        asks: bool,
    },
    /// The connection could not be opened or was lost.
    Disconnected {
        name: String,
        reason: String,
    },
    /// A command from the UI arrived while no headphones were connected and
    /// was dropped (it is never kept for later).
    CommandDropped,
}

/// Handle to the background thread. Dropping it stops the thread.
pub struct Worker {
    events: Receiver<DeviceEvent>,
    commands: Sender<Request>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl Worker {
    pub fn spawn() -> Worker {
        let (tx, events) = channel();
        let (commands, commands_rx) = channel();
        let stop = Arc::new(AtomicBool::new(false));
        let thread = {
            let stop = stop.clone();
            std::thread::Builder::new()
                .name("xmc-device".into())
                .spawn(move || run(&tx, &commands_rx, &stop))
                .ok()
        };
        if thread.is_none() {
            // Without a thread nothing would ever report; say so.
            let (tx, rx) = channel();
            let _ = tx.send(DeviceEvent::BluetoothError(
                "could not start the device thread".into(),
            ));
            return Worker {
                events: rx,
                commands,
                stop,
                thread: None,
            };
        }
        Worker {
            events,
            commands,
            stop,
            thread,
        }
    }

    /// Sends a request (typically a setting change) to the headphones. The
    /// result arrives as events: the confirming state, or a failure.
    pub fn send(&self, request: Request) {
        // The receiver lives as long as the thread; if the thread is gone
        // there is nobody to report to, and the UI already shows the error.
        let _ = self.commands.send(request);
    }

    /// Events received since the last call, without blocking.
    pub fn try_events(&self) -> Vec<DeviceEvent> {
        self.events.try_iter().collect()
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

/// Sleeps up to `d`, waking early when asked to stop. Returns false on stop.
fn wait(stop: &AtomicBool, d: Duration) -> bool {
    let end = Instant::now() + d;
    while Instant::now() < end {
        if stop.load(Ordering::Relaxed) {
            return false;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    !stop.load(Ordering::Relaxed)
}

/// Drops commands sent while disconnected, telling the UI.
fn drop_commands(commands: &Receiver<Request>, tx: &Sender<DeviceEvent>) {
    if commands.try_iter().count() > 0 {
        let _ = tx.send(DeviceEvent::CommandDropped);
    }
}

fn run(tx: &Sender<DeviceEvent>, commands: &Receiver<Request>, stop: &AtomicBool) {
    while !stop.load(Ordering::Relaxed) {
        drop_commands(commands, tx);
        let headphones = match bluetooth::paired_supported_headphones() {
            Ok(list) => list,
            Err(e) => {
                let _ = tx.send(DeviceEvent::BluetoothError(e.to_string()));
                if !wait(stop, RETRY_DELAY) {
                    return;
                }
                continue;
            }
        };
        let Some(target) = headphones.into_iter().next() else {
            let _ = tx.send(DeviceEvent::NoHeadphones);
            if !wait(stop, RETRY_DELAY) {
                return;
            }
            continue;
        };
        let reason = session(&target, tx, commands, stop);
        let _ = tx.send(DeviceEvent::Disconnected {
            name: target.name.clone(),
            reason,
        });
        if !wait(stop, RETRY_DELAY) {
            return;
        }
    }
}

enum Incoming {
    Bytes(Vec<u8>),
    Closed(String),
}

/// One connection, from connect to disconnect. Returns why it ended.
fn session(
    target: &Paired,
    tx: &Sender<DeviceEvent>,
    commands: &Receiver<Request>,
    stop: &AtomicBool,
) -> String {
    let _ = tx.send(DeviceEvent::Connecting {
        name: target.name.clone(),
    });
    let socket = match connect_rfcomm(target.address, MDR_V1) {
        Ok(s) => Arc::new(s),
        Err(e) => return e,
    };
    let _ = tx.send(DeviceEvent::Connected {
        name: target.name.clone(),
    });

    let (bytes_tx, bytes_rx) = channel();
    let reader = {
        let socket = socket.clone();
        std::thread::spawn(move || read_loop(&socket, &bytes_tx))
    };

    let origin = Instant::now();
    let now = || origin.elapsed().as_millis() as u64;
    let mut client = Client::new(Config::default());
    // Anything the UI sent before this connection was opened is stale.
    drop_commands(commands, tx);
    let mut reason = forward(client.start(now()), &socket, tx, &client).err();

    while reason.is_none() {
        if stop.load(Ordering::Relaxed) {
            reason = Some("stopped".into());
            break;
        }
        let events = match bytes_rx.recv_timeout(POLL_INTERVAL) {
            Ok(Incoming::Bytes(b)) => client.receive(&b, now()),
            Ok(Incoming::Closed(why)) => {
                reason = Some(why);
                break;
            }
            Err(RecvTimeoutError::Timeout) => Vec::new(),
            Err(RecvTimeoutError::Disconnected) => {
                reason = Some("reader thread ended".into());
                break;
            }
        };
        let mut all = events;
        for request in commands.try_iter() {
            all.extend(client.command(request, now()));
        }
        all.extend(client.poll(now()));
        if let Err(e) = forward(all, &socket, tx, &client) {
            reason = Some(e);
        }
    }

    socket.shutdown();
    let _ = reader.join();
    reason.unwrap_or_default()
}

fn read_loop(socket: &RfcommSocket, out: &Sender<Incoming>) {
    let mut buf = vec![0u8; 1024];
    loop {
        match socket.recv(&mut buf) {
            Ok(0) => {
                let _ = out.send(Incoming::Closed(
                    "the headphones closed the connection".into(),
                ));
                return;
            }
            Ok(n) => {
                if out.send(Incoming::Bytes(buf[..n].to_vec())).is_err() {
                    return;
                }
            }
            Err(e) => {
                let _ = out.send(Incoming::Closed(e));
                return;
            }
        }
    }
}

/// Writes outgoing frames and reports the rest to the UI.
fn forward(
    events: Vec<ClientEvent>,
    socket: &RfcommSocket,
    tx: &Sender<DeviceEvent>,
    client: &Client,
) -> Result<(), String> {
    for ev in events {
        match ev {
            ClientEvent::Send(wire) => socket.send_all(&wire)?,
            ClientEvent::StateChanged => {
                let _ = tx.send(DeviceEvent::State(client.state().clone()));
            }
            ClientEvent::RequestFailed { opcode, outcome } => {
                let _ = tx.send(DeviceEvent::RequestFailed { opcode, outcome });
            }
            ClientEvent::ProtocolError(e) => {
                let _ = tx.send(DeviceEvent::ProtocolError(e));
            }
            ClientEvent::Alert { message, asks } => {
                let _ = tx.send(DeviceEvent::Alert { message, asks });
            }
        }
    }
    Ok(())
}
