// Managed by bedrock. Do not edit in consumer repos: changes arrive through bedrock sync PRs.
//
// Steps of bedrock-rust-ci.yml for crates that build a Windows GUI binary, that is a crate
// with a `#![windows_subsystem = "windows"]` attribute (also inside cfg_attr). Such a binary
// has no console in release builds, which debug builds (console subsystem) hide:
//
// --panic-hook  step "GUI binaries install a panic hook" (rule rust-gui-panic-hook). stderr goes
//   nowhere, so the default panic message, the only trace of a crash, is lost. The crate's .rs
//   files must call panic::set_hook / set_hook( or a reporter's install_panic_hook(, or say why
//   not with "bedrock: panic hook ok: <reason>". Lesson dooperdrew/bedrock#187 (knurl-probe-app
//   had no hook; dooperdrew/knurl#83 A2, #84 H3).
// --console     step "GUI binaries start children without a console" (rule
//   rust-gui-no-console-window). A console program started by a process without a console gets
//   a new console window, unless the parent passes CREATE_NO_WINDOW through
//   CommandExt::creation_flags. Each .rs file of the crate that calls Command::new( must also
//   call creation_flags(, or carry "bedrock: console ok: <reason>" on the line of the call or
//   the line above. Lesson dooperdrew/bedrock#188 (knurl-probe-app ran `cmd /C ver`;
//   dooperdrew/knurl#84 H1).
//
// A crate is the nearest folder with a tracked Cargo.toml; files of nested crates belong to
// them. Comments are ignored, except for the waivers. No dependencies.
import { execFileSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

const GUI = /#!\s*\[[^\]]*\bwindows_subsystem\s*=\s*"windows"/;
const HOOK = /(?<![\w$])(?:set_hook|install_panic_hook)\s*\(/;
const HOOK_WAIVER = /bedrock:\s*panic hook ok:\s*[^\s<]/;
const COMMAND = /(?<![\w$])Command\s*::\s*new\s*\(/g;
const FLAGS = /\.\s*creation_flags\s*\(/;
const CONSOLE_WAIVER = /bedrock:\s*console ok:\s*[^\s<]/;

/** The text with Rust comments replaced by spaces (newlines kept). Strings stay. */
export function withoutRustComments(text) {
  let out = "";
  let i = 0;
  while (i < text.length) {
    const c = text[i];
    const next = text[i + 1];
    const raw =
      c === "r" && !/[\w$]/.test(text[i - 1] ?? "") ? /^r(#*)"/.exec(text.slice(i)) : null;
    if (raw) {
      // Raw string r"…", r#"…"#: ends at the quote followed by the same number of #.
      const close = text.indexOf(`"${raw[1]}`, i + raw[0].length);
      const stop = close === -1 ? text.length : close + 1 + raw[1].length;
      out += text.slice(i, stop);
      i = stop;
    } else if (
      c === "'" &&
      (text[i + 2] === "'" || (next === "\\" && /^'\\[^']{1,8}'/.test(text.slice(i))))
    ) {
      // A char literal ('"', '\''), not a lifetime.
      const stop = text[i + 2] === "'" && next !== "\\" ? i + 3 : text.indexOf("'", i + 3) + 1;
      out += text.slice(i, stop);
      i = stop;
    } else if (c === '"') {
      let j = i + 1;
      while (j < text.length && text[j] !== '"') {
        if (text[j] === "\\") j++;
        j++;
      }
      out += text.slice(i, j + 1);
      i = j + 1;
    } else if (c === "/" && next === "/") {
      const end = text.indexOf("\n", i);
      const stop = end === -1 ? text.length : end;
      out += " ".repeat(stop - i);
      i = stop;
    } else if (c === "/" && next === "*") {
      // Rust block comments nest.
      let depth = 0;
      let j = i;
      while (j < text.length) {
        if (text[j] === "/" && text[j + 1] === "*") {
          depth++;
          j += 2;
        } else if (text[j] === "*" && text[j + 1] === "/") {
          depth--;
          j += 2;
          if (depth === 0) break;
        } else j++;
      }
      out += text.slice(i, j).replace(/[^\n]/g, " ");
      i = j;
    } else {
      out += c;
      i++;
    }
  }
  return out;
}

/** True when a Rust text sets windows_subsystem = "windows" in an attribute (not a comment or string). */
function isGui(text) {
  // Blank string contents except the attribute's own value, then look for the attribute.
  const code = withoutRustComments(text).replace(/"(?:[^"\\]|\\.)*"/g, (s) =>
    s === '"windows"' ? s : '""',
  );
  return GUI.test(code);
}

/** The crate folder ("" for the root) of a tracked path, given the folders with a Cargo.toml. */
function crateOf(path, crates) {
  const parts = path.split("/").slice(0, -1);
  for (let n = parts.length; n >= 0; n--) {
    const dir = parts.slice(0, n).join("/");
    if (crates.has(dir)) return dir;
  }
  return null;
}

/** GUI crates and their .rs files: Map(crate -> [paths]). */
export function guiCrates(tracked, read) {
  const crates = new Set(
    tracked
      .filter((f) => /(^|\/)Cargo\.toml$/.test(f) && !/(^|\/)target\//.test(f))
      .map((f) => f.split("/").slice(0, -1).join("/")),
  );
  const files = new Map();
  for (const f of tracked) {
    if (!f.endsWith(".rs") || /(^|\/)target\//.test(f)) continue;
    const c = crateOf(f, crates);
    if (c === null) continue;
    // Build scripts, tests, examples and benches are not the GUI binary: they run with a
    // console (build.rs runs git or rc.exe), and a hook in a test does not cover the app.
    const inCrate = c === "" ? f : f.slice(c.length + 1);
    if (inCrate === "build.rs" || /^(tests|examples|benches)\//.test(inCrate)) continue;
    if (!files.has(c)) files.set(c, []);
    files.get(c).push(f);
  }
  const gui = new Map();
  for (const [c, list] of files) {
    if (list.some((f) => isGui(read(f)))) gui.set(c, list);
  }
  return gui;
}

/** GUI crates with no panic hook: [{ crate, file }] (file = the one with the attribute). */
export function missingHooks(tracked, read) {
  const problems = [];
  for (const [crate, list] of guiCrates(tracked, read)) {
    const texts = list.map((f) => [f, read(f)]);
    if (texts.some(([, t]) => HOOK_WAIVER.test(t))) continue;
    if (texts.some(([, t]) => HOOK.test(withoutRustComments(t)))) continue;
    const file = texts.find(([, t]) => isGui(t))[0];
    problems.push({ crate: crate || ".", file });
  }
  return problems;
}

/** Command::new calls in GUI crates whose file never sets creation_flags: [{ file, line }]. */
export function consoleCommands(tracked, read) {
  const problems = [];
  for (const [, list] of guiCrates(tracked, read)) {
    for (const f of list) {
      const text = read(f);
      const code = withoutRustComments(text);
      if (FLAGS.test(code)) continue;
      const raw = text.split(/\r?\n/);
      for (const m of code.matchAll(COMMAND)) {
        const line = code.slice(0, m.index).split("\n").length;
        if (CONSOLE_WAIVER.test(raw[line - 1] ?? "") || CONSOLE_WAIVER.test(raw[line - 2] ?? ""))
          continue;
        problems.push({ file: f, line });
      }
    }
  }
  return problems;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const mode = process.argv[2];
  if (mode !== "--panic-hook" && mode !== "--console") {
    console.log("usage: windows-gui.mjs --panic-hook | --console");
    process.exit(2);
  }
  let tracked = [];
  try {
    tracked = execFileSync("git", ["ls-files", "-z"], { encoding: "utf8" })
      .split("\0")
      .filter(Boolean)
      .filter((f) => existsSync(f));
  } catch {
    console.log("Not a git checkout: nothing to check.");
    process.exit(0);
  }
  const read = (f) => readFileSync(f, "utf8");
  const gui = guiCrates(tracked, read).size;
  if (gui === 0) {
    console.log('No crate sets windows_subsystem = "windows": nothing to check.');
    process.exit(0);
  }
  if (mode === "--panic-hook") {
    const problems = missingHooks(tracked, read);
    for (const p of problems)
      console.log(
        `::error file=${p.file}::crate ${p.crate} builds a Windows GUI binary but installs no panic hook: with no console a panic leaves no trace. Call std::panic::set_hook (log, message box or report) or the reporter's install_panic_hook, or add the comment "bedrock: panic hook ok:" followed by the reason`,
      );
    if (problems.length === 0) console.log(`Every GUI crate (${gui}) installs a panic hook.`);
    process.exit(problems.length ? 1 : 0);
  }
  const problems = consoleCommands(tracked, read);
  for (const p of problems)
    console.log(
      `::error file=${p.file},line=${p.line}::Command::new in a Windows GUI crate without creation_flags: a console program started from it opens a console window. Set CREATE_NO_WINDOW (0x08000000) with std::os::windows::process::CommandExt::creation_flags, prefer a Win32 API to parsing a command's localized output, or add the comment "bedrock: console ok:" followed by the reason`,
    );
  if (problems.length === 0)
    console.log(`Every Command in the GUI crates (${gui}) sets creation_flags.`);
  process.exit(problems.length ? 1 : 0);
}
