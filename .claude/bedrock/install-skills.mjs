// Installs the skills listed in user-skills.yml into the user's skills folder.
//
//   node .claude/bedrock/install-skills.mjs [--check] [--dest <dir>] [--list <file>] [--skip-on-demand]
//   node .claude/bedrock/install-skills.mjs --export <file>   (bedrock only: writes the cloud list)
//
// The same file ships to every repo (kit.yml): in a cloud session the SessionStart hook
// (session-start.mjs) runs it on cloud-skills.json, the JSON copy of the external skills,
// so it needs no dependency there (yaml is loaded only for a .yml list).
//
// External skills are fetched at their pinned commit with core.autocrlf off, so files
// keep the line endings the upstream repo chose (its .gitattributes still applies, and
// must: impeccable ships a .cmd launcher marked eol=crlf next to an LF sh launcher
// that breaks with CRLF). Only the skill's folder and its license files are kept. Each installed folder gets a .bedrock-skill.json marker,
// and only folders with that marker are ever replaced: a skill the user installed by
// hand under the same name is left alone and reported. A new copy is built outside
// the skills folder and swapped in only when complete, so a failure never leaves a
// skill half-installed.
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import {
  cpSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readdirSync,
  readFileSync,
  renameSync,
  rmSync,
  statSync,
  writeFileSync,
} from "node:fs";
import { homedir, tmpdir } from "node:os";
import { basename, dirname, isAbsolute, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(fileURLToPath(import.meta.url), "../../..");
const MARKER = ".bedrock-skill.json";
const COMMIT = /^[0-9a-f]{40}$/;
const NAME = /^[a-z0-9][a-z0-9-]*$/;
// Repo-relative paths: plain segments only (no "..", no leading "-" or "/", no glob syntax).
const SAFE_PATH = /^(?!-)[A-Za-z0-9._-]+(\/[A-Za-z0-9._-]+)*$/;

function arg(args, name, fallback) {
  const i = args.indexOf(name);
  if (i < 0) return fallback;
  if (!args[i + 1] || args[i + 1].startsWith("--")) throw new Error(`${name} needs a value`);
  return args[i + 1];
}

const safePath = (p) =>
  typeof p === "string" &&
  SAFE_PATH.test(p) &&
  !p.split("/").some((seg) => seg === ".." || seg === ".") &&
  !isAbsolute(p);

async function readList(file) {
  const text = readFileSync(file, "utf8");
  if (file.endsWith(".json")) return JSON.parse(text).skills ?? [];
  const { parse } = await import("yaml");
  return parse(text)?.skills ?? [];
}

export async function loadList(file, { allowFileUrls = false } = {}) {
  const list = await readList(file);
  const seen = new Set();
  for (const s of list) {
    if (!NAME.test(s.name ?? "")) throw new Error(`bad skill name: ${s.name}`);
    if (seen.has(s.name)) throw new Error(`${s.name}: listed twice`);
    seen.add(s.name);
    if (s.local !== undefined) {
      if (!safePath(s.local)) throw new Error(`${s.name}: bad local path`);
      continue;
    }
    const scheme = allowFileUrls ? /^(https|file):\/\// : /^https:\/\//;
    if (!scheme.test(s.repo ?? "")) throw new Error(`${s.name}: repo must be an https URL`);
    if (!COMMIT.test(s.commit ?? ""))
      throw new Error(`${s.name}: commit must be a full 40-character SHA`);
    if (!safePath(s.path)) throw new Error(`${s.name}: bad path`);
    if (s["on-demand"] !== undefined && typeof s["on-demand"] !== "boolean")
      throw new Error(`${s.name}: on-demand must be true or false`);
    for (const f of s["license-files"] ?? [])
      if (!safePath(f)) throw new Error(`${s.name}: bad license file ${f}`);
  }
  return list;
}

// Hash of every file in a folder (relative path + content), for local skills.
function hashDir(dir) {
  const h = createHash("sha256");
  const walk = (d) => {
    for (const e of readdirSync(d, { withFileTypes: true }).sort((a, b) =>
      a.name.localeCompare(b.name),
    )) {
      const p = join(d, e.name);
      if (e.isDirectory()) walk(p);
      else
        h.update(`${relative(dir, p).replace(/\\/g, "/")}\0`)
          .update(readFileSync(p))
          .update("\0");
    }
  };
  walk(dir);
  return h.digest("hex");
}

// What the marker records: everything that decides the installed content.
const want = (s) =>
  s.local !== undefined
    ? { local: s.local, sha256: hashDir(join(root, s.local)) }
    : {
        repo: s.repo,
        commit: s.commit,
        path: s.path,
        licenseFiles: (s["license-files"] ?? []).join(","),
      };
const same = (have, target) => Object.keys(target).every((k) => have[k] === target[k]);

function installed(dir) {
  try {
    return JSON.parse(readFileSync(join(dir, MARKER), "utf8"));
  } catch {
    return null;
  }
}

// Byte-identical checkouts: no CRLF conversion, whatever the user's git config says.
const git = (args, cwd) =>
  execFileSync(
    "git",
    ["-c", "core.autocrlf=false", "-c", "core.eol=lf", "-c", "core.symlinks=false", ...args],
    { cwd, stdio: ["ignore", "pipe", "pipe"], encoding: "utf8" },
  );

function fetchExternal(s, work) {
  const repo = join(work, s.name);
  git(["init", "-q", repo]);
  git(["remote", "add", "origin", s.repo], repo);
  git(["fetch", "-q", "--depth", "1", "--filter=blob:none", "origin", s.commit], repo);
  const links = git(["ls-tree", "-r", "FETCH_HEAD", "--", s.path], repo)
    .split("\n")
    .filter((l) => l.startsWith("120000"));
  if (links.length)
    throw new Error(
      `${s.name}: ${s.path} contains symbolic links, which would install as text stubs`,
    );
  // Anchored patterns: only these exact paths, not same-named files deeper in the repo.
  git(
    [
      "sparse-checkout",
      "set",
      "--no-cone",
      `/${s.path}/`,
      ...(s["license-files"] ?? []).map((f) => `/${f}`),
    ],
    repo,
  );
  git(["-c", "advice.detachedHead=false", "checkout", "-q", "FETCH_HEAD"], repo);
  const got = git(["rev-parse", "HEAD"], repo).trim();
  if (got !== s.commit) throw new Error(`${s.name}: fetched ${got}, expected ${s.commit}`);
  if (!existsSync(join(repo, s.path, "SKILL.md")))
    throw new Error(`${s.name}: no SKILL.md in ${s.path}`);
  return repo;
}

// Windows refuses to rename a folder while a file in it is open (an editor, a scan
// by Defender, a running script): retry for a while before giving up.
function renameRetry(from, to) {
  for (let i = 0; ; i++) {
    try {
      return renameSync(from, to);
    } catch (e) {
      if (i >= 20 || !["EPERM", "EBUSY", "EACCES"].includes(e.code)) throw e;
      Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, 250 * (i + 1));
    }
  }
}

