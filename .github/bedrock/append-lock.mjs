// Managed by bedrock. Do not edit in consumer repos: changes arrive through bedrock sync PRs.
//
// Step "appends take an exclusive lock" of bedrock-repo-checks.yml (rule
// shared-append-exclusive-lock). On Linux and macOS .NET maps every FileShare value except None
// to a shared lock, and FileMode.Append seeks to the end once, at open: two processes that open
// the same file with FileShare.Read write at the same offset and one line overwrites the other.
// Lesson dooperdrew/bedrock#199 (the runner hooks' runs log, scripts/runners/hooks/job-log.ps1).
//
// In tracked PowerShell and C# files (*.ps1, *.psm1, *.cs; vendored folders, node_modules,
// bin/ and obj/ skipped), every open for append fails when its share is not None:
//   - [IO.File]::Open / File.Open, a FileStream constructor and New-Object IO.FileStream whose
//     mode is FileMode.Append (the enum, or 'Append' as a string) and whose share argument is
//     FileShare.Read, ReadWrite, Write, Delete or Inheritable (enum, cast or string);
//   - a FileStream constructor for append with no share argument (its default is Read; File.Open
//     without one opens with None and passes);
//   - a FileStreamOptions with Mode = FileMode.Append and a Share other than None, or no Share.
// A share passed in a variable is not guessed. A file with one writer only says so on the line of
// the open or the line above: "bedrock: one writer: <reason>". Retries on sharing violations,
// readers that retry too and a test with concurrent writers are checked in review.
// No dependencies: consumers may have no node_modules.
import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

