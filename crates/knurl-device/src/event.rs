//! What the device thread reports to the UI. A plain type with no Windows
//! dependency, so code that only reads events (the app's view logic and its
//! tests) builds on every platform.

use knurl_protocol::client::DeviceState;
use knurl_protocol::session::Outcome;

/// What the worker reports. Every state the UI shows comes from one of these.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeviceEvent {
    /// No paired WH-1000XM3 / WH-1000XM4 was found in Windows.
    NoHeadphones,
    /// Windows refused or failed the Bluetooth lookup.
    BluetoothError(String),
    /// A problem of this PC's setup found when the device thread started. It
    /// stays true for the app's whole run, so the UI keeps showing it.
    SetupError(String),
    /// A failure inside Knurl itself, not of Bluetooth or the headphones:
    /// the device thread panicked (`recovers`: it starts again by itself),
    /// or it is gone or could not start (`recovers` false: nothing reaches
    /// the headphones until the app is started again).
    Internal {
        error: String,
        recovers: bool,
    },
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
