//! Knurl Probe, the Windows app. The screens and their flow are in the
//! library (`flow`, `view`, `ui/`); here: finding the headphones and the
//! check on background threads (never on the window's thread), saving the
//! report, sending it to the relay (WinHTTP, on a worker thread), and opening
//! Windows pages.

// No console window behind the app in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(windows)]
mod windows_app;
#[cfg(windows)]
mod winhttp;

/// Release builds have no console: a window that cannot open, or a panic on
/// the window's thread, is said in a message box instead of a silent exit.
/// Every panic, on any thread, is first written to the crash log.
#[cfg(windows)]
fn main() -> std::process::ExitCode {
    install_panic_hook();
    let failure = match std::panic::catch_unwind(windows_app::run) {
        Ok(Ok(())) => return std::process::ExitCode::SUCCESS,
        Ok(Err(e)) => format!("the window could not open: {e}"),
        Err(panic) => format!("internal error: {}", windows_app::panic_text(&*panic)),
    };
    windows_app::fatal(&failure);
    std::process::ExitCode::FAILURE
}

/// Writes every panic to `%LOCALAPPDATA%\Knurl Probe\crash.log`
/// ([`knurl_probe_app::crash`]) before it unwinds; the default hook prints to
/// a console that release builds do not have. The screens still say it: a
/// worker's panic becomes an error on its screen, the window's a message box.
#[cfg(windows)]
fn install_panic_hook() {
    let default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        use knurl_probe_app::crash;
        let thread = std::thread::current();
        let line = crash::line(
            knurl_probe::check::now_secs(),
            env!("CARGO_PKG_VERSION"),
            thread.name().unwrap_or("unnamed"),
            &info.to_string(),
        );
        if let Some(path) = crash::log_path(std::env::var_os("LOCALAPPDATA")) {
            // Nothing else can be done about a log that cannot be written
            // while panicking; the screens still report the failure.
            let _ = crash::append(&path, &line, crash::MAX_LOG_BYTES);
        }
        // Debug builds have a console: keep the usual message there.
        default(info);
    }));
}

#[cfg(not(windows))]
fn main() {
    eprintln!("Knurl Probe runs on Windows only.");
    std::process::exit(1);
}
