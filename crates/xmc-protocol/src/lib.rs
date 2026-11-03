//! Codec for the framing layer of the Sony "MDR" control protocol spoken over
//! Bluetooth RFCOMM by Sony headphones.
//!
//! This crate is deliberately small and pure: no I/O, no OS calls, no `unsafe`.
//! It only turns bytes into [`Frame`]s and back. Command semantics (opcodes,
//! payload layouts) live elsewhere and are added only once they are verified
//! against captures from real WH-1000XM3 / WH-1000XM4 headphones.
//!
//! # Wire format (as assumed until the hardware spike confirms it)
//!
//! ```text
//! 0x3E | data_type | seq | len (u32 big-endian) | payload[len] | checksum | 0x3C
//! ```
//!
//! * `checksum` is the 8-bit wrapping sum of `data_type`, `seq`, the four length
//!   bytes and every payload byte.
//! * Between the markers, any `0x3C`, `0x3D` or `0x3E` byte is escaped as
//!   `0x3D, byte & !0x10`. Decoding reverses it with `byte | 0x10`.

#![forbid(unsafe_code)]

mod frame;
mod reader;

pub use frame::{DataType, Frame, FrameError, ack_for};
pub use reader::{FrameReader, ReadEvent};

/// Start-of-frame marker (`>`).
pub const START: u8 = 0x3E;
/// End-of-frame marker (`<`).
pub const END: u8 = 0x3C;
/// Escape byte (`=`).
pub const ESCAPE: u8 = 0x3D;
/// Bit cleared on escaped bytes and set again when unescaping.
pub const ESCAPE_MASK: u8 = 0x10;
/// Largest payload this codec accepts. Real MDR payloads are tiny; this bound
/// protects against garbage length fields allocating huge buffers.
pub const MAX_PAYLOAD: usize = 4096;
