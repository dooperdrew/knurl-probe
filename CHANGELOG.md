# Changelog

What changed in each release of knurl-probe. The release workflow puts the section of the tagged version on the GitHub release.

## 0.1.0

Not released yet.

Fixed after the audits of 2026-11-21 (not yet run on headphones):
- **Finding the headphones:** `run` and the app also take the paired classic devices with a Sony model name and open their services with Winsock. WinRT lists no MDR service on the WH-1000XM3, so they found no headphones there. When such a device answers on MDR v1, MDR v2 is reported as `not_tried`.
- **Windows version** read from the registry: `cmd /C ver` flashed a console from the app and was not read on Windows in Italian or Spanish.
- **The app never hangs or vanishes:** a panic while searching, checking or sending becomes an error on that screen; a window that cannot open, or a panic, is said in a message box. Opening a page or the folder says when it fails.
- **Choose and Result scroll**, with the buttons always in view; with nothing read, "Check again" is the primary button.
- **Reports:** bytes on MDR v2 that are not a frame no longer count as an answer ("not decoded" became "nothing read").
- **Protocol:** a request waiting for a retry keeps its own retry count when an alert answer goes first.
- **Crash log:** the app writes every panic, on any thread, to `%LOCALAPPDATA%\Knurl Probe\crash.log` (kept under 64 KiB, never sent), because release builds have no console to print it to. Opening the folder starts Explorer with `CREATE_NO_WINDOW`. Both are bedrock rules from kit 0.28.1 (`rust-gui-panic-hook`, `rust-gui-no-console-window`). Not run on Windows yet.
- **Send as a `probe` report** (bedrock-relay ADR-0003): the report travels as a JSON object behind the envelope the probes share, so the issue is labelled `probe` and `probe:knurl-probe`, and the limit is 45000 characters instead of 40000. Reports have `checked_at`, the time of the check, for the envelope. Not verified with a real send: it needs the relay's deploy of `probe` reports.

- **The zip** `knurl-probe-0.1.0-windows-x64.zip`: `knurl-probe.exe` (the app, with the Knurl icon and its version information) and `knurl-probe-cli.exe` (the command line), both READMEs, both licences and the third-party notices for both exes, plus its SHA-256. `scripts/package.py` checks it (exact file list, each exe byte for byte, the app's version information, notices up to date, checksum); CI runs the same check on every pull request, and a release is built only from a commit on `main` whose checks passed.
- The probe, the Sony MDR protocol and the Windows connection move here from the Knurl app's repository, with their history (`xmc-probe`, `xmc-protocol`, `xmc-device` there). Licensed MIT OR Apache-2.0.
- `run` (now `knurl-probe-cli run`): checks one pair of headphones read-only and writes a report (JSON) to attach to an issue, with no Bluetooth address and no personal names (crate `knurl-report`, tested by replaying the WH-1000XM3 capture of 2026-11-07). When the model is described, it prints how the report compares.
- Model descriptions in `headphones/` (TOML), with a status that never claims more than the reports show: WH-1000XM3 `reported` (the 2026-11-07 capture agrees with it, checked by a test), WH-1000XM4 `unverified`.
- `known`, `check`, `draft`, `validate`: list, compare, draft and check descriptions. They work on files only and run on any system.
- **Knurl Probe, the app** (`knurl-probe.exe`, crate `knurl-probe-app`, Slint, the Knurl look, light and dark): welcome, choose your headphones, checking, the result in plain words, and the report with "Save to a file" and "See what will be sent". "Send" posts the report to the bedrock relay, which opens an issue here (no GitHub account needed); "See what will be sent" shows the exact request. Not run on Windows or on headphones yet; a real send needs the relay's deploy of `probe` reports. In English, Italian and Spanish (Windows' language, or a choice on the welcome screen).
- Commands: `devices` (paired headphones with a Sony MDR service), `rfcomm-check` (open each service and close it, no data sent), `capture` (record the traffic, also as a guided session), `watch` (run the same connection thread as the app), `audio` (Windows audio endpoints).
