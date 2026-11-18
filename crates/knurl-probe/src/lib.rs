//! What the `knurl-probe` command line and the Knurl Probe app share: finding
//! the paired Sony headphones and running the read-only check that becomes a
//! report (crate `knurl-report`). The Windows parts are compiled on Windows
//! only; [`ServiceKind`] and [`CheckStep`] exist everywhere, so the app's flow
//! can be tested on any system.

#[cfg(windows)]
pub mod bluetooth;
#[cfg(windows)]
pub mod check;

/// One of the two Sony control services.
#[derive(Clone, Copy, PartialEq, Eq, Debug, clap::ValueEnum)]
pub enum ServiceKind {
    /// Sony MDR service, UUID 96cc203e-5068-46ad-b32d-e316f5e069ba (XM3/XM4 per public sources).
    V1,
    /// Sony MDR service, UUID 956c7b26-d49a-4ba8-b03f-b17d393cb6e2.
    V2,
}

/// Where a check is, in the order it goes through the steps. `Listening`
/// happens only when Windows offers the newer service (MDR v2).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CheckStep {
    /// Opening the control channel of the first service.
    Opening,
    /// Asking model, firmware, features and settings (MDR v1).
    Asking,
    /// Asking the protocol question on MDR v2 and listening.
    Listening,
    /// Putting the report together.
    Finishing,
}

/// "knurl-probe 0.1.0": the `tool` written in every report.
pub const TOOL: &str = concat!("knurl-probe ", env!("CARGO_PKG_VERSION"));
