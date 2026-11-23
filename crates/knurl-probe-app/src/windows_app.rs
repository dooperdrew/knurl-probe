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
use knurl_probe_app::{ISSUE_FORM, i18n, send, t, view};
use slint::ComponentHandle;
use std::os::windows::process::CommandExt as _;
use windows::Win32::Globalization::GetUserDefaultLocaleName;
use windows::Win32::System::Com::CoTaskMemFree;
use windows::Win32::UI::Shell::{
    FOLDERID_Documents, KF_FLAG_DEFAULT, SHGetKnownFolderPath, ShellExecuteW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    MB_ICONERROR, MB_ICONWARNING, MB_OK, MESSAGEBOX_STYLE, MessageBoxW, SW_SHOWNORMAL,
};
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

    // Rust's texts before the window exists; the window's own texts once it
    // does (Slint knows its bundled translations only then).
    i18n::set(i18n::for_choice(0, &locale()));
    let ui = MainWindow::new()?;
    select_window_language();
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
    ui.on_choose_language(choose_language);
    ui.on_send(|| dispatch(Event::Send));
    ui.on_open_issue(|| {
        let issue = APP.with(|app| match app.borrow().as_ref()?.flow.sending {
            knurl_probe_app::flow::Sending::Sent(n) => Some(n),
            _ => None,
        });
        if let Some(n) = issue {
            open_or_say(&send::issue_url(n));
        }
    });
    ui.on_save(|| {
        let result = save_report();
        dispatch(Event::Saved(result));
    });
    ui.on_show_in_folder(show_in_folder);
    ui.on_open_bluetooth_settings(|| open_or_say("ms-settings:bluetooth"));
    ui.on_open_issue_form(|| open_or_say(ISSUE_FORM));

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

/// Windows' display language, e.g. `it-IT` (empty if Windows does not say).
fn locale() -> String {
    let mut name = [0u16; 85];
    // SAFETY: the buffer is valid for its whole length, which is passed.
    let len = unsafe { GetUserDefaultLocaleName(&mut name) };
    usize::try_from(len)
        .ok()
        .filter(|&n| n > 0)
        .map(|n| String::from_utf16_lossy(&name[..n - 1]))
        .unwrap_or_default()
}

/// Switches the windows' own texts to the current language. A failure leaves
/// them in English; Rust's texts follow anyway.
fn select_window_language() {
    let _ = slint::select_bundled_translation(i18n::slint_code(i18n::current()));
}

/// A language picked on the welcome screen: Rust's texts and the window's.
fn choose_language(choice: i32) {
    i18n::set(i18n::for_choice(choice, &locale()));
    select_window_language();
    APP.with(|app| {
        if let Some(app) = app.borrow_mut().as_mut() {
            app.ui.set_language_choice(choice);
            view::apply(&app.ui, &app.flow);
        }
    });
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
        Some(Effect::Send) => spawn_send(),
        None => {}
    }
}

/// The text of a caught panic.
pub fn panic_text(panic: &(dyn std::any::Any + Send)) -> String {
    panic
        .downcast_ref::<&str>()
        .map(ToString::to_string)
        .or_else(|| panic.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "unknown panic".into())
}

/// Runs a worker's job; a panic becomes an error text, so the screen that
/// waits for the answer ("Searching...", "Checking...", "Sending...") gets
/// one instead of waiting forever.
fn guarded<T>(job: impl FnOnce() -> T) -> Result<T, String> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(job))
        .map_err(|panic| t!("internal error: {e}", e = panic_text(&*panic)))
}

fn message_box(text: &str, style: MESSAGEBOX_STYLE) {
    let title = HSTRING::from("Knurl Probe");
    let text = HSTRING::from(text);
    // SAFETY: NUL-terminated strings that outlive the call; no owner window.
    unsafe { MessageBoxW(None, &text, &title, style | MB_OK) };
}

/// The app cannot go on: said in a box (there is no console in release).
pub fn fatal(text: &str) {
    message_box(&t!("Knurl Probe stopped: {e}", e = text), MB_ICONERROR);
}

/// Opens a page; a failure is said, not ignored.
fn open_or_say(target: &str) {
    if let Err(e) = shell_open(&HSTRING::from(target)) {
        message_box(
            &t!("Could not open {target}: {e}", target = target, e = e),
            MB_ICONWARNING,
        );
    }
}

