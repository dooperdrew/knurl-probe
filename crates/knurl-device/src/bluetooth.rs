//! Paired Bluetooth Classic devices, through WinRT (enumeration works; only
//! the RFCOMM service lookup is unreliable, see the crate docs).

use windows::Devices::Bluetooth::BluetoothDevice;
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
}

/// Paired headphones of a supported model (WH-1000XM3 or WH-1000XM4).
pub fn paired_supported_headphones() -> windows::core::Result<Vec<Paired>> {
    let mut found = Vec::new();
    for device in paired_classic_devices()? {
        let name = device.Name()?.to_string_lossy();
        if knurl_protocol::client::is_supported_model(&name) {
            found.push(Paired {
                name,
                address: device.BluetoothAddress()?,
            });
        }
    }
    Ok(found)
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
    fn formats_addresses() {
        assert_eq!(format_address(0x0000_1234_5678_9ABC), "12:34:56:78:9A:BC");
    }
}
