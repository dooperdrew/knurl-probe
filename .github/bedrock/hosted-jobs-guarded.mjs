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
// label is not. Whether the expression also keeps Dependabot off the PC runner is checked only
// on bedrock's own workflows (tooling tests), not here.
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

/**
 * Jobs that can land on a GitHub-hosted runner without a guard: [{ job, line, label, expression }].
 * A literal hosted runs-on needs an if: on BEDROCK_HOSTED or BEDROCK_RUNNER; an expression with a
 * quoted hosted fallback needs an if: on BEDROCK_HOSTED.
 */
export function unguardedHostedJobs(text) {
  const lines = text.replace(/\r/g, "").split("\n");
  const start = lines.findIndex((l) => /^jobs:\s*(#.*)?$/.test(l));
  if (start === -1) return [];
  const found = [];
  const onPullRequests = runsOnPullRequests(text);
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
    for (let k = 0; k < body.length; k++) {
      const b = body[k];
      if (isBlank(b) || indentOf(b) !== keyIndent) continue;
      const m = /^\s*(runs-on|if):\s*(.*)$/.exec(b);
      if (!m) continue;
      // The value, with its continuation lines (if: >-, runs-on: as a block list).
      let value = m[2];
      for (
        let n = k + 1;
        n < body.length && (isBlank(body[n]) || indentOf(body[n]) > keyIndent);
        n++
      )
        if (!isBlank(body[n])) value += ` ${body[n].trim()}`;
      if (m[1] === "if") cond = value;
      else
        runsOn = { value: value.replace(/^\s*-\s*/, "").replace(/\s+-\s+/g, ","), line: i + 2 + k };
    }
    if (runsOn?.value.includes("${{")) {
      if (!onPullRequests) {
        i = end;
        continue;
      }
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
    i = end;
  }
  return found;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
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
    for (const j of unguardedHostedJobs(readFileSync(join(dir, f), "utf8"))) {
      if (j.expression) {
        console.log(
          `::error file=${dir}/${f},line=${j.line}::job "${j.job}" falls back to GitHub-hosted ${j.label} (Dependabot PRs, or no BEDROCK_RUNNER): without hosted minutes it fails without running. Give it "if: \${{ !((github.actor == 'dependabot[bot]' || github.event.pull_request.user.login == 'dependabot[bot]') && vars.BEDROCK_HOSTED == 'off') }}" (with an existing if:, join both with &&) so it is skipped (rule ci-dependabot-fallback-explicit)`,
        );
        failed = 1;
        continue;
      }
      console.log(
        `::error file=${dir}/${f},line=${j.line}::job "${j.job}" runs only on GitHub-hosted ${j.label}: without hosted minutes it fails without running. Add to its if: "vars.BEDROCK_HOSTED != 'off'" (or a BEDROCK_RUNNER condition) so it is skipped, and say in the PR what it did not verify`,
      );
      failed = 1;
    }
  }
  if (!failed)
    console.log(
      `${files.length} workflow(s): every GitHub-hosted-only job is skipped when hosted runners are off.`,
    );
  process.exit(failed);
}
