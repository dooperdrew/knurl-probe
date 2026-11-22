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
#[cfg(windows)]
fn main() -> std::process::ExitCode {
    let failure = match std::panic::catch_unwind(windows_app::run) {
        Ok(Ok(())) => return std::process::ExitCode::SUCCESS,
        Ok(Err(e)) => format!("the window could not open: {e}"),
        Err(panic) => format!("internal error: {}", windows_app::panic_text(&*panic)),
    };
    windows_app::fatal(&failure);
    std::process::ExitCode::FAILURE
}

#[cfg(not(windows))]
fn main() {
    eprintln!("Knurl Probe runs on Windows only.");
    std::process::exit(1);
}
