---
title: Design reference
summary: How Knurl Probe looks and why - the Knurl family's identity, tokens with measured contrast, components, the five screens, decisions, and what is verified.
type: reference
topic: design
status: current
verified: 2026-11-18
order: 30
---

# Design reference: Knurl Probe

The app's code (`crates/knurl-probe-app/ui/`) follows this file. It is the Knurl app's design reference applied to a small wizard: same identity, same tokens, same components, copied from `DooperDrew/knurl` on 2026-11-18 (`design/tokens.json`, `crates/xmc-app/ui/theme.slint`, `components.slint`, `design/icons/`). A change to Knurl's design is brought here by hand; nothing syncs it.

## 1. Identity and voice
- **Name:** Knurl Probe, in the window title and the header of every screen. It is part of Knurl: it exists to teach Knurl new headphones.
- **Mark:** Knurl's knob seen from above (graphite disc, hub, orange index at 45°); on dark backgrounds the disc turns stone. Files: `design/icons/knob.svg`, `knob-dark.svg`, `app-icon.svg`, `app-icon-small.svg`, the same as Knurl's.
- **Voice:** for people who are not technical. Plain sentences, one idea each; what the app does and what it never does said before it does it; no Bluetooth jargon on the screens ("control service" and "MDR" appear only in "What was read" and in errors, where they help a developer reading a report).
- **Honesty:** nothing fake. "Sent" appears only when the relay answers with the issue it opened, and links to it; any other answer is "Not sent" with the reason in plain words; "Saved" appears only after the file is written.

## 2. Tokens
The colours, radii and spaces are Knurl's, unchanged: [tokens.json](tokens.json). `ui/theme.slint` mirrors the colours, picked by Windows' light or dark setting through Slint's `Palette`; `tests/design_tokens.rs` fails when the two differ or when a `.slint` file other than `theme.slint` writes a colour.

### Colours and contrast (WCAG 2.x, measured with `python design/contrast.py`)
| Token | Light | Dark | Use |
|---|---|---|---|
| surface | #F7F7F5 | #1C1D1F | window background |
| card | #FFFFFF | #28292B | cards, secondary buttons |
| line | #DCDCD7 | #3A3B3E | card borders (decorative) |
| control | #7E7F83 | #75777B | secondary button border, waiting step |
| ink | #1C1D1F | #EEEEEB | text |
| muted | #5C5E62 | #A6A7A8 | secondary text |
| disabled | #9A9B97 | #66676B | disabled button text |
| accent | #C2501B | #EF7A45 | primary button, spinner, progress |
| accent-text | #A8431A | #EF7A45 | links |
| on-accent | #FFFFFF | #1C1D1F | text on the primary button |
| ok | #276B3C | #62C987 | done step, "the report holds" ticks |
| error / error-bg | #B42318 / #FDECEA | #FF8B7E / #3A1F1E | error notices |

All 60 measured pairs (30 per theme) pass their minimum (text 4.5:1, borders and icons 3:1). The lowest for text is `on-accent` on `accent` in light, 4.71:1 (the primary buttons); `control` on `card` is 4.00 (light) and 3.25 (dark). `line` on `card` and `surface` (1.30-1.51:1) is decorative and never fails: the card's content carries the meaning.

### Type, shape, space
- Segoe UI everywhere (`default-font-family`), Consolas for the report's JSON. Screen title 20 px semibold, body 13-14 px, small print 12 px.
- Cards and notices: radius 8 px, 1 px `line` border. Buttons: 32 px high, radius 6 px.
- Window: 440 × 660 px, fixed, padding 24 px, 16 px between blocks.

## 3. Components
From Knurl's `components.slint` (copied as they are): `BrandButton` (primary filled `accent`, secondary bordered `control`, disabled, focus ring), `LinkText`, `Notice` (error, note), `Card`, `KnobMark`, `BrandSpinner`, `SearchBar`, `StrokeIcon`, `RowGlyph`, `CloseButton`. New here, in `app.slint`: `StepMark` (waiting circle, spinner, done tick in `ok`, "not offered" dash), `Heading` (screen title and line), `Bullet` (tick or cross and a sentence).

