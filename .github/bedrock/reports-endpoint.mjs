// Managed by bedrock. Do not edit in consumer repos: changes arrive through bedrock sync PRs.
//
// Step "reports endpoint is a base URL" of bedrock-repo-checks.yml (rule
// reports-endpoint-base-url). The report clients post to `<endpoint>/v1/report`, so
// `reports.endpoint` is the relay's base URL. keyfold set it to `.../v1/report`: every report
// went to `/v1/report/v1/report` and the relay answered 404, with CI green. This fails when the
// endpoint of an enabled `reports:` part, in .bedrock.yml or in bedrock/report/config.json
// (what the clients read), ends with /v1/report or is not https://. bedrock sync refuses the
// same values with the same function (endpointProblem). Lesson dooperdrew/bedrock#167.
// No dependencies: consumers may have no node_modules.
import { existsSync, readFileSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { partEnabled } from "./kit-parts-kept.mjs";
import { readKitConfig } from "./managed-via-sync.mjs";

export const CONFIG_JSON = "bedrock/report/config.json";

/** Why `endpoint` is not a valid relay base URL, or null when it is. */
export function endpointProblem(endpoint) {
  if (typeof endpoint !== "string" || endpoint.trim() === "")
    return "reports.endpoint is missing: set it to the relay's base URL, e.g. https://bedrock-relay.<account>.workers.dev";
  let url;
  try {
    url = new URL(endpoint.trim());
  } catch {
    return `reports.endpoint ${JSON.stringify(endpoint)} is not a URL: use the relay's base URL, e.g. https://bedrock-relay.<account>.workers.dev`;
  }
  if (url.protocol !== "https:")
    return `reports.endpoint ${JSON.stringify(endpoint)} must start with https://`;
  if (/\/v1\/report\/*$/i.test(url.pathname))
    return `reports.endpoint ${JSON.stringify(endpoint)} must be the relay's base URL, without /v1/report: the clients add that path themselves (use ${JSON.stringify(endpoint.trim().replace(/\/v1\/report\/*$/i, ""))})`;
  return null;
}

/**
 * The value of `key` in the `name:` part of .bedrock.yml, in block form (an own key on the
 * first indentation level under it) or one-line form `name: { key: value, ... }`; null if absent.
 */
export function partValue(text, name, key) {
  const unquote = (v) => v.trim().replace(/^(["'])(.*)\1$/, "$2");
  const lines = (text ?? "").replace(/\r/g, "").split("\n");
  const at = lines.findIndex((l) => new RegExp(`^${name}:`).test(l));
  if (at < 0) return null;
  const rest = lines[at]
    .slice(name.length + 1)
    .replace(/\s#.*$/, "")
    .trim();
  if (rest.startsWith("{")) {
    for (const pair of rest.replace(/^\{|\}$/g, "").split(",")) {
      const m = new RegExp(`^\\s*${key}:\\s*(.*)$`).exec(pair);
      if (m) return unquote(m[1]);
    }
    return null;
  }
  let indent = null;
  for (const l of lines.slice(at + 1)) {
    if (l.trim() === "" || l.trim().startsWith("#")) continue;
    if (!/^\s/.test(l)) break;
    const own = l.length - l.trimStart().length;
    indent ??= own;
    const m = new RegExp(`^\\s+${key}:\\s*(.*?)\\s*(\\s#.*)?$`).exec(l);
    if (own === indent && m) return unquote(m[1]);
  }
  return null;
}

/** Problems of a repo, from its .bedrock.yml text and its config.json text (or null). */
export function endpointProblems({ config, json }) {
  const problems = [];
  if (partEnabled(config, "reports")) {
    const p = endpointProblem(partValue(config, "reports", "endpoint"));
    if (p) problems.push({ file: ".bedrock.yml", message: p });
  }
  if (json != null) {
    let parsed;
    try {
      parsed = JSON.parse(json);
    } catch {
      problems.push({
        file: CONFIG_JSON,
        message: `${CONFIG_JSON} is not valid JSON: run bedrock sync`,
      });
    }
    if (parsed?.enabled === true) {
      const p = endpointProblem(parsed.endpoint);
      if (p)
        problems.push({
          file: CONFIG_JSON,
          message: `${p} (fix .bedrock.yml, then run bedrock sync)`,
        });
    }
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
  const json = existsSync(CONFIG_JSON) ? readFileSync(CONFIG_JSON, "utf8") : null;
  if (!partEnabled(config, "reports") && json == null) {
    console.log("The kit part reports is off: nothing to check.");
    process.exit(0);
  }
  const problems = endpointProblems({ config, json });
  for (const p of problems) console.log(`::error file=${p.file}::${p.message}`);
  if (!problems.length) console.log("reports.endpoint is the relay's https base URL.");
  process.exit(problems.length ? 1 : 0);
}
