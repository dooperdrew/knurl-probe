---
title: Hardware tests
title_it: Prove sull'hardware
summary: What was tested on real headphones, and the checklist still to run, step by step.
summary_it: Cosa è stato provato su cuffie vere e la lista di prove ancora da fare, passo per passo.
lang: en
type: how-to
topic: hardware-tests
status: current
verified: 2026-11-19
order: 70
---

# Hardware tests

The probe talks to real headphones, so every claim about them needs a run on real headphones. This page lists what was run, then what is still to run, in the order that gives the most for the least time. Mark a step done in the PR that records its result, and note the firmware.

## Already tested
- **WH-1000XM3, firmware 4.5.2, Windows 10, 2026-11-07**, with the probe under its old name `xmc-probe` (guided capture). Results in [docs/04](04-risultati-xm3.md) (in Italian).
  - The MDR v1 service opens only through Winsock; v2 is absent.
  - Framing, checksum, escaping and acknowledgements match the captured frames.
  - The read-only start-up answers decode: protocol, model, firmware, 19 features, battery, noise mode, equalizer.
- The frames of that capture (without the address) are the test data in `crates/knurl-protocol/tests/` and `crates/knurl-report/tests/replay_xm3.rs`. Replayed, they give a report that agrees with `headphones/sony-wh-1000xm3.toml`.

Nothing else has run on headphones: not the app, not `knurl-probe-cli run`, not any model other than the XM3.

## Before you start
- A Windows 10 or 11 PC with Bluetooth; the headphones paired as a classic device (the entry without `LE_`).
- The latest release zip (or a build from source: `cargo build --release -p knurl-probe-app -p knurl-probe`).
- Close Sony's own app on the PC, if it runs.

## Checklist
### 1. The app opens and looks right (no headphones needed)
1. Run `knurl-probe.exe`.
2. Check the title "Knurl Probe", the welcome texts, the Slint badge at the bottom.
3. Switch Windows to dark mode: the window follows.
4. Change the language on the welcome screen to Italiano and Español: every screen is translated, nothing is cut off.
5. Try 125 % and 150 % display scaling: nothing is cut off.
6. Move through the screens with Tab and Enter only.

### 2. A check on the WH-1000XM3 from the app
1. Headphones on and connected. Open the app, Start.
2. The XM3 is listed by model. Pick it.
3. The checking steps advance and end within about 15 s.
4. The result says Knurl already supports the WH-1000XM3. The facts show firmware 4.5.2 (or the current one) and 19 features.
5. Save to a file: the JSON is in `Documents\Knurl Probe`, and it holds no Bluetooth address and no name you gave the headphones.
6. Note anything that differs from the capture of 2026-11-07.

### 3. The same check from the command line
1. `knurl-probe-cli.exe run`.
2. The summary matches the app's result, and the report is written to `reports\`.
3. `knurl-probe-cli.exe check reports\<file>.json`: no differences from the XM3 description. If the firmware differs, add it to `headphones/sony-wh-1000xm3.toml` in a PR, with the report.

### 4. The headphones' name
1. Rename the XM3 in Windows Bluetooth settings (for example to a first name), then run the app again.
2. The report and the message must not contain the new name; they say "(renamed by the user: not included)".
3. Rename it back.

### 5. Send (after the relay's deploy of `probe` reports)
1. On the Send screen, open "See what will be sent": it is the exact request, the report inside it under `headphones`.
2. Press Send. Expect "Sent" with a link to a new issue in DooperDrew/knurl-probe titled `probe: knurl-probe <version> · Headphone report: <model>`, labelled `ap:new`, `src:user`, `probe`, `probe:knurl-probe`, with the envelope and the report in a `json` block.
3. Before the deploy the app must say that the server does not accept reports yet. It must never say "Sent".
4. Close the test issue.

### 6. WH-1000XM4
1. Repeat steps 2 to 4 with an XM4.
2. The result should say Knurl supports it. `headphones/sony-wh-1000xm4.toml` is `unverified`; a matching report makes it `reported` (raise `reports` and fill in firmware, protocol version and features from the report).

### 7. A newer model (MDR v2), when one is available
1. Run the app with, for example, a WH-1000XM5.
2. The result should say the headphones answered on the newer service, which Knurl cannot read yet.
3. Send or attach the report: its frames are the test data for a future decoder.

## Not verified
Everything in the checklist above, as of the `verified` date of this page.
