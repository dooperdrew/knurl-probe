---
title: "ADR-0003: An app for people who are not technical, in the Knurl look"
title_it: "ADR-0003: Un'app per chi non è tecnico, con l'aspetto di Knurl"
summary: Knurl Probe is a Slint app with five screens in the Knurl family look; the command line stays for developers.
summary_it: Knurl Probe è un'app Slint con cinque schermate nell'aspetto della famiglia Knurl; la riga di comando resta per gli sviluppatori.
lang: en
type: decision
status: current
order: 30
---

# ADR-0003: An app for people who are not technical, in the Knurl look

**Date.** 2026-11-18, decided with the owner (plan: #2; app: #6, languages: #10, release: #11).

**Context.** The people who own new models are not developers. A command-line tool that writes a file to attach to a GitHub issue asks too much of them.

**Decision.**
- **Knurl Probe** (`knurl-probe.exe`) is a Windows app in Slint, like Knurl: one window and five screens (welcome, choose your headphones, checking, result in plain words, send).
- **The Knurl family look.** Tokens and components are copied from Knurl rather than shared through a crate, because the two repositories are separate (one private, one public). Details: [design reference](../../design/README.md).
- **Languages:** English, Italian and Spanish, with the same system as Knurl; Windows' language by default, with a choice on the welcome screen.
- **Slint licence:** Slint is used under its Royalty-free licence 2.0, which asks for the AboutSlint widget in an About or splash screen. The badge sits in the footer of the welcome screen.
- **The command line stays** for developers, as `knurl-probe-cli.exe`, and both exes ship in one zip.

**Consequences.**
- The flow and the texts have no I/O and are tested on any system; the window itself is checked by renders from an example, not by a person on Windows (not verified yet).
- The welcome screen counts as the "splash" screen for the Slint attribution. The owner has not confirmed this yet.
