// Managed by bedrock. Do not edit in consumer repos: changes arrive through bedrock sync PRs.
//
// Step "tests do not pin the kit version" of bedrock-repo-checks.yml (rule
// repo-tests-no-kit-version). bedrock sync rewrites `bedrock:` in .bedrock.yml on every
// release, and the sync PR may touch only managed files: a test that reads the live
// .bedrock.yml and asserts the version written there passes on main and fails on exactly
// the PR that cannot fix it. Lesson dooperdrew/bedrock#163 (bedrock-hub's
// src/kit-config.test.ts matched `bedrock: 0\.21\.0` and kept the 0.22.1 sync PR red).
//
// A tracked JavaScript or TypeScript test file (*.test.*, *.spec.*, or under test/, tests/,
// __tests__/, e2e/) that reads the live .bedrock.yml (a string that is exactly
// ".bedrock.yml", "./.bedrock.yml" or "../.bedrock.yml") must not have a line with `bedrock:`
// and the version .bedrock.yml holds now, plain or regex-escaped (0.21.0, 0\.21\.0).
// A file that also defines an inline .bedrock.yml with that same version is not reported (its
// assertions may be about the fixture). Compare with the version read from the file instead, or
// use an inline fixture. A file that
// keeps the literal on purpose says why with the comment "bedrock: kit version ok: <reason>".
// Skipped on the bedrock/sync branch (see below). No dependencies: consumers may have no
// node_modules.
import { execFileSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

const SOURCE = /\.(m|c)?(j|t)sx?$/;
const TEST = /(^|\/)(test|tests|__tests__|e2e)\/|\.(test|spec)\.(m|c)?(j|t)sx?$/;
const SKIP = /(^|\/)(node_modules|vendor|vendored|dist|build|coverage)\//;
const WAIVER = /bedrock:\s*kit version ok:\s*[^\s<]/;

/** The kit version in a .bedrock.yml text (`bedrock: 0.22.1`), or null. */
export function kitVersion(yml) {
  const m = /^\uFEFF?bedrock:\s*["']?v?(\d+\.\d+\.\d+)["']?\s*(#.*)?$/m.exec(yml);
  return m ? m[1] : null;
}

/** Tracked test sources, without node_modules, vendored folders and build output. */
export function testFiles(tracked) {
  return tracked.filter((f) => SOURCE.test(f) && TEST.test(f) && !SKIP.test(f));
}

/**
 * True when a text reads the repo's own .bedrock.yml: a string that is exactly the file name,
 * optionally behind ./ or ../ (readFileSync(".bedrock.yml"), new URL("../.bedrock.yml", …),
 * join(root, ".bedrock.yml")). A longer path such as "repos/o/r/contents/.bedrock.yml" is a
 * fixture key, not the live file.
 */
export function readsLiveFile(text) {
  return /["'\x60](?:\.\.?\/)*\.bedrock\.yml["'\x60]/.test(text);
}

/**
 * Line numbers (1-based) where a test text pins `version`, or [] when it does not read the
 * live .bedrock.yml, does not pin it, or carries the waiver. A line pins the version when it
 * has `bedrock:` and the version with its dots plain or escaped by one or two backslashes,
 * not inside a longer number.
 */
export function pinnedLines(text, version) {
  if (!version || !readsLiveFile(text) || WAIVER.test(text)) return [];
  const [a, b, c] = version.split(".");
  // The file also writes an inline .bedrock.yml with this version (a template-literal line
  // `bedrock: 0.20.3`, or "...\nbedrock: 0.20.3\n"): its assertions may be about that fixture,
  // so the file is ambiguous and not reported.
  const fixture = new RegExp(String.raw`(^\s*|\\n)bedrock:\s*${a}\.${b}\.${c}(\\n|\s*$)`, "m");
  if (fixture.test(text)) return [];
  const dot = String.raw`\\{0,2}\.`;
  const re = new RegExp(String.raw`bedrock:.*(?<![\d.])${a}${dot}${b}${dot}${c}(?![\d])`);
  const lines = [];
  text.split(/\r?\n/).forEach((line, i) => {
    if (re.test(line)) lines.push(i + 1);
  });
  return lines;
}

/** Problems for a list of tracked files: [{ file, line }]. read(path) returns a file's text. */
export function check(tracked, read) {
  if (!tracked.includes(".bedrock.yml")) return { version: null, problems: [] };
  const version = kitVersion(read(".bedrock.yml"));
  const problems = [];
  if (!version) return { version, problems };
  for (const file of testFiles(tracked)) {
    let text;
    try {
      text = read(file);
    } catch {
      continue;
    }
    for (const line of pinnedLines(text, version)) problems.push({ file, line });
  }
  return { version, problems };
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  // On the sync PR the live version has already changed: a pin no longer matches it (the test
  // fails there instead), and an inline fixture that happens to hold the new version must not
  // block the one PR that cannot change project files. The pin is caught on every other branch.
  if (process.env.GITHUB_HEAD_REF === "bedrock/sync") {
    console.log("bedrock/sync PR: not checked here (the version just changed); checked on main.");
    process.exit(0);
  }
  // Untracked files too, unless ignored: a run before git add checks what the PR will (#196).
  const tracked = execFileSync("git", ["ls-files", "-co", "--exclude-standard", "-z"], {
    encoding: "utf8",
  })
    .split("\0")
    .filter(Boolean);
  const { version, problems } = check(tracked, (p) =>
    existsSync(p) ? readFileSync(p, "utf8") : "",
  );
  if (!version) console.log("No kit version in .bedrock.yml: nothing to check.");
  for (const p of problems) {
    console.log(
      `::error file=${p.file},line=${p.line}::this test reads .bedrock.yml and pins the kit version ${version}, which bedrock sync rewrites: the next sync PR will fail here. Compare with the version read from the file, use an inline fixture, or add the comment "bedrock: kit version ok:" followed by the reason`,
    );
  }
  if (version && problems.length === 0) console.log(`No test pins the kit version ${version}.`);
  if (problems.length) process.exit(1);
}
