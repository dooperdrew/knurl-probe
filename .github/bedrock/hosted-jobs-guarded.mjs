// Managed by bedrock. Do not edit in consumer repos: changes arrive through bedrock sync PRs.
//
// Step "hosted-only jobs skip when there are no hosted minutes" of bedrock-repo-checks.yml
// (rule ci-hosted-jobs-skip-without-minutes). A job pinned to a GitHub-hosted label
// (runs-on: windows-latest) fails after two seconds, red and with no word about the code,
// when the account has no Actions minutes. Its job-level if: must read one of the runner
// variables, BEDROCK_HOSTED (off = no hosted minutes) or BEDROCK_RUNNER (set = the PC
// runner), so that it is skipped instead. Lesson dooperdrew/bedrock#100.
//
// A runs-on expression (the BEDROCK_RUNNER pattern) that can still resolve to a quoted
// GitHub-hosted label, as the fallback for Dependabot PRs or when BEDROCK_RUNNER is unset,
// needs a job-level if: that reads vars.BEDROCK_HOSTED: BEDROCK_RUNNER alone does not stop a
// Dependabot PR from reaching the hosted fallback (rule ci-dependabot-fallback-explicit,
// lesson dooperdrew/bedrock#135); an if: that admits only PRs opened by the repo owner also
// counts (the match is loose, like GUARD: it looks for the variable, not its polarity). Only
// workflows Dependabot can start are checked for this (pull requests, a push not
// limited to named branches, reusable workflows), and an expression with no quoted hosted
// label is not. Neither is a job whose own if: requires a tag, a non-Dependabot branch ref or an
// event Dependabot never sends (release, schedule, workflow_dispatch, repository_dispatch):
// a release job in a workflow that also runs on PRs never starts for a Dependabot PR
// (lesson dooperdrew/bedrock#155, keyfold-probe's draft release job). Whether the expression also keeps Dependabot off the PC runner is checked only
// on bedrock's own workflows (tooling tests), not here. The guard tests the PR author
// (github.event.pull_request.user.login), never github.actor: zizmor's bot-conditions audit
// fails an if: on github.actor == 'dependabot[bot]' as spoofable, and this check reports one
// (lesson dooperdrew/bedrock#168).
//
// With --forks it is step "fork pull requests never reach the self-hosted runner" (rule
// ci-fork-prs-never-self-hosted, lesson dooperdrew/bedrock#173): in a workflow a pull request
// can start (pull_request*, workflow_call), a runs-on that can pick the PC (vars.BEDROCK_RUNNER,
// the runs-on input, a self-hosted label) must also test that the PR comes from this repo
// (head.repo.full_name, or an if: that admits only the owner's PRs), so a fork's npm test,
// build.rs or proc-macros never run on the owner's PC, also once the repo is public.
// No YAML parser (consumers may have no node_modules): jobs are read line by line.
import { readdirSync, readFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const HOSTED = /^(ubuntu|windows|macos)-[\w.-]+$/;
const GUARD = /\bvars\.BEDROCK_(HOSTED|RUNNER)\b/;
// A job that runs only for PRs opened by the repo owner never gets a Dependabot PR either.
const HOSTED_GUARD =
  /\bvars\.BEDROCK_HOSTED\b|\bpull_request\.user\.login\s*==\s*github\.repository_owner\b|\bgithub\.repository_owner\s*==\s*github\.event\.pull_request\.user\.login\b/;
const QUOTED_HOSTED = /['"]((?:ubuntu|windows|macos)-[\w.-]+)['"]/;
// github.actor (or triggering_actor) == 'x[bot]': anyone can trigger a run as that actor.
const SPOOFABLE_ACTOR =
  /\bgithub\.(triggering_)?actor\s*==\s*'[^']*\[bot\]'|'[^']*\[bot\]'\s*==\s*github\.(triggering_)?actor\b/i;
// A runs-on that can pick the PC: the runner variable, the runs-on input or a self-hosted label.
const SELF_HOSTED = /\bvars\.BEDROCK_RUNNER\b|\binputs\.runs-on\b|\bself-hosted\b/;
// Same-repository tests: the PR's head repo, or only PRs opened by the repo owner.
const SAME_REPO =
  /\bhead\.repo\.(full_name|fork)\b|\bpull_request\.user\.login\s*==\s*github\.repository_owner\b|\bgithub\.repository_owner\s*==\s*github\.event\.pull_request\.user\.login\b/;

/** The job-level if: bedrock suggests: Dependabot PRs (by PR author) and fork PRs skip without hosted minutes. */
export const SKIP_WITHOUT_MINUTES =
  "!((github.event.pull_request.user.login == 'dependabot[bot]' || (github.event.pull_request.head.repo.full_name || github.repository) != github.repository) && vars.BEDROCK_HOSTED == 'off')";
/** The same-repository test that keeps a fork PR off BEDROCK_RUNNER (true on pushes). */
export const SAME_REPO_TEST =
  "(github.event.pull_request.head.repo.full_name || github.repository) == github.repository";

const indentOf = (l) => l.length - l.trimStart().length;
const isBlank = (l) => l.trim() === "" || l.trim().startsWith("#");
const unquote = (v) =>
  v
    .replace(/\s+#.*$/, "")
    .trim()
    .replace(/^(['"])(.*)\1$/, "$2");

/** Hosted labels of a runs-on value: a plain label or a one-line list ([a, b]). */
function hostedLabels(value) {
  const v = unquote(value);
  if (v.includes("${{")) return [];
  const items = (v.startsWith("[") && v.endsWith("]") ? v.slice(1, -1).split(",") : [v]).map(
    unquote,
  );
  // A list with self-hosted never goes to GitHub's runners.
  if (items.includes("self-hosted")) return [];
  return items.filter((x) => HOSTED.test(x));
}

/**
 * True when Dependabot can start the workflow: on pull requests, on a push that is not limited
 * to named branches (Dependabot pushes dependabot/** branches), or as a reusable workflow
 * (a PR workflow may call it).
 */
export function runsOnPullRequests(text) {
  const lines = text.replace(/\r/g, "").split("\n");
  const start = lines.findIndex((l) => /^['"]?on['"]?:/.test(l));
  if (start === -1) return false;
  let block = lines[start].replace(/^['"]?on['"]?:/, "");
  for (let i = start + 1; i < lines.length && (isBlank(lines[i]) || indentOf(lines[i]) > 0); i++)
    if (!isBlank(lines[i])) block += `\n${lines[i]}`;
  block = block.replace(/#.*$/gm, "");
  if (/\b(pull_request\w*|workflow_call|workflow_run)\b/.test(block)) return true;
  if (!/\bpush\b/.test(block)) return false;
  // A push limited to branches (branches:) or to tags only never runs for dependabot/** branches.
  const push = /(^|\n)([ \t]*)push:(.*)((\n\2[ \t]+.*)*)/.exec(block);
  if (!push) return true; // on: push or on: [push, ...]
  const filter = push[3] + push[4];
  // branches: ['**'] or a dependabot/ pattern still matches Dependabot's branches.
  if (/\bbranches:/.test(filter)) return /['"]?\*\*['"]?|dependabot/.test(filter);
  return !/\btags:/.test(filter);
}

// Conditions that no Dependabot PR or dependabot/** push can satisfy. Not a branch ref such as
// 'refs/heads/main': under pull_request_target or workflow_run a Dependabot PR runs with the
// base or default branch as github.ref. Expressions compare strings ignoring case, hence /i.
const NOT_DEPENDABOT = [
  /^startsWith\(\s*github\.ref\s*,\s*'refs\/tags\/[^']*'\s*\)$/i,
  /^github\.ref_type\s*==\s*'tag'$/i,
  /^github\.ref\s*==\s*'refs\/tags\/[^']+'$/i,
  /^github\.event_name\s*==\s*'(release|schedule|workflow_dispatch|repository_dispatch)'$/i,
];

/** Splits an expression on a top-level operator (outside parentheses and quotes). */
function splitTop(expr, op) {
  const parts = [];
  let depth = 0;
  let quote = false;
  let last = 0;
  for (let i = 0; i < expr.length; i++) {
    const c = expr[i];
    if (c === "'") quote = !quote;
    if (quote) continue;
    if (c === "(") depth++;
    else if (c === ")") depth--;
    else if (depth === 0 && expr.startsWith(op, i)) {
      parts.push(expr.slice(last, i));
      last = i + op.length;
      i += op.length - 1;
    }
  }
  parts.push(expr.slice(last));
  return parts.map((p) => p.trim());
}

/** Drops parentheses that wrap the whole expression: "(a && b)" -> "a && b", not "(a) || (b)". */
function unwrap(expr) {
  let e = expr.trim();
  while (e.startsWith("(") && e.endsWith(")")) {
    let depth = 0;
    let closesAtEnd = true;
    for (let i = 0; i < e.length - 1; i++) {
      if (e[i] === "(") depth++;
      else if (e[i] === ")") depth--;
      if (depth === 0) {
        closesAtEnd = false;
        break;
      }
    }
    if (!closesAtEnd) break;
    e = e.slice(1, -1).trim();
  }
  return e;
}

/**
 * True when a job-level if: can never hold for a Dependabot PR: one of the conditions joined by
 * && (none of them under || or !) requires a tag, a branch ref other than dependabot/**, or an
 * event Dependabot does not send.
 */
export function unreachableForDependabot(cond) {
  const expr = unwrap(
    cond
      .replace(/\s+#.*$/gm, "")
      .trim()
      .replace(/^[>|][-+]?\s*/, "")
      .replace(/^"([\s\S]*)"$/, "$1")
      .trim()
      .replace(/^\$\{\{([\s\S]*)\}\}$/, "$1"),
  );
  if (!expr || splitTop(expr, "||").length > 1) return false;
  return splitTop(expr, "&&").some((part) => NOT_DEPENDABOT.some((r) => r.test(unwrap(part))));
}

/** The jobs of a workflow: [{ job, line (of the if:, else of the key), runsOn: { value, line } | null, cond }]. */
export function jobsOf(text) {
  const lines = text.replace(/\r/g, "").split("\n");
  const start = lines.findIndex((l) => /^jobs:\s*(#.*)?$/.test(l));
  if (start === -1) return [];
  const jobs = [];
  let jobIndent = -1;
  let i = start + 1;
  while (i < lines.length) {
    const l = lines[i];
    if (isBlank(l)) {
      i++;
      continue;
    }
    const ind = indentOf(l);
    if (ind === 0) break; // the next top-level key
    if (jobIndent === -1) jobIndent = ind;
    if (ind !== jobIndent || !/^\s*[\w.-]+:\s*(#.*)?$/.test(l)) {
      i++;
      continue;
    }
    const job = l.trim().replace(/:.*$/, "");
    // The job's block: every following line more indented than the job key.
    let end = i + 1;
    while (end < lines.length && (isBlank(lines[end]) || indentOf(lines[end]) > jobIndent)) end++;
    const body = lines.slice(i + 1, end);
    const keyIndent = Math.min(...body.filter((b) => !isBlank(b)).map(indentOf));
    let runsOn = null;
    let cond = "";
    let condLine = i + 1;
    for (let k = 0; k < body.length; k++) {
      const b = body[k];
      if (isBlank(b) || indentOf(b) !== keyIndent) continue;
      const m = /^\s*(runs-on|if):\s*(.*)$/.exec(b);
      if (!m) continue;
      // The value, with its continuation lines (if: >-, runs-on: as a block list), without
      // comments: a comment that names a guard is not one.
      const bare = (t) => t.replace(/\s+#.*$/, "");
      let value = bare(m[2]);
      for (
        let n = k + 1;
        n < body.length && (isBlank(body[n]) || indentOf(body[n]) > keyIndent);
        n++
      )
        if (!isBlank(body[n])) value += ` ${bare(body[n].trim())}`;
      if (m[1] === "if") {
        cond = value;
        condLine = i + 2 + k;
      } else
        runsOn = { value: value.replace(/^\s*-\s*/, "").replace(/\s+-\s+/g, ","), line: i + 2 + k };
    }
    jobs.push({ job, line: condLine, runsOn, cond });
    i = end;
  }
  return jobs;
}

/**
 * Jobs that can land on a GitHub-hosted runner without a guard: [{ job, line, label, expression }].
 * A literal hosted runs-on needs an if: on BEDROCK_HOSTED or BEDROCK_RUNNER; an expression with a
 * quoted hosted fallback needs an if: on BEDROCK_HOSTED. A job-level if: that tests github.actor
 * against a bot name is reported too (spoofable: zizmor bot-conditions fails it, lesson #168).
 */
export function unguardedHostedJobs(text) {
  const found = [];
  const onPullRequests = runsOnPullRequests(text);
  for (const { job, line, runsOn, cond } of jobsOf(text)) {
    if (SPOOFABLE_ACTOR.test(cond)) found.push({ job, line, spoofable: true });
    if (runsOn?.value.includes("${{")) {
      if (!onPullRequests || unreachableForDependabot(cond)) continue;
      const fallback = QUOTED_HOSTED.exec(runsOn.value);
      if (fallback && !HOSTED_GUARD.test(cond))
        found.push({ job, line: runsOn.line, label: fallback[1], expression: true });
    } else if (runsOn) {
      const v = runsOn.value.trim();
      const listed = /^[\w.-]+(,[\w.-]+)+$/.test(v) ? `[${v}]` : v;
      const labels = hostedLabels(listed);
      if (labels.length && !GUARD.test(cond))
        found.push({ job, line: runsOn.line, label: labels[0] });
    }
  }
  return found;
}

/** True when a pull request from a fork can start the workflow (pull_request*, workflow_call). */
export function runsOnForkPullRequests(text) {
  const lines = text.replace(/\r/g, "").split("\n");
  const start = lines.findIndex((l) => /^['"]?on['"]?:/.test(l));
  if (start === -1) return false;
  let block = lines[start].replace(/^['"]?on['"]?:/, "");
  for (let i = start + 1; i < lines.length && (isBlank(lines[i]) || indentOf(lines[i]) > 0); i++)
    if (!isBlank(lines[i])) block += `\n${lines[i]}`;
  return /\b(pull_request\w*|workflow_call)\b/.test(block.replace(/#.*$/gm, ""));
}

/**
 * Jobs that can run a fork's pull request on the self-hosted runner: [{ job, line }]. A runs-on
 * that can yield vars.BEDROCK_RUNNER (or the runs-on input, or a self-hosted label) needs a
 * same-repository test in runs-on or in the job's if: (lesson #173).
 */
export function forkReachableJobs(text) {
  if (!runsOnForkPullRequests(text)) return [];
  return jobsOf(text)
    .filter(({ runsOn, cond }) => {
      if (!runsOn || !SELF_HOSTED.test(runsOn.value)) return false;
      if (unreachableForDependabot(cond)) return false; // tags, schedules: never a fork PR
      return !SAME_REPO.test(runsOn.value) && !SAME_REPO.test(cond);
    })
    .map(({ job, runsOn }) => ({ job, line: runsOn.line }));
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const forks = process.argv.includes("--forks");
  const dir = ".github/workflows";
  let files = [];
  try {
    files = readdirSync(dir).filter((f) => /\.ya?ml$/.test(f));
  } catch {
    console.log("No .github/workflows folder: nothing to check.");
    process.exit(0);
  }
  let failed = 0;
  for (const f of files) {
    const text = readFileSync(join(dir, f), "utf8");
    if (forks) {
      for (const j of forkReachableJobs(text)) {
        console.log(
          `::error file=${dir}/${f},line=${j.line}::job "${j.job}" can run a fork's pull request on the self-hosted runner. Put "${SAME_REPO_TEST} &&" before vars.BEDROCK_RUNNER in its runs-on (or test head.repo.full_name in its if:), so a fork PR goes to a GitHub-hosted runner (rule ci-fork-prs-never-self-hosted)`,
        );
        failed = 1;
      }
      continue;
    }
    for (const j of unguardedHostedJobs(text)) {
      failed = 1;
      if (j.spoofable) {
        console.log(
          `::error file=${dir}/${f},line=${j.line}::job "${j.job}" tests github.actor against a bot in its if:, which anyone can spoof (zizmor bot-conditions). Test github.event.pull_request.user.login instead, as in "if: \${{ ${SKIP_WITHOUT_MINUTES} }}" (rule ci-dependabot-fallback-explicit)`,
        );
      } else if (j.expression) {
        console.log(
          `::error file=${dir}/${f},line=${j.line}::job "${j.job}" falls back to GitHub-hosted ${j.label} (Dependabot or fork PRs, or no BEDROCK_RUNNER): without hosted minutes it fails without running. Give it "if: \${{ ${SKIP_WITHOUT_MINUTES} }}" (with an existing if:, join both with &&) so it is skipped (rule ci-dependabot-fallback-explicit)`,
        );
      } else {
        console.log(
          `::error file=${dir}/${f},line=${j.line}::job "${j.job}" runs only on GitHub-hosted ${j.label}: without hosted minutes it fails without running. Add to its if: "vars.BEDROCK_HOSTED != 'off'" (or a BEDROCK_RUNNER condition) so it is skipped, and say in the PR what it did not verify`,
        );
      }
    }
  }
  if (!failed)
    console.log(
      forks
        ? `${files.length} workflow(s): no fork pull request can reach the self-hosted runner.`
        : `${files.length} workflow(s): every GitHub-hosted-only job is skipped when hosted runners are off.`,
    );
  // CI runs this script twice, without and with --forks; run by hand, a green first half read
  // as a green check (seen in keyfold's 0.28.1 sync, where only --forks failed).
  if (!forks && !process.env.GITHUB_ACTIONS)
    console.log(
      "Fork pull requests are checked separately: run it again with --forks, as CI does.",
    );
  process.exit(failed);
}
