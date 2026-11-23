//! Winsock (`AF_BTH`) RFCOMM connection, the path the existing open-source
//! Sony clients use on Windows. Windows resolves the service UUID to an RFCOMM
//! channel itself, so this reaches services the WinRT lookup misses: on our
//! WH-1000XM3 (2026-11-07) WinRT listed no MDR service, while this connected
//! to MDR v1.

use std::sync::atomic::{AtomicBool, Ordering};
use windows::Win32::Devices::Bluetooth::{AF_BTH, BTHPROTO_RFCOMM, SOCKADDR_BTH};

use windows::Win32::Networking::WinSock::{
    SD_BOTH, SEND_RECV_FLAGS, SO_RCVTIMEO, SO_SNDTIMEO, SOCK_STREAM, SOCKADDR, SOCKET, SOL_SOCKET,
    WSACleanup, WSADATA, WSAETIMEDOUT, WSAGetLastError, WSAStartup, closesocket, connect, recv,
    send, setsockopt, shutdown, socket,
};
use windows::core::GUID;

/// A failed Winsock call and its error code (`WSAGetLastError`, or the code
/// `WSAStartup` returned). [`crate::DisconnectReason::from_wsa`] turns it into
/// words for the app; the text form is for logs and the probe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WsaError {
    pub call: &'static str,
    pub code: i32,
}

impl std::fmt::Display for WsaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} failed: WSA error {}", self.call, self.code)
    }
}

impl From<WsaError> for String {
    fn from(e: WsaError) -> String {
        e.to_string()
    }
}

/// Winsock initialised for the lifetime of this value.
struct WsaGuard;

impl WsaGuard {
    fn new() -> Result<Self, WsaError> {
        let mut data = WSADATA::default();
        // SAFETY: `data` is a valid, writable WSADATA; version 2.2 is supported
        // by every Windows version this tool targets.
        let rc = unsafe { WSAStartup(0x0202, &mut data) };
        if rc != 0 {
            return Err(WsaError {
                call: "WSAStartup",
                code: rc,
            });
        }
        Ok(Self)
    }
}

impl Drop for WsaGuard {
    fn drop(&mut self) {
        // SAFETY: paired with the successful WSAStartup in `new`.
        unsafe {
            WSACleanup();
        }
    }
}

/// An open RFCOMM socket, closed on drop. Winsock allows one thread to send
/// while another receives on the same socket, so it can be shared in an `Arc`.
pub struct RfcommSocket {
    socket: SOCKET,
    _wsa: WsaGuard,
    /// Set by `shutdown`: a `recv` that times out then returns as closed.
    closing: AtomicBool,
}

/// How long one `recv` waits before checking whether the socket is closing.
/// `shutdown` alone is not documented to wake a `recv` blocked on an AF_BTH
/// socket, so the reader never waits longer than this after a shutdown.
const RECV_TIMEOUT_MS: u32 = 1000;

/// How long one `send` may block. Without it a send to headphones that
/// stopped reading could wait for ever (audit 2026-11-23, point 5); a
/// timeout fails with WSAETIMEDOUT, which ends the connection.
const SEND_TIMEOUT_MS: u32 = 5000;

impl RfcommSocket {
    /// Sends every byte of `bytes`.
    pub fn send_all(&self, mut bytes: &[u8]) -> Result<(), WsaError> {
        while !bytes.is_empty() {
            // SAFETY: `self.socket` is open for the lifetime of `self` and
            // `bytes` is a valid slice.
            let n = unsafe { send(self.socket, bytes, SEND_RECV_FLAGS(0)) };
            if n <= 0 {
                return Err(last_error("send"));
            }
            bytes = &bytes[n as usize..];
        }
        Ok(())
    }

    /// Blocks until data arrives. `Ok(0)` means the remote side closed, or
    /// that this socket was shut down (seen within [`RECV_TIMEOUT_MS`]).
    pub fn recv(&self, buf: &mut [u8]) -> Result<usize, WsaError> {
        loop {
            // SAFETY: `self.socket` is open for the lifetime of `self` and
            // `buf` is a valid, writable slice.
            let n = unsafe { recv(self.socket, buf, SEND_RECV_FLAGS(0)) };
            if n >= 0 {
                return Ok(n as usize);
            }
            // SAFETY: called right after the failing recv on this thread.
            let err = unsafe { WSAGetLastError() };
            if err != WSAETIMEDOUT {
                return Err(WsaError {
                    call: "recv",
                    code: err.0,
                });
            }
            // Only a timeout: nothing arrived. Stop if we are closing.
            if self.closing.load(Ordering::Acquire) {
                return Ok(0);
            }
        }
    }

    /// Shuts both directions down. A `recv` blocked in another thread
    /// returns at the latest when its timeout expires.
    pub fn shutdown(&self) {
        self.closing.store(true, Ordering::Release);
        // SAFETY: `self.socket` is open for the lifetime of `self`.
        unsafe {
            shutdown(self.socket, SD_BOTH);
        }
    }
}

impl Drop for RfcommSocket {
    fn drop(&mut self) {
        // SAFETY: `socket` was returned by a successful `socket()` call and is
        // closed exactly once, here.
        unsafe {
            closesocket(self.socket);
        }
    }
}

fn last_error(call: &'static str) -> WsaError {
    // SAFETY: called right after the failing Winsock call on this thread.
    let err = unsafe { WSAGetLastError() };
    WsaError { call, code: err.0 }
}

/// Opens an RFCOMM connection to `address`, letting Windows look up the
/// channel for `service` through SDP. Sends no data.
pub fn connect_rfcomm(address: u64, service: GUID) -> Result<RfcommSocket, WsaError> {
    let wsa = WsaGuard::new()?;
    // SAFETY: plain socket creation with constant, valid arguments.
    let socket = unsafe { socket(i32::from(AF_BTH), SOCK_STREAM, BTHPROTO_RFCOMM as i32) }
        .map_err(|_| last_error("socket(AF_BTH)"))?;
    let opened = RfcommSocket {
        socket,
        _wsa: wsa,
        closing: AtomicBool::new(false),
    };
    let timeout = RECV_TIMEOUT_MS.to_le_bytes();
    // SAFETY: `opened.socket` is a valid socket; SO_RCVTIMEO takes a DWORD in
    // milliseconds, passed as its 4 bytes.
    if unsafe { setsockopt(opened.socket, SOL_SOCKET, SO_RCVTIMEO, Some(&timeout)) } != 0 {
        return Err(last_error("setsockopt(SO_RCVTIMEO)"));
    }
    let timeout = SEND_TIMEOUT_MS.to_le_bytes();
    // SAFETY: as above; SO_SNDTIMEO takes the same DWORD in milliseconds.
    if unsafe { setsockopt(opened.socket, SOL_SOCKET, SO_SNDTIMEO, Some(&timeout)) } != 0 {
        return Err(last_error("setsockopt(SO_SNDTIMEO)"));
    }
    let addr = SOCKADDR_BTH {
        addressFamily: AF_BTH,
        btAddr: address,
        serviceClassId: service,
        port: 0,
    };
    // SAFETY: `addr` is a fully initialised SOCKADDR_BTH that outlives the call,
    // and the length passed matches its size.
    let rc = unsafe {
        connect(
            opened.socket,
            std::ptr::from_ref(&addr).cast::<SOCKADDR>(),
            std::mem::size_of::<SOCKADDR_BTH>() as i32,
        )
    };
    if rc != 0 {
        return Err(last_error("connect"));
    }
    Ok(opened)
}
