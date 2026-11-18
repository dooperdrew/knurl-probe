"""Contrast ratios (WCAG 2.x) of the design reference tokens (design/tokens.json), light and dark.
Copied from the Knurl app (same tokens, same pairs).
Run: python design/contrast.py   Exits 1 if a pair is under its minimum."""
import json, pathlib, sys

def lum(h):
    h = h.lstrip('#'); c = [int(h[i:i + 2], 16) / 255 for i in (0, 2, 4)]
    c = [x / 12.92 if x <= 0.03928 else ((x + 0.055) / 1.055) ** 2.4 for x in c]
    return 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]

def cr(a, b):
    la, lb = sorted([lum(a), lum(b)], reverse=True); return (la + 0.05) / (lb + 0.05)

tokens = json.loads((pathlib.Path(__file__).parent / 'tokens.json').read_text(encoding='utf-8'))
# (foreground, background, minimum): text 4.5, borders and icons 3.0
PAIRS = [('ink', 'surface', 4.5), ('ink', 'card', 4.5), ('muted', 'surface', 4.5), ('muted', 'card', 4.5),
         ('accent-text', 'card', 4.5), ('accent-text', 'surface', 4.5), ('accent-text', 'accent-soft', 4.5),
         ('on-accent', 'accent', 4.5), ('accent', 'card', 3.0), ('accent', 'surface', 3.0),
         ('warn', 'warn-bg', 4.5), ('ink', 'warn-bg', 4.5), ('muted', 'warn-bg', 4.5),
         ('error', 'error-bg', 4.5), ('ink', 'error-bg', 4.5), ('muted', 'error-bg', 4.5),
         ('ink', 'accent-soft', 4.5), ('muted', 'accent-soft', 4.5), ('muted', 'panel', 4.5), ('ink', 'track', 4.5),
         ('ink', 'track-hover', 4.5), ('muted', 'track-hover', 4.5), ('error', 'card', 4.5), ('error', 'surface', 4.5),
         ('ok', 'card', 3.0), ('control', 'card', 3.0), ('control', 'surface', 3.0),
         ('accent-text', 'error-bg', 4.5),
         # Warning text on the page and on cards (Knurl's low battery colour).
         ('warn', 'surface', 4.5), ('warn', 'card', 4.5)]
# Decorative borders, measured for the record and never failing: `line` frames
# rows, tiles and the footer, where the content (text, icon) carries the meaning.
DECORATIVE = [('line', 'card'), ('line', 'surface')]
bad = 0
for theme in ('light', 'dark'):
    t = tokens['color'][theme]
    for fg, bg, need in PAIRS:
        r = cr(t[fg], t[bg]); ok = r >= need; bad += not ok
        print(f"{theme:5} {fg:>11} on {bg:<11} {r:5.2f}  (min {need}){'' if ok else '  FAIL'}")
    for fg, bg in DECORATIVE:
        print(f"{theme:5} {fg:>11} on {bg:<11} {cr(t[fg], t[bg]):5.2f}  (decorative, informational)")
sys.exit(1 if bad else 0)