// Staging lives next to the skills folder (same volume, so renames are cheap), not
// inside it: Claude Code must never see a half-built or leftover copy as a skill.
const stagingDir = (dest) => join(dirname(dest), ".bedrock-skills-tmp");

// A crash between the two renames leaves the previous copy aside: put it back first.
function recover(s, dir, dest) {
  const old = join(stagingDir(dest), `${s.name}.old`);
  if (!existsSync(dir) && existsSync(join(old, MARKER))) {
    renameRetry(old, dir);
    console.log(`${s.name}: restored the previous copy left aside by an interrupted install`);
  }
}

function installOne(s, dir, dest, work, target) {
  const source = s.local !== undefined ? join(root, s.local) : fetchExternal(s, work);
  const from = s.local !== undefined ? source : join(source, s.path);
  const tmp = stagingDir(dest);
  const staging = join(tmp, `${s.name}.new`);
  const old = join(tmp, `${s.name}.old`);
  mkdirSync(tmp, { recursive: true });
  mkdirSync(dest, { recursive: true });
  rmSync(staging, { recursive: true, force: true, maxRetries: 5 });
  try {
    cpSync(from, staging, { recursive: true, filter: (p) => basename(p) !== ".git" });
    for (const f of s["license-files"] ?? []) {
      const src = join(source, f);
      if (!existsSync(src) || !statSync(src).isFile())
        throw new Error(`${s.name}: license file ${f} not found at the pinned commit`);
      const inside = join(staging, basename(f));
      const to =
        existsSync(inside) && relative(from, src) !== basename(f)
          ? join(staging, `UPSTREAM-${basename(f)}`)
          : inside;
      if (!existsSync(to)) cpSync(src, to);
    }
    writeFileSync(
      join(staging, MARKER),
      `${JSON.stringify({ ...target, license: s.license ?? "see bedrock", installedAt: new Date().toISOString() }, null, 2)}\n`,
    );
    // Swap: the old copy is moved aside first and deleted only after the new one is in place.
    rmSync(old, { recursive: true, force: true, maxRetries: 5 });
    if (existsSync(dir)) renameRetry(dir, old);
    try {
      renameRetry(staging, dir);
    } catch (e) {
      if (existsSync(old) && !existsSync(dir)) renameRetry(old, dir);
      throw e;
    }
  } finally {
    rmSync(staging, { recursive: true, force: true, maxRetries: 5 });
  }
  try {
    rmSync(old, { recursive: true, force: true, maxRetries: 5 });
  } catch (e) {
    console.log(
      `${s.name}: installed; the previous copy in ${old} could not be removed yet (${e.code})`,
    );
  }
}

