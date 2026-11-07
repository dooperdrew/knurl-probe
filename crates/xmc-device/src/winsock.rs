//! Winsock (`AF_BTH`) RFCOMM connection, the path the existing open-source
//! Sony clients use on Windows. Windows resolves the service UUID to an RFCOMM
//! channel itself, so this reaches services the WinRT lookup misses: on our
//! WH-1000XM3 (2026-11-07) WinRT listed no MDR service, while this connected
//! to MDR v1.

use windows::Win32::Devices::Bluetooth::{AF_BTH, BTHPROTO_RFCOMM, SOCKADDR_BTH};
use windows::Win32::Networking::WinSock::{
    SD_BOTH, SEND_RECV_FLAGS, SOCK_STREAM, SOCKADDR, SOCKET, WSACleanup, WSADATA, WSAGetLastError,
    WSAStartup, closesocket, connect, recv, send, shutdown, socket,
};
use windows::core::GUID;

/// Winsock initialised for the lifetime of this value.
struct WsaGuard;

impl WsaGuard {
    fn new() -> Result<Self, String> {
        let mut data = WSADATA::default();
        // SAFETY: `data` is a valid, writable WSADATA; version 2.2 is supported
        // by every Windows version this tool targets.
        let rc = unsafe { WSAStartup(0x0202, &mut data) };
        if rc != 0 {
            return Err(format!("WSAStartup failed with code {rc}"));
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
}

impl RfcommSocket {
    /// Sends every byte of `bytes`.
    pub fn send_all(&self, mut bytes: &[u8]) -> Result<(), String> {
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

    /// Blocks until data arrives. `Ok(0)` means the remote side closed.
    pub fn recv(&self, buf: &mut [u8]) -> Result<usize, String> {
        // SAFETY: `self.socket` is open for the lifetime of `self` and `buf`
        // is a valid, writable slice.
        let n = unsafe { recv(self.socket, buf, SEND_RECV_FLAGS(0)) };
        if n < 0 {
            return Err(last_error("recv"));
        }
        Ok(n as usize)
    }

    /// Shuts both directions down, which unblocks a pending `recv` in another
    /// thread.
    pub fn shutdown(&self) {
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

fn last_error(call: &str) -> String {
    // SAFETY: called right after the failing Winsock call on this thread.
    let err = unsafe { WSAGetLastError() };
    format!("{call} failed: WSA error {}", err.0)
}

/// Opens an RFCOMM connection to `address`, letting Windows look up the
/// channel for `service` through SDP. Sends no data.
pub fn connect_rfcomm(address: u64, service: GUID) -> Result<RfcommSocket, String> {
    let wsa = WsaGuard::new()?;
    // SAFETY: plain socket creation with constant, valid arguments.
    let socket = unsafe { socket(i32::from(AF_BTH), SOCK_STREAM, BTHPROTO_RFCOMM as i32) }
        .map_err(|e| format!("socket(AF_BTH) failed: {e}"))?;
    let opened = RfcommSocket { socket, _wsa: wsa };
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
