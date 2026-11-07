//! What the app knows about the headphones, kept up to date from replies and
//! notifications. Built on [`Session`], still without I/O.
//!
//! After [`Client::start`] it runs the read-only start-up sequence (protocol
//! info, model, firmware, supported functions, battery, NC/ASM, EQ) one
//! request at a time, then refreshes the battery every
//! [`BATTERY_REFRESH_MS`]. Notifications update the state at any time.

use std::collections::VecDeque;

use crate::message::{Charging, DeviceInfoKind, Message, NcAsmMode, Request};
use crate::session::{Config, Event, Outcome, Session};

/// Battery refresh period. A choice of ours, not a protocol requirement.
pub const BATTERY_REFRESH_MS: u64 = 60_000;

/// The read-only requests sent after connecting, in this order. It is the order
/// of our XM3 guided capture, so the replay test exercises it end to end.
pub const STARTUP: [Request; 7] = [
    Request::ProtocolInfo,
    Request::DeviceInfo(DeviceInfoKind::ModelName),
    Request::DeviceInfo(DeviceInfoKind::Firmware),
    Request::SupportedFunctions,
    Request::Battery,
    Request::NoiseCancelling,
    Request::Equalizer,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Battery {
    pub level: u8,
    pub charging: Charging,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Equalizer {
    pub preset: u8,
    pub clear_bass: i8,
    pub bands: [i8; 5],
}

/// Everything learned so far. `None` means "not received", never a default.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DeviceState {
    pub protocol_version: Option<u16>,
    pub model: Option<String>,
    pub firmware: Option<String>,
    pub functions: Option<Vec<u8>>,
    pub battery: Option<Battery>,
    pub nc_asm: Option<NcAsmMode>,
    pub equalizer: Option<Equalizer>,
}

/// Output of the client.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClientEvent {
    /// Write these bytes to the socket.
    Send(Vec<u8>),
    /// [`Client::state`] changed.
    StateChanged,
    /// A request ended without a reply. `outcome` says why.
    RequestFailed { opcode: u8, outcome: Outcome },
    /// The headphones sent something that could not be decoded.
    ProtocolError(String),
}

#[derive(Debug)]
pub struct Client {
    session: Session,
    queue: VecDeque<Request>,
    state: DeviceState,
    started: bool,
    last_battery_request: Option<u64>,
}

impl Client {
    pub fn new(config: Config) -> Self {
        Self {
            session: Session::new(config),
            queue: VecDeque::new(),
            state: DeviceState::default(),
            started: false,
            last_battery_request: None,
        }
    }

    pub fn state(&self) -> &DeviceState {
        &self.state
    }

    /// Queues the start-up sequence and sends its first request.
    pub fn start(&mut self, now: u64) -> Vec<ClientEvent> {
        self.started = true;
        self.queue.extend(STARTUP);
        let mut out = Vec::new();
        self.send_next(now, &mut out);
        out
    }

    /// Processes bytes read from the socket.
    pub fn receive(&mut self, bytes: &[u8], now: u64) -> Vec<ClientEvent> {
        let events = self.session.receive(bytes, now);
        self.handle(events, now)
    }

    /// Call regularly (for example every 50 ms): timeouts, resends and the
    /// periodic battery refresh.
    pub fn poll(&mut self, now: u64) -> Vec<ClientEvent> {
        if self.started
            && self.queue.is_empty()
            && !self.session.is_busy()
            && self
                .last_battery_request
                .is_some_and(|t| now.saturating_sub(t) >= BATTERY_REFRESH_MS)
        {
            self.queue.push_back(Request::Battery);
        }
        let events = self.session.poll(now);
        self.handle(events, now)
    }

    fn handle(&mut self, events: Vec<Event>, now: u64) -> Vec<ClientEvent> {
        let mut out = Vec::new();
        for event in events {
            match event {
                Event::Send(wire) => out.push(ClientEvent::Send(wire)),
                Event::Received { message, .. } => match message {
                    Ok(message) => {
                        if self.apply(message) {
                            out.push(ClientEvent::StateChanged);
                        }
                    }
                    Err(e) => out.push(ClientEvent::ProtocolError(e.to_string())),
                },
                Event::Duplicate => {}
                Event::RequestDone { opcode, outcome } => {
                    if outcome != Outcome::Answered {
                        out.push(ClientEvent::RequestFailed { opcode, outcome });
                    }
                }
                Event::Invalid(text) => out.push(ClientEvent::ProtocolError(text)),
            }
        }
        self.send_next(now, &mut out);
        out
    }

    fn send_next(&mut self, now: u64, out: &mut Vec<ClientEvent>) {
        if self.session.is_busy() {
            return;
        }
        let Some(request) = self.queue.pop_front() else {
            return;
        };
        if request == Request::Battery {
            self.last_battery_request = Some(now);
        }
        if let Ok(wire) = self.session.send(request.payload(), now) {
            out.push(ClientEvent::Send(wire));
        }
    }

    /// Updates the state; returns whether anything changed.
    fn apply(&mut self, message: Message) -> bool {
        let s = &mut self.state;
        let before = s.clone();
        match message {
            Message::ProtocolInfo { version } => s.protocol_version = Some(version),
            Message::ModelName(m) => s.model = Some(m),
            Message::Firmware(f) => s.firmware = Some(f),
            Message::SupportedFunctions(f) => s.functions = Some(f),
            Message::Battery {
                level, charging, ..
            } => s.battery = Some(Battery { level, charging }),
            Message::NcAsm { mode, .. } => s.nc_asm = Some(mode),
            Message::Equalizer {
                preset,
                clear_bass,
                bands,
                ..
            } => {
                s.equalizer = Some(Equalizer {
                    preset,
                    clear_bass,
                    bands,
                })
            }
            Message::Codec { .. } | Message::Volume(_) | Message::Unknown(_) => {}
        }
        *s != before
    }
}

/// True for a device name this app supports. Names as the headphones report
/// them to Windows; only these two models are tested.
pub fn is_supported_model(name: &str) -> bool {
    let upper = name.to_ascii_uppercase();
    !upper.starts_with("LE_") && (upper.contains("WH-1000XM3") || upper.contains("WH-1000XM4"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supported_models() {
        assert!(is_supported_model("WH-1000XM3"));
        assert!(is_supported_model("WH-1000XM4"));
        assert!(!is_supported_model("LE_WH-1000XM3"));
        assert!(!is_supported_model("WH-1000XM5"));
    }

    #[test]
    fn start_sends_protocol_info_first() {
        let mut c = Client::new(Config::default());
        let out = c.start(0);
        assert_eq!(
            out,
            vec![ClientEvent::Send(vec![
                0x3E, 0x0C, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x0E, 0x3C
            ])]
        );
        assert_eq!(c.state(), &DeviceState::default());
    }

    #[test]
    fn unanswered_request_is_reported_and_the_queue_moves_on() {
        let cfg = Config {
            ack_timeout_ms: 10,
            max_attempts: 1,
            reply_timeout_ms: 10,
        };
        let mut c = Client::new(cfg);
        c.start(0);
        let out = c.poll(10);
        assert!(out.contains(&ClientEvent::RequestFailed {
            opcode: 0x00,
            outcome: Outcome::NoAck
        }));
        // The next start-up request (model name) goes out.
        assert!(
            out.iter()
                .any(|e| matches!(e, ClientEvent::Send(w) if w[7..9] == [0x04, 0x01]))
        );
    }
}
