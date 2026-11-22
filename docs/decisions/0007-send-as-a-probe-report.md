---
title: "ADR-0007: Send as a probe report"
title_it: "ADR-0007: Invio come report di tipo probe"
summary: Send posts the report as the relay's probe kind, a JSON object behind the envelope the probes share, instead of a data attachment.
summary_it: Invia manda il report come tipo probe del relay, un oggetto JSON dietro l'intestazione comune alle sonde, invece che come allegato data.
lang: en
type: decision
status: current
order: 70
---

# ADR-0007: Send as a probe report

**Date.** 2026-11-22. Relay side: bedrock-relay ADR-0003 (dooperdrew/bedrock#132).

**Context.** [Decision 0004](0004-send-through-the-relay.md) sends the report as a `data` report: the JSON travels as a string attachment of at most 40000 characters, and the relay knows neither which probe sent it nor that it is a report. The relay now has a `probe` kind for Keyfold Probe and Knurl Probe: the report as a JSON object, with a checked envelope (`probe`, `schema`, `version`, `date` as an ISO 8601 date-time with a zone, `steps[].outcome`), up to 45000 characters, and the issue labelled `probe` and `probe:<name>`.

**Decision.**
- **Send posts `kind: "probe"`.** The `report` object is the envelope followed by the report exactly as saved to a file, under `headphones` (`send.rs`, `probe_report`). `probe` is `knurl-probe`, `schema` the report's `format`, `version` the app's version.
- **`date` is a new report field, `checked_at`**: the time the check ended, to the second, in UTC. The report keeps `date`, the day, as before; a report without `checked_at` still reads, but cannot be sent (the app sends only reports it has just made).
- **`steps`** are what this probe asks: the two services (`opened`, `not-offered`, `not-tried`, `failed`), then each start-up question on MDR v1 (`arrived`, `silent`, `not-reached`).
- **Limits checked before sending**, as the relay counts them: the report object at most 45000 UTF-16 units as JSON with 2-space indentation, the request at most 64 KiB. Over them, Send is off and Save to a file stays.
- The report's `format` stays 1: `checked_at` is a new field, and no field changes meaning.

**Consequences.**
- "See what will be sent" shows the report indented and readable instead of an escaped string.
- The relay scrubs the report after its own length check, and the scrub can lengthen it (an email becomes `<email>`): a report just under the limit can still be refused with a 400 that names the limit. The report holds no emails or profile paths, so this is not expected.
- A real send needs the relay's deploy of `probe` reports; until then the relay answers that the kind is unknown and the app says the server does not accept reports yet. Not verified with a real send.
- The envelope in Keyfold Probe's `docs/probe-concept.md` gives `date` as `YYYY-MM-DD`; the relay wants a date-time. Here the envelope follows the relay.
