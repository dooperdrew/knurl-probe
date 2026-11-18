// Managed by bedrock. Do not edit in consumer repos: changes arrive through bedrock sync PRs.
//
// Step "no two migrations share a number" of bedrock-repo-checks.yml (rule repo-migration-numbers-unique).
// A file of its own: long heredocs in a workflow made actionlint hang on Windows runners.
import { execFileSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";

// Two branches open at once both take "the next number"; when they meet, one migration has
// to be renamed after it may already have run somewhere. Lesson dooperdrew/bedrock#61.
const files = execFileSync("git", ["ls-files", "-z", "--", "*.sql"], { encoding: "utf8" })
  .split("\0")
  .filter(Boolean);
// Folders of third-party files (vendored: in .bedrock.yml) are not the repo's migrations.
const vendored = [];
if (existsSync(".bedrock.yml")) {
  let inList = false;
  for (const line of readFileSync(".bedrock.yml", "utf8").replace(/\r/g, "").split("\n")) {
    const clean = (v) =>
      v
        .replace(/\s+#.*$/, "")
        .trim()
        .replace(/^(["'])(.*)\1$/, "$2")
        .replace(/^\.\//, "")
        .replace(/\/+$/, "");
    const top = /^vendored:\s*(.*)$/.exec(line);
    if (top) {
      const rest = top[1].replace(/\s+#.*$/, "").trim();
      if (rest.startsWith("["))
        for (const v of rest.replace(/^\[|\]$/g, "").split(",")) {
          if (clean(v)) vendored.push(clean(v));
        }
      else if (rest) vendored.push(clean(rest));
      inList = !rest;
      continue;
    }
    if (/^\s*(#.*)?$/.test(line)) continue;
    if (inList && /^\s+-\s+/.test(line)) vendored.push(clean(line.replace(/^\s+-\s+/, "")));
    else if (inList && /^\S/.test(line)) inList = false;
  }
}
const isVendored = (f) => vendored.some((v) => v && (f === v || f.startsWith(`${v}/`)));
// Only migration folders: a path segment named migrations, migrate, drizzle, flyway or db.
// Seeds and fixtures below them are not migrations.
const isMigrationDir = (dir) =>
  /(^|\/)(migrations?|migrate|drizzle|flyway|db)(\/|$)/i.test(dir) &&
  !/(^|\/)(seeds?|fixtures?|__fixtures__|testdata)(\/|$)/i.test(dir);
const byNumber = new Map();
for (const file of files) {
  if (isVendored(file)) continue;
  const slash = file.lastIndexOf("/");
  const dir = slash < 0 ? "." : file.slice(0, slash);
  if (!isMigrationDir(dir)) continue;
  const base = file.slice(slash + 1);
  // Flyway or refinery V1__x.sql / V1_1__x.sql (version 1.1). U files (Flyway undo,
  // refinery unversioned) and R repeatables carry no new version: skipped.
  if (/^[UR]\d*__/.test(base)) continue;
  let number;
  const versioned = /^V([\d._]+?)__/.exec(base);
  const dated = /^(\d{4}-\d{2}-\d{2})[_-]/.exec(base);
  const plain = /^(\d+)[_-]/.exec(base);
  if (versioned)
    number = versioned[1]
      .replace(/_/g, ".")
      .split(".")
      .map((p) => p.replace(/^0+(?=\d)/, ""))
      .join(".");
  else if (dated) number = dated[1];
  else if (plain)
    number = plain[1].replace(/^0+(?=\d)/, ""); // a string: 19-digit timestamps stay exact
  else continue;
  // The up and down halves of one migration share its number.
  const name = base.replace(/([._](up|down))?\.sql$/i, "");
  const key = `${dir}\0${number}`;
  if (!byNumber.has(key)) byNumber.set(key, new Map());
  const names = byNumber.get(key);
  if (!names.has(name)) names.set(name, file);
}
let failed = false;
let checked = 0;
for (const [key, names] of byNumber) {
  checked += names.size;
  if (names.size < 2) continue;
  const [dir, number] = key.split("\0");
  const list = [...names.values()].join(", ");
  console.log(
    `::error file=${[...names.values()][1]}::migrations in ${dir} share number ${number}: ${list}. Renumber one of them (the one not yet applied anywhere) before merging`,
  );
  failed = true;
}
if (checked === 0)
  console.log(
    "No numbered .sql files in a migrations, migrate, drizzle, flyway or db folder: nothing to check.",
  );
else if (!failed) console.log(`${checked} numbered migrations, every number used once per folder`);
if (failed) process.exit(1);
