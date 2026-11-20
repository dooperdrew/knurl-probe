// Managed by bedrock. Do not edit in consumer repos: changes arrive through bedrock sync PRs.
//
// Step "local HTTP servers check the Host header" of bedrock-repo-checks.yml (rule
// local-server-checks-host). A server on 127.0.0.1 or the LAN feels private, but the
// browser of the same machine is a client any website can drive: with DNS rebinding a
// page on an attacker's domain reaches the server from loopback and passes an address
// check. Only its Host header (the attacker's domain) gives it away. Lesson
// dooperdrew/bedrock#111 (bedrock-hub served the Actions variable values of every repo).
//
// A tracked source file that creates an HTTP server and listens (createServer + .listen)
// must read the Host header, or say why not with the comment
// "bedrock: no host check: <reason>". Tests, fixtures and vendored folders are skipped.
// No dependencies: consumers may have no node_modules.
import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

const SOURCE = /\.(m|c)?(j|t)sx?$/;
const SKIP =
  /(^|\/)(node_modules|vendor|vendored|fixtures?|test|tests|__tests__|e2e|dist|build|\.next|coverage)\/|\.(test|spec)\.(m|c)?(j|t)sx?$/;
const SERVER = /\bcreateServer\s*\(/;
const LISTEN = /\.listen\s*\(/;
const READS_HOST =
  /headers\s*(\.\s*host\b|\[\s*["'`]host["'`]\s*\])|\.getHeader\s*\(\s*["'`]host["'`]\s*\)|headers\.get\s*\(\s*["'`]host["'`]\s*\)/i;
const WAIVER = /bedrock:\s*no host check:\s*\S/;

/** Problems of one file's text: null when it is fine or starts no server. */
export function hostProblem(text) {
  if (!SERVER.test(text) || !LISTEN.test(text)) return null;
  if (READS_HOST.test(text) || WAIVER.test(text)) return null;
  return "creates an HTTP server but never reads the Host header";
}

/** True for a test, a fixture, build output or a file in a vendored folder. */
export function skipped(file, vendored = []) {
  return SKIP.test(file) || vendored.some((d) => file.startsWith(d.replace(/\/+$/, "") + "/"));
}

/** Paths to check among tracked files, minus tests, fixtures and the vendored folders. */
export function candidates(files, vendored = []) {
  return files.filter((f) => SOURCE.test(f) && !skipped(f, vendored));
}

/** The `vendored:` list of .bedrock.yml, read without a YAML parser. */
export function vendoredOf(yml) {
  const lines = (yml ?? "").replace(/\r/g, "").split("\n");
  const i = lines.findIndex((l) => /^vendored:\s*(\[.*\])?\s*(#.*)?$/.test(l));
  if (i === -1) return [];
  const inline = lines[i].match(/\[(.*)\]/);
  if (inline)
    return inline[1]
      .split(",")
      .map((s) => s.trim().replace(/^['"]|['"]$/g, ""))
      .filter(Boolean);
  const out = [];
  for (const l of lines.slice(i + 1)) {
    const m = l.match(/^\s+-\s*(.+?)\s*(#.*)?$/);
    if (!m) break;
    out.push(m[1].replace(/^['"]|['"]$/g, ""));
  }
  return out;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  let files = [];
  try {
    files = execFileSync("git", ["ls-files"], { encoding: "utf8" }).split("\n").filter(Boolean);
  } catch {
    console.log("Not a git checkout: nothing to check.");
    process.exit(0);
  }
  let yml = null;
  try {
    yml = readFileSync(".bedrock.yml", "utf8");
  } catch {}
  let failed = 0;
  const list = candidates(files, vendoredOf(yml));
  for (const f of list) {
    let text;
    try {
      text = readFileSync(f, "utf8");
    } catch {
      continue;
    }
    const p = hostProblem(text);
    if (p) {
      console.log(
        `::error file=${f}::${p}. A page that uses DNS rebinding reaches it from the browser of this machine and passes an address check: answer only when Host is an IP literal, localhost or a configured name (see hostOk in bedrock's autopilot/board/server.mjs), or add the comment "bedrock: no host check: <reason>"`,
      );
      failed = 1;
    }
  }
  if (!failed)
    console.log(`${list.length} source file(s): every local HTTP server reads the Host header.`);
  process.exit(failed);
}
