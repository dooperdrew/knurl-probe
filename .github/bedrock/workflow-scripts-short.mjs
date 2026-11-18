// Managed by bedrock. Do not edit in consumer repos: changes arrive through bedrock sync PRs.
//
// Step "workflow run blocks keep long scripts in files" of bedrock-repo-checks.yml
// (rule workflow-scripts-in-files). actionlint passes every run: block to shellcheck: a
// long node or python heredoc inside one made it hang on the Windows runner until the
// job timed out. Lesson dooperdrew/bedrock#76.
//
// No YAML parser (consumers may have no node_modules): the heredocs are found line by line.
import { readdirSync, readFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

export const MAX_LINES = 30;
const INTERPRETERS =
  /\b(node|python3?|ruby|perl|pwsh|deno|bun|php)\b[^\n<]*<<-?\s*(['"]?)([A-Za-z_][\w-]*)\2/;

/** Heredocs of an interpreter longer than MAX_LINES: [{ line, lines, tool }]. */
export function longHeredocs(text) {
  const lines = text.replace(/\r/g, "").split("\n");
  const found = [];
  for (let i = 0; i < lines.length; i++) {
    const m = INTERPRETERS.exec(lines[i]);
    if (!m) continue;
    const end = lines.findIndex((l, j) => j > i && l.trim() === m[3]);
    const length = (end === -1 ? lines.length : end) - i - 1;
    if (length > MAX_LINES) found.push({ line: i + 1, lines: length, tool: m[1] });
    if (end !== -1) i = end;
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
    for (const h of longHeredocs(readFileSync(join(dir, f), "utf8"))) {
      console.log(
        `::error file=${dir}/${f},line=${h.line}::a ${h.lines}-line ${h.tool} script inside a run: block; move it to its own file and call it (more than ${MAX_LINES} lines makes actionlint slow or hang)`,
      );
      failed = 1;
    }
  }
  if (!failed)
    console.log(
      `${files.length} workflow(s): no interpreter script over ${MAX_LINES} lines inside run: blocks.`,
    );
  process.exit(failed);
}
