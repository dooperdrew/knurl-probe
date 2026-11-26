//! Paired Bluetooth Classic devices, through WinRT (enumeration works; only
//! the RFCOMM service lookup is unreliable, see the crate docs).

use windows::Devices::Bluetooth::{BluetoothConnectionStatus, BluetoothDevice};
use windows::Devices::Enumeration::DeviceInformation;
use windows::core::GUID;

/// Sony MDR service, protocol V1 (WH-1000XM3; WH-1000XM4 per the sources).
pub const MDR_V1: GUID = GUID::from_u128(0x96cc203e_5068_46ad_b32d_e316f5e069ba);
/// Sony MDR service, protocol V2 (newer models, per the sources).
pub const MDR_V2: GUID = GUID::from_u128(0x956c7b26_d49a_4ba8_b03f_b17d393cb6e2);

/// Paired classic (BR/EDR) Bluetooth devices. Devices Windows cannot open are
/// skipped.
pub fn paired_classic_devices() -> windows::core::Result<Vec<BluetoothDevice>> {
    let selector = BluetoothDevice::GetDeviceSelectorFromPairingState(true)?;
    let infos = DeviceInformation::FindAllAsyncAqsFilter(&selector)?.join()?;
    let mut devices = Vec::new();
    for info in infos {
        if let Ok(device) = BluetoothDevice::FromIdAsync(&info.Id()?)?.join() {
            devices.push(device);
        }
    }
    Ok(devices)
}

/// A paired device by name and Bluetooth address.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Paired {
    pub name: String,
    pub address: u64,
    /// Whether Windows sees it connected to this PC (`ConnectionStatus`);
    /// `None` when the status could not be read.
    pub connected: Option<bool>,
}

/// Paired headphones of a supported model (WH-1000XM3 or WH-1000XM4). A
/// device whose name or address cannot be read is skipped: one faulty
/// pairing must not hide the headphones (audit of Knurl, 2026-11-24).
pub fn paired_supported_headphones() -> windows::core::Result<Vec<Paired>> {
    Ok(supported(paired_classic_devices()?.iter().map(|device| {
        let name = device.Name()?.to_string_lossy();
        let connected = device
            .ConnectionStatus()
            .ok()
            .map(|s| s == BluetoothConnectionStatus::Connected);
        Ok::<_, windows::core::Error>((name, device.BluetoothAddress()?, connected))
    })))
}

/// The supported headphones among `devices` (name and address each, or the
/// error reading them), skipping the ones that could not be read.
fn supported<E>(
    devices: impl Iterator<Item = Result<(String, u64, Option<bool>), E>>,
) -> Vec<Paired> {
    devices
        .filter_map(Result::ok)
        .filter(|(name, _, _)| knurl_protocol::client::is_supported_model(name))
        .map(|(name, address, connected)| Paired {
            name,
            address,
            connected,
        })
        .collect()
}

/// `AA:BB:CC:DD:EE:FF` for a 48-bit Bluetooth address.
pub fn format_address(addr: u64) -> String {
    let b = addr.to_be_bytes();
    format!(
        "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
        b[2], b[3], b[4], b[5], b[6], b[7]
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_device_that_cannot_be_read_does_not_hide_the_others() {
        let devices = vec![
            Err("faulty pairing"),
            Ok(("Keyboard".to_string(), 1, Some(true))),
            Ok(("WH-1000XM3".to_string(), 2, Some(false))),
        ];
        assert_eq!(
            supported(devices.into_iter()),
            vec![Paired {
                name: "WH-1000XM3".into(),
                address: 2,
                connected: Some(false),
            }]
        );
    }

    #[test]
    fn formats_addresses() {
        assert_eq!(format_address(0x0000_1234_5678_9ABC), "12:34:56:78:9A:BC");
    }
}
