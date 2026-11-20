// Managed by bedrock. Do not edit in consumer repos: changes arrive through bedrock sync PRs.
//
// Step "no committed link points outside the repo" of bedrock-repo-checks.yml (rule
// repo-no-outside-links). A symlink is a file to git, so "node_modules/" (directories only)
// does not ignore a node_modules link, and `git add -A` commits it. On a clean checkout the
// link dangles; on Windows without symlinks it becomes a text file that breaks `npm ci`.
// Lesson dooperdrew/bedrock#126.
//
// Fails on a committed symlink whose target is absolute or leaves the repo. Warns (does not
// fail) when a tracked .gitignore lists a dependency or build folder only with a trailing slash.
import { execFileSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import { posix, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const FOLDERS = ["node_modules", "target", ".venv", "vendor", "dist"];

/** True when a link at `path` with this target points outside the repo. */
export function leavesRepo(path, target) {
  const t = target.replace(/\\/g, "/");
  if (t.startsWith("/") || /^[A-Za-z]:/.test(t)) return true;
  const joined = posix.normalize(posix.join(posix.dirname(path), t));
  return joined === ".." || joined.startsWith("../");
}

/** Folder names .gitignore lists only with a trailing slash (so a link with that name is kept). */
export function slashOnlyFolders(gitignore) {
  const lines = gitignore
    .replace(/\r/g, "")
    .split("\n")
    .map((l) => l.trim());
  return FOLDERS.filter((name) => {
    const slash = lines.some((l) => l === `${name}/` || l === `/${name}/` || l === `**/${name}/`);
    const plain = lines.some((l) => l === name || l === `/${name}` || l === `**/${name}`);
    return slash && !plain;
  });
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const git = (args) => execFileSync("git", args, { encoding: "utf8", maxBuffer: 1 << 28 });
  let failed = 0;
  let links = 0;
  for (const line of git(["ls-files", "-s", "-z"]).split("\0")) {
    const m = /^120000 ([0-9a-f]+) \d+\t(.+)$/s.exec(line);
    if (!m) continue;
    links++;
    const target = git(["cat-file", "-p", m[1]]);
    if (leavesRepo(m[2], target)) {
      console.log(
        `::error file=${m[2]}::committed symlink points outside the repo (${target}). Remove it from git (git rm --cached ${m[2]}) and ignore the name without a trailing slash`,
      );
      failed = 1;
    }
  }
  // Every tracked .gitignore, the root one first (a site folder often has its own).
  const ignores = git(["ls-files", "-z", "--", ".gitignore", "*/.gitignore"])
    .split("\0")
    .filter(Boolean);
  if (existsSync(".gitignore") && !ignores.includes(".gitignore")) ignores.unshift(".gitignore");
  for (const file of ignores) {
    if (!existsSync(file)) continue;
    for (const name of slashOnlyFolders(readFileSync(file, "utf8")))
      console.log(
        `::warning file=${file}::"${name}/" ignores only a folder: a ${name} symlink is still committed by git add -A. Write "${name}" without the slash`,
      );
  }
  if (!failed) console.log(`${links} committed symlink(s): none points outside the repo.`);
  process.exit(failed);
}
