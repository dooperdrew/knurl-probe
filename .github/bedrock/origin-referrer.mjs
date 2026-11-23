// Managed by bedrock. Do not edit in consumer repos: changes arrive through bedrock sync PRs.
//
// Step "no-referrer does not break Origin checks" of
// bedrock-repo-checks.yml (rule web-origin-check-keeps-referrer). Browsers tie the Origin
// header of a form POST to the referrer policy: under no-referrer a same-origin form arrives
// with "Origin: null", so a server that compares Origin with Host as its CSRF defence refuses
// every legitimate write. Lesson dooperdrew/bedrock#141 (bedrock-hub's first writes).
//
// When a tracked source reads the Origin header, no tracked source or page may set
// Referrer-Policy (or a referrer meta tag) to exactly "no-referrer", unless that file says
// why with the comment "bedrock: no-referrer ok: <reason>". The header and the check often
// live in different files, so the whole repo is read. Tests, fixtures and vendored folders
// are skipped, as in the Host check. Not seen: a framework default (helmet sends no-referrer
// unless configured) and policies set outside the code (proxy, CDN): those are checked in review.
// No dependencies: consumers may have no node_modules.
import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { candidates, skipped, vendoredOf } from "./local-server-host.mjs";

const PAGE = /\.html?$/;
const READS_ORIGIN =
  /headers\s*(\.\s*origin\b|\[\s*["'`]origin["'`]\s*\])|\.getHeader\s*\(\s*["'`]origin["'`]\s*\)|headers\.get\s*\(\s*["'`]origin["'`]\s*\)|\.header\s*\(\s*["'`]origin["'`]\s*\)/i;
// "Referrer-Policy": "no-referrer", setHeader("Referrer-Policy", "no-referrer"), helmet's
// referrerPolicy: { policy: "no-referrer" }, <meta name="referrer" content="no-referrer">.
// Exactly no-referrer: no-referrer-when-downgrade still sends Origin. Not the element attribute
// referrerpolicy="no-referrer" (<img>, <a>, <iframe>, <link>, <script>) nor el.referrerPolicy =
// "no-referrer": they govern the request of that one element, not the page's policy, so the
// page's own form posts keep their Origin (lesson dooperdrew/bedrock#153). Nor the per-request
// option of fetch() and new Request() (an unquoted referrerPolicy: key): a header name is quoted
// ("Referrer-Policy": …, setHeader("Referrer-Policy", …)) or written with its hyphen.
const NO_REFERRER = [
  /(["'`]referrer-?policy["'`]\s*[:,]|\breferrer-policy\s*:)\s*["'`]no-referrer["'`]/i,
  /referrerPolicy\s*:\s*\{[^}]*policy\s*:\s*(\[\s*)?["'`]no-referrer["'`]/i,
  /<meta[^>]*name\s*=\s*["']referrer["'][^>]*content\s*=\s*["']no-referrer["']/i,
];
const WAIVER = /bedrock:\s*no-referrer ok:\s*\S/;

export const readsOrigin = (text) => READS_ORIGIN.test(text);
export const setsNoReferrer = (text) => NO_REFERRER.some((r) => r.test(text)) && !WAIVER.test(text);

/** Files to read: the Host check's sources, plus HTML pages outside the same skipped folders. */
export function files(tracked, vendored = []) {
  return [
    ...candidates(tracked, vendored),
    ...tracked.filter((f) => PAGE.test(f) && !skipped(f, vendored)),
  ];
}

/** Problems for a map of path -> text: one per file that sets no-referrer while some file reads Origin. */
export function problems(texts) {
  const origin = [...texts].find(([, t]) => readsOrigin(t))?.[0];
  if (!origin) return [];
  return [...texts].filter(([, t]) => setsNoReferrer(t)).map(([f]) => ({ file: f, origin }));
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  let tracked = [];
  try {
    tracked = execFileSync("git", ["ls-files"], { encoding: "utf8" }).split("\n").filter(Boolean);
  } catch {
    console.log("Not a git checkout: nothing to check.");
    process.exit(0);
  }
  let yml = null;
  try {
    yml = readFileSync(".bedrock.yml", "utf8");
  } catch {}
  const texts = new Map();
  for (const f of files(tracked, vendoredOf(yml))) {
    try {
      texts.set(f, readFileSync(f, "utf8"));
    } catch {}
  }
  const found = problems(texts);
  for (const p of found)
    console.log(
      `::error file=${p.file}::sets Referrer-Policy to no-referrer while ${p.origin} reads the Origin header: browsers then send "Origin: null" on same-origin form posts, so an Origin-versus-Host CSRF check refuses them. Use same-origin or strict-origin-when-cross-origin, or add the comment "bedrock: no-referrer ok: <reason>"`,
    );
  if (found.length === 0)
    console.log(
      `${texts.size} file(s): no Origin check is paired with Referrer-Policy: no-referrer.`,
    );
  process.exit(found.length ? 1 : 0);
}
