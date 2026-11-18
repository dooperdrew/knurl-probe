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
- **Honesty:** nothing fake. The Send button is shown and disabled until sending exists; errors are shown with Windows' reason; "Saved" appears only after the file is written.

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
| 1. Welcome | what the app does; what the report holds and never holds; Start; the Slint badge and the version | `01-welcome` |
| 2. Choose your headphones | searching (progress bar); the paired Sony headphones, each with Check; none found (how to pair, Search again, Open Bluetooth settings); the lookup failed (Windows' reason) | `02` to `05` |
| 3. Checking | the four steps: opening the channel, asking model/firmware/features, listening on the newer service, putting the report together; a step the headphones do not offer is marked "not offered" | `06-checking` |
| 4. Result | in plain words: already supported, new model, newer service (not decoded), nothing read; "What was read"; problems as error notices; Next, Check again | `07` to `10` |
| 5. Send | Save to a file (Documents\Knurl Probe), Send (disabled in this version), See what will be sent (the exact JSON), the GitHub issue form; Saved with Show in folder, or the reason it was not saved with Try again | `11` to `13` |

`cargo run -p knurl-probe-app --example snapshot -- <folder>` renders the 13 states in light and dark (26 PNGs) with Slint's software renderer and **sample** values. The images are not committed (they are regenerated, and on Linux the fonts differ from Segoe UI).

## 5. Decisions
- **2026-11-18, the Knurl family, copied.** The owner chose the Knurl look for the probe (DooperDrew/knurl-probe#2). Tokens and components are copied rather than shared through a crate: the two repositories are separate (one private, one public) and the probe needs only a few components.
- **2026-11-18, one fixed window, five screens.** A wizard: one thing per screen, Back on the screens where going back is safe (not while checking).
- **2026-11-18, Send disabled, not hidden.** The button is on the Send screen from the start, disabled, with a sentence saying sending is not available yet and how to send the file instead; the relay client comes in a later pull request.
- **2026-11-18, the Slint badge on the first screen.** The Slint Royalty-free licence 2.0 asks for the AboutSlint widget in an About or splash screen; the welcome screen is the app's first screen, so the badge sits in its footer, small, as in Knurl's About window.
- **2026-11-18, the Windows name on the Choose screen only.** The list shows the name Windows gives the headphones, which may be one the user chose; it stays on the PC. The report keeps it only if it is a Sony model name (`knurl-report`).

## 6. Verified and not verified
**Verified**
- `python design/contrast.py`: 60 pairs, light and dark, none under its minimum.
- `cargo test -p knurl-probe-app`: theme equals tokens, no colour outside the theme, the flow between the screens, the plain-words result, the step states.
- The 26 images of the snapshot example, looked at one by one on 2026-11-18 (rendered on Linux, with a fallback font instead of Segoe UI and Consolas).

**Not verified**
- The app on Windows: the real window, Segoe UI, scaling at 125-200 %, keyboard and screen reader use.
- The check from the app on real headphones (the same code as `knurl-probe run`, itself not yet run on headphones since the move).
- Sending (not in this version) and the Italian and Spanish texts (a later pull request).
