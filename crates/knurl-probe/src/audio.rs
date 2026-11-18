//! Lists Windows audio endpoints through the Core Audio COM API.
//!
//! Read-only: nothing here changes a default device. The goal is to see how the
//! headphones' stereo ("Headphones") and Hands-Free ("Headset") endpoints are
//! described, so the app can tell them apart reliably.

use windows::Win32::Devices::FunctionDiscovery::PKEY_Device_FriendlyName;
use windows::Win32::Foundation::PROPERTYKEY;
use windows::Win32::Media::Audio::{
    DEVICE_STATE, DEVICE_STATE_ACTIVE, DEVICE_STATEMASK_ALL, EDataFlow, ERole, IMMDevice,
    IMMDeviceEnumerator, IMMEndpoint, MMDeviceEnumerator, PKEY_AudioEndpoint_FormFactor, eAll,
    eCapture, eCommunications, eConsole, eRender,
};
use windows::Win32::System::Com::{
    CLSCTX_ALL, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx, CoTaskMemFree,
    CoUninitialize, STGM_READ,
};
use windows::core::{Interface, PWSTR};

/// Balances `CoInitializeEx` on every exit path.
struct ComGuard;

impl ComGuard {
    fn new() -> windows::core::Result<Self> {
        // SAFETY: plain COM initialisation of the calling thread; paired with
        // CoUninitialize in Drop only when it succeeded (S_OK or S_FALSE).
        unsafe { CoInitializeEx(None, COINIT_MULTITHREADED).ok()? };
        Ok(Self)
    }
}

impl Drop for ComGuard {
    fn drop(&mut self) {
        // SAFETY: matches the successful CoInitializeEx in `new`; all COM
        // interfaces created under this guard are dropped before it.
        unsafe { CoUninitialize() };
    }
}

struct Endpoint {
    id: String,
    name: String,
    flow: &'static str,
    state: &'static str,
    form_factor: String,
}

fn form_factor_name(v: u32) -> String {
    let name = match v {
        0 => "RemoteNetworkDevice",
        1 => "Speakers",
        2 => "LineLevel",
        3 => "Headphones",
        4 => "Microphone",
        5 => "Headset",
        6 => "Handset",
        7 => "UnknownDigitalPassthrough",
        8 => "SPDIF",
        9 => "DigitalAudioDisplayDevice",
        10 => "UnknownFormFactor",
        _ => return format!("Unknown({v})"),
    };
    name.to_string()
}

fn state_name(s: DEVICE_STATE) -> &'static str {
    match s.0 {
        1 => "active",
        2 => "disabled",
        4 => "not-present",
        8 => "unplugged",
        _ => "other",
    }
}

/// Takes ownership of a COM-allocated wide string and frees it.
fn take_pwstr(p: PWSTR) -> String {
    // SAFETY: `p` was returned by a COM method that documents the caller must
    // free it with CoTaskMemFree; it is a valid NUL-terminated UTF-16 string.
    let s = unsafe { p.to_string() }.unwrap_or_default();
    // SAFETY: frees exactly the buffer allocated by COM for `p`, once.
    unsafe { CoTaskMemFree(Some(p.0 as *const _)) };
    s
}

fn read_prop_string(dev: &IMMDevice, key: &PROPERTYKEY) -> windows::core::Result<String> {
    // SAFETY: `dev` is a live IMMDevice; the returned store is ref-counted.
    let store = unsafe { dev.OpenPropertyStore(STGM_READ)? };
    // SAFETY: `key` points to a valid static PROPERTYKEY.
    let value = unsafe { store.GetValue(key)? };
    Ok(value.to_string())
}

fn read_prop_u32(dev: &IMMDevice, key: &PROPERTYKEY) -> Option<u32> {
    // SAFETY: as in `read_prop_string`.
    let store = unsafe { dev.OpenPropertyStore(STGM_READ).ok()? };
    // SAFETY: as in `read_prop_string`.
    let value = unsafe { store.GetValue(key).ok()? };
    u32::try_from(&value).ok()
}

