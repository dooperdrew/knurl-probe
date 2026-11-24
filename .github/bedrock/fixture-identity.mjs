// Managed by bedrock. Do not edit in consumer repos: changes arrive through bedrock sync PRs.
//
// Step "fixtures hold no personal identity" of bedrock-repo-checks.yml (rule
// telemetry-fields-allowlisted). Claude Code's OpenTelemetry export puts the account email
// (user.email) and account ids on every metric point and log record; a raw capture saved as a test
// fixture keeps them in git for good. Lesson dooperdrew/bedrock#215 (dooperdrew/bedrock-hub#48:
// the raw OTLP captures held the email and had to be deleted).
//
// Fixture files are data files (.json, .jsonl, .ndjson, .har, .log, .txt, .yaml, .yml, .toml, .xml,
// .csv, .tsv, .eml, .snap, .http) under a fixtures, testdata, captures, snapshots, recordings or
// cassettes folder, or under test, tests, __tests__, e2e or spec, or with fixture, capture or
// recording in their name. Source code is not read here. A fixture fails when it holds:
// - an email address, except at a domain reserved for examples (example.com/.org/.net and their
//   subdomains, the .example, .test, .invalid and .localhost names), the GitHub noreply addresses
//   and the noreply, no-reply and git mailboxes (commit trailers, git@github.com);
// - user.account_uuid, user.account_id or user.id holding a UUID or a long hex id
//   (what Claude Code exports) that is not an obvious placeholder (one repeated digit per group);
//   the email under user.email is caught by the first point.
// The report scrubbers replace addresses with "<email>", which this check never matches. A file
// that keeps an address on purpose says why anywhere in it, as a comment or a JSON string:
// "bedrock: identity ok: <reason>". node_modules, vendored folders and build output are skipped.
// No dependencies: consumers may have no node_modules.
import { execFileSync } from "node:child_process";
import { closeSync, existsSync, openSync, readSync, statSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

const DATA = /\.(json|jsonl|ndjson|har|log|txt|ya?ml|toml|xml|csv|tsv|eml|snap|http)$/i;
const FIXTURE_DIR =
  /(^|\/)(fixtures?|__fixtures__|testdata|test-data|test_data|captures?|__snapshots__|snapshots|recordings|cassettes|tests?|__tests__|e2e|spec)\//i;
const FIXTURE_NAME = /(fixture|capture|recording)[^/]*$/i;
const SKIP = /(^|\/)(node_modules|vendor|vendored|dist|build|coverage|target|\.git)\//;
const WAIVER = /bedrock:\s*identity ok:\s*[^\s<"]/;
const EMAIL = /[A-Za-z0-9._%+-]+@((?:[A-Za-z0-9-]+\.)+([A-Za-z]{2,}))(?![A-Za-z0-9-])/g;
// A name ending like a file (icon@2x.png, pkg@scope.js) is not an address.
const FILE_TLD = new Set(
  "png jpg jpeg gif svg webp ico js mjs cjs ts tsx jsx json css html md txt yml yaml toml rs lock wasm map".split(
    " ",
  ),
);
const EXAMPLE_DOMAIN = /(^|\.)(example\.(com|org|net)|example|test|invalid|localhost)$/i;
const NOREPLY_DOMAIN = /(^|\.)noreply\.github\.com$/i;
const NOREPLY_BOX = /^(noreply|no-reply|git)$/i;
// The value of an identity key: flat ("user.id": "x", user.id = x) or OTLP
// ({"key": "user.id", "value": {"stringValue": "x"}}), on one line or several.
const ID_KEY =
  /user\.(account_uuid|account_id|id)(?![\w.])["']?\s*(?:[:=,]\s*)?(?:\{?\s*["']?value["']?\s*:\s*\{?\s*["']?string_?value["']?\s*:\s*)?["']?([0-9A-Fa-f][0-9A-Fa-f-]{15,})/gi;
const MAX_BYTES = 6 * 1024 * 1024;

/** Tracked or untracked data files that are test fixtures or captures. */
export function fixtureFiles(files) {
  return files.filter(
    (f) => DATA.test(f) && !SKIP.test(f) && (FIXTURE_DIR.test(f) || FIXTURE_NAME.test(f)),
  );
}

/** True for an address a fixture may keep: example domains, noreply addresses, git@. */
export function allowedEmail(address) {
  const [box, domain] = [
    address.slice(0, address.lastIndexOf("@")),
    address.slice(address.lastIndexOf("@") + 1),
  ];
  const tld = domain.split(".").at(-1).toLowerCase();
  return (
    FILE_TLD.has(tld) ||
    EXAMPLE_DOMAIN.test(domain) ||
    NOREPLY_DOMAIN.test(domain) ||
    NOREPLY_BOX.test(box)
  );
}

/** A UUID or hex id that is plainly a placeholder: every group one repeated character. */
function placeholderId(value) {
  const groups = value.split("-").filter(Boolean);
  return (
    groups.every((g) => new Set(g.toLowerCase()).size === 1) ||
    value === "123e4567-e89b-12d3-a456-426614174000"
  );
}

const lineAt = (text, index) => text.slice(0, index).split("\n").length;

/** What a fixture text holds that it should not: [{ line, what }], [] with the waiver. */
export function identityFindings(text) {
  if (WAIVER.test(text)) return [];
  const found = [];
  for (const m of text.matchAll(EMAIL)) {
    if (!allowedEmail(m[0])) found.push({ line: lineAt(text, m.index), what: "an email address" });
  }
  for (const m of text.matchAll(ID_KEY)) {
    const value = m[2].replace(/-+$/, "");
    const hex = value.replace(/-/g, "");
    const uuid = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i.test(value);
    if ((uuid || (!value.includes("-") && hex.length >= 32)) && !placeholderId(value))
      found.push({ line: lineAt(text, m.index), what: `a real-looking user.${m[1]}` });
  }
  return found.sort((a, b) => a.line - b.line);
}

/** Problems for a list of files: [{ file, line, what }]. read(path) returns text or null (binary). */
export function check(files, read) {
  const problems = [];
  for (const file of fixtureFiles(files)) {
    const text = read(file);
    if (text === null) continue;
    for (const f of identityFindings(text)) problems.push({ file, ...f });
  }
  return problems;
}

function readText(path) {
  if (!existsSync(path) || !statSync(path).isFile()) return null;
  const size = Math.min(statSync(path).size, MAX_BYTES);
  const buf = Buffer.alloc(size);
  const fd = openSync(path, "r");
  try {
    readSync(fd, buf, 0, size, 0);
  } finally {
    closeSync(fd);
  }
  return buf.includes(0) ? null : buf.toString("utf8");
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  // Untracked files too, unless ignored: a run before git add checks what the PR will (#196).
  const files = execFileSync("git", ["ls-files", "-co", "--exclude-standard", "-z"], {
    encoding: "utf8",
    maxBuffer: 64 * 1024 * 1024,
  })
    .split("\0")
    .filter(Boolean);
  const fixtures = fixtureFiles(files);
  const problems = check(files, readText);
  for (const p of problems) {
    console.log(
      `::error file=${p.file},line=${p.line}::this fixture holds ${p.what}: captures keep personal data in git for good. Replace it with a placeholder (someone@example.com, <email>), or add "bedrock: identity ok:" followed by the reason`,
    );
  }
  if (fixtures.length === 0) console.log("No fixture or capture files: nothing to check.");
  else if (problems.length === 0)
    console.log(`${fixtures.length} fixture file(s) hold no personal identity.`);
  if (problems.length) process.exit(1);
}