fn spawn_search() {
    std::thread::spawn(|| {
        let found = guarded(check::find_headphones).and_then(|r| r);
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
        let name = target.name.clone();
        let report = guarded(|| {
            check::run(&target, |step| {
                let _ = slint::invoke_from_event_loop(move || dispatch(Event::Progress(step)));
            })
        })
        .unwrap_or_else(|problem| failed_check(&name, problem));
        let _ = slint::invoke_from_event_loop(move || dispatch(Event::Checked(Box::new(report))));
    });
}

/// Sends the report on a worker thread; the answer comes back as an event.
fn spawn_send() {
    let report = APP.with(|app| app.borrow().as_ref()?.flow.report.clone());
    let Some(report) = report else {
        dispatch(Event::SendDone(
            Err(t!("there is no report to send").into()),
        ));
        return;
    };
    std::thread::spawn(move || {
        let outcome = guarded(|| match send::relay_config() {
            Ok(config) => send::send(
                &crate::winhttp::WinHttp,
                &config,
                &report,
                env!("CARGO_PKG_VERSION"),
            ),
            Err(reason) => send::Outcome::NotSent(reason),
        })
        .unwrap_or_else(send::Outcome::NotSent);
        let result = match outcome {
            send::Outcome::Sent { issue } => Ok(issue),
            send::Outcome::NotSent(reason) => Err(reason),
        };
        let _ = slint::invoke_from_event_loop(move || dispatch(Event::SendDone(result)));
    });
}

/// The report of a check that stopped on an internal error: nothing read,
/// and the error in its problems, so the result screen says it.
fn failed_check(name: &str, problem: String) -> knurl_report::Report {
    let now = check::now_secs();
    knurl_report::build(knurl_report::ReportInput {
        tool: knurl_probe::TOOL,
        date: &knurl_report::date_from_unix(now),
        checked_at: &knurl_report::time_from_unix(now),
        os: &check::os_version(),
        windows_name: name,
        services: knurl_report::Services {
            v1: knurl_report::ServiceResult::NotTried,
            v2: knurl_report::ServiceResult::NotTried,
        },
        outcomes: Vec::new(),
        problems: vec![problem],
    })
}

/// The user's Documents folder (wherever Windows keeps it, OneDrive too).
fn documents() -> Result<PathBuf, String> {
    // SAFETY: a known folder id and no token; on success the returned string
    // is ours and is freed with CoTaskMemFree below, after it is copied.
    let pwstr = unsafe { SHGetKnownFolderPath(&FOLDERID_Documents, KF_FLAG_DEFAULT, None) }
        .map_err(|e| t!("Windows did not give the Documents folder: {e}", e = e))?;
    // SAFETY: SHGetKnownFolderPath returned a valid NUL-terminated string.
    let path = unsafe { pwstr.to_string() };
    // SAFETY: the string came from SHGetKnownFolderPath and is freed once.
    unsafe { CoTaskMemFree(Some(pwstr.0.cast())) };
    path.map(PathBuf::from)
        .map_err(|e| t!("the Documents folder name is not valid: {e}", e = e))
}

/// Writes the report to Documents\Knurl Probe\ and returns the file's path.
fn save_report() -> Result<String, String> {
    let report = APP
        .with(|app| app.borrow().as_ref()?.flow.report.clone())
        .ok_or_else(|| t!("there is no report to save").to_string())?;
    let folder = documents()?.join("Knurl Probe");
    std::fs::create_dir_all(&folder).map_err(|e| {
        t!(
            "could not create {path}: {e}",
            path = folder.display(),
            e = e
        )
    })?;
    let path = folder.join(knurl_report::file_name(&report, check::now_secs()));
    std::fs::write(&path, report.to_json())
        .map_err(|e| t!("could not write {path}: {e}", path = path.display(), e = e))?;
    Ok(path.display().to_string())
}

/// `CREATE_NO_WINDOW` from the Windows process creation flags: a child never
/// gets a console window of its own.
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Opens Explorer with the saved file selected.
fn show_in_folder() {
    let saved = APP.with(|app| match app.borrow().as_ref()?.flow.saved.clone() {
        Some(Ok(path)) => Some(path),
        _ => None,
    });
    if let Some(path) = saved
        && let Err(e) = std::process::Command::new("explorer.exe")
            .arg(format!("/select,{path}"))
            // Explorer has a window of its own; never a console behind it.
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
    {
        message_box(&t!("Could not open the folder: {e}", e = e), MB_ICONWARNING);
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
