---
name: bedrock-design
description: Create, update or review the design reference of a DooperDrew project (brand book, tokens, components, screens). Use before UI work (read it first), when producing UX/UI material, when a change alters colors, type, components or screens, when asked to review the design of any project, or when a project with a UI has no design reference yet.
---

# Design reference

Every project with a UI keeps one design reference: the brand book that UI code, front-end checks and design sessions all follow. Its path is the `design:` key of `.bedrock.yml` (for a new project use `design/DESIGN.md`) and is repeated in the bedrock block of CLAUDE.md / AGENTS.md.

## Anyone may review

Any session, in any project, may read and review the design reference of any DooperDrew project, and is encouraged to:
- find them with `gh api repos/DooperDrew/<repo>/contents/.bedrock.yml --jq .content | base64 -d` (key `design:`), or in bedrock's `docs/design-index.md`;
- propose changes with a PR in that project's repo. Never edit another repo's files from the current one.
- Published canvases and pages (claude.ai artifacts) linked from the reference may be opened, screenshotted and re-checked at any time.

## Structure of the reference

Keep it in this order, so every project reads the same way:
1. **Identity and voice**: name, one-line promise, audience, values, tone (with do/don't examples), disclaimers.
2. **Tokens**: colors with their measured contrast ratios (WCAG 2.x: text ≥ 4.5:1, UI borders and icons ≥ 3:1), type scale in rem, spacing, radius, elevation. If code consumes them, keep them machine-readable in `design/tokens.json` (or the project's theme file) and name that file here.
3. **Components**: each one with its states (default, hover, focus, disabled, error, loading) and where it lives in the code.
4. **Screens**: a list with a link to each canvas, artifact or page, and the widths checked (at least 375 px and 1280 px for web).
5. **Decisions**: dated log, as "what, why".
6. **Verified / not verified**: what was measured or tested, and what is still an assumption.

## Working rules

- **Same PR**: a change to UI colors, type, components or screens updates the reference in the same PR.
- **Parallel design session**: design work runs in its own session, on short `design/<topic>` branches merged often (at least once a day it merges main back in). Do not keep a permanent design branch: it drifts away from the code. Implementation sessions read the reference and never change it silently; if the code needs a design change, they say so in the PR.
- **Checks**: when tokens are machine-readable, add a front-end test that the code uses only tokens (e.g. no raw hex colors, no px font sizes). dallaterraallaluna's `tests/design.mjs` is an example.
- **Images**: no committed file over 5 MiB (rule `repo-no-large-files`). Prefer published artifacts or CI artifacts for screenshots; commit only small images the reference needs.
- **No invented content**: names, quotes and figures that are not known stay as [placeholders].
- After publishing or changing a canvas or page, open it, look for defects (cut text, overlaps, contrast, alignment, horizontal scroll) and fix them in further passes.
- **Side-by-side proposals**: when several alternative proposals are open at once for comparison, each PR keeps every change inside its own folder (for example `design/<topic>/`, as a second `design-flow` flow does). Shared files (lint and format config, the design reference, app or theme code) change only in the later PR that adopts the chosen one. If a tool must skip proposal folders, add one glob exclusion on main first, in its own PR.
- **Third-party files in the repo** (for example a design kit's assets): commit only the files the page loads, in one folder per source with the source's licence file beside them (`LICENSE` or `LICENSE-<source>.txt`), list the folder under `vendored:` in `.bedrock.yml` (CI checks the licence is there) and exclude it from linters by path, never by turning a rule off for the whole repo.
- **Checks that read only English**: when a design skill's check is language-specific (for example the heading check of `design-taste-frontend`) and the page is Italian, keep the copy in the project's language, record a waiver with the reason in the skill's waiver file and list it under "Not verified". Never reword or translate copy only to make a check pass.

## Design skills on this PC

To create or redo a project's look step by step (brief, three directions, review rounds, lock, build), follow the `design-flow` skill (`/design-flow`): it says which of these skills each phase uses.


bedrock installs these skills once per PC in `~/.claude/skills` (`npm run install-skills` in bedrock; list and pinned versions in its `user-skills.yml`). Use the one that fits the task; the design reference above always wins over their defaults (colors, type, voice):
- `frontend-design`: aesthetic direction for new or reshaped UI, avoiding templated defaults. A light first pass.
- `impeccable`: design commands (critique, audit, polish, layout, colorize, harden) and a detector of common UI defects. Its engine downloads once into `~/.impeccable/bin`: ask the user before that first download.
- `ui-ux-pro-max`: searchable data (styles, palettes, font pairings, UX guidelines, charts, per-stack advice) through local Python scripts.
- `design-taste-frontend` (taste-skill): full marketing and company websites with its own scripts, assets and checks. Heavy: use it for a whole site, not a small UI change.
- `brandkit`: brand-kit boards, logo systems and identity decks as images.

If a skill is missing (another PC, a cloud session), work without it and say so; do not install it from inside a project.

A lesson that another project could hit (a tool limit, an accessibility trap) goes to bedrock with the `bedrock-lesson` skill.
