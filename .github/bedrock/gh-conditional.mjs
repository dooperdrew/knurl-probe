// Managed by bedrock. Do not edit in consumer repos: changes arrive through bedrock sync PRs.
//
// Step "conditional gh api reads use --include" of bedrock-repo-checks.yml (rule
// github-api-304-unchanged). GitHub answers a conditional GET (If-None-Match with an ETag, or
// If-Modified-Since) with 304 Not Modified when nothing changed, and that answer does not count
// against the rate limit. `gh api` treats every status above 299 as a failure: on a 304 it exits 1
// and prints "gh: HTTP 304" on stderr, so a wrapper that throws unless exit 0 reports "nothing
// changed" as an error. With --include (-i) gh still prints the status line and the headers on
// stdout before exiting 1, and the code can read 304 from there. Lesson dooperdrew/bedrock#220
// (dooperdrew/bedrock-hub#50, the fast poll of the autopilot items).
//
// In code files (JavaScript, TypeScript, Python, shell, PowerShell, Ruby, Go, Rust, PHP; not
// node_modules, vendored folders or build output), a file that sends If-None-Match or
// If-Modified-Since on a line that is not a comment, and calls gh api (`gh api ...` in a command,
// or "gh" and "api" as quoted argv items anywhere in the file), must also pass --include, or -i
// as a quoted argv item or after `gh api` on the same line. The check works per file, not per call: a file with two
// gh calls passes when one has --include. Reading the status from the first stdout line (304 is
// "unchanged", a success with no body) and the test with a recorded 304 and exit 1 are checked in
// review. A file that does it another way says why, anywhere in it:
// "bedrock: conditional ok: <reason>". No dependencies: consumers may have no node_modules.
import { execFileSync } from "node:child_process";
import { existsSync, readFileSync, statSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

const CODE = /\.((m|c)?(j|t)sx?|py|sh|bash|ps1|psm1|rb|go|rs|php)$/i;
const SKIP = /(^|\/)(node_modules|vendor|vendored|dist|build|\.next|coverage|target|\.git)\//;
const HEADER = /If-(None-Match|Modified-Since)/i;
// `gh api` in a command string, or "gh" and "api" as quoted argv items anywhere in the file
// (execFile("gh", args) with args = ["api", ...] built earlier).
const GH_COMMAND = /\bgh\s+api\b/;
const GH_ARGV = [/["'\x60]gh["'\x60]/, /["'\x60]api["'\x60]/];
const INCLUDE = /--include\b|["'\x60]-i["'\x60]|\bgh\s+api\b[^\n]*\s-i\b/;
const WAIVER = /bedrock:\s*conditional ok:\s*[^\s<]/;
// A line that is only a comment (//, /*, * inside a block comment, #) does not count.
const COMMENT_LINE = /^\s*(\/\/|\/\*|\*|#)/;
const MAX_BYTES = 2 * 1024 * 1024;

/** Code files the check reads, among tracked or untracked paths. */
export function codeFiles(files) {
  return files.filter((f) => CODE.test(f) && !SKIP.test(f));
}

/** The text with comment-only lines blanked (line numbers kept). */
function codeLines(text) {
  return text
    .split("\n")
    .map((l) => (COMMENT_LINE.test(l) ? "" : l))
    .join("\n");
}

/** The line of the first conditional header sent to gh api without --include, or null. */
export function missingInclude(text) {
  if (WAIVER.test(text)) return null;
  const code = codeLines(text);
  const at = code.search(HEADER);
  const gh = GH_COMMAND.test(code) || GH_ARGV.every((re) => re.test(code));
  if (at === -1 || !gh || INCLUDE.test(code)) return null;
  return code.slice(0, at).split("\n").length;
}

/** Problems for a list of files: [{ file, line }]. read(path) returns the text or null. */
export function check(files, read) {
  const problems = [];
  for (const file of codeFiles(files)) {
    const text = read(file);
    if (text === null) continue;
    const line = missingInclude(text);
    if (line !== null) problems.push({ file, line });
  }
  return problems;
}

function readText(path) {
  if (!existsSync(path) || !statSync(path).isFile() || statSync(path).size > MAX_BYTES) return null;
  return readFileSync(path, "utf8");
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  let files = [];
  try {
    // Untracked files too, unless ignored: a run before git add checks what the PR will (#196).
    files = execFileSync("git", ["ls-files", "-co", "--exclude-standard", "-z"], {
      encoding: "utf8",
      maxBuffer: 64 * 1024 * 1024,
    })
      .split("\0")
      .filter(Boolean);
  } catch {
    console.log("Not a git checkout: nothing to check.");
    process.exit(0);
  }
  const problems = check(files, readText);
  for (const p of problems)
    console.log(
      `::error file=${p.file},line=${p.line}::a conditional request (If-None-Match or If-Modified-Since) goes through gh api without --include: on a 304 Not Modified gh exits 1 and prints only "gh: HTTP 304" on stderr, so "nothing changed" looks like an error. Pass --include and read the status from the first stdout line (304 = unchanged, no body), or add "bedrock: conditional ok:" followed by the reason`,
    );
  if (problems.length === 0) console.log("No conditional gh api call without --include.");
  process.exit(problems.length ? 1 : 0);
}
