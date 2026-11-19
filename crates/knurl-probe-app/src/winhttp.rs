//! The Windows transport for [`knurl_probe_app::send`]: one HTTPS POST with
//! WinHTTP, part of Windows (system proxy settings, TLS from Windows), the
//! same way the Knurl app sends its reports. Called on a worker thread only.

use std::ffi::c_void;

use knurl_probe_app::send::{Transport, target};
use windows::Win32::Networking::WinHttp::{
    WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY, WINHTTP_FLAG_SECURE, WINHTTP_OPEN_REQUEST_FLAGS,
    WINHTTP_QUERY_FLAG_NUMBER, WINHTTP_QUERY_STATUS_CODE, WinHttpCloseHandle, WinHttpConnect,
    WinHttpOpen, WinHttpOpenRequest, WinHttpQueryHeaders, WinHttpReadData, WinHttpReceiveResponse,
    WinHttpSendRequest, WinHttpSetTimeouts,
};
use windows::core::{HSTRING, PCWSTR, w};

/// A WinHTTP handle, closed when dropped.
struct Handle(*mut c_void);

impl Handle {
    fn new(raw: *mut c_void, what: &str) -> Result<Handle, String> {
        if raw.is_null() {
            Err(format!("{what}: {}", windows::core::Error::from_thread()))
        } else {
            Ok(Handle(raw))
        }
    }
}

impl Drop for Handle {
    fn drop(&mut self) {
        // SAFETY: the handle came from WinHTTP, is not null and is closed once.
        let _ = unsafe { WinHttpCloseHandle(self.0) };
    }
}

pub struct WinHttp;

impl Transport for WinHttp {
    fn post(&self, url: &str, key: &str, body: &str) -> Result<(u16, String), String> {
        let target = target(url)?;
        let agent = HSTRING::from(format!("KnurlProbe/{}", env!("CARGO_PKG_VERSION")));
        // SAFETY: the agent string outlives the call; null proxy strings are allowed.
        let session = Handle::new(
            unsafe {
                WinHttpOpen(
                    &agent,
                    WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY,
                    PCWSTR::null(),
                    PCWSTR::null(),
                    0,
                )
            },
            "WinHttpOpen",
        )?;
        // SAFETY: valid session handle; timeouts in milliseconds.
        unsafe { WinHttpSetTimeouts(session.0, 10_000, 10_000, 20_000, 20_000) }
            .map_err(|e| format!("WinHttpSetTimeouts: {e}"))?;
        let host = HSTRING::from(target.host.as_str());
        // SAFETY: valid session handle; the host string outlives the call.
        let connection = Handle::new(
            unsafe { WinHttpConnect(session.0, &host, target.port, 0) },
            "WinHttpConnect",
        )?;
        let path = HSTRING::from(target.path.as_str());
        let flags = if target.secure {
            WINHTTP_FLAG_SECURE
        } else {
            WINHTTP_OPEN_REQUEST_FLAGS(0)
        };
        // SAFETY: valid connection handle; the strings outlive the call; null
        // version, referrer and accept types are allowed.
        let request = Handle::new(
            unsafe {
                WinHttpOpenRequest(
                    connection.0,
                    w!("POST"),
                    &path,
                    PCWSTR::null(),
                    PCWSTR::null(),
                    std::ptr::null(),
                    flags,
                )
            },
            "WinHttpOpenRequest",
        )?;
        let headers: Vec<u16> =
            format!("content-type: application/json; charset=utf-8\r\nx-bedrock-key: {key}\r\n")
                .encode_utf16()
                .collect();
        let bytes = body.as_bytes();
        let length =
            u32::try_from(bytes.len()).map_err(|_| "the report is too large".to_string())?;
        // SAFETY: valid request handle; headers and body outlive this
        // synchronous call and `length` is the body's size.
        unsafe {
            WinHttpSendRequest(
                request.0,
                Some(&headers),
                Some(bytes.as_ptr().cast()),
                length,
                length,
                0,
            )
        }
        .map_err(|e| e.message())?;
        // SAFETY: valid request handle, after a successful send.
        unsafe { WinHttpReceiveResponse(request.0, std::ptr::null_mut()) }
            .map_err(|e| format!("no answer: {}", e.message()))?;
        let mut status: u32 = 0;
        let mut size = std::mem::size_of::<u32>() as u32;
        // SAFETY: valid request handle; `status` is a u32 and `size` says so.
        unsafe {
            WinHttpQueryHeaders(
                request.0,
                WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER,
                PCWSTR::null(),
                Some((&raw mut status).cast()),
                &mut size,
                std::ptr::null_mut(),
            )
        }
        .map_err(|e| format!("unreadable answer: {e}"))?;
        let status = u16::try_from(status).map_err(|_| format!("HTTP status {status}"))?;
        Ok((status, read_body(&request)))
    }
}

/// Up to 4 KiB of the answer: the issue number or the relay's reason.
fn read_body(request: &Handle) -> String {
    let mut out = Vec::new();
    let mut buffer = [0u8; 1024];
    while out.len() < 4096 {
        let mut read: u32 = 0;
        // SAFETY: valid request handle; the buffer is writable for its length.
        let ok = unsafe {
            WinHttpReadData(
                request.0,
                buffer.as_mut_ptr().cast(),
                buffer.len() as u32,
                &mut read,
            )
        };
        if ok.is_err() || read == 0 {
            break;
        }
        out.extend_from_slice(&buffer[..read as usize]);
    }
    String::from_utf8_lossy(&out).into_owned()
}
