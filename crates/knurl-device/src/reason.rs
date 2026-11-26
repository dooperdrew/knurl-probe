//! Why a connection ended, in kinds the app can say in words, and the
//! watch that ends a connection the headphones stopped answering. No Windows
//! dependency: built and tested on every platform.

use knurl_protocol::session::Outcome;

use std::time::{Duration, SystemTime};

/// Requests in a row with no ACK, while nothing else came from the
/// headphones, after which the connection is closed and opened again: a
/// link that stopped answering. Each request already tried 10 times, one
/// second apart, before it counts, so one is enough: a dead link is found in
/// about 10 s, not 2-3 minutes (audit 2026-11-24, point 5).
pub const MAX_UNACKED: u32 = 1;

/// A jump of the wall clock, between two turns of the connection loop, at
/// least this long means the PC slept: the loop turns every 50 ms or so
/// (a blocked send waits at most 5 s).
pub const SLEEP_GAP: Duration = Duration::from_secs(30);

/// Why the connection to the headphones could not open or ended. The app
/// shows the kind in its language; `detail` (the Winsock call and its code)
/// goes only to the log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DisconnectReason {
    /// The connection did not open: the headphones are off, out of range
    /// or busy with another device (connect timed out, was refused, or
    /// Bluetooth is down).
    Unreachable { detail: String },
    /// An open connection broke: out of range, turned off, link reset.
    Lost { detail: String },
    /// Windows does not see the headphones connected to this PC (off, out
    /// of range or with another device), so Knurl does not try: a try would
    /// page them over the radio every few seconds (audit of Knurl
    /// 2026-11-26, point 1). It connects once Windows sees them.
    NotConnected,
    /// Another program holds the headphones' control channel (WSA 10048),
    /// for example an old XMCompanion still running.
    InUse { detail: String },
    /// The headphones closed the connection themselves.
    ClosedByHeadphones,
    /// The headphones stopped answering: [`MAX_UNACKED`] requests in a row
    /// got no ACK, so Knurl closed the connection to open it again.
    NotAnswering,
    /// The PC woke from sleep: what the headphones reported before may be
    /// old, so Knurl closed the connection to open it again.
    Resumed,
    /// Knurl was asked to stop.
    Stopped,
    /// Anything else, with what failed.
    Failed { detail: String },
}

impl DisconnectReason {
    /// The kind of a Winsock failure. `connecting`: it happened while the
    /// connection was being opened.
    pub fn from_wsa(connecting: bool, call: &str, code: i32) -> DisconnectReason {
        let detail = format!("{call} failed: WSA error {code}");
        // Winsock codes (winerror.h): 10050 network down, 10051 network
        // unreachable, 10052 dropped on reset, 10053 aborted, 10054 reset by
        // peer, 10060 timed out, 10061 refused, 10064 host down, 10065 host
        // unreachable, 10048 address in use (another program holds the
        // channel).
        match code {
            10048 => DisconnectReason::InUse { detail },
            10050 | 10051 | 10060 | 10061 | 10064 | 10065 if connecting => {
                DisconnectReason::Unreachable { detail }
            }
            10050 | 10052 | 10053 | 10054 | 10060 | 10064 | 10065 if !connecting => {
                DisconnectReason::Lost { detail }
            }
            _ => DisconnectReason::Failed { detail },
        }
    }

    /// What failed, for the log; empty when the kind says it all.
    pub fn detail(&self) -> &str {
        match self {
            DisconnectReason::Unreachable { detail }
            | DisconnectReason::Lost { detail }
            | DisconnectReason::InUse { detail }
            | DisconnectReason::Failed { detail } => detail,
            DisconnectReason::NotConnected
            | DisconnectReason::ClosedByHeadphones
            | DisconnectReason::NotAnswering
            | DisconnectReason::Resumed
            | DisconnectReason::Stopped => "",
        }
    }
}

impl std::fmt::Display for DisconnectReason {
    /// English, for the log and the activity log of a problem report.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let kind = match self {
            DisconnectReason::Unreachable { .. } => "unreachable",
            DisconnectReason::Lost { .. } => "connection lost",
            DisconnectReason::NotConnected => "not connected to this PC",
            DisconnectReason::InUse { .. } => "another program holds the control channel",
            DisconnectReason::ClosedByHeadphones => "closed by the headphones",
            DisconnectReason::NotAnswering => "the headphones stopped answering",
            DisconnectReason::Resumed => "the PC woke from sleep",
            DisconnectReason::Stopped => "stopped",
            DisconnectReason::Failed { .. } => "failed",
        };
        match self.detail() {
            "" => f.write_str(kind),
            detail => write!(f, "{kind} ({detail})"),
        }
    }
}

