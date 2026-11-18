// Managed by bedrock. Do not edit in consumer repos: changes arrive through bedrock sync PRs.
//
// Step "managed files change only in the bedrock sync PR" of bedrock-repo-checks.yml
// (rule repo-managed-files-via-sync). Feature PRs that ran `bedrock sync` themselves
// re-stamped bedrock-owned files and the kit version, so they overlapped the sync PR.
// On a pull request whose head branch is not bedrock/sync, this fails when the PR
// changes a path in the base's `managed:` list of .bedrock.yml, or the `bedrock:`
// version. A PR stacked on bedrock/sync is compared with that branch, so it passes.
// Lesson dooperdrew/bedrock#101.
//
// Environment: EVENT, HEAD_REF, PR_BASE, PR_HEAD (set by the workflow step).
import { execFileSync } from "node:child_process";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

export const SYNC_BRANCH = "bedrock/sync";

// A YAML scalar without its comment (a "#" after whitespace) and its quotes.
const scalar = (v) =>
  v
    .replace(/(^|\s)#.*$/, "")
    .trim()
    .replace(/^(['"])(.*)\1$/, "$2");

/** The `managed:` list (block or one-line) and the `bedrock:` value of a .bedrock.yml text. */
export function readKitConfig(text) {
  const lines = (text ?? "").replace(/\r/g, "").split("\n");
  const managed = [];
  let version = null;
  let inList = false;
  for (const l of lines) {
    const top = /^([\w-]+):(.*)$/.exec(l);
    if (top) {
      const value = scalar(top[2]);
      inList = top[1] === "managed" && value === "";
      if (top[1] === "bedrock") version = value || null;
      if (top[1] === "managed" && value.startsWith("["))
        managed.push(
          ...value
            .replace(/^\[|\]$/g, "")
            .split(",")
            .map(scalar)
            .filter(Boolean),
        );
      continue;
    }
    if (!inList) continue;
    // Block items, indented or at column 0 (both are valid YAML).
    const item = /^\s*-\s+(.+)$/.exec(l);
    if (item) managed.push(scalar(item[1]));
    else if (l.trim() && !l.trim().startsWith("#")) inList = false;
  }
  return { managed, version };
}

/** Problems of a PR: changed paths against the kit config of its base and its head. */
export function managedProblems({ headRef, changed, baseConfig, headConfig, sameRepo = true }) {
  if (headRef === SYNC_BRANCH && sameRepo) return [];
  const base = readKitConfig(baseConfig);
  const head = readKitConfig(headConfig);
  const problems = [];
  // The base's list and the head's: a feature PR that ran sync to turn a kit part on
  // adds new managed paths without moving the version.
  const all = [...new Set([...base.managed, ...head.managed])];
  const owned = new Set(all);
  const dirs = all.filter((p) => p.endsWith("/"));
  for (const f of changed) {
    if (f === ".bedrock.yml") continue;
    if (owned.has(f) || dirs.some((d) => f.startsWith(d)))
      problems.push(`${f} is managed by bedrock: it changes only in the ${SYNC_BRANCH} PR`);
  }
  if (base.version && head.version !== base.version)
    problems.push(
      `.bedrock.yml: the kit version moved from ${base.version} to ${head.version ?? "nothing"}; only the ${SYNC_BRANCH} PR sets it`,
    );
  if (base.managed.join("\n") !== head.managed.join("\n"))
    problems.push(`.bedrock.yml: the managed list changed; only the ${SYNC_BRANCH} PR writes it`);
  return problems;
}

function git(args) {
  try {
    return execFileSync("git", args, { encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] });
  } catch {
    return null;
  }
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const { EVENT, HEAD_REF, HEAD_REPO, REPO, PR_BASE, PR_HEAD } = process.env;
  if (EVENT !== "pull_request") {
    console.log("Not a pull request: nothing to check.");
    process.exit(0);
  }
  // Only this repo's own bedrock/sync branch, not a fork's branch with the same name.
  // An empty head repo (a deleted fork) is not this repo.
  const sameRepo = Boolean(HEAD_REPO && REPO && HEAD_REPO.toLowerCase() === REPO.toLowerCase());
  if (HEAD_REF === SYNC_BRANCH && sameRepo) {
    console.log(`${SYNC_BRANCH} PR: it may change managed files.`);
    process.exit(0);
  }
  // Compare with the point the branch started from, not the base's current tip: a branch
  // older than a merged sync PR has not changed the version itself.
  const fork = git(["merge-base", PR_BASE, PR_HEAD])?.trim();
  if (!fork) {
    console.log(
      `::error::cannot find the merge base of ${PR_BASE} and ${PR_HEAD} (checkout needs fetch-depth: 0)`,
    );
    process.exit(1);
  }
  const baseConfig = git(["show", `${fork}:.bedrock.yml`]);
  if (baseConfig === null) {
    console.log("No .bedrock.yml on the base: nothing to check.");
    process.exit(0);
  }
  const diff = git(["diff", "--name-only", "--no-renames", "-z", `${PR_BASE}...${PR_HEAD}`]);
  if (diff === null) {
    console.log(`::error::cannot compare ${PR_BASE}...${PR_HEAD} (checkout needs fetch-depth: 0)`);
    process.exit(1);
  }
  const changed = diff.split("\0").filter(Boolean);
  const headConfig = git(["show", `${PR_HEAD}:.bedrock.yml`]) ?? "";
  const problems = managedProblems({
    headRef: HEAD_REF,
    changed,
    baseConfig,
    headConfig,
    sameRepo,
  });
  for (const p of problems)
    console.log(
      `::error::${p}. Turn kit parts on through the sync PR, or stack this PR on ${SYNC_BRANCH}; never run bedrock sync in a feature PR`,
    );
  if (!problems.length)
    console.log(`${changed.length} changed file(s): no managed file and no kit version touched.`);
  process.exit(problems.length ? 1 : 0);
}
