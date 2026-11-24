// Managed by bedrock. Do not edit in consumer repos: changes arrive through bedrock sync PRs.
//
// Step "Start with Windows reads StartupApproved" of bedrock-repo-checks.yml (rule
// windows-autostart-startup-approved). Since Windows 8 a value under
// HKCU\Software\Microsoft\Windows\CurrentVersion\Run starts with Windows only if the value of the
// same name under ...\Explorer\StartupApproved\Run does not say "off" (binary, odd first byte).
// Task Manager and Settings › Apps › Startup write only that second value. An app that reads or
// writes only Run shows "Start with Windows: on" while Windows keeps it off, and turning it on
// again changes nothing. Lesson dooperdrew/bedrock#207 (dooperdrew/keyfold#89 C7 and
// dooperdrew/knurl#93 point 8, found the same day by two audits).
//
// For each Cargo crate (the nearest folder with a Cargo.toml; target/ skipped): when code (not a
// comment) in one of its .rs files names the key CurrentVersion\Run (not RunOnce), code in some .rs
// file of the same crate must name StartupApproved. Otherwise every file that names the key says
// why not, in that file: "bedrock: startup approved ok: <reason>" (a migration that only removes
// an old value). A waiver in one file never excuses another. Vendored crates are skipped. Reading the
// first byte, clearing the value when the user turns the setting on and the test with the value
// present are checked in review. A repo with no Rust crate has nothing to check.
// No dependencies: consumers may have no node_modules.
import { execFileSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { withoutRustComments } from "./windows-gui.mjs";

const RUN_KEY = /CurrentVersion\\{1,2}Run(?![\w])/i;
const APPROVED = /StartupApproved/i;
const WAIVER = /bedrock:\s*startup approved ok:\s*[^\s<]/;
const SKIP = /(^|\/)(target|vendor|vendored)\//;

/** The crate folder ("" for the root) of a path, given the folders with a Cargo.toml. */
function crateOf(path, crates) {
  const parts = path.split("/").slice(0, -1);
  for (let n = parts.length; n >= 0; n--) {
    const dir = parts.slice(0, n).join("/");
    if (crates.has(dir)) return dir;
  }
  return null;
}

/** Crates that name the Run key without StartupApproved: [{ crate, file, line }]. */
export function missingStartupApproved(files, read) {
  const crates = new Set(
    files
      .filter((f) => /(^|\/)Cargo\.toml$/.test(f) && !SKIP.test(f))
      .map((f) => f.split("/").slice(0, -1).join("/")),
  );
  const byCrate = new Map();
  for (const f of files) {
    if (!f.endsWith(".rs") || SKIP.test(f)) continue;
    const c = crateOf(f, crates);
    if (c === null) continue;
    if (!byCrate.has(c)) byCrate.set(c, []);
    byCrate.get(c).push(f);
  }
  const problems = [];
  for (const [crate, list] of byCrate) {
    const codes = list.map((f) => [f, withoutRustComments(read(f))]);
    if (codes.some(([, code]) => APPROVED.test(code))) continue;
    // Files naming the key without their own waiver.
    const named = codes.find(([f, code]) => RUN_KEY.test(code) && !WAIVER.test(read(f)));
    if (!named) continue;
    const [file, code] = named;
    const line = code.slice(0, code.search(RUN_KEY)).split("\n").length;
    problems.push({ crate: crate || ".", file, line });
  }
  return problems;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  let files = [];
  try {
    // Untracked files too, unless ignored: a run before git add checks what the PR will (#196).
    files = execFileSync("git", ["ls-files", "-co", "--exclude-standard", "-z"], {
      encoding: "utf8",
    })
      .split("\0")
      .filter(Boolean)
      .filter((f) => existsSync(f));
  } catch {
    console.log("Not a git checkout: nothing to check.");
    process.exit(0);
  }
  const problems = missingStartupApproved(files, (f) => readFileSync(f, "utf8"));
  for (const p of problems)
    console.log(
      `::error file=${p.file},line=${p.line}::crate ${p.crate} uses the CurrentVersion\\Run key but never reads Explorer\\StartupApproved\\Run: when the user turns the app off in Task Manager or Settings, Windows writes only that value, so the app shows "Start with Windows" as on while it does not start. Read it (an odd first byte means off) and delete it when the user turns the setting on, or add, in this file, the comment "bedrock: startup approved ok:" followed by the reason`,
    );
  if (problems.length === 0) console.log("No crate uses the Run key without StartupApproved.");
  process.exit(problems.length ? 1 : 0);
}
