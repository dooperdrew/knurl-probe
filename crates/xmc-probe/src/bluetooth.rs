//! Discovery of Sony MDR RFCOMM services through WinRT.

use crate::ServiceKind;
use windows::Devices::Bluetooth::Rfcomm::{RfcommDeviceService, RfcommServiceId};
use windows::Devices::Enumeration::DeviceInformation;
use windows::core::{GUID, HSTRING};

pub const MDR_V1: GUID = GUID::from_u128(0x96cc203e_5068_46ad_b32d_e316f5e069ba);
pub const MDR_V2: GUID = GUID::from_u128(0x956c7b26_d49a_4ba8_b03f_b17d393cb6e2);

pub fn uuid_of(kind: ServiceKind) -> GUID {
    match kind {
        ServiceKind::V1 => MDR_V1,
        ServiceKind::V2 => MDR_V2,
    }
}

pub struct FoundService {
    pub kind: ServiceKind,
    pub device_name: String,
    pub address: u64,
    pub service: RfcommDeviceService,
}

/// Every paired device exposing one of the two MDR services. Windows may need
/// the headphones to be switched on for the service to be reachable.
pub fn find_services() -> windows::core::Result<Vec<FoundService>> {
    let mut found = Vec::new();
    for kind in [ServiceKind::V1, ServiceKind::V2] {
        let id = RfcommServiceId::FromUuid(uuid_of(kind))?;
        let selector = RfcommDeviceService::GetDeviceSelector(&id)?;
        let infos = DeviceInformation::FindAllAsyncAqsFilter(&selector)?.join()?;
        for info in infos {
            let Ok(service) = RfcommDeviceService::FromIdAsync(&info.Id()?)?.join() else {
                continue;
            };
            let device = service.Device()?;
            found.push(FoundService {
                kind,
                device_name: device.Name()?.to_string_lossy(),
                address: device.BluetoothAddress()?,
                service,
            });
        }
    }
    Ok(found)
}

pub fn format_address(addr: u64) -> String {
    let b = addr.to_be_bytes();
    format!(
        "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
        b[2], b[3], b[4], b[5], b[6], b[7]
    )
}

pub fn print_devices() -> Result<(), Box<dyn std::error::Error>> {
    let services = find_services()?;
    if services.is_empty() {
        println!(
            "No paired device exposes a Sony MDR service.\n\
             Check that the headphones are paired in Windows Settings > Bluetooth\n\
             (the classic entry, e.g. \"WH-1000XM4\", not only \"LE_WH-1000XM4\") and switched on."
        );
        return Ok(());
    }
    println!(
        "{:<4} {:<24} {:<18} host / service",
        "MDR", "device", "address"
    );
    for s in &services {
        let host = s
            .service
            .ConnectionHostName()
            .and_then(|h| h.DisplayName())
            .unwrap_or_else(|_| HSTRING::from("?"));
        let svc = s
            .service
            .ConnectionServiceName()
            .unwrap_or_else(|_| HSTRING::from("?"));
        println!(
            "{:<4} {:<24} {:<18} {} / {}",
            format!("{:?}", s.kind).to_lowercase(),
            s.device_name,
            format_address(s.address),
            host,
            svc
        );
    }
    Ok(())
}