// The cloud list: only the external skills (local ones reach repos through kit.yml).
export async function exportCloudList(from = join(root, "user-skills.yml")) {
  const skills = (await loadList(from)).filter((s) => s.local === undefined);
  const json = JSON.stringify(
    {
      "//": "Generated from user-skills.yml by `npm run cloud-skills`: do not edit by hand.",
      skills,
    },
    null,
    2,
  );
  return `${biomeArrays(json)}\n`;
}

// Node projects lint every file with Biome (line width 100), which puts a short array of
// strings on one line: write it that way, so the copy in each repo passes their lint.
function biomeArrays(json) {
  return json.replace(
    /^([^\n]*)\[\n((?:\s*"(?:[^"\\\n]|\\.)*",?\n)+)\s*\]/gm,
    (all, head, body) => {
      const items = body
        .split("\n")
        .map((l) => l.trim().replace(/,$/, ""))
        .filter(Boolean);
      const line = `${head}[${items.join(", ")}]`;
      return line.length <= 100 ? line : all;
    },
  );
}

export async function run(args, { allowFileUrls = false } = {}) {
  const exportTo = arg(args, "--export", null);
  if (exportTo) {
    writeFileSync(resolve(exportTo), await exportCloudList());
    console.log(`wrote ${exportTo}`);
    return { exitCode: 0 };
  }
  const check = args.includes("--check");
  const skipOnDemand = args.includes("--skip-on-demand");
  const dest = resolve(arg(args, "--dest", join(homedir(), ".claude", "skills")));
  const list = (
    await loadList(resolve(arg(args, "--list", join(root, "user-skills.yml"))), { allowFileUrls })
  ).filter((s) => !(skipOnDemand && s["on-demand"]));
  const report = { installed: [], current: [], skipped: [], outdated: [], failed: [] };
  const work = check ? null : mkdtempSync(join(tmpdir(), "bedrock-skills-"));
  try {
    for (const s of list) {
      const dir = join(dest, s.name);
      try {
        if (!check) recover(s, dir, dest);
        const have = existsSync(dir) ? installed(dir) : null;
        if (existsSync(dir) && !have) {
          report.skipped.push(s.name);
          console.log(`${s.name}: ${dir} exists and was not installed by bedrock; left alone`);
          continue;
        }
        const target = want(s);
        if (have && same(have, target)) {
          report.current.push(s.name);
          continue;
        }
        if (check) {
          report.outdated.push(s.name);
          console.log(`${s.name}: ${have ? "out of date" : "missing"}`);
          continue;
        }
        installOne(s, dir, dest, work, target);
        report.installed.push(s.name);
        console.log(`${s.name}: installed in ${dir}`);
      } catch (e) {
        report.failed.push(s.name);
        console.error(
          `${s.name}: failed: ${
            String(e.stderr || e.message)
              .trim()
              .split("\n")[0]
          }`,
        );
      }
    }
  } finally {
    if (work) rmSync(work, { recursive: true, force: true });
  }
  console.log(
    `installed ${report.installed.length}, up to date ${report.current.length}, left alone ${report.skipped.length}, failed ${report.failed.length}` +
      (check ? `, to install ${report.outdated.length}` : ""),
  );
  report.exitCode =
    report.failed.length || (check && (report.outdated.length || report.skipped.length)) ? 1 : 0;
  return report;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  process.exitCode = (await run(process.argv.slice(2))).exitCode;
}
