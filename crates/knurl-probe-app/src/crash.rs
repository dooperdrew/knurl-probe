//! The crash log. Release builds have no console (`windows_subsystem =
//! "windows"`), so the default panic message goes nowhere: the panic hook
//! appends it here, on any thread, before the process unwinds or dies. The
//! file stays on the PC (`%LOCALAPPDATA%\Knurl Probe\crash.log`); nothing
//! sends it. No Windows calls: tested on any system.

use std::ffi::OsString;
use std::io::Write as _;
use std::path::{Path, PathBuf};

/// The log keeps only its newest half once it grows past this size.
pub const MAX_LOG_BYTES: u64 = 64 * 1024;

/// `<LOCALAPPDATA>\Knurl Probe\crash.log`, or None without LOCALAPPDATA.
pub fn log_path(local_app_data: Option<OsString>) -> Option<PathBuf> {
    let base = local_app_data.filter(|b| !b.is_empty())?;
    Some(PathBuf::from(base).join("Knurl Probe").join("crash.log"))
}

/// One line of the log: the time (UTC), the app's version, the thread and
/// the panic message, on one line.
pub fn line(unix_secs: u64, version: &str, thread: &str, message: &str) -> String {
    format!(
        "{} knurl-probe {version} panic on thread {thread}: {message}",
        knurl_report::time_from_unix(unix_secs)
    )
    .replace(['\r', '\n'], " ")
}

/// Appends `line` to the log at `path`, creating its folder; a log over
/// `max_bytes` first keeps only its newest half, from a whole line.
///
/// # Errors
/// The folder or the file could not be written, in words.
pub fn append(path: &Path, line: &str, max_bytes: u64) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    if let Ok(meta) = std::fs::metadata(path)
        && meta.len() > max_bytes
    {
        let old = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
        std::fs::write(path, newest_half(&old, max_bytes))
            .map_err(|e| format!("{}: {e}", path.display()))?;
    }
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| format!("{}: {e}", path.display()))?;
    writeln!(file, "{line}").map_err(|e| format!("{}: {e}", path.display()))
}

/// The last `max_bytes / 2` bytes of `old`, from the first whole line.
fn newest_half(old: &[u8], max_bytes: u64) -> &[u8] {
    let half = usize::try_from(max_bytes / 2).unwrap_or(usize::MAX);
    let keep = &old[old.len().saturating_sub(half)..];
    let start = keep.iter().position(|&b| b == b'\n').map_or(0, |i| i + 1);
    &keep[start..]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("knurl-probe-crash-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn the_log_lives_under_local_app_data() {
        let path = log_path(Some(OsString::from("base"))).expect("a path");
        assert!(path.ends_with(Path::new("Knurl Probe").join("crash.log")));
        assert!(path.starts_with("base"));
        assert_eq!(log_path(None), None);
        assert_eq!(log_path(Some(OsString::new())), None);
    }

    #[test]
    fn a_line_has_the_time_version_thread_and_message_on_one_line() {
        let l = line(1_794_960_000, "0.1.0", "main", "boom\nat src/x.rs:1");
        assert_eq!(
            l,
            "2026-11-18T00:00:00Z knurl-probe 0.1.0 panic on thread main: boom at src/x.rs:1"
        );
    }

    #[test]
    fn lines_are_appended_and_the_folder_is_created() {
        let dir = scratch("append");
        let path = dir.join("Knurl Probe").join("crash.log");
        append(&path, "first", MAX_LOG_BYTES).expect("first");
        append(&path, "second", MAX_LOG_BYTES).expect("second");
        assert_eq!(
            std::fs::read_to_string(&path).expect("log"),
            "first\nsecond\n"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_big_log_keeps_its_newest_half_from_a_whole_line() {
        let dir = scratch("trim");
        let path = dir.join("crash.log");
        std::fs::create_dir_all(&dir).expect("dir");
        let old: String = (0..40).map(|i| format!("line {i:02}\n")).collect();
        std::fs::write(&path, &old).expect("old");
        append(&path, "new", 100).expect("append");
        let text = std::fs::read_to_string(&path).expect("log");
        assert!(text.len() <= 50 + "new\n".len(), "{text}");
        assert!(text.starts_with("line "), "starts at a whole line: {text}");
        assert!(text.ends_with("line 39\nnew\n"), "{text}");
    }

    #[test]
    fn a_log_that_cannot_be_written_says_so() {
        let dir = scratch("blocked");
        std::fs::create_dir_all(&dir).expect("dir");
        // A file where the folder should be.
        let blocker = dir.join("Knurl Probe");
        std::fs::write(&blocker, "x").expect("blocker");
        let err = append(&blocker.join("crash.log"), "x", MAX_LOG_BYTES).expect_err("blocked");
        assert!(err.contains("Knurl Probe"), "{err}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
