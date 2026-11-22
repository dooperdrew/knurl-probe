---
title: File formats
title_it: Formati dei file
summary: The report (JSON) written by `run` and the model description (TOML) in headphones/.
summary_it: Il report (JSON) scritto da `run` e la descrizione del modello (TOML) in headphones/.
lang: en
type: reference
topic: formats
status: current
verified: 2026-11-18
order: 20
---

# File formats

A **report** is what the probe writes after checking one pair of headphones: JSON, because it is a record written by a program and sent as it is. A **description** is what a person writes about one model: TOML, because people read and edit it, with comments. Codes are written in hexadecimal, as in the protocol sources.

## The report (`reports/knurl-report-<model>-<time>.json`)

Written by Knurl Probe (the app) and by `knurl-probe-cli run`. One file per check. It holds no Bluetooth address and no personal names: a Windows device name that is not a Sony model name is replaced by `(renamed by the user: not included)`.

```json
{
  "format": 1,
  "tool": "knurl-probe 0.1.0",
  "date": "2026-11-07",
  "os": "Windows 10.0.26100",
  "device_name": "WH-1000XM3",
  "services": {
    "v1": { "state": "opened" },
    "v2": { "state": "not_offered" }
  },
  "answers": {
    "protocol_version": "0x4010",
    "model": "WH-1000XM3",
    "firmware": "4.5.2",
    "functions": [{ "code": "0x11", "name": "battery level" }],
    "battery_percent": 100,
    "charging": "no",
    "noise": "noise cancelling",
    "equalizer_preset": "off",
    "equalizer_bands": [0, 0, 0, 0, 0, 0]
  },
  "verdict": "supported",
  "frames": [{ "t_ms": 0, "dir": "tx", "hex": "3E 0C 00 00 00 00 02 00 00 0E 3C" }],
  "problems": []
}
```

| Field | Meaning |
|---|---|
| `services` | For each Sony service: `opened`; `not_offered` (Windows lists it for the other service only, or a device found by its Sony name could not open it); `not_tried` (found by name, and MDR v1 already answered); or `failed` with a `reason` |
| `answers` | What the headphones answered on MDR v1, decoded; empty on MDR v2, which nothing decodes yet |
| `verdict` | `supported` (a description ships in `headphones/`), `new_model`, `not_decoded` (only MDR v2 answered), `nothing_read` |
| `frames` | Every frame in both directions, the raw truth behind `answers` |
| `problems` | What went wrong, in plain words |

A tool reads only reports whose `format` it knows; a newer one is refused with an error.

## The description (`headphones/<id>.toml`)

One file per model. Knurl supports the models described here.

```toml
format = 1
id = "sony-wh-1000xm3"         # same as the file name: lowercase, digits, dashes
name = "Sony WH-1000XM3"
model = "WH-1000XM3"           # the model name the headphones answer with
service = "mdr-v1"             # mdr-v1 or mdr-v2
protocol_version = 0x4010      # optional: from the first answer
firmware = ["4.5.2"]           # versions seen in reports
functions = [0x11, 0xC1]       # feature codes the headphones declare
note = "anything a person should know"

[verification]
status = "reported"            # unverified, reported (1 report), verified (2 or more agree)
reports = 1
last = "2026-11-07"
source = "where the facts come from"
```

`knurl-probe-cli validate FILE...` checks a description: an id that is not lowercase letters, digits and dashes, a file not named after its id, an empty name or model, a `reported` or `verified` status without enough reports, a code or firmware listed twice, a newer format.

## Comparing a report with a description

`knurl-probe-cli check REPORT [--against FILE]` compares them (without `--against`, with the shipped description of the report's model) and ends with exit code 1 when they differ. It compares the model name, the service that answered, the protocol version, the feature codes and the firmware. What the report could not read (a missing answer, an MDR v2 conversation) is not compared.

## Status of a description
- `unverified`: written from documents or other tools. Nobody ran the probe on the headphones.
- `reported`: one report agrees with it (`knurl-probe-cli check` passes).
- `verified`: two or more reports from different people agree.

A description never claims more than its reports show. The same rule and the same three steps are used by Keyfold Probe for keyboards.
