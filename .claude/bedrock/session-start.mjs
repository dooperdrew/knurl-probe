// bedrock SessionStart hook (wired in .claude/settings.json, shipped to every repo).
//
//   node .claude/bedrock/session-start.mjs           as the hook: acts only in a cloud session
//   node .claude/bedrock/session-start.mjs --check   anywhere: only reports what is missing
//
// On the PC it does nothing: the design skills are already in ~/.claude/skills
// (npm run install-skills in bedrock) and the tools are installed. In a cloud session
// (CLAUDE_CODE_REMOTE=true) the machine is new every time and ~/.claude/skills is not
// read, so it installs the external design skills from cloud-skills.json (pinned
// commits, same as the PC) into this repo's .claude/skills, keeps them out of git
// through .git/info/exclude, and reports the tools this repo's stacks need.
// It also sets this repo's git identity to the GitHub noreply author found in its history when
// git would commit as the cloud default (noreply@anthropic.com): otherwise every commit, merge or
// rebase made without -c user.email gets a committer that bedrock's repo checks reject after the
// push (lesson dooperdrew/bedrock#229, dooperdrew/keyfold#96).
// What it prints is added to the session's context.
// It never fails the session: every problem is reported, the exit code is always 0.
import { execFileSync } from "node:child_process";
import { appendFileSync, existsSync, mkdirSync, readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const repo = resolve(here, "../..");

// Tools each stack needs; a missing one is reported, not installed.
export const TOOLS = {
  all: ["git", "gh", "node"],
  rust: ["cargo", "rustc"],
  node: ["pnpm"],
  wordpress: ["php", "composer"],
  base: [],
};

// .bedrock.yml is written by sync with a plain block list under "stacks:"; a hand edit
// may use the inline form "stacks: [node, rust]".
export function readStacks(text) {
  const inline = /^stacks:[ \t]*\[([^\]]*)\]/m.exec(text);
  if (inline) {
    return inline[1]
      .split(",")
      .map((s) => s.trim().replace(/^["']|["']$/g, ""))
      .filter(Boolean);
  }
  const m = /^stacks:\s*\n((?:\s+-\s+.+\n?)+)/m.exec(text);
  return m
    ? m[1]
        .split("\n")
        .map((l) => l.replace(/^\s*-\s*/, "").trim())
        .filter(Boolean)
    : [];
}

function has(cmd) {
  try {
    // cmd comes from TOOLS only; Windows needs the shell to find .cmd launchers (pnpm).
    if (process.platform === "win32")
      execFileSync(`${cmd} --version`, { stdio: "ignore", timeout: 15000, shell: true });
    else execFileSync(cmd, ["--version"], { stdio: "ignore", timeout: 15000 });
    return true;
  } catch {
    return false;
  }
}

// Installed skills must never be committed: list them in .git/info/exclude (local to
// this clone, unlike .gitignore, which belongs to the project).
export function excludeFromGit(repoDir, names) {
  const info = join(repoDir, ".git", "info");
  if (!existsSync(join(repoDir, ".git"))) return;
  mkdirSync(info, { recursive: true });
  const file = join(info, "exclude");
  const have = existsSync(file) ? readFileSync(file, "utf8").split(/\r?\n/) : [];
  const add = names.map((n) => `/.claude/skills/${n}/`).filter((l) => !have.includes(l));
  if (add.length)
    appendFileSync(file, `${(have.length && have.at(-1) !== "" ? "\n" : "") + add.join("\n")}\n`);
}

function skillNames() {
  return JSON.parse(readFileSync(join(here, "cloud-skills.json"), "utf8")).skills.map(
    (s) => s.name,
  );
}

// "24", "v24.1.0", "lts/*" -> 24, 24, null
export function nodeMajor(text) {
  const m = /^\s*v?(\d+)/.exec(text);
  return m ? Number(m[1]) : null;
}

const NOREPLY = /@users\.noreply\.github\.com$/i;

function git(repoDir, args) {
  try {
    return execFileSync("git", args, {
      cwd: repoDir,
      encoding: "utf8",
      stdio: ["ignore", "pipe", "ignore"],
    }).trim();
  } catch {
    return "";
  }
}

/**
 * Makes git commit as the repo's GitHub noreply author. Returns one line for the session, or null
 * when the identity is already a noreply address. With fix = false it only reports. The identity is
 * the most recent author of the repo's history with a noreply address; it is set in the repo's own
 * config (never --global).
 */
export function gitIdentity(repoDir, { fix = true } = {}) {
  const email = git(repoDir, ["config", "user.email"]);
  if (NOREPLY.test(email)) return null;
  const authors = git(repoDir, ["log", "-n", "500", "--format=%an%x09%ae"]).split("\n");
  // Bots (dependabot[bot], github-actions[bot]) have noreply addresses too, and rebase merges keep
  // their authorship: never commit as them.
  // Copilot's agent commits as Copilot <…+Copilot@users.noreply.github.com>, without [bot]. The
  // repo owner's login (from the origin URL) wins over a rebase-merged outside contributor.
  const people = authors
    .map((l) => l.split("\t"))
    .filter(([n, e]) => n && e && NOREPLY.test(e) && !/\[bot\]|(^|\+)copilot@/i.test(`${n} ${e}`));
  const owner = /github\.com[/:]([^/]+)\//i.exec(
    git(repoDir, ["remote", "get-url", "origin"]),
  )?.[1];
  const login = (e) =>
    e
      .replace(NOREPLY, "")
      .replace(/^\d+\+/, "")
      .toLowerCase();
  const found = (owner && people.find(([, e]) => login(e) === owner.toLowerCase())) || people[0];
  const now = email ? `git commits as ${email}` : "git has no user.email";
  if (!found)
    return `git identity: ${now}, not a GitHub noreply address, and no noreply author in this repo's history: set user.name and user.email to the owner's noreply identity before committing (rule repo-commit-identity).`;
  const [name, address] = found;
  if (!fix)
    return `git identity: ${now}; commits must use ${address} (run this script without --check, or \`git config user.email ${address}\`).`;
  git(repoDir, ["config", "user.name", name]);
  git(repoDir, ["config", "user.email", address]);
  // What git will really use: GIT_AUTHOR_EMAIL / GIT_COMMITTER_EMAIL override the config.
  const ident = (v) => (/<([^>]*)>/.exec(git(repoDir, ["var", v])) ?? [])[1] ?? "";
  return NOREPLY.test(ident("GIT_AUTHOR_IDENT")) && NOREPLY.test(ident("GIT_COMMITTER_IDENT"))
    ? `git identity: ${now}; set this repo's user.name and user.email to ${name} <${address}>, so authors and committers of commits, merges and rebases pass the repo checks.`
    : `git identity: ${now}, and setting ${address} FAILED: set it by hand before committing.`;
}

export function checkTools(stacks, probe = has) {
  const need = [...new Set([...TOOLS.all, ...stacks.flatMap((s) => TOOLS[s] ?? [])])];
  return need.filter((t) => !probe(t));
}

export async function main(args = process.argv.slice(2), env = process.env) {
  const remote = env.CLAUDE_CODE_REMOTE === "true";
  const check = args.includes("--check");
  if (!remote && !check) return [];
  const out = [];
  const config = join(repo, ".bedrock.yml");
  const stacks = existsSync(config) ? readStacks(readFileSync(config, "utf8")) : [];
  out.push(
    `bedrock ${check ? (remote ? "check (cloud)" : "check") : "cloud session"} (stacks: ${stacks.join(", ") || "unknown"})`,
  );

  if (remote) {
    try {
      const { run } = await import("./install-skills.mjs");
      const log = console.log;
      const lines = [];
      console.log = (...a) => lines.push(a.join(" "));
      try {
        if (!check) excludeFromGit(repo, skillNames());
        const r = await run(
          [
            ...(check ? ["--check"] : []),
            "--list",
            join(here, "cloud-skills.json"),
            "--skip-on-demand",
            "--dest",
            join(repo, ".claude", "skills"),
          ],
          { allowFileUrls: env.BEDROCK_ALLOW_FILE_URLS === "1" },
        ); // file:// only for bedrock's tests
        if (check) {
          out.push(
            `design skills: ${r.current.length} present${r.outdated.length ? `, missing or old: ${r.outdated.join(", ")} (run this script without --check)` : ""}.`,
          );
        } else {
          out.push(
            `design skills: installed ${r.installed.length}, already there ${r.current.length}` +
              (r.failed.length ? `, FAILED ${r.failed.join(", ")}` : "") +
              ".",
          );
        }
        if (!check)
          out.push(
            "design-taste-frontend is on demand: `node .claude/bedrock/install-skills.mjs --list .claude/bedrock/cloud-skills.json --dest .claude/skills`. If a skill does not show up, run /reload-skills.",
          );
      } finally {
        console.log = log;
      }
      out.push(...lines.filter((l) => /: failed: /.test(l)));
    } catch (e) {
      out.push(`design skills: not installed (${e.message.split("\n")[0]}).`);
    }
  }

  const missing = checkTools(stacks);
  out.push(missing.length ? `missing tools: ${missing.join(", ")}.` : "tools: all present.");
  const nvmrc = join(repo, ".nvmrc");
  const wanted = existsSync(nvmrc) ? nodeMajor(readFileSync(nvmrc, "utf8")) : null;
  if (wanted && nodeMajor(process.version) !== wanted) {
    out.push(
      `node: ${process.version} here, .nvmrc wants ${wanted}` +
        (remote
          ? `: install it in the environment's setup script (e.g. \`n ${wanted}\` or nvm) or results may differ from CI.`
          : "."),
    );
  }
  if (remote) {
    const id = gitIdentity(repo, { fix: !check });
    if (id) out.push(id);
    out.push(
      "CI checks that need the GitHub API (zizmor online audits) or extra linters (actionlint) may not run here: leave them to CI and say so.",
    );
    out.push(
      "gh in the cloud: `gh pr` and `gh issue` (GraphQL) are refused by the proxy; use `gh api` (REST), e.g. `gh api repos/OWNER/REPO/pulls -f title=... -f head=... -f base=main`.",
    );
    out.push(
      'Not available in the cloud: hardware (devices, keyboards, headphones), the PC\'s runners and files outside this repo. Mark such checks "not verified" in the PR.',
    );
  }
  return out;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    const lines = await main();
    if (lines.length) console.log(lines.join("\n"));
  } catch (e) {
    console.log(`bedrock session-start: ${e.message}`);
  }
  process.exitCode = 0;
}
