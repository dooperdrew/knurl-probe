//! The Windows side of the app. The flow lives on the window's thread
//! ([`APP`]); the search and the check run on their own threads and come back
//! through `slint::invoke_from_event_loop`. The Bluetooth address stays inside
//! `knurl_probe::check::Headphones` and is never shown or written.

use std::cell::RefCell;
use std::path::PathBuf;
use std::sync::Arc;

use knurl_probe::check::{self, Headphones};
use knurl_probe_app::flow::{Effect, Event, Flow};
use knurl_probe_app::ui::MainWindow;
use knurl_probe_app::{ISSUE_FORM, view};
use slint::ComponentHandle;
use windows::Win32::System::Com::CoTaskMemFree;
use windows::Win32::UI::Shell::{
    FOLDERID_Documents, KF_FLAG_DEFAULT, SHGetKnownFolderPath, ShellExecuteW,
};
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
use windows::core::{HSTRING, w};

struct App {
    ui: MainWindow,
    flow: Flow,
    /// The last search, in the order shown on the Choose screen.
    headphones: Vec<Arc<Headphones>>,
}

thread_local! {
    static APP: RefCell<Option<App>> = const { RefCell::new(None) };
}

pub fn run() -> Result<(), slint::PlatformError> {
    // Keeps the process's COM apartment alive for the Bluetooth lookups on
    // the worker threads (as Knurl does; issue DooperDrew/knurl#78). Without
    // it every lookup still works, so a failure is not fatal.
    let _ = knurl_device::keep_com_apartment();

    let ui = MainWindow::new()?;
    ui.on_begin(|| dispatch(Event::Start));
    ui.on_search_again(|| dispatch(Event::SearchAgain));
    ui.on_choose(|i| {
        if let Ok(index) = usize::try_from(i) {
            dispatch(Event::Choose(index));
        }
    });
    ui.on_check_again(|| dispatch(Event::CheckAgain));
    ui.on_to_send(|| dispatch(Event::ToSend));
    ui.on_toggle_data(|| dispatch(Event::ToggleData));
    ui.on_back(|| dispatch(Event::Back));
    ui.on_save(|| {
        let result = save_report();
        dispatch(Event::Saved(result));
    });
    ui.on_show_in_folder(show_in_folder);
    ui.on_open_bluetooth_settings(|| {
        let _ = shell_open(&HSTRING::from("ms-settings:bluetooth"));
    });
    ui.on_open_issue_form(|| {
        let _ = shell_open(&HSTRING::from(ISSUE_FORM));
    });

    let flow = Flow::default();
    view::apply(&ui, &flow);
    APP.with(|app| {
        *app.borrow_mut() = Some(App {
            ui: ui.clone_strong(),
            flow,
            headphones: Vec::new(),
        });
    });
    ui.run()
}

/// Applies an event on the window's thread, redraws, and starts the work it
/// asks for.
fn dispatch(event: Event) {
    let effect = APP.with(|app| {
        let mut app = app.borrow_mut();
        let app = app.as_mut()?;
        let effect = app.flow.handle(event);
        view::apply(&app.ui, &app.flow);
        effect
    });
    match effect {
        Some(Effect::Search) => spawn_search(),
        Some(Effect::Check(index)) => spawn_check(index),
        None => {}
    }
}

fn spawn_search() {
    std::thread::spawn(|| {
        let found = check::find_headphones();
        // If the window has closed there is nobody to tell.
        let _ = slint::invoke_from_event_loop(move || {
            let names = match &found {
                Ok(list) => Ok(list.iter().map(|h| h.name.clone()).collect()),
                Err(reason) => Err(reason.clone()),
            };
            APP.with(|app| {
                if let Some(app) = app.borrow_mut().as_mut() {
                    app.headphones = found
                        .unwrap_or_default()
                        .into_iter()
                        .map(Arc::new)
                        .collect();
                }
            });
            dispatch(Event::DevicesFound(names));
        });
    });
}

fn spawn_check(index: usize) {
    let target = APP.with(|app| app.borrow().as_ref()?.headphones.get(index).cloned());
    let Some(target) = target else {
        return;
    };
    std::thread::spawn(move || {
        let report = check::run(&target, |step| {
            let _ = slint::invoke_from_event_loop(move || dispatch(Event::Progress(step)));
        });
        let _ = slint::invoke_from_event_loop(move || dispatch(Event::Checked(Box::new(report))));
    });
}

/// The user's Documents folder (wherever Windows keeps it, OneDrive too).
fn documents() -> Result<PathBuf, String> {
    // SAFETY: a known folder id and no token; on success the returned string
    // is ours and is freed with CoTaskMemFree below, after it is copied.
    let pwstr = unsafe { SHGetKnownFolderPath(&FOLDERID_Documents, KF_FLAG_DEFAULT, None) }
        .map_err(|e| format!("Windows did not give the Documents folder: {e}"))?;
    // SAFETY: SHGetKnownFolderPath returned a valid NUL-terminated string.
    let path = unsafe { pwstr.to_string() };
    // SAFETY: the string came from SHGetKnownFolderPath and is freed once.
    unsafe { CoTaskMemFree(Some(pwstr.0.cast())) };
    path.map(PathBuf::from)
        .map_err(|e| format!("the Documents folder name is not valid: {e}"))
}

/// Writes the report to Documents\Knurl Probe\ and returns the file's path.
fn save_report() -> Result<String, String> {
    let report = APP
        .with(|app| app.borrow().as_ref()?.flow.report.clone())
        .ok_or("there is no report to save")?;
    let folder = documents()?.join("Knurl Probe");
    std::fs::create_dir_all(&folder)
        .map_err(|e| format!("could not create {}: {e}", folder.display()))?;
    let path = folder.join(knurl_report::file_name(&report, check::now_secs()));
    std::fs::write(&path, report.to_json())
        .map_err(|e| format!("could not write {}: {e}", path.display()))?;
    Ok(path.display().to_string())
}

/// Opens Explorer with the saved file selected.
fn show_in_folder() {
    let saved = APP.with(|app| match app.borrow().as_ref()?.flow.saved.clone() {
        Some(Ok(path)) => Some(path),
        _ => None,
    });
    if let Some(path) = saved {
        let _ = std::process::Command::new("explorer.exe")
            .arg(format!("/select,{path}"))
            .spawn();
    }
}

/// Opens a URL or a `ms-settings:` page with the default handler.
fn shell_open(target: &HSTRING) -> Result<(), String> {
    // SAFETY: NUL-terminated strings that outlive the call; no owner window.
    let result = unsafe { ShellExecuteW(None, w!("open"), target, None, None, SW_SHOWNORMAL) };
    // Values above 32 mean success (ShellExecute documentation).
    if result.0 as usize > 32 {
        Ok(())
    } else {
        Err(format!("ShellExecute returned {}", result.0 as usize))
    }
}
