//! Windows side of Knurl: finds the paired headphones, opens the Sony
//! MDR channel over Winsock and runs [`knurl_protocol::client::Client`] on a
//! background thread.
//!
//! Verified on our WH-1000XM3 (2026-11-07): the MDR v1 service is reachable
//! only through Winsock `AF_BTH`; the WinRT RFCOMM lookup does not list it.

//!
//! Only [`DeviceEvent`] exists on other platforms: the connection itself is
//! Windows-only, and the event type lets the app's pure logic build and be
//! tested anywhere.

#[cfg(windows)]
pub mod bluetooth;
mod event;
#[cfg(windows)]
pub mod winsock;
#[cfg(windows)]
mod worker;

pub use event::DeviceEvent;
#[cfg(windows)]
pub use worker::Worker;

/// Keeps the process's COM multithreaded apartment (MTA) alive until the
/// process exits. Call it before the first WinRT call on a thread.
///
/// Why (crash of 2026-11-08, from the dump): the `windows` crate caches WinRT
/// factories, such as the one behind `BluetoothDevice`, in statics. A thread
/// that never initialises COM uses the MTA another thread created. When that
/// thread left the apartment (the Hands-Free scan did every 2 s), Windows could
/// tear the MTA down and unload `Windows.Devices.Bluetooth.dll`, and the next
/// lookup called the cached factory inside the unloaded DLL: access violation.
/// The usage reference taken here is never released, so the MTA, and with it
/// the DLLs, stay as long as the app runs.
#[cfg(windows)]
pub fn keep_com_apartment() -> Result<(), String> {
    use std::sync::OnceLock;
    static KEPT: OnceLock<Result<(), String>> = OnceLock::new();
    KEPT.get_or_init(|| {
        // SAFETY: plain call with no arguments; the returned cookie is
        // deliberately never passed to CoDecrementMTAUsage.
        unsafe { windows::Win32::System::Com::CoIncrementMTAUsage() }
            .map(|_cookie| ())
            .map_err(|e| format!("could not keep the COM apartment: {e}"))
    })
    .clone()
}

#[cfg(all(test, windows))]
mod tests {
    #[test]
    fn the_apartment_is_kept_once_and_reported() {
        assert_eq!(super::keep_com_apartment(), Ok(()));
        assert_eq!(super::keep_com_apartment(), Ok(()));
    }
}
