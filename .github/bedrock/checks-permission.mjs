// Managed by bedrock. Do not edit in consumer repos: changes arrive through bedrock sync PRs.
//
// Step "check-run permissions" of bedrock-repo-checks.yml
// (rule actions-checks-permission). With an explicit permissions: block every scope that is
// not listed is none, and contents: read does not cover check runs or commit statuses: a
// job that calls those APIs fails in Actions with HTTP 403 "Resource not accessible by
// integration", while the same script passes on a developer's own gh login.
// Lesson dooperdrew/bedrock#122.
//
// For each job of .github/workflows/*.yml whose permissions are explicit (job-level, or
// workflow-level when the job has none), the text of the job and of the repo scripts its
// commands run (node/bash/sh/pwsh/python <file>, one level deep) is searched for:
//   check-runs, check-suites, gh pr checks, github(.rest).checks.*  -> checks: read (or write)
//   /statuses, commits/<sha>/status, getCombinedStatusForRef,
//   listCommitStatusesForRef                                         -> statuses: read (or write)
//   statusCheckRollup (gh pr view --json, GraphQL)                   -> both
// gh pr checks also reads statusCheckRollup; which scopes GitHub enforces there is not
// verified, so it asks only for checks. Calls that write (create a check run or a status)
// need write, but read passes here: the rule is about reading.
// Step names and trailing " # " comments of the workflow, whole-line comments of the called
// scripts and bedrock's own .github/bedrock scripts are not searched.
// read-all and write-all cover both. Jobs with no permissions anywhere use the repo's
// default token and are not checked here; zizmor asks for explicit permissions anyway.
// No YAML parser (consumers may have no node_modules): jobs are read line by line.
import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const NEEDS = [
  {
    scope: "checks",
    what: "the check-runs API",
    re: /check-runs|check-suites|\bgh\s+pr\s+checks\b|statusCheckRollup|\bgithub\.(?:rest\.)?checks\.\w+/,
  },
  {
    scope: "statuses",
    what: "the commit-status API",
    re: /(?:repos\/[^/\s]+\/[^/\s]+|commits\/(?:\$\{\{[^}]*\}\}|\$\{[^}]*\}|[^\s/])+)\/status(?:es)?\b|getCombinedStatusForRef|listCommitStatusesForRef|statusCheckRollup/,
  },
];