## 4. Screens
| Screen | What it shows | Rendered by the example as |
|---|---|---|
| 1. Welcome | what the app does; what the report holds and never holds; Start; the language (Same as Windows, English, Italiano, Español); the Slint badge and the version | `01-welcome` |
| 2. Choose your headphones | searching (progress bar); the paired Sony headphones, each with Check; none found (how to pair, Search again, Open Bluetooth settings); the lookup failed (Windows' reason) | `02` to `05` |
| 3. Checking | the four steps: opening the channel, asking model/firmware/features, listening on the newer service, putting the report together; a step the headphones do not offer is marked "not offered" | `06-checking` |
| 4. Result | in plain words: already supported, new model, newer service (not decoded), nothing read; "What was read"; problems as error notices; Next, Check again | `07` to `10` |
| 5. Send | Send (primary) and Save to a file (Documents\Knurl Probe) as buttons; See what will be sent (the exact request that leaves the PC) and the GitHub issue form as links; Sending… (Send and Back disabled); Sent with the issue link and Open the issue; Not sent with the reason and Try again; Saved with Show in folder, or the reason it was not saved; a report over the relay's limit can only be saved | `11` to `16` |

`cargo run -p knurl-probe-app --example snapshot -- <folder>` renders the 16 states in English light and dark, and in Italian and Spanish light (64 PNGs, suffix `-it`, `-es`), with Slint's software renderer and **sample** values. The images are not committed (they are regenerated, and on Linux the fonts differ from Segoe UI).

## 5. Decisions
- **2026-11-18, the Knurl family, copied.** The owner chose the Knurl look for the probe (DooperDrew/knurl-probe#2). Tokens and components are copied rather than shared through a crate: the two repositories are separate (one private, one public) and the probe needs only a few components.
- **2026-11-18, one fixed window, five screens.** A wizard: one thing per screen, Back on the screens where going back is safe (not while checking).
- **2026-11-18, Send disabled, not hidden** (until the relay client existed).
- **2026-11-19, Send through the bedrock relay.** Send posts a `data` report to the relay, which opens an issue in this repository (no GitHub account needed). Send is the primary button; it is disabled while sending and once sent, so one report opens one issue (a new check can be sent again). "See what will be sent" shows the exact request body, the report included as an escaped string: harder to read than the report alone, but it is byte for byte what leaves the PC. The attachment is named `knurl-report.json`, short, because the relay hides long letter-and-digit runs.
- **2026-11-18, the Slint badge on the first screen.** The Slint Royalty-free licence 2.0 asks for the AboutSlint widget in an About or splash screen; the welcome screen is the app's first screen, so the badge sits in its footer, small, as in Knurl's About window.
- **2026-11-18, the Windows name on the Choose screen only.** The list shows the name Windows gives the headphones, which may be one the user chose; it stays on the PC. The report keeps it only if it is a Sony model name (`knurl-report`).

- **2026-11-19, English, Italian and Spanish, as Knurl.** The same system as the Knurl app: Rust texts through `t!` with `lang/<code>/app.po`, the window's texts through `@tr` with `lang/<code>/LC_MESSAGES/knurl-probe-app.po` bundled in the exe; shared terms worded as in Knurl (Indietro/Atrás, Invia/Enviar, Come Windows/Como Windows, Apri le impostazioni Bluetooth). The choice is on the welcome screen, "Same as Windows" first; the language names are never translated, so a person finds theirs. It is not saved: the app is used once and starts in Windows' language. The report and the relay's message stay in English (they are read by developers on GitHub); "a report" is "il report" and "el informe".
- **2026-11-19, See what will be sent as a link.** Three buttons did not fit in one row in Spanish while sending ("Enviando..."): Send and Save stay buttons, the data toggle becomes a link like the issue form's. The "What was read" labels column is 150 px, for "Funciones declaradas".

- **2026-11-22, Choose and Result scroll** (audit of 2026-11-21): the device list and the facts and problems sit in a `ScrollView` and the buttons stay under it, so the fixed 440 x 660 window never pushes them out. With nothing read, "Check again" is the primary button and "Next" the secondary. Errors opening a page or the folder show in a message box (the app has no error line outside its screens). Not yet seen on Windows.
- **2026-11-22, the report shown as JSON, not as a string.** Send now posts a `probe` report ([decision 0007](../docs/decisions/0007-send-as-a-probe-report.md)): the report is a JSON object inside the request, so "See what will be sent" shows it indented and readable, after the envelope (probe, schema, version, date, steps), instead of an escaped string. Still byte for byte what leaves the PC. The screens did not change; the send-state images were not rendered again.

## 6. Verified and not verified
**Verified**
- `python design/contrast.py`: 60 pairs, light and dark, none under its minimum.
- `cargo test -p knurl-probe-app`: theme equals tokens, no colour outside the theme, the flow between the screens (send once, try again after a failure, never twice after success), the plain-words result, the step states, the request body (exactly what is shown), the too-long refusal, every relay answer read as sent or not sent; every text of the code and of the window has an Italian and a Spanish translation that keeps its values, the same text has one translation in both catalogues, the language choice.
- The images of the snapshot example, looked at one by one on 2026-11-18, the send states (11 to 16) on 2026-11-19, and every screen in Italian and Spanish on 2026-11-19 (no text cut or out of the window) (rendered on Linux, with a fallback font instead of Segoe UI and Consolas).

**Not verified**
- The app on Windows: the real window, Segoe UI, scaling at 125-200 %, keyboard and screen reader use.
- The check from the app on real headphones (the same code as `knurl-probe-cli run`, itself not yet run on headphones since the move).
- Sending for real: the relay was redeployed on 2026-11-23 from a version that accepts `probe` reports (DooperDrew/bedrock-relay#24), but no report has been sent to it yet. The WinHTTP transport has not run on Windows.
- The Italian and Spanish texts read by a native speaker; the language switch on Windows (only rendered here).
