//! Knurl Probe, the Windows app. The screens and their flow are in the
//! library (`flow`, `view`, `ui/`); here: finding the headphones and the
//! check on background threads (never on the window's thread), saving the
//! report, and opening Windows pages. Sending is not in this version.

// No console window behind the app in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(windows)]
mod windows_app;

#[cfg(windows)]
fn main() -> Result<(), slint::PlatformError> {
    windows_app::run()
}

#[cfg(not(windows))]
fn main() {
    eprintln!("Knurl Probe runs on Windows only.");
    std::process::exit(1);
}
