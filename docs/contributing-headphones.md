---
title: Adding a headphone model
title_it: Aggiungere un modello di cuffie
summary: How to check your Sony headphones with the probe and turn the report into a description of the model.
summary_it: Come controllare le tue cuffie Sony con il probe e trasformare il report nella descrizione del modello.
lang: en
type: how-to
topic: contributing
status: current
verified: 2026-11-18
order: 10
---

# Adding a headphone model

You need the headphones, a Windows 10 or 11 PC, and Rust until the first release ships a ready exe.

1. **Pair the headphones** with Windows as a classic Bluetooth device (the entry without `LE_`), and close Sony's own app if it runs on the PC.
2. **Run the probe** with the headphones on and connected:
   ```powershell
   cargo run --release -p knurl-probe -- run
   ```
   It asks the headphones only read-only questions (model, firmware, protocol, the features they declare, battery and settings) and writes a report to `reports/`. If several Sony headphones are paired, add `--device <part of the name>`.
3. **Read the report.** It holds no Bluetooth address. If you renamed the headphones in Windows, the name is replaced by a placeholder. Every frame exchanged is in it, so others can check the decoded answers.
4. **If the model is already described**, the probe compares the report with the description and prints the differences. No differences: your report is a confirmation. Differences (a new firmware, another feature code): that is exactly what is worth reporting.
5. **If the model is new**, draft a description:
   ```powershell
   cargo run -p knurl-probe -- draft reports\knurl-report-WH-1000XM6-1794960000.json --id sony-wh-1000xm6 --name "Sony WH-1000XM6"
   ```
   Review it: add a `note` where you know something the report cannot say.
6. **Send it**: open a "New headphone model" issue with the report attached, or a pull request with the description in `headphones/` and the report attached. Say which Windows version you used.

## What a report cannot tell
- **MDR v2 models** (per public sources, the newer ones): the probe asks only which protocol they speak and records what they send, because nothing decodes the v2 answers yet. A report from one is still useful: it is the test data the decoder will be built on.
- **Whether a setting works**: the probe never changes anything on the headphones. That a feature code is declared does not prove Knurl can drive it.
- **Firmware differences** beyond the version: two reports with different firmware are the only way to see them.
