//! Background thread: find the headphones, connect, run the client, and
//! reconnect after a failure. The UI only reads [`DeviceEvent`]s.

use std::cell::Cell;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, TryRecvError, channel};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use knurl_protocol::client::{Client, ClientEvent};
use knurl_protocol::message::Request;
use knurl_protocol::session::Config;

use crate::DeviceEvent;
use crate::bluetooth::{self, MDR_V1, Paired};
use crate::winsock::{RfcommSocket, connect_rfcomm};

/// Wait before searching or connecting again after a failure.
const RETRY_DELAY: Duration = Duration::from_secs(10);
/// How often the client is polled when no data arrives.
const POLL_INTERVAL: Duration = Duration::from_millis(50);

/// Handle to the background thread. Dropping it stops the thread.
pub struct Worker {
    events: Receiver<DeviceEvent>,
    commands: Sender<Request>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
    /// Set once the UI was told that the thread is gone.
    gone_reported: Cell<bool>,
}

impl Worker {
    pub fn spawn() -> Worker {
        let (tx, events) = channel();
        let (commands, commands_rx) = channel();
        let stop = Arc::new(AtomicBool::new(false));
        let thread = {
            let stop = stop.clone();
            std::thread::Builder::new()
                .name("knurl-device".into())
                .spawn(move || {
                    // Before any WinRT call: the Bluetooth lookup must not
                    // depend on an apartment another thread may tear down.
                    if let Err(e) = crate::keep_com_apartment() {
                        let _ = tx.send(DeviceEvent::SetupError(e));
                    }
                    supervise(&tx, &stop, RETRY_DELAY, || run(&tx, &commands_rx, &stop));
                })
                .ok()
        };
        if thread.is_none() {
            // Without a thread nothing would ever report; say so.
            let (tx, rx) = channel();
            let _ = tx.send(DeviceEvent::Internal {
                error: "the device thread could not start".into(),
                recovers: false,
            });
            return Worker {
                events: rx,
                commands,
                stop,
                thread: None,
                gone_reported: Cell::new(false),
            };
        }
        Worker {
            events,
            commands,
            stop,
            thread,
            gone_reported: Cell::new(false),
        }
    }

    /// Sends a request (typically a setting change) to the headphones. The
    /// result arrives as events: the confirming state, or a failure.
    pub fn send(&self, request: Request) {
        // The receiver lives as long as the thread. If the thread is gone the
        // request is lost; try_events reports that the thread stopped.
        let _ = self.commands.send(request);
    }

    /// Events received since the last call, without blocking. If the device
    /// thread is gone (it only ends on stop, so this means it died), the UI
    /// gets one error instead of silence: nothing would reach the headphones.
    pub fn try_events(&self) -> Vec<DeviceEvent> {
        let mut events = Vec::new();
        loop {
            match self.events.try_recv() {
                Ok(event) => events.push(event),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    if !self.gone_reported.replace(true) {
                        events.push(DeviceEvent::Internal {
                            error: "the device thread stopped".into(),
                            recovers: false,
                        });
                    }
                    break;
                }
            }
        }
        events
    }
}

impl Drop for Worker {
    /// Asks the thread to stop without waiting for it: it can be inside a
    /// blocking Bluetooth connect for 10-30 s, and waiting would keep the app
    /// (and its single-instance lock) alive after Quit with no icon left.
    /// Windows closes the socket when the process ends.
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        drop(self.thread.take());
    }
}

