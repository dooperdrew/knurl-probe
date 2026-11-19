---
title: "ADR-0006: The project uses the bedrock kit"
title_it: "ADR-0006: Il progetto usa il kit bedrock"
summary: Shared rules, checks and skills come from bedrock through sync PRs, as in every DooperDrew project.
summary_it: Regole, controlli e skill comuni arrivano da bedrock con PR di sync, come in ogni progetto DooperDrew.
lang: en
type: decision
status: current
order: 60
---

# ADR-0006: The project uses the bedrock kit

**Date.** 2026-11-18 (first install, kit 0.20.2, in #5; listed in bedrock's `consumers.yml` in DooperDrew/bedrock#133).

**Context.** The project is written mostly by AI agents. Before the kit, rules such as the commit author's identity were checked by nobody, and two merge commits with the wrong author reached `main` on 2026-11-18.

**Decision.**
- knurl-probe gets the kit like the other DooperDrew repositories: the `bedrock-*` workflows, the shared skills and the rules block in CLAUDE.md.
- Later versions arrive as "bedrock sync" pull requests.
- The project's own CI (`ci.yml`) stays, because it already runs fmt, clippy, tests, the zip check and `cargo deny`.

**Consequences.**
- Files listed as managed in `.bedrock.yml` change only in sync pull requests; a general problem becomes a lesson in bedrock.
- The docs modules `core` and `hardware` are on (#8). This index and the README check run in CI.
