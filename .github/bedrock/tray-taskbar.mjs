// Managed by bedrock. Do not edit in consumer repos: changes arrive through bedrock sync PRs.
//
// Step "tray icons come back on TaskbarCreated" of bedrock-repo-checks.yml (rule
// windows-tray-taskbar-created). Shell_NotifyIcon(NIM_ADD) can fail at logon, when an app that
// starts with Windows runs before Explorer has a taskbar, and every icon is lost when Explorer
// restarts. Windows then broadcasts the registered message "TaskbarCreated"; an app that never
// listens for it stays without icon, menu and notifications for the whole session. Lesson
// dooperdrew/bedrock#230 (dooperdrew/keyfold#95 K4: the agent had no icon for good after a failed
// NIM_ADD; dooperdrew/keyfold#97: a fixed GUID left by a crash refused the new icon).
//
// For each Cargo crate (the nearest folder with a Cargo.toml; target/ and vendored crates skipped):
// when code (not a comment) in one of its .rs files names NIM_ADD, code in some .rs file of the
// same crate must name TaskbarCreated (RegisterWindowMessageW(w!("TaskbarCreated"))). Otherwise
// every file that names NIM_ADD says why not, in that file: "bedrock: taskbar created ok: <reason>"
// (for example the window procedure that re-adds lives in another crate, named in the reason).
// Crates that use a tray crate (tray-icon and similar) never name NIM_ADD and are not checked.
// Retrying a refused GUID (delete the stale icon, then fall back to no GUID) is checked in review.
// A repo with no Rust crate has nothing to check. No dependencies.
import { execFileSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { withoutRustComments } from "./windows-gui.mjs";

const ADD = /(?<![\w$])NIM_ADD(?![\w$])/;
const CREATED = /(?<![\w$])TaskbarCreated(?![\w$])/;
const WAIVER = /bedrock:\s*taskbar created ok:\s*[^\s<]/;
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

/** Crates that add a tray icon without handling TaskbarCreated: [{ crate, file, line }]. */
export function missingTaskbarCreated(files, read) {
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
    // TaskbarCreated is looked for with strings kept (it is one: w!("TaskbarCreated"); a log
    // message naming it also counts, left to review); NIM_ADD with strings blanked.
    const codes = list.map((f) => [f, withoutRustComments(read(f))]);
    if (codes.some(([, code]) => CREATED.test(code))) continue;
    for (const [file] of codes) {
      const code = withoutRustComments(read(file), { blankStrings: true });
      if (!ADD.test(code) || WAIVER.test(read(file))) continue;
      const line = code.slice(0, code.search(ADD)).split("\n").length;
      problems.push({ crate: crate || ".", file, line });
    }
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
  const problems = missingTaskbarCreated(files, (f) => readFileSync(f, "utf8"));
  for (const p of problems)
    console.log(
      `::error file=${p.file},line=${p.line}::crate ${p.crate} adds a tray icon with NIM_ADD but never handles TaskbarCreated: NIM_ADD can fail at logon before Explorer has a taskbar, and an Explorer restart removes every icon, so the app stays without icon and menu. Register the "TaskbarCreated" message (RegisterWindowMessageW) and add the icon again when it arrives, or add, in this file, the comment "bedrock: taskbar created ok:" followed by the reason`,
    );
  if (problems.length === 0)
    console.log("No crate adds a tray icon without handling TaskbarCreated.");
  process.exit(problems.length ? 1 : 0);
}