/// Runs `body` until it returns (a stop was asked). A panic inside it is
/// reported to the UI as an error, and `body` starts again after `delay`,
/// so a bug in the protocol code costs a reconnect, not a frozen window.
/// The panic itself is logged by the app's panic hook.
fn supervise(tx: &Sender<DeviceEvent>, stop: &AtomicBool, delay: Duration, mut body: impl FnMut()) {
    loop {
        match catch_unwind(AssertUnwindSafe(&mut body)) {
            Ok(()) => return,
            Err(payload) => {
                let detail = payload
                    .downcast_ref::<&str>()
                    .map(|s| (*s).to_string())
                    .or_else(|| payload.downcast_ref::<String>().cloned())
                    .unwrap_or_else(|| "unknown error".into());
                let _ = tx.send(DeviceEvent::Internal {
                    error: format!("internal error in the device thread ({detail})"),
                    recovers: true,
                });
                if !wait(stop, delay) {
                    return;
                }
            }
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
    // With more than one supported pair paired (an XM3 and an XM4), each try
    // takes the next one, so a pair that is off does not hide the other.
    let mut turn: usize = 0;
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
        let count = headphones.len();
        let Some(target) = headphones.into_iter().nth(pick(turn, count)) else {
            let _ = tx.send(DeviceEvent::NoHeadphones);
            if !wait(stop, RETRY_DELAY) {
                return;
            }
            continue;
        };
        turn = turn.wrapping_add(1);
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

/// Which of `count` paired headphones the try number `turn` uses.
fn pick(turn: usize, count: usize) -> usize {
    if count == 0 { 0 } else { turn % count }
}

/// Something that can be shut down to unblock a reader on another thread.
trait Shutdown {
    fn shutdown(&self);
}

impl Shutdown for RfcommSocket {
    fn shutdown(&self) {
        RfcommSocket::shutdown(self);
    }
}

/// Shuts the socket down when dropped, also when a panic unwinds the
/// session: the reader thread holds its own reference and blocks in `recv`,
/// so without the shutdown the socket (and the headphones' MDR channel)
/// would stay open until the app ends.
struct ShutdownOnDrop<S: Shutdown>(Arc<S>);

impl<S: Shutdown> Drop for ShutdownOnDrop<S> {
    fn drop(&mut self) {
        self.0.shutdown();
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
        Ok(s) => ShutdownOnDrop(Arc::new(s)),
        Err(e) => return e,
    };
    let _ = tx.send(DeviceEvent::Connected {
        name: target.name.clone(),
    });

    let (bytes_tx, bytes_rx) = channel();
    let reader = {
        let socket = socket.0.clone();
        std::thread::spawn(move || read_loop(&socket, &bytes_tx))
    };

    let origin = Instant::now();
    let now = || origin.elapsed().as_millis() as u64;
    let mut client = Client::new(Config::default());
    // Anything the UI sent before this connection was opened is stale.
    drop_commands(commands, tx);
    let mut reason = forward(client.start(now()), &socket.0, tx, &client).err();

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
        if let Err(e) = forward(all, &socket.0, tx, &client) {
            reason = Some(e);
        }
    }

    // Shuts the socket down, which ends the reader's `recv`.
    drop(socket);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_panic_is_reported_and_the_body_restarts() {
        let (tx, rx) = channel();
        let stop = AtomicBool::new(false);
        let mut runs = 0;
        // The panic message would go to stderr through the default hook.
        supervise(&tx, &stop, Duration::ZERO, || {
            runs += 1;
            if runs == 1 {
                panic!("decoder bug");
            }
        });
        assert_eq!(runs, 2, "the body must run again after the panic");
        let events: Vec<_> = rx.try_iter().collect();
        assert_eq!(
            events,
            vec![DeviceEvent::Internal {
                error: "internal error in the device thread (decoder bug)".into(),
                recovers: true,
            }]
        );
    }

    #[test]
    fn a_stop_during_the_restart_delay_ends_the_supervisor() {
        let (tx, _rx) = channel();
        let stop = AtomicBool::new(true);
        let mut runs = 0;
        supervise(&tx, &stop, Duration::from_secs(60), || {
            runs += 1;
            panic!("always");
        });
        assert_eq!(runs, 1);
    }

    #[test]
    fn a_gone_thread_is_reported_once() {
        let (tx, events) = channel();
        let (commands, _commands_rx) = channel();
        let worker = Worker {
            events,
            commands,
            stop: Arc::new(AtomicBool::new(false)),
            thread: None,
            gone_reported: Cell::new(false),
        };
        tx.send(DeviceEvent::NoHeadphones).expect("receiver alive");
        drop(tx);
        assert_eq!(
            worker.try_events(),
            vec![
                DeviceEvent::NoHeadphones,
                DeviceEvent::Internal {
                    error: "the device thread stopped".into(),
                    recovers: false,
                },
            ]
        );
        assert_eq!(worker.try_events(), vec![]);
    }

    #[derive(Default)]
    struct FakeSocket {
        shut: AtomicBool,
    }

    impl Shutdown for FakeSocket {
        fn shutdown(&self) {
            self.shut.store(true, Ordering::Relaxed);
        }
    }

    #[test]
    fn a_panicking_session_still_shuts_the_socket_down() {
        let socket = Arc::new(FakeSocket::default());
        let reader_copy = socket.clone();
        let result = catch_unwind(AssertUnwindSafe(|| {
            let _guard = ShutdownOnDrop(socket);
            panic!("decoder bug");
        }));
        assert!(result.is_err());
        assert!(
            reader_copy.shut.load(Ordering::Relaxed),
            "the socket must be shut down while unwinding"
        );
    }

    #[test]
    fn a_normal_end_shuts_the_socket_down() {
        let socket = Arc::new(FakeSocket::default());
        drop(ShutdownOnDrop(socket.clone()));
        assert!(socket.shut.load(Ordering::Relaxed));
    }

    #[test]
    fn tries_take_each_paired_headphones_in_turn() {
        assert_eq!((0..4).map(|t| pick(t, 2)).collect::<Vec<_>>(), [0, 1, 0, 1]);
        assert_eq!(pick(5, 1), 0);
        assert_eq!(pick(3, 0), 0);
    }
}
