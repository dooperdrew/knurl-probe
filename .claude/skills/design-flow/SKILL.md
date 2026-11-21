---
name: design-flow
description: Step-by-step flow for the look and front end of a DooperDrew project - brief, three visual directions, review rounds, lock the design reference, then build one screen per PR. Use when the user types /design-flow (optionally with a phase - brief, direzioni, iterazione, fissa, costruisci - or "stato"), asks to start, resume or redo the design or visual identity of a project, or wants a new site, app UI or redesign planned before code.
---

# Design flow

Five phases, each closed by the user's approval. The state lives in the repo, so any session (today or next week) resumes where the last one stopped. The design reference of `bedrock-design` is the outcome: read that skill too, it defines the reference's structure and rules.

Talk to the user in their language (Italian for DooperDrew). Never run this flow in an unattended or scheduled session, automatic mode included: every phase needs the user, and automatic mode (below) is started by the user live in the session, after answering the permission questions.

## Start: read the state

1. Read `.bedrock.yml` (key `design:`, the design reference path; default `design/DESIGN.md`) and every `FLOW.md` under `design/`, on main, on open `design/*` branches and on the branches of PRs stacked on them (phase 5 screens); when copies differ, the most advanced one is the state. Several flows can be open at once (see "A second flow in the same repo"): if the request does not say which one, list them with their phase and ask.
2. Decide the phase:
   - argument given (`brief`, `direzioni`, `iterazione`, `fissa`, `costruisci`) → that phase, after saying what earlier phase is not approved yet, if any;
   - `stato` → only report the state and the next step, change nothing;
   - otherwise → the first phase in FLOW.md that is not approved.
3. No FLOW.md: create it from the template below. Pick the mode:
   - **new**: no design reference yet, or it is only a stub;
   - **redesign**: a reference exists. Read it first; the brief and the directions start from it (keep what the user wants to keep, say what changes).
4. Tell the user in two lines: the phase, and what you will produce.

Work on a branch `design/<topic>` (from main, merging main back at least daily). Commit after every approved step and push.

## A second flow in the same repo

A side-by-side proposal (a new identity next to a redesign already in progress, a "from scratch" round) is its own flow:
- everything lives in `design/<topic>/`: its `FLOW.md`, `BRIEF.md`, `directions/`, and at phase 4 its `DESIGN.md` and `tokens.json`;
- until a PR adopts it, it does not change the shared files: the reference named in `.bedrock.yml`, the `design:` key, app or theme code, the token test, linter configs (rule `design-proposals-own-folder`). Phase 4 of such a flow stops at the proposal's own reference; the PR that adopts it moves the files and points `.bedrock.yml` there;
- FLOW.md says in its header line which flow it is and which other open flow it sits next to.

## Automatic mode

Only when the user explicitly asks you to answer the flow yourself (for example "rispondi in automatico usando il contesto della repo"):
- take answers only from the repo (docs, current reference, earlier proposals) and what the request allows; quote the request with its date at the top of FLOW.md;
- every gate you pass is `approvato (auto)` in the phase table, with the reason in Registro, and one line in Attriti per automatic gate;
- the PR text starts by saying the choices are automatic and still to be confirmed;
- stop at the phase 4 gate: open the PR and wait. Phase 5 starts only after the user approves that PR, unless the request names phase 5 or building the screens ("tutto il flow" or "rispondi a tutto" alone does not count); then write that in FLOW.md, stack the screen PRs on the phase 4 PR, and never merge a screen PR before the user approves the phase 4 PR;
- everything that needs permission (downloads, purchases, publishing outside private pages) still needs it: ask at the start, before going automatic.

## Skill per phase

Use these, so results stay consistent between sessions. If one is not installed (another PC, a cloud session), work without it and write so in FLOW.md. Before phase 2, check that each skill can run: `impeccable` downloads its engine on first use, which needs the user's permission, so ask for it at the start; if the user says no, follow its checklist by hand and write so.

| Phase | Skills |
|---|---|
| 1 Brief | none: interview |
| 2 Directions | `frontend-design` for direction; `ui-ux-pro-max` for palettes, font pairings and UX data; `brandkit` only for a logo or identity board; `design-taste-frontend` instead of `frontend-design` for a full marketing or company website |
| 3 Iteration | `impeccable` (critique and audit) every round |
| 4 Lock | `bedrock-design` (reference structure, tokens, token test) |
| 5 Build | `impeccable` audit on each screen PR; the reference wins over every skill default |

## Phase 1: Brief

Interview the user before writing anything visual; ask a few questions at a time (use the question tool when available), and propose answers they can just confirm. Fill `design/BRIEF.md`:

```markdown
# Brief: <project>

## Cos'è e per chi
<problem solved, audience, the one thing a visitor must understand or do>

## Tono
<2-3 adjectives> · Riferimenti: <site or app> - <what exactly the user likes there>
Da evitare: <styles or feelings the user dislikes>

## Vincoli
- Marchio: <logo, mandatory colors, fonts, or "da creare">
- Stack: <framework and styling, from the repo>
- Dispositivi: <mobile first? touch, mouse or keyboard? (widths go in Superfici)>
- Accessibilità: WCAG 2.x AA (testo >= 4.5:1, bordi e icone >= 3:1)
- Contenuti: <real texts and images available; the rest stays [placeholder]>

## Superfici
- <surface: phone app (iOS, Android), web app, desktop app, marketing site...>: <widths and themes it is shown at>

## Schermate chiave
1. <screen>: <goal>
2. <screen>: <goal>
3. <screen>: <goal>
```

Schermate chiave includes the screens where the user acts (forms, settings, confirmations), not only the ones that show data; the success and error states of an action can be shown on its screen rather than as separate screens.

