---
name: bedrock-docs
description: Create, restructure or review a DooperDrew project's documentation - README (English and Italian), docs/ with front matter, the generated index and map, diagrams, and GitHub community files. Use when starting a project, when its docs are missing, scattered or out of date, when the docs index check fails, when writing a README, or when the user asks for documentation.
---

# Project documentation

Every project gets a solid base and the modules that fit its nature. The structure is fixed; the content stays the project's own. Write only what is true: anything not measured or tested goes under "not verified".

## 1. Pick the modules

Set them in `.bedrock.yml`; this also turns on the docs checks in CI:

```yaml
docs: [core, app, hardware]
```

| Module | For | Topics it expects |
|---|---|---|
| `core` | every project | `vision`, `architecture`, `verification`, decisions |
| `app` | desktop, mobile or web apps | `ux`, `user-guide` (plus the design reference, skill `bedrock-design`) |
| `hardware` | anything that talks to devices | `hardware-tests` (checklists the user runs), `protocol` |
| `product` | products with users and a market | `market`, `mvp` |
| `site` | websites and content managed by editors | `editorial-guide` (for non-technical editors), `content-model` |
| `kit` | libraries, kits, tools for developers | `reference`, `how-to` |
| `analysis` | data or financial analysis | `method`, `data-sources`, `limits` (disclaimers: no advice) |

The generated index lists, under "Da scrivere · To write", every expected topic that no doc covers yet. That list is the to-do list for the docs.

## 2. Every doc under docs/ starts with front matter

```markdown
---
title: Architettura
title_en: Architecture
summary: Come è fatto il programma e perché, in una pagina.
summary_en: How the program is built and why, on one page.
type: explanation
topic: architecture
status: current
verified: 2026-11-07
order: 20
---
```

- `type` follows Diátaxis:
  - `tutorial` (learn by doing);
  - `how-to` (a task, step by step);
  - `reference` (facts to look up);
  - `explanation` (why and how it works);
  - `decision` (one per choice, dated, as "context, decision, consequences");
  - `record` (logs, test results, the project diary).
- `lang` defaults to `it`; for an English doc set `lang: en` and use `title_it` and `summary_it`.
- `status`:
  - `draft` (not reviewed yet);
  - `current`;
  - `outdated` (known to be wrong: say what in the first line).
- `verified` is the date the doc was last checked against the code or the hardware. Leave it out rather than guess.
- File names: short, kebab-case. Numbering (`01-vision.md`) is fine for reading order; `order` controls the index.

Then run `node .github/bedrock/docs-index.mjs`. It writes `docs/README.md`: a Mermaid map, one table per type, the links to the design reference, docs without front matter, and the topics still to write. Text you add above or below the generated block is kept. CI fails when the index is stale (`--check`) or a relative link is broken (`--links`).

## 3. README in two languages

`README.md` is in English, because it is what GitHub and the community see first. `README.it.md` is in Italian. Both have the same sections in the same order, and each starts with a language switch: `English · [Italiano](README.it.md)`.

Sections:
1. **Title, one sentence on what it does, badges.** Use only real badges: the CI workflow badge (`https://github.com/<owner>/<repo>/actions/workflows/<file>/badge.svg`), the latest release and the license. Never self-awarded badges ("100% quality", "agent-ready").
2. **Why**: the problem, in 2-3 lines.
3. **Status**: what works today and what is not verified. Link the milestones and releases instead of copying a version number that goes stale.
4. **Quick start**: install or run in a few commands, copy-pasteable.
5. **How it works**: one Mermaid diagram plus a few lines.
6. **Documentation**: a link to `docs/README.md` and to the design reference.
7. **Contributing / working on it**: branches, PRs, checks. For agent-driven repos, point to CLAUDE.md / AGENTS.md.
8. **License** (or "private, all rights reserved").

The other docs stay in the language they were written in. When a doc matters to readers of the other language, add `summary_en` / `summary_it` instead of translating it all. CI checks that both READMEs exist, link each other and have the same number of `##` sections.

## 4. Visual and navigable

- **Diagrams**: Mermaid in Markdown, which GitHub renders: flowchart for flows, sequenceDiagram for protocols, stateDiagram for UI states, gantt for roadmaps. Keep each one to the point; label edges.
- **Images**: small screenshots in `docs/img/` only when needed (rule `repo-no-large-files`). Write alt text. Link large or interactive material (claude.ai canvases, artifacts) from the docs instead of committing it.
- **Tables** for inventories, **callouts** for warnings (`> [!WARNING]`, `> [!NOTE]`, which GitHub renders), relative links between docs, anchors for long pages.

## 5. GitHub community files and features

`bedrock sync` seeds these once, and after that the project owns and adapts them:
- `.github/pull_request_template.md`, with sections "verified / not verified" and the checklist;
- `.github/ISSUE_TEMPLATE/` with bug and feature forms.

Use the GitHub features as the single source of truth instead of copying their content into docs:
- **Milestones** for the roadmap: the roadmap doc links them.
- **Releases** for the changelog, with notes per version.
- **Labels**: `hardware-check` (waits for the user's device tests: never merge), `design`, `docs`, `lesson`, `bedrock-sync`.
- **Issues** for open problems, rather than a "problems" doc that nobody closes.

For public repos add `LICENSE`, `SECURITY.md` (private vulnerability reporting) and `CONTRIBUTING.md`.

## 6. Restructuring existing docs

Do it in a PR of its own (`docs/<topic>` branch):
1. Add front matter to each doc and set the `docs:` modules.
2. Generate the index.
3. Move content only when a doc mixes topics.
4. Keep git history with `git mv`.
5. Never delete knowledge: an outdated doc gets `status: outdated` and a first line saying what changed.
6. Report the topics still to write in the PR.

A lesson that other projects could use goes to bedrock with the `bedrock-lesson` skill.
