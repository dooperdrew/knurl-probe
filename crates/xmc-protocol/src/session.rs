//! Conversation state for one MDR connection, without I/O ("sans-IO").
//!
//! The caller owns the socket and the clock. It passes received bytes to
//! [`Session::receive`], calls [`Session::poll`] regularly, and writes every
//! [`Event::Send`] to the socket. Times are milliseconds from any fixed origin.
//!
//! Rules, from the public sources and checked against our XM3 captures:
//! * one request in flight at a time (stop-and-wait);
//! * every data frame from the headphones is acknowledged, duplicates included;
//! * the next request uses the sequence number of the headphones' last ACK;
//! * a request without ACK is sent again with the same sequence number;
//! * the reply may arrive before the ACK (seen on our XM3).
//!
//! The headphones repeat a frame (same sequence, same payload) when they did
//! not get our ACK in time. Our XM3 did this once for an EQ reply and 11 times
//! for an NC notification. Repeats are acknowledged again but reported only as
//! [`Event::Duplicate`].

use crate::message::{Message, MessageError};
use crate::{DataType, Frame, FrameReader, ReadEvent, ack_for};

/// Timing limits. The defaults follow mos9527/SonyHeadphonesClient (ACK wait
/// 1 s, 10 attempts, reply wait 5 s); they are not tuned on our headphones yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Config {
    pub ack_timeout_ms: u64,
    pub max_attempts: u32,
    pub reply_timeout_ms: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            ack_timeout_ms: 1000,
            max_attempts: 10,
            reply_timeout_ms: 5000,
        }
    }
}

/// How a request ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// Acknowledged and answered.
    Answered,
    /// Acknowledged, but no reply arrived in time. Some requests never get a
    /// reply (setters, or GETs the model does not support).
    NoReply,
    /// Never acknowledged after every attempt.
    NoAck,
}

/// Something the caller must do or know.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// Write these bytes to the socket.
    Send(Vec<u8>),
    /// A new data frame from the headphones. `reply` is true when it answers
    /// the request in flight; otherwise it is a notification.
    Received {
        data_type: DataType,
        payload: Vec<u8>,
        message: Result<Message, MessageError>,
        reply: bool,
    },
    /// The headphones repeated the previous frame; it was acknowledged again.
    Duplicate,
    /// The request whose first payload byte is `opcode` is finished.
    RequestDone { opcode: u8, outcome: Outcome },
    /// Bytes that did not form a valid frame (already described by the reader).
    Invalid(String),
}

/// Returned by [`Session::send`] while another request is in flight.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Busy;

#[derive(Debug)]
struct Pending {
    opcode: u8,
    reply_opcode: u8,
    seq: u8,
    wire: Vec<u8>,
    sent_at: u64,
    attempts: u32,
    acked_at: Option<u64>,
    replied: bool,
}

/// State of one connection.
#[derive(Debug)]
pub struct Session {
    config: Config,
    reader: FrameReader,
    next_seq: u8,
    last_rx: Option<(u8, Vec<u8>)>,
    pending: Option<Pending>,
}

