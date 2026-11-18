// Managed by bedrock. Do not edit in consumer repos: changes arrive through bedrock sync PRs.
//
// Step "hosted-only jobs skip when there are no hosted minutes" of bedrock-repo-checks.yml
// (rule ci-hosted-jobs-skip-without-minutes). A job pinned to a GitHub-hosted label
// (runs-on: windows-latest) fails after two seconds, red and with no word about the code,
// when the account has no Actions minutes. Its job-level if: must read one of the runner
// variables, BEDROCK_HOSTED (off = no hosted minutes) or BEDROCK_RUNNER (set = the PC
// runner), so that it is skipped instead. Lesson dooperdrew/bedrock#100.
//
// Jobs whose runs-on is an expression (the BEDROCK_RUNNER pattern) are not checked here.
// No YAML parser (consumers may have no node_modules): jobs are read line by line.
import { readdirSync, readFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const HOSTED = /^(ubuntu|windows|macos)-[\w.-]+$/;
const GUARD = /\bvars\.BEDROCK_(HOSTED|RUNNER)\b/;

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

/** Jobs with a literal GitHub-hosted runs-on and no runner-variable guard: [{ job, line, label }]. */
export function unguardedHostedJobs(text) {
  const lines = text.replace(/\r/g, "").split("\n");
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
    if (runsOn) {
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
