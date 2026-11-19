---
title: Vision and scope
title_it: Visione e perimetro
summary: What knurl-probe is for, who uses it, and what stays out.
summary_it: A cosa serve knurl-probe, chi lo usa e cosa resta fuori.
lang: en
type: explanation
topic: vision
status: current
verified: 2026-11-19
order: 5
---

# Vision and scope

**knurl-probe lets someone with Sony headphones that Knurl does not know yet record how they talk, and send it, in a few minutes and without technical knowledge.** What it records becomes test data and, once checked, support for the model in [Knurl](https://github.com/DooperDrew/knurl), the Windows tray app for Sony headphones.

## Who it is for
- **People with the headphones.** They open Knurl Probe (`knurl-probe.exe`), follow five screens and press Send: no terminal, no GitHub account, no Bluetooth knowledge.
- **Developers.** They use the command line (`knurl-probe-cli.exe`) to capture traffic, compare a report with a model description, draft a new description, and they read the reports that arrive as issues.
- **The Knurl app.** It is built on the protocol code that lives here (`knurl-protocol`, `knurl-device`); see [architecture](03-architecture.md).

## What it does
1. Finds the Sony headphones paired with Windows and opens their control channel (the "MDR" service, over Bluetooth RFCOMM).
2. Asks only read-only questions: protocol version, model, firmware, the features the headphones declare, battery and current settings. Nothing on the headphones changes.
3. Writes a report with the answers and every frame exchanged, **without the Bluetooth address and without names a person gave the headphones**.
4. Says in plain words what it found ("Knurl already supports…", "new model…") and lets the person save the report or send it.

## Principles
- **Read-only by default.** The app and `knurl-probe-cli run` never send a command that changes a setting. Only `capture` sends what a developer types at its prompt.
- **Private by design.** The address is used to connect and written nowhere; a Windows name is kept only if it is a Sony model name. Raw captures stay in `captures/raw/`, which git ignores.
- **Never claim more than was measured.** A model description is `unverified` until a report agrees with it ([decision 0002](decisions/0002-reports-and-descriptions.md)); every claim in the docs is backed by a test, a capture or written as "not verified".
- **A failure is a failure.** A failed send, check or lookup is shown as such, never as "ok".

## Out of scope
- **Controlling the headphones.** Changing noise cancelling, the equalizer and so on is Knurl's job, not the probe's.
- **Other operating systems.** The device side is Windows-only. The pure crates (`knurl-protocol`, `knurl-report`) build and test anywhere, but no macOS or Linux connection is planned.
- **Decoding the newer protocol (MDR v2).** The probe opens it and records what the headphones send, but nothing decodes the answers yet.
- **Choosing which models Knurl supports.** The probe works with any Sony headphones that expose an MDR service; the list Knurl supports (`is_supported_model`) stays the app's choice.

## Not verified
- The app and `knurl-probe-cli run` on real headphones: see [hardware tests](05-hardware-tests.md).
- A real send through the relay.