fn default_id(en: &IMMDeviceEnumerator, flow: EDataFlow, role: ERole) -> Option<String> {
    // SAFETY: `en` is a live enumerator; the call fails cleanly when no default exists.
    let dev = unsafe { en.GetDefaultAudioEndpoint(flow, role).ok()? };
    // SAFETY: `dev` is a live IMMDevice.
    unsafe { dev.GetId().ok() }.map(take_pwstr)
}

/// Endpoint id paired with a human-readable default role.
type DefaultRole = (String, &'static str);

fn collect(all: bool) -> windows::core::Result<(Vec<Endpoint>, Vec<DefaultRole>)> {
    let _com = ComGuard::new()?;
    // SAFETY: standard creation of the system MMDeviceEnumerator.
    let en: IMMDeviceEnumerator =
        unsafe { CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)? };
    let mask = if all {
        DEVICE_STATE(DEVICE_STATEMASK_ALL)
    } else {
        DEVICE_STATE_ACTIVE
    };
    // SAFETY: `en` is live; mask and flow are valid constants.
    let coll = unsafe { en.EnumAudioEndpoints(eAll, mask)? };
    // SAFETY: `coll` is a live collection.
    let count = unsafe { coll.GetCount()? };

    let mut endpoints = Vec::new();
    for i in 0..count {
        // SAFETY: `i` is within 0..count.
        let dev = unsafe { coll.Item(i)? };
        // SAFETY: `dev` is a live IMMDevice.
        let id = take_pwstr(unsafe { dev.GetId()? });
        // SAFETY: `dev` is a live IMMDevice.
        let state = unsafe { dev.GetState()? };
        let flow = match dev.cast::<IMMEndpoint>() {
            // SAFETY: `ep` is a live IMMEndpoint obtained from `dev`.
            Ok(ep) => match unsafe { ep.GetDataFlow() } {
                Ok(f) if f == eRender => "render",
                Ok(f) if f == eCapture => "capture",
                _ => "?",
            },
            Err(_) => "?",
        };
        endpoints.push(Endpoint {
            name: read_prop_string(&dev, &PKEY_Device_FriendlyName).unwrap_or_default(),
            form_factor: read_prop_u32(&dev, &PKEY_AudioEndpoint_FormFactor)
                .map(form_factor_name)
                .unwrap_or_else(|| "?".into()),
            state: state_name(state),
            flow,
            id,
        });
    }

    let mut defaults = Vec::new();
    for (flow, fname) in [(eRender, "render"), (eCapture, "capture")] {
        for (role, rname) in [(eConsole, "console"), (eCommunications, "communications")] {
            if let Some(id) = default_id(&en, flow, role) {
                defaults.push((
                    id,
                    match (fname, rname) {
                        ("render", "console") => "default render",
                        ("render", _) => "default render (communications)",
                        (_, "console") => "default capture",
                        _ => "default capture (communications)",
                    },
                ));
            }
        }
    }
    Ok((endpoints, defaults))
}

pub fn print_endpoints(all: bool) -> Result<(), Box<dyn std::error::Error>> {
    let (endpoints, defaults) = collect(all)?;
    println!(
        "{:<8} {:<12} {:<26} {:<45} roles",
        "flow", "state", "form factor", "name"
    );
    for e in &endpoints {
        let roles: Vec<&str> = defaults
            .iter()
            .filter(|(id, _)| *id == e.id)
            .map(|(_, r)| *r)
            .collect();
        println!(
            "{:<8} {:<12} {:<26} {:<45} {}",
            e.flow,
            e.state,
            e.form_factor,
            e.name,
            roles.join(", ")
        );
    }
    println!(
        "\n{} endpoint(s). Ids are omitted; use --all to include inactive ones.",
        endpoints.len()
    );
    Ok(())
}
