//! Why a connection ended, in kinds the app can say in words, and the
//! watch that ends a connection the headphones stopped answering. No Windows
//! dependency: built and tested on every platform.

use knurl_protocol::session::Outcome;

/// Requests in a row with no ACK after which the connection is closed and
/// opened again: a link that stopped answering (audit 2026-11-23, point 4).
/// Each request already retries before it counts as one.
pub const MAX_UNACKED: u32 = 3;

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
    /// The headphones closed the connection themselves.
    ClosedByHeadphones,
    /// The headphones stopped answering: [`MAX_UNACKED`] requests in a row
    /// got no ACK, so Knurl closed the connection to open it again.
    NotAnswering,
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
        // unreachable.
        match code {
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
            | DisconnectReason::Failed { detail } => detail,
            DisconnectReason::ClosedByHeadphones
            | DisconnectReason::NotAnswering
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
            DisconnectReason::ClosedByHeadphones => "closed by the headphones",
            DisconnectReason::NotAnswering => "the headphones stopped answering",
            DisconnectReason::Stopped => "stopped",
            DisconnectReason::Failed { .. } => "failed",
        };
        match self.detail() {
            "" => f.write_str(kind),
            detail => write!(f, "{kind} ({detail})"),
        }
    }
}

/// Counts requests in a row that got no ACK. Anything received from the
/// headphones shows the link is alive and starts the count again.
#[derive(Debug, Default)]
pub struct AckWatch {
    unacked: u32,
}

impl AckWatch {
    /// Bytes arrived from the headphones.
    pub fn received(&mut self) {
        self.unacked = 0;
    }

    /// A request ended with `outcome`. True when the connection should be
    /// closed: [`MAX_UNACKED`] requests in a row got no ACK.
    pub fn request_done(&mut self, outcome: Outcome) -> bool {
        if outcome == Outcome::NoAck {
            self.unacked += 1;
        } else {
            self.unacked = 0;
        }
        self.unacked >= MAX_UNACKED
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
    fn three_unacked_requests_in_a_row_close_the_connection() {
        let mut watch = AckWatch::default();
        assert!(!watch.request_done(Outcome::NoAck));
        assert!(!watch.request_done(Outcome::NoAck));
        assert!(watch.request_done(Outcome::NoAck));
    }

    #[test]
    fn an_answer_or_any_byte_starts_the_count_again() {
        let mut watch = AckWatch::default();
        watch.request_done(Outcome::NoAck);
        watch.request_done(Outcome::NoAck);
        watch.received();
        assert!(!watch.request_done(Outcome::NoAck));
        assert!(!watch.request_done(Outcome::NoAck));
        assert!(!watch.request_done(Outcome::NoReply), "acknowledged");
        assert!(!watch.request_done(Outcome::NoAck));
    }
}