const SOURCE = /\.(ps1|psm1|cs)$/i;
const SKIP = /(^|\/)(node_modules|vendor|vendored|bin|obj)\//i;
const WAIVER = /bedrock:\s*one writer:\s*[^\s<]/;
const CALL =
  /\[(?:System\.)?IO\.File\]::Open\s*\(|\bFile\.Open\s*\(|\bFileStream\s*\(|FileStream\]::new\s*\(|New-Object\s+(?:-TypeName\s+)?(?:System\.)?IO\.FileStream\s+(?:-ArgumentList\s+)?\(?/gi;
const OPTIONS = /\bFileStreamOptions\s*(?:\(\s*\))?\s*\{/g;

/** The text with comment lines blanked (PowerShell #, C# //), so line numbers stay right. */
function code(text) {
  return text
    .split(/\r?\n/)
    .map((l) => (/^\s*(#|\/\/)/.test(l) ? "" : l))
    .join("\n");
}

/** From `start` (just after an opening bracket), the text up to the matching closing one. */
function inside(text, start, open = "(", close = ")") {
  let depth = 1;
  for (let i = start; i < text.length && i < start + 2000; i++) {
    if (text[i] === open) depth++;
    else if (text[i] === close && --depth === 0) return text.slice(start, i);
  }
  return null;
}

/** Top-level arguments of a call (commas inside brackets or strings do not split). */
function args(text) {
  const out = [];
  let depth = 0;
  let quote = null;
  let cur = "";
  for (const ch of text) {
    if (quote) {
      if (ch === quote) quote = null;
    } else if (ch === "'" || ch === '"') quote = ch;
    else if ("([{".includes(ch)) depth++;
    else if (")]}".includes(ch)) depth--;
    else if (ch === "," && depth === 0) {
      out.push(cur.trim());
      cur = "";
      continue;
    }
    cur += ch;
  }
  if (cur.trim()) out.push(cur.trim());
  return out;
}

const APPEND =
  /FileMode\]?\s*(?:::|\.)\s*Append\b|^\s*['"]Append['"]\s*$|FileMode\]\s*['"]Append['"]/i;
/** The FileShare names in an argument (enum, cast or a plain string), or null if not readable. */
function shareNames(arg) {
  const names = [...arg.matchAll(/FileShare\]?\s*(?:::|\.)\s*(\w+)/gi)].map((m) => m[1]);
  const cast = arg.match(/FileShare\]\s*['"]([\w\s,]+)['"]/i);
  if (cast) names.push(...cast[1].split(/[\s,]+/).filter(Boolean));
  const plain = arg.match(/^\s*['"]([\w\s,]+)['"]\s*$/);
  if (plain) names.push(...plain[1].split(/[\s,]+/).filter(Boolean));
  return names.length ? names : null;
}
const notNone = (names) => names.some((n) => n.toLowerCase() !== "none");

/** Problems in one file's text: [{ line, why }]. */
export function appendProblems(text) {
  const src = code(text);
  const lines = text.split(/\r?\n/);
  const lineOf = (i) => src.slice(0, i).split("\n").length;
  const waived = (line) => WAIVER.test(lines[line - 1] ?? "") || WAIVER.test(lines[line - 2] ?? "");
  const problems = [];
  for (const m of src.matchAll(CALL)) {
    const start = m.index + m[0].length;
    // New-Object with its arguments on the line: `$p, 'Append', ...` or `@($p, 'Append', ...)`.
    const body = m[0].endsWith("(")
      ? inside(src, start)
      : src
          .slice(start)
          .split("\n")[0]
          .replace(/^\s*@?\(/, "")
          .replace(/\)\s*$/, "");
    if (body === null) continue;
    const a = args(body);
    if (!a.some((x) => APPEND.test(x))) continue;
    const line = lineOf(m.index);
    if (waived(line)) continue;
    const isFileOpen = /File\]?::Open|File\.Open/i.test(m[0]);
    // The share is the 4th argument, or any argument naming FileShare (named C# arguments too).
    const named = a.find((x) => /FileShare/i.test(x));
    const share = named ? shareNames(named) : a[3] !== undefined ? shareNames(a[3]) : null;
    if (share && notNone(share))
      problems.push({ line, why: `opens a file for append with FileShare.${share.join("|")}` });
    else if (!share && a[3] === undefined && !named && !isFileOpen)
      problems.push({ line, why: "opens a FileStream for append with the default share (Read)" });
  }
  for (const m of src.matchAll(OPTIONS)) {
    const body = inside(src, m.index + m[0].length, "{", "}");
    if (body === null || !/\bMode\s*=\s*FileMode\.Append\b/.test(body)) continue;
    const line = lineOf(m.index);
    if (waived(line)) continue;
    const share = body.match(/\bShare\s*=\s*([^,}\n]+)/);
    const names = share ? shareNames(share[1]) : null;
    if (!share)
      problems.push({ line, why: "FileStreamOptions for append with the default share (Read)" });
    else if (names && notNone(names))
      problems.push({
        line,
        why: `FileStreamOptions for append with FileShare.${names.join("|")}`,
      });
  }
  return problems;
}

/** The files to read: tracked or new, not ignored, PowerShell or C#, outside skipped folders. */
export function sourceFiles(listed) {
  return listed.filter((f) => SOURCE.test(f) && !SKIP.test(f));
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const listed = execFileSync("git", ["ls-files", "-co", "--exclude-standard", "-z"], {
    encoding: "utf8",
    maxBuffer: 1 << 28,
  })
    .split("\0")
    .filter(Boolean);
  let failed = 0;
  const files = sourceFiles(listed);
  for (const file of files) {
    let text;
    try {
      text = readFileSync(file, "utf8");
    } catch {
      continue; // listed but deleted in the working tree
    }
    for (const p of appendProblems(text)) {
      console.log(
        `::error file=${file},line=${p.line}::${p.why}: on Linux and macOS only FileShare.None locks, so concurrent writers overwrite each other's lines. Use FileShare.None (retrying on sharing violations, readers too) or a named mutex, or say "bedrock: one writer: <reason>"`,
      );
      failed = 1;
    }
  }
  if (!failed) console.log(`${files.length} PowerShell or C# file(s): every append open locks.`);
  process.exit(failed);
}
