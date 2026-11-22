---
title: "ADR-0004: Send through the bedrock relay as a data report"
title_it: "ADR-0004: Invio tramite il relay di bedrock come report di tipo data"
summary: Send posts the report to the bedrock relay, which opens an issue here; no GitHub account, the exact request shown first.
summary_it: Invia manda il report al relay di bedrock, che apre una issue qui; nessun account GitHub, e la richiesta esatta è mostrata prima.
lang: en
type: decision
status: current
order: 40
---

# ADR-0004: Send through the bedrock relay as a data report

**Date.** 2026-11-18 (decided with the owner, #2) and 2026-11-19 (built, #9; relay side: DooperDrew/bedrock-relay#7).

**Update, 2026-11-22.** The report now goes as a `probe` report, not a `data` attachment: [decision 0007](0007-send-as-a-probe-report.md). The rest of this decision (no GitHub account, the exact request shown first, every answer read as sent or not sent) stays.

**Context.** Asking people to open a GitHub issue and attach a file loses most reports. Knurl already sends problem reports through the bedrock relay, a Cloudflare Worker that opens issues with a GitHub App, so no token ships in the app. But the relay's existing report kinds treat two reports with the same message as one: a "seen again" comment would replace the second report and lose its data. The fingerprint replaces every number, so even "WH-1000XM5" and "WH-1000XM6" would match.

**Decision.**
- **A new kind in the relay, `data`**, with a required attachment: it always opens a new issue, labelled `ap:new`, `src:user`, `data`.
- **The app sends** the report as attachment `knurl-report.json`, with the public key of this project and the endpoint from `crates/knurl-probe-app/relay.json`. The name is short because the relay hides long letter-and-digit runs.
- **The exact request is shown first** ("See what will be sent"). Send works once per report; Try again after a failure.
- **Every answer of the relay** reads as Sent (with the issue link) or Not sent (with the reason). Save to a file always works, also offline.

**Consequences.**
- The relay scrubs the text again, now also profile paths with escaped backslashes, as they appear inside JSON.
- A report over 40000 characters is not sent; the app offers Save to a file instead.
- A real send needs the relay's deploy of `data` reports. Until then the relay answers that the kind is unknown, and the app says the server does not accept reports yet. Not verified with a real send yet.
