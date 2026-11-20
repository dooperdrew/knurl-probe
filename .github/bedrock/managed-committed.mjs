// Managed by bedrock. Do not edit in consumer repos: changes arrive through bedrock sync PRs.
//
// Step "managed files are committed, not ignored" of bedrock-repo-checks.yml (rule
// repo-managed-files-committed). In dallaterraallaluna `.gitignore` had `.claude/*`, so four
// managed files (the shared Claude settings and the SessionStart hook) were written by sync
// but never reached a commit: the sync PR silently dropped them and every fresh clone or cloud
// session ran without them, while `.bedrock.yml` listed them as installed.
// This fails when a path of the `managed:` list is excluded by the repo's ignore rules
// (`git check-ignore --no-index`, which also sees tracked files) or is not tracked at all.
// Lesson dooperdrew/bedrock#103.
import { execFileSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { readKitConfig } from "./managed-via-sync.mjs";

/** The parent folders of a path, outermost first, each with a trailing slash. */
export function parents(p) {
  const parts = p.replace(/\/$/, "").split("/").slice(0, -1);
  return parts.map((_, i) => `${parts.slice(0, i + 1).join("/")}/`);
}

/**
 * Problems of a managed list: `ignored` maps a path (or a parent folder, with a trailing
 * slash) to the ignore rule that excludes it ("source:line:pattern"); `tracked` is the set
 * of tracked paths. Git cannot re-include a file whose folder is excluded, so the advice
 * names the excluded parent folders first.
 */
export function committedProblems({ managed, ignored, tracked }) {
  const problems = [];
  for (const p of managed) {
    const isDir = p.endsWith("/");
    if (ignored.has(p)) {
      const fix = [...parents(p).filter((d) => ignored.has(d)), p].map((x) => `!${x}`).join(", ");
      problems.push(
        `${p} is managed by bedrock but ignored by ${ignored.get(p)}: add ${fix} after that line`,
      );
    } else if (isDir ? ![...tracked].some((t) => t.startsWith(p)) : !tracked.has(p))
      problems.push(`${p} is listed in managed but not committed: run bedrock sync and commit it`);
  }
  return problems;
}

/**
 * `git check-ignore -v -z` output: source NUL line NUL pattern NUL path NUL, four fields per
 * match. With -v git also prints paths whose last matching pattern is a negation ("!x"):
 * those are not ignored.
 */
export function parseCheckIgnore(out) {
  const f = out.split("\0");
  const map = new Map();
  for (let i = 0; i + 3 < f.length; i += 4) {
    const [source, line, pattern, path] = f.slice(i, i + 4);
    // Only the repo's own .gitignore files: .git/info/exclude and a global excludes file
    // belong to one machine, and a CI runner or a PC must not fail the check for them.
    if (!source || pattern.startsWith("!") || !/(^|\/)\.gitignore$/.test(source)) continue;
    map.set(path, `${source}:${line}:${pattern}`);
  }
  return map;
}

function git(args, input) {
  // check-ignore exits 1 when nothing matches: that is an answer, not a failure.
  try {
    return execFileSync("git", args, { input, encoding: "utf8", stdio: ["pipe", "pipe", "pipe"] });
  } catch (e) {
    if (e.status === 1 && args.includes("check-ignore")) return e.stdout ?? "";
    throw e;
  }
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  if (!existsSync(".bedrock.yml")) {
    console.log("No .bedrock.yml: nothing to check.");
    process.exit(0);
  }
  const { managed, version } = readKitConfig(readFileSync(".bedrock.yml", "utf8"));
  // bedrock itself has no kit version: it is the source of the kit, not a synced repo.
  if (!version || managed.length === 0) {
    console.log(
      "No synced kit in .bedrock.yml (no bedrock: version or managed list): nothing to check.",
    );
    process.exit(0);
  }
  // The managed paths and their parent folders, so the advice can name excluded folders.
  // A folder is asked twice. Without the trailing slash, git treats it as a folder only when
  // it exists on disk (not in CI, when its files were never committed). With the slash,
  // ".claude/*" would also match ".claude/" itself, so that answer counts only for a
  // folder-only pattern (one ending in "/", like ".claude/bedrock/").
  const dirs = [...new Set(managed.flatMap(parents))];
  const bare = dirs.map((d) => d.slice(0, -1));
  const input = [...managed, ...bare, ...dirs].map((p) => `${p}\0`).join("");
  const found = parseCheckIgnore(
    git(["-c", "core.excludesFile=", "check-ignore", "--no-index", "-v", "-z", "--stdin"], input),
  );
  const ignored = new Map();
  for (const [k, v] of found) {
    if (bare.includes(k)) ignored.set(`${k}/`, v);
    else if (dirs.includes(k)) {
      if (v.endsWith("/") && !ignored.has(k)) ignored.set(k, v);
    } else ignored.set(k, v);
  }
  const tracked = new Set(
    git(["-c", "core.quotepath=off", "ls-files", "-z"]).split("\0").filter(Boolean),
  );
  const problems = committedProblems({ managed, ignored, tracked });
  for (const p of problems) console.log(`::error file=.bedrock.yml::${p}`);
  if (!problems.length)
    console.log(`${managed.length} managed path(s): all committed, none ignored.`);
  process.exit(problems.length ? 1 : 0);
}
