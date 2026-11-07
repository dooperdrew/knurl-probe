//! Discovery of Sony MDR RFCOMM services through WinRT.

use crate::ServiceKind;
use windows::Devices::Bluetooth::Rfcomm::{RfcommDeviceService, RfcommServiceId};
use windows::Devices::Bluetooth::{BluetoothCacheMode, BluetoothDevice};
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

/// Every paired device exposing one of the two MDR services. The headphones
/// must be switched on for the live lookup to reach them.
///
/// Two lookups are combined:
/// 1. the Windows device index for each MDR UUID (fast, but Windows does not
///    always index custom RFCOMM services);
/// 2. a live SDP query on every paired classic device
///    (`BluetoothCacheMode::Uncached`), which finds the service even when it
///    is not indexed. On our WH-1000XM3 (2026-11-07) the index was empty.
pub fn find_services() -> windows::core::Result<Vec<FoundService>> {
    let mut found: Vec<FoundService> = Vec::new();

    for kind in [ServiceKind::V1, ServiceKind::V2] {
        let id = RfcommServiceId::FromUuid(uuid_of(kind))?;
        let selector = RfcommDeviceService::GetDeviceSelector(&id)?;
        let infos = DeviceInformation::FindAllAsyncAqsFilter(&selector)?.join()?;
        for info in infos {
            if let Ok(service) = RfcommDeviceService::FromIdAsync(&info.Id()?)?.join() {
                push_unique(&mut found, kind, service)?;
            }
        }
    }

    for device in paired_classic_devices()? {
        for kind in [ServiceKind::V1, ServiceKind::V2] {
            let id = RfcommServiceId::FromUuid(uuid_of(kind))?;
            let Ok(result) = device
                .GetRfcommServicesForIdWithCacheModeAsync(&id, BluetoothCacheMode::Uncached)?
                .join()
            else {
                continue;
            };
            for service in result.Services()? {
                push_unique(&mut found, kind, service)?;
            }
        }
    }
    Ok(found)
}

fn push_unique(
    found: &mut Vec<FoundService>,
    kind: ServiceKind,
    service: RfcommDeviceService,
) -> windows::core::Result<()> {
    let device = service.Device()?;
    let address = device.BluetoothAddress()?;
    if found.iter().any(|f| f.kind == kind && f.address == address) {
        return Ok(());
    }
    found.push(FoundService {
        kind,
        device_name: device.Name()?.to_string_lossy(),
        address,
        service,
    });
    Ok(())
}

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

/// Prints every RFCOMM service each paired classic device reports in a live
/// SDP query. Diagnostic only.
pub fn print_all_rfcomm() -> Result<(), Box<dyn std::error::Error>> {
    let devices = paired_classic_devices()?;
    if devices.is_empty() {
        println!("No paired classic Bluetooth device could be opened.");
        return Ok(());
    }
    for device in devices {
        println!("{}", device.Name()?.to_string_lossy());
        match device
            .GetRfcommServicesWithCacheModeAsync(BluetoothCacheMode::Uncached)?
            .join()
        {
            Ok(result) => {
                let services = result.Services()?;
                if services.Size()? == 0 {
                    println!("  no RFCOMM services (SDP result: {:?})", result.Error()?);
                }
                for service in services {
                    let uuid = service.ServiceId()?.Uuid()?;
                    let tag = if uuid == MDR_V1 {
                        "  <- Sony MDR v1"
                    } else if uuid == MDR_V2 {
                        "  <- Sony MDR v2"
                    } else {
                        ""
                    };
                    println!("  {uuid:?}{tag}");
                }
            }
            Err(e) => println!("  SDP query failed: {e}"),
        }
    }
    Ok(())
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
             (the classic entry, e.g. \"WH-1000XM4\", not only \"LE_WH-1000XM4\") and switched on.\n\
             Run `devices --all-services` to see what each paired device reports."
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
