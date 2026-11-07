//! Windows side of XMCompanion: finds the paired headphones, opens the Sony
//! MDR channel over Winsock and runs [`xmc_protocol::client::Client`] on a
//! background thread.
//!
//! Verified on our WH-1000XM3 (2026-11-07): the MDR v1 service is reachable
//! only through Winsock `AF_BTH`; the WinRT RFCOMM lookup does not list it.

#![cfg(windows)]

pub mod bluetooth;
pub mod winsock;
mod worker;

pub use worker::{DeviceEvent, Worker};
