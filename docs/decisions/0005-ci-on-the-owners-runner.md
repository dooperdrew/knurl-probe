---
title: "ADR-0005: CI on the owner's runner, never for forks or Dependabot"
title_it: "ADR-0005: CI sul runner del proprietario, mai per i fork e Dependabot"
summary: While the repository is private and the account has no hosted minutes, the Windows checks run on a self-hosted runner; forks and Dependabot always go to GitHub-hosted runners.
summary_it: Finché il repository è privato e l'account non ha minuti, i controlli Windows girano su un runner self-hosted; fork e Dependabot vanno sempre sui runner di GitHub.
lang: en
type: decision
status: current
order: 50
---

# ADR-0005: CI on the owner's runner, never for forks or Dependabot

**Date.** 2026-11-18 (the workflow change in #1; runner registered by the owner the same day).

**Context.** Private repositories use the account's GitHub-hosted minutes, which ran out, so no CI job could start. The owner already runs a self-hosted runner on their PC for Knurl. This repository is meant to be public, and on a public repository a fork's pull request must never run its code on the owner's PC.

**Decision.**
- **The Windows and `cargo deny` jobs** take their runner from the repository variable `BEDROCK_RUNNER`, else `windows-latest`. The build cache is kept next to the checkout.
- **Dependabot and pull requests from forks** always run on GitHub-hosted runners.
- **`BEDROCK_HOSTED=off`** skips, visibly, the jobs that need hosted minutes (the Linux job).
- **The release workflow** uses the same runner, and builds only a tag on `main` whose checks passed.

**Consequences.**
- One runner does one job at a time, so CI waits in a queue when several pull requests run.
- When the repository is public, hosted runners are free: `BEDROCK_HOSTED` can be removed so the Linux job runs again.