const indentOf = (l) => l.length - l.trimStart().length;
const isBlank = (l) => l.trim() === "" || l.trim().startsWith("#");
const stripComment = (v) => v.replace(/\s+#.*$/, "").trim();
const unquote = (v) => v.trim().replace(/^(['"])(.*)\1$/, "$2");

/**
 * A permissions value: its inline part (after "permissions:") and the indented lines below it.
 * Returns { all: "read" | "write" | null, scopes: { name: level } }.
 */
export function parsePermissions(inline, block = []) {
  const perms = { all: null, scopes: {} };
  const v = stripComment(inline).replace(/^(['"])(.*)\1$/, "$2");
  if (v === "read-all") perms.all = "read";
  else if (v === "write-all") perms.all = "write";
  else if (v.startsWith("{")) {
    for (const part of v.replace(/^\{|\}$/g, "").split(",")) {
      const m = /^\s*([\w-]+)\s*:\s*(.+?)\s*$/.exec(part);
      if (m) perms.scopes[m[1]] = unquote(m[2]);
    }
  }
  for (const l of block) {
    if (isBlank(l)) continue;
    const m = /^\s*([\w-]+)\s*:\s*(.+?),?$/.exec(stripComment(l));
    if (m) perms.scopes[m[1]] = unquote(m[2]);
  }
  return perms;
}

const grants = (perms, scope) =>
  perms.all !== null || ["read", "write"].includes(perms.scopes[scope]);

/** The value of a key at `indent` starting at lines[k]: [inline, continuation lines]. */
function valueAt(lines, k, indent) {
  const inline = lines[k].replace(/^\s*[\w-]+:\s?/, "");
  const block = [];
  for (let n = k + 1; n < lines.length && (isBlank(lines[n]) || indentOf(lines[n]) > indent); n++)
    block.push(lines[n]);
  return [inline, block];
}

/** A script's text without whole-line comments (#, //, block comment lines). */
const withoutComments = (t) =>
  t
    .split("\n")
    .filter((l) => !/^\s*(#|\/\/|\/\*|\*)/.test(l))
    .join("\n");

/** Repo scripts that a job's text runs: paths relative to the repo root. */
export function calledScripts(text) {
  const out = new Set();
  const re =
    /(?:^|[\s;&|(])(?:node|bash|sh|pwsh|powershell|python3?)\s+(?:-{1,2}[\w-]+(?:=\S+)?\s+)*["']?(?:\$\{?GITHUB_WORKSPACE\}?\/|\.\/)?([\w./-]+\.(?:mjs|cjs|js|sh|bash|ps1|py))\b/g;
  for (const m of text.matchAll(re)) out.add(m[1].replace(/^\.\//, ""));
  // bedrock's own check scripts name APIs in their comments and patterns; none calls them.
  for (const p of out) if (p.startsWith(".github/bedrock/")) out.delete(p);
  return [...out];
}

/**
 * Jobs that call the check-runs or commit-status APIs without the scope:
 * [{ job, line, scope, what, where }]. `readScript(path)` returns a repo file's text or null.
 */
export function missingCheckPermissions(text, readScript = () => null) {
  const lines = text.replace(/\r/g, "").split("\n");
  let workflowPerms = null;
  for (let k = 0; k < lines.length; k++)
    if (/^permissions:/.test(lines[k])) workflowPerms = parsePermissions(...valueAt(lines, k, 0));
  const start = lines.findIndex((l) => /^jobs:\s*(#.*)?$/.test(l));
  if (start === -1) return [];
  const found = [];
  let jobIndent = -1;
  let i = start + 1;
  while (i < lines.length) {
    const l = lines[i];
    if (isBlank(l)) {
      i++;
      continue;
    }
    const ind = indentOf(l);
    if (ind === 0) break;
    if (jobIndent === -1) jobIndent = ind;
    if (ind !== jobIndent || !/^\s*[\w.-]+:\s*(#.*)?$/.test(l)) {
      i++;
      continue;
    }
    const job = l.trim().replace(/:.*$/, "");
    let end = i + 1;
    while (end < lines.length && (isBlank(lines[end]) || indentOf(lines[end]) > jobIndent)) end++;
    const body = lines.slice(i + 1, end);
    const keyIndent = Math.min(...body.filter((b) => !isBlank(b)).map(indentOf));
    let perms = workflowPerms;
    let permLine = i + 1;
    let reusable = false;
    for (let k = 0; k < body.length; k++) {
      if (isBlank(body[k]) || indentOf(body[k]) !== keyIndent) continue;
      if (/^\s*permissions:/.test(body[k])) {
        perms = parsePermissions(...valueAt(body, k, keyIndent));
        permLine = i + 2 + k;
      }
      // A job that calls a reusable workflow: its steps are in the called file, which this
      // check reads on its own (and only when that file has permissions of its own).
      if (/^\s*uses:/.test(body[k])) reusable = true;
    }
    if (perms && !reusable) {
      const own = body.filter((b) => !isBlank(b)).join("\n");
      // Step names and trailing comments only talk about the APIs.
      const code = body
        .filter((b) => !isBlank(b) && !/^\s*(-\s+)?name:/.test(b))
        .map((b) => b.replace(/\s+#.*$/, ""))
        .join("\n");
      const sources = [{ where: "its steps", text: code }];
      for (const p of calledScripts(own)) {
        const t = readScript(p);
        if (t !== null) sources.push({ where: p, text: withoutComments(t) });
      }
      for (const need of NEEDS) {
        if (grants(perms, need.scope)) continue;
        const hit = sources.find((s) => need.re.test(s.text));
        if (hit)
          found.push({ job, line: permLine, scope: need.scope, what: need.what, where: hit.where });
      }
    }
    i = end;
  }
  return found;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const root = process.cwd();
  const dir = ".github/workflows";
  let files = [];
  try {
    files = readdirSync(dir).filter((f) => /\.ya?ml$/.test(f));
  } catch {
    console.log("No .github/workflows folder: nothing to check.");
    process.exit(0);
  }
  const readScript = (p) => {
    const full = resolve(root, p);
    const rel = relative(root, full);
    if (rel.startsWith("..") || !existsSync(full) || !statSync(full).isFile()) return null;
    return readFileSync(full, "utf8");
  };
  let failed = 0;
  for (const f of files) {
    for (const j of missingCheckPermissions(readFileSync(join(dir, f), "utf8"), readScript)) {
      console.log(
        `::error file=${dir}/${f},line=${j.line}::job "${j.job}" calls ${j.what} (in ${j.where}) but its permissions do not list ${j.scope}: read. With explicit permissions every unlisted scope is none, and the call fails in Actions with HTTP 403 "Resource not accessible by integration"`,
      );
      failed = 1;
    }
  }
  if (!failed)
    console.log(
      `${files.length} workflow(s): every job that reads check runs or commit statuses has the permission.`,
    );
  process.exit(failed);
}
