---
title: "ADR-0002: Reports in JSON, model descriptions in TOML, aligned with Keyfold Probe"
title_it: "ADR-0002: Report in JSON, descrizioni dei modelli in TOML, come Keyfold Probe"
summary: A report is a JSON record written by the program; a description is a TOML file people edit, with a status that never claims more than the reports show.
summary_it: Il report è un JSON scritto dal programma; la descrizione è un TOML che le persone modificano, con uno stato che non dice mai più di quanto mostrano i report.
lang: en
type: decision
status: current
order: 20
---

# ADR-0002: Reports in JSON, model descriptions in TOML, aligned with Keyfold Probe

**Date.** 2026-11-18, decided with the owner (#3, #4).

**Context.** Keyfold Probe does the same job for keyboards. Using the same concepts in both probes lets one person, and one set of agents, maintain both. A report is written by a program and sent as it is; a description is written and reviewed by people.

**Decision.**
- **The report is JSON** (`format: 1`, with tool and date). A tool refuses a report whose format is newer than it knows.
- **The model description is TOML**, one file per model in `headphones/<id>.toml`, with codes written in hexadecimal.
- **A description has a status**:
  - `unverified`: written from documents or other tools;
  - `reported`: one report agrees with it;
  - `verified`: two or more reports from different people agree.
- **The same commands as Keyfold Probe**: `check` (a report against a description), `draft` (a first description from a report) and `validate`.

**Consequences.**
- The WH-1000XM3 description is `reported`: the 2026-11-07 capture agrees with it, checked by a test. The WH-1000XM4 one is `unverified`: public sources only.
- Keyfold Probe planned to move its reports to JSON too. That is not done here and not verified.
- Formats: [docs/format.md](../format.md).
