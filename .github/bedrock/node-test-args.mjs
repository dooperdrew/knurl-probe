// Managed by bedrock. Do not edit in consumer repos: changes arrive through bedrock sync PRs.
//
// Step "node --test gets files or globs, not folders" of bedrock-repo-checks.yml (rule
// node-test-no-bare-folder). Since Node 21 the arguments of `node --test` are files or
// glob patterns: a folder is no longer searched, so the tests stop running after a Node
// upgrade. Lesson dooperdrew/bedrock#77.
import { execFileSync } from "node:child_process";
import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

/** Unquoted arguments of every `node --test` in a command line (flags left out). */
export function testArgs(command) {
  const out = [];
  for (const m of command.matchAll(/\bnode\s+(?:--[\w-]+(?:=\S+)?\s+)*--test\b([^&|;\n]*)/g)) {
    const tokens = m[1].match(/"[^"]*"|'[^']*'|\S+/g) ?? [];
    let skip = false;
    for (const t of tokens) {
      if (skip) {
        skip = false;
        continue;
      }
      if (t.startsWith("--")) {
        // --test-reporter spec, --test-name-pattern x: the next token is the flag's value.
        if (
          !t.includes("=") &&
          /^--test-(reporter|reporter-destination|name-pattern|skip-pattern|concurrency|timeout|shard)$/.test(
            t,
          )
        )
          skip = true;
        continue;
      }
      if (/^["']/.test(t)) continue; // a quoted glob or path: fine
      out.push(t);
    }
  }
  return out;
}

const isDir = (p) => existsSync(p) && statSync(p).isDirectory();

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const problems = [];
  // Untracked files too, unless ignored: a run before git add checks what the PR will (#196).
  const tracked = execFileSync(
    "git",
    ["ls-files", "-co", "--exclude-standard", "-z", "--", "package.json", "**/package.json"],
    {
      encoding: "utf8",
    },
  )
    .split("\0")
    .filter((f) => f && !f.includes("node_modules/"));
  for (const file of tracked) {
    let scripts = {};
    try {
      scripts = JSON.parse(readFileSync(file, "utf8")).scripts ?? {};
    } catch {
      continue;
    }
    for (const [name, command] of Object.entries(scripts)) {
      for (const arg of testArgs(String(command))) {
        if (isDir(join(dirname(file), arg)))
          problems.push(`${file}: script "${name}" passes the folder "${arg}" to node --test`);
      }
    }
  }
  const wf = ".github/workflows";
  for (const f of existsSync(wf) ? readdirSync(wf).filter((x) => /\.ya?ml$/.test(x)) : []) {
    for (const arg of testArgs(readFileSync(join(wf, f), "utf8"))) {
      if (isDir(arg)) problems.push(`${wf}/${f}: passes the folder "${arg}" to node --test`);
    }
  }
  for (const p of problems) {
    console.log(
      `::error::${p}; give files or a quoted glob, e.g. node --test "${p.match(/"([^"]+)" to node/)?.[1] ?? "test"}/**/*.test.mjs"`,
    );
  }
  if (problems.length === 0) console.log("No node --test call gets a bare folder.");
  process.exit(problems.length ? 1 : 0);
}