impl Session {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            reader: FrameReader::new(),
            next_seq: 0,
            last_rx: None,
            pending: None,
        }
    }

    /// True while a request waits for its ACK or reply.
    pub fn is_busy(&self) -> bool {
        self.pending.is_some()
    }

    /// Starts a request and returns the bytes to write to the socket.
    pub fn send(&mut self, payload: Vec<u8>, now: u64) -> Result<Vec<u8>, Busy> {
        if self.pending.is_some() {
            return Err(Busy);
        }
        let opcode = payload.first().copied().unwrap_or(0);
        let frame = Frame::new(DataType::DataMdr, self.next_seq, payload);
        let wire = frame.encode();
        self.pending = Some(Pending {
            opcode,
            reply_opcode: opcode.wrapping_add(1),
            seq: self.next_seq,
            wire: wire.clone(),
            sent_at: now,
            attempts: 1,
            acked_at: None,
            replied: false,
        });
        Ok(wire)
    }

    /// Processes bytes read from the socket.
    pub fn receive(&mut self, bytes: &[u8], now: u64) -> Vec<Event> {
        let mut events = Vec::new();
        for ev in self.reader.push(bytes) {
            match ev {
                ReadEvent::Frame { frame, .. } => self.on_frame(frame, now, &mut events),
                ReadEvent::Invalid { wire, error } => {
                    events.push(Event::Invalid(format!("{error} ({} bytes)", wire.len())));
                }
                ReadEvent::Discarded(bytes) => {
                    events.push(Event::Invalid(format!(
                        "{} bytes outside a frame",
                        bytes.len()
                    )));
                }
            }
        }
        self.finish_if_done(now, &mut events);
        events
    }

    /// Handles timeouts: resends an unacknowledged request or ends it.
    pub fn poll(&mut self, now: u64) -> Vec<Event> {
        let mut events = Vec::new();
        if let Some(p) = &mut self.pending
            && p.acked_at.is_none()
            && now.saturating_sub(p.sent_at) >= self.config.ack_timeout_ms
        {
            if p.attempts >= self.config.max_attempts {
                let opcode = p.opcode;
                self.pending = None;
                events.push(Event::RequestDone {
                    opcode,
                    outcome: Outcome::NoAck,
                });
                return events;
            }
            p.attempts += 1;
            p.sent_at = now;
            events.push(Event::Send(p.wire.clone()));
        }
        self.finish_if_done(now, &mut events);
        events
    }

    fn on_frame(&mut self, frame: Frame, now: u64, events: &mut Vec<Event>) {
        if frame.data_type == DataType::Ack {
            if let Some(p) = &mut self.pending
                && p.acked_at.is_none()
                && frame.seq & 1 != p.seq
            {
                p.acked_at = Some(now);
                self.next_seq = frame.seq & 1;
            }
            // An ACK with the request's own sequence number, or with nothing in
            // flight, is stale and ignored.
            return;
        }

        events.push(Event::Send(ack_for(&frame).encode()));
        let key = (frame.seq, frame.payload.clone());
        if self.last_rx.as_ref() == Some(&key) {
            events.push(Event::Duplicate);
            return;
        }
        self.last_rx = Some(key);

        let message = if frame.data_type == DataType::DataMdr {
            Message::decode(&frame.payload)
        } else {
            Ok(Message::Unknown(frame.payload.clone()))
        };
        let reply = match &mut self.pending {
            Some(p)
                if !p.replied
                    && frame.data_type == DataType::DataMdr
                    && frame.payload.first() == Some(&p.reply_opcode) =>
            {
                p.replied = true;
                true
            }
            _ => false,
        };
        events.push(Event::Received {
            data_type: frame.data_type,
            payload: frame.payload,
            message,
            reply,
        });
    }

    fn finish_if_done(&mut self, now: u64, events: &mut Vec<Event>) {
        let Some(p) = &self.pending else { return };
        let Some(acked_at) = p.acked_at else { return };
        let outcome = if p.replied {
            Outcome::Answered
        } else if now.saturating_sub(acked_at) >= self.config.reply_timeout_ms {
            Outcome::NoReply
        } else {
            return;
        };
        events.push(Event::RequestDone {
            opcode: p.opcode,
            outcome,
        });
        self.pending = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bytes(s: &str) -> Vec<u8> {
        s.split_whitespace()
            .map(|b| u8::from_str_radix(b, 16).expect("hex"))
            .collect()
    }

    fn sends(events: &[Event]) -> Vec<Vec<u8>> {
        events
            .iter()
            .filter_map(|e| match e {
                Event::Send(w) => Some(w.clone()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn one_request_at_a_time() {
        let mut s = Session::new(Config::default());
        assert!(s.send(vec![0x00, 0x00], 0).is_ok());
        assert_eq!(s.send(vec![0x04, 0x01], 1), Err(Busy));
    }

    #[test]
    fn reply_before_ack_completes_after_ack() {
        let mut s = Session::new(Config::default());
        s.send(vec![0x00, 0x00], 0).expect("idle");
        let ev = s.receive(&bytes("3E 0C 00 00 00 00 04 01 00 40 10 61 3C"), 10);
        assert_eq!(sends(&ev), vec![bytes("3E 01 01 00 00 00 00 02 3C")]);
        assert!(
            ev.iter()
                .any(|e| matches!(e, Event::Received { reply: true, .. }))
        );
        assert!(s.is_busy(), "still waiting for the ACK");
        let ev = s.receive(&bytes("3E 01 01 00 00 00 00 02 3C"), 12);
        assert!(ev.contains(&Event::RequestDone {
            opcode: 0x00,
            outcome: Outcome::Answered
        }));
        assert!(!s.is_busy());
        // Next request uses the sequence number from the device's ACK.
        let wire = s.send(vec![0x04, 0x01], 20).expect("idle");
        assert_eq!(wire[2], 0x01);
    }

    #[test]
    fn resends_with_same_seq_then_gives_up() {
        let cfg = Config {
            ack_timeout_ms: 100,
            max_attempts: 3,
            reply_timeout_ms: 500,
        };
        let mut s = Session::new(cfg);
        let first = s.send(vec![0x10, 0x00], 0).expect("idle");
        assert!(s.poll(50).is_empty());
        assert_eq!(sends(&s.poll(100)), vec![first.clone()]);
        assert_eq!(sends(&s.poll(200)), vec![first]);
        assert_eq!(
            s.poll(300),
            vec![Event::RequestDone {
                opcode: 0x10,
                outcome: Outcome::NoAck
            }]
        );
        assert!(!s.is_busy());
    }

    #[test]
    fn acked_without_reply_times_out() {
        let mut s = Session::new(Config::default());
        s.send(vec![0x22, 0x00, 0x01], 0).expect("idle");
        s.receive(&bytes("3E 01 01 00 00 00 00 02 3C"), 5);
        assert!(s.poll(4000).is_empty());
        assert_eq!(
            s.poll(5005),
            vec![Event::RequestDone {
                opcode: 0x22,
                outcome: Outcome::NoReply
            }]
        );
    }

    #[test]
    fn stale_ack_is_ignored() {
        let mut s = Session::new(Config::default());
        s.send(vec![0x10, 0x00], 0).expect("idle");
        // Same seq as the request: not a valid ACK for it.
        s.receive(&bytes("3E 01 00 00 00 00 00 01 3C"), 5);
        assert_eq!(sends(&s.poll(1000)).len(), 1, "request is sent again");
    }

    #[test]
    fn notification_and_its_repeats() {
        let mut s = Session::new(Config::default());
        let ntfy = bytes("3E 0C 01 00 00 00 08 69 02 01 02 00 01 00 14 98 3C");
        let ev = s.receive(&ntfy, 0);
        assert!(
            ev.iter()
                .any(|e| matches!(e, Event::Received { reply: false, .. }))
        );
        let ev = s.receive(&ntfy, 1);
        assert_eq!(
            ev,
            vec![
                Event::Send(bytes("3E 01 00 00 00 00 00 01 3C")),
                Event::Duplicate
            ]
        );
    }
}
