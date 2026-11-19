---
title: "ADR-0001: A public repository that holds the probe and the protocol"
title_it: "ADR-0001: Un repository pubblico con il probe e il protocollo"
summary: knurl-probe is public, MIT OR Apache-2.0, and holds the protocol crates the private Knurl app uses.
summary_it: knurl-probe è pubblico, MIT OR Apache-2.0, e contiene i crate del protocollo usati dall'app Knurl, privata.
lang: en
type: decision
status: current
order: 10
---

# ADR-0001: A public repository that holds the probe and the protocol

**Date.** 2026-11-18, decided with the owner (plan: DooperDrew/knurl#76; setup in #1).

**Context.** Knurl supports two models. Adding more needs people who own other Sony headphones to run a probe and send what it records. The Knurl app stays in a private repository, for the owner's own development. The probe and the protocol code lived there as `xmc-probe`, `xmc-protocol` and `xmc-device`.

**Decision.**
- knurl-probe is its own repository, meant to be public, licensed MIT OR Apache-2.0.
- The three crates moved here with their history (filtered, scanned for personal data before the import) and were renamed `knurl-probe`, `knurl-protocol` and `knurl-device`.
- The Knurl app will depend on `knurl-protocol` and `knurl-device` from here once this repository is public.

**Consequences.**
- A change that breaks those crates' API is flagged in its PR and needs the matching PR in DooperDrew/knurl (rule in CLAUDE.md).
- Privacy rules for a public repository: no Bluetooth addresses, PC names, user names or personal paths in files, tests, commits, issues or PRs; community reports are cleaned before they become fixtures.
- Until Knurl switches to these crates, the two copies must be kept in step by hand (not verified as a process yet).