/// Counts requests in a row that got no ACK while nothing else arrived.
/// Anything received from the headphones shows the link is alive: it starts
/// the count again, and a request that ends without an ACK after something
/// else arrived does not count.
#[derive(Debug, Default)]
pub struct AckWatch {
    unacked: u32,
    /// Something arrived since the last request ended.
    heard: bool,
}

impl AckWatch {
    /// Bytes arrived from the headphones.
    pub fn received(&mut self) {
        self.unacked = 0;
        self.heard = true;
    }

    /// A request ended with `outcome`. True when the connection should be
    /// closed: [`MAX_UNACKED`] requests in a row got no ACK and nothing else
    /// arrived meanwhile.
    pub fn request_done(&mut self, outcome: Outcome) -> bool {
        let heard = std::mem::take(&mut self.heard);
        if outcome != Outcome::NoAck {
            self.unacked = 0;
        } else if !heard {
            self.unacked += 1;
        }
        self.unacked >= MAX_UNACKED
    }
}

/// Notices that the PC slept, from a jump of the wall clock between two
/// turns of the connection loop ([`SLEEP_GAP`]). The wall clock keeps
/// counting while the PC sleeps; a clock set forward by that much also
/// counts, and costs only a reconnect.
#[derive(Debug)]
pub struct WakeWatch {
    last: SystemTime,
}

impl WakeWatch {
    pub fn new(now: SystemTime) -> Self {
        Self { last: now }
    }

    /// One turn of the loop at `now`. True when the PC slept since the
    /// last turn.
    pub fn woke(&mut self, now: SystemTime) -> bool {
        let gap = now.duration_since(self.last).unwrap_or_default();
        self.last = now;
        gap >= SLEEP_GAP
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn winsock_failures_become_kinds_with_the_code_kept_for_the_log() {
        let off = DisconnectReason::from_wsa(true, "connect", 10060);
        assert_eq!(
            off,
            DisconnectReason::Unreachable {
                detail: "connect failed: WSA error 10060".into()
            }
        );
        assert_eq!(
            off.to_string(),
            "unreachable (connect failed: WSA error 10060)"
        );
        for code in [10050, 10051, 10061, 10064, 10065] {
            assert!(matches!(
                DisconnectReason::from_wsa(true, "connect", code),
                DisconnectReason::Unreachable { .. }
            ));
        }
        for code in [10053, 10054, 10060, 10064] {
            assert!(
                matches!(
                    DisconnectReason::from_wsa(false, "recv", code),
                    DisconnectReason::Lost { .. }
                ),
                "{code}"
            );
        }
        let odd = DisconnectReason::from_wsa(true, "connect", 10022);
        assert_eq!(odd.detail(), "connect failed: WSA error 10022");
        assert!(matches!(odd, DisconnectReason::Failed { .. }));
        assert_eq!(DisconnectReason::NotAnswering.detail(), "");
    }

    #[test]
    fn one_unacked_request_with_nothing_else_heard_closes_the_connection() {
        let mut watch = AckWatch::default();
        assert!(watch.request_done(Outcome::NoAck));
    }

    #[test]
    fn an_answer_or_any_byte_keeps_the_connection() {
        let mut watch = AckWatch::default();
        assert!(!watch.request_done(Outcome::NoReply), "acknowledged");
        watch.received();
        assert!(
            !watch.request_done(Outcome::NoAck),
            "something arrived while the request waited"
        );
        assert!(watch.request_done(Outcome::NoAck), "then silence");
        watch.received();
        assert!(!watch.request_done(Outcome::NoAck));
    }

    #[test]
    fn a_long_jump_of_the_clock_means_the_pc_slept() {
        let t0 = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000);
        let mut watch = WakeWatch::new(t0);
        assert!(!watch.woke(t0 + Duration::from_millis(50)));
        assert!(!watch.woke(t0 + Duration::from_secs(6)), "a slow send");
        assert!(watch.woke(t0 + Duration::from_secs(6) + SLEEP_GAP));
        assert!(!watch.woke(t0 + Duration::from_secs(37)));
        // A clock set back is not a sleep.
        assert!(!watch.woke(t0));
    }

    #[test]
    fn a_held_channel_has_its_own_kind() {
        for connecting in [true, false] {
            let r = DisconnectReason::from_wsa(connecting, "connect", 10048);
            assert!(matches!(r, DisconnectReason::InUse { .. }), "{r:?}");
            assert_eq!(r.detail(), "connect failed: WSA error 10048");
        }
        assert_eq!(
            DisconnectReason::NotConnected.to_string(),
            "not connected to this PC"
        );
    }

    #[test]
    fn resumed_has_words_and_no_detail() {
        assert_eq!(DisconnectReason::Resumed.detail(), "");
        assert_eq!(
            DisconnectReason::Resumed.to_string(),
            "the PC woke from sleep"
        );
    }
}