Superfici lists every place the product is used or shown, each with its widths. Ask about the ones the user did not name, because they are the ones forgotten later: a phone app usually also has a website, a website has a phone width, an app with accounts may have a web version. Directions and rounds are planned from this list. In automatic mode, take the surfaces from the repo (code, docs, earlier proposals) and mark the uncertain ones `[da decidere]`.

Gaps stay marked `[da decidere]`, never invented. **Gate:** the user approves the brief, Superfici included.

## Phase 2: Three directions

Make three really different directions, not three shades of one. For each:
- one static HTML page with the most important key screen, in `design/directions/<a|b|c>.html`, self-contained (inline CSS, no build), using real content from the brief;
- at the top of the file, a comment with a 3-line rationale: the idea, why it fits the brief, the risk it takes;
- the key screen on every surface of the brief's Superfici (one frame per surface is enough), or a line on the page saying which surface it leaves out and why. Frames wider than the viewport are scaled down to fit or stacked, so the page itself never scrolls sideways at 375 px.

Each direction must respect the brief's constraints; frontend-design deliberately takes an aesthetic risk, so check it against the brand before showing. Then:
- screenshot each at 375 px and 1280 px (browser pane or a headless browser), also for a desktop app, because the pages are reviewed on the phone; for a desktop app add the real window size from Superfici. Fix cut text, overlaps, horizontal scroll and contrast before showing. Horizontal scroll is measured, not eyeballed: at 375 px `document.documentElement.scrollWidth` must not exceed `innerWidth`. A common cause is a grid column of `1fr` holding a wide child: use `minmax(0, 1fr)`;
- when the Artifact tool is available, publish each as a private page so the user can compare them from the phone, and write the links in FLOW.md.

**Gate:** the user picks one direction (or asks for a mix: then make that as direction d and ask again).

## Phase 3: Iteration

Rounds on the chosen direction, extended to the other key screens on every surface of the brief, the screens where the user acts included. Before a round, write in FLOW.md which screens on which surfaces it will produce, and which key screens are still undrawn after it, so a missing surface or a missing action shows before the user has to point it out.
1. apply the user's feedback (screenshots they send count as feedback);
2. run `impeccable` critique and audit, fix what is blocking, list the rest. Write which parts really ran (detector, critique with its separate sub-agents, or the checklist by hand). A detector finding is dismissed only with the evidence that contradicts it, written in FLOW.md: the contrast measured in the browser, the padding in px, or the reference line that sets a brand colour (the reference wins over the detector's taste rules);
3. measure contrast of every text/background pair and write the ratios;
4. screenshot at 375 and 1280 px and at the widths of each surface in Superfici, in each theme it lists, check for defects, republish;
5. commit with a message naming the round, log the round in FLOW.md.

**Gate:** every key screen of the brief is drawn on every surface (or FLOW.md says why a screen or surface is left out), no blocking audit finding, all ratios pass, and the user says it is right.

## Phase 4: Lock the design

Extract the system from the approved pages into the design reference, in the structure of `bedrock-design` (identity and voice, tokens, components with states, screens, decisions, verified / not verified):
- machine-readable tokens in `design/tokens.json` (or the project's theme file), named in the reference; a second flow keeps them in its own folder;
- if the project has code, a front-end test that the code uses only tokens (see `bedrock-design`, Checks); a second flow adds it only in the PR that adopts it;
- the `design:` key of `.bedrock.yml` points to the reference (not for a second flow: see "A second flow in the same repo").

Open a PR from the design branch. **Gate:** CI green and the user approves the PR (never merge it yourself unless the user says so).

## Phase 5: Build

One screen (or one component family) per PR, in the order of the brief's key screens:
- read the reference first; use only its tokens and components; a needed design change goes in the reference in the same PR and is said in the PR text;
- before opening the PR: `impeccable` audit, screenshots at 375 and 1280 px, the token test green;
- tick the screen in FLOW.md in the same PR.

Each PR follows the project's usual merge rules.

## Friction log

The flow improves from what goes wrong while it is used: a review (bedrock's nightly task) reads these notes and proposes changes to this skill. Whenever one of these happens, add one dated line under `## Attriti` in FLOW.md, in the same commit as the work:
- the user corrected you, repeated a request, or said a step is slow, confusing or unnecessary (quote their words briefly);
- a skill was missing or failed (and what you did instead);
- you skipped or merged a phase, or passed a gate without the user's approval (automatic mode included);
- a round only fixed a defect you should have caught yourself (cut text, contrast, overlap, horizontal scroll);
- the instructions of this skill were unclear or contradicted another skill.

Write the fact, not a judgment: "<date> fase 2: the user asked twice for less empty space in the hero; direction b ignored the brand colors". No line when nothing went wrong.

## FLOW.md template

```markdown
# Design flow: <project>

Modalità: new | redesign · Branch: design/<topic> · Riferimento: <path from .bedrock.yml, or this folder's DESIGN.md for a second flow> · Accanto a: <other open flow, if any>

| Fase | Stato | Data | Note |
|---|---|---|---|
| 1 Brief | da fare / in corso / approvato | | design/BRIEF.md |
| 2 Direzioni | | | scelta: <a/b/c>, link alle pagine |
| 3 Iterazione | | | giri fatti: <n> |
| 4 Fissa | | | PR <link> |
| 5 Costruisci | | | |

## Schermate (fase 5)
- [ ] <screen 1> - PR <link>

## Registro
- <date>: <what was done, what the user decided>

## Non verificato
- <skills missing, widths not checked, assumptions>

## Attriti
- <date> fase <n>: <what went wrong, in facts>
```

Keep FLOW.md short: decisions and links, not the discussion. A lesson other projects could hit goes to bedrock with `bedrock-lesson`.
