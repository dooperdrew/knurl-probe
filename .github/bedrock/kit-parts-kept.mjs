// Managed by bedrock. Do not edit in consumer repos: changes arrive through bedrock sync PRs.
//
// Step "optional kit parts match .bedrock.yml" of bedrock-repo-checks.yml (rule
// repo-kit-parts-kept). In dognutcorner-v2 a merge of main resolved a conflict in .bedrock.yml
// by taking main's side: the `reports:` block and its three managed paths were dropped, while
// the files stayed in the tree, unmanaged, with CI green. This fails when:
// - a tracked file of an optional part is not in `managed`, or its part is not enabled;
// - a part is enabled but none of its files is managed (sync was never run, or was undone).
// PARTS must equal the `optional:` table of bedrock's kit.yml (a bedrock test compares them),
// since consumers have no kit.yml. Lesson dooperdrew/bedrock#109.
import { execFileSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { readKitConfig } from "./managed-via-sync.mjs";

/** Every file an optional part can install (any stack, any setting), and its config file. */
export const PARTS = {
  autopilot: [
    ".github/workflows/bedrock-autopilot.yml",
    ".claude/bedrock/autopilot/autopilot.mjs",
    ".claude/bedrock/autopilot/agents/scout.md",
    ".claude/bedrock/autopilot/agents/fixer.md",
    ".claude/bedrock/autopilot/agents/healer.md",
    ".claude/bedrock/autopilot/agents/merger.md",
    ".claude/bedrock/autopilot/agents/verifier.md",
    ".claude/bedrock/autopilot/agents/notifier.md",
    ".claude/bedrock/autopilot/agents/optimizer.md",
    ".claude/bedrock/autopilot/config.json",
  ],
  reports: [
    "bedrock/report/node/package.json",
    "bedrock/report/node/report.ts",
    "bedrock/report/rust/Cargo.toml",
    "bedrock/report/rust/src/lib.rs",
    "bedrock/report/wordpress/bedrock-report.php",
    "bedrock/report/web/report.js",
    "bedrock/report/config.json",
  ],
};

/**
 * Whether `.bedrock.yml` turns a part on: `name:` followed by an indented `enabled: true`,
 * or a one-line mapping `name: { enabled: true, ... }`.
 */
export function partEnabled(text, name) {
  const lines = (text ?? "").replace(/\r/g, "").split("\n");
  const at = lines.findIndex((l) => new RegExp(`^${name}:`).test(l));
  if (at < 0) return false;
  const rest = lines[at]
    .slice(name.length + 1)
    .replace(/\s#.*$/, "")
    .trim();
  // YAML reads true, True and TRUE as the same boolean.
  if (rest.startsWith("{")) return /(^|[{,\s])enabled:\s*(true|True|TRUE)\s*(,|}|$)/.test(rest);
  if (rest !== "") return false;
  // Only the part's own keys: the first indentation level under it, not a nested mapping.
  let indent = null;
  for (const l of lines.slice(at + 1)) {
    if (l.trim() === "" || l.trim().startsWith("#")) continue;
    if (!/^\s/.test(l)) break;
    const own = l.length - l.trimStart().length;
    indent ??= own;
    if (own === indent && /^\s+enabled:\s*(true|True|TRUE)\s*(#.*)?$/.test(l)) return true;
  }
  return false;
}

/** Problems of a repo: its .bedrock.yml text and its tracked files. */
export function partProblems({ config, tracked, parts = PARTS }) {
  const managed = new Set(readKitConfig(config).managed);
  const problems = [];
  for (const [name, files] of Object.entries(parts)) {
    const enabled = partEnabled(config, name);
    for (const f of files.filter((x) => tracked.has(x))) {
      if (!enabled)
        problems.push(
          `${f} belongs to the kit part "${name}", which .bedrock.yml does not enable: restore "${name}: { enabled: true, ... }" (a merge may have dropped it) or remove the part with bedrock sync`,
        );
      else if (!managed.has(f))
        problems.push(
          `${f} belongs to the kit part "${name}" but is not in managed: run bedrock sync (a merge may have dropped it)`,
        );
    }
    if (enabled && !files.some((f) => managed.has(f)))
      problems.push(
        `the kit part "${name}" is enabled but none of its files is managed: run bedrock sync`,
      );
  }
  return problems;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  if (!existsSync(".bedrock.yml")) {
    console.log("No .bedrock.yml: nothing to check.");
    process.exit(0);
  }
  const config = readFileSync(".bedrock.yml", "utf8");
  // bedrock itself has no kit version: it holds the parts' sources, not installed copies.
  if (!readKitConfig(config).version) {
    console.log("No bedrock: version in .bedrock.yml (not a synced repo): nothing to check.");
    process.exit(0);
  }
  const out = execFileSync("git", ["-c", "core.quotepath=off", "ls-files", "-z"], {
    encoding: "utf8",
  });
  const tracked = new Set(out.split("\0").filter(Boolean));
  const problems = partProblems({ config, tracked });
  for (const p of problems) console.log(`::error file=.bedrock.yml::${p}`);
  if (!problems.length) {
    const on = Object.keys(PARTS).filter((n) => partEnabled(config, n));
    console.log(
      `Optional kit parts enabled: ${on.length ? on.join(", ") : "none"}; their files match .bedrock.yml.`,
    );
  }
  process.exit(problems.length ? 1 : 0);
}
