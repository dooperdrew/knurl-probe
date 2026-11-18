# Changelog

What changed in each release of knurl-probe. The release workflow puts the section of the tagged version on the GitHub release.

## 0.1.0

Not released yet.

- The probe, the Sony MDR protocol and the Windows connection move here from the Knurl app's repository, with their history (`xmc-probe`, `xmc-protocol`, `xmc-device` there). Licensed MIT OR Apache-2.0.
- `report`: checks one pair of headphones read-only and writes a report to attach to an issue, with no Bluetooth address and no personal names (crate `knurl-report`, tested by replaying the WH-1000XM3 capture of 2026-11-07).
- Commands: `devices` (paired headphones with a Sony MDR service), `rfcomm-check` (open each service and close it, no data sent), `capture` (record the traffic, also as a guided session), `watch` (run the same connection thread as the app), `audio` (Windows audio endpoints).
