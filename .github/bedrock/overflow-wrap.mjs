// Managed by bedrock. Do not edit in consumer repos: changes arrive through bedrock sync PRs.
//
// Step "no global overflow-wrap anywhere" of bedrock-repo-checks.yml (rule
// web-overflow-wrap-not-global). `overflow-wrap: anywhere` does not only break a word that would
// overflow: it also adds a soft-wrap opportunity between any two letters to the min-content size of
// every box, so grid and flex columns that size themselves shrink until ordinary words break
// ("Platfor-m", "Recor-d sale"). `overflow-wrap: break-word` breaks only a word that really
// overflows and leaves min-content sizing alone. Lesson dooperdrew/bedrock#224 (dognutcorner-v2:
// one global `body { overflow-wrap: anywhere }`, fix in dognutcorner-v2#188).
//
// What it finds, in tracked and untracked files (not node_modules, vendored folders, build output,
// minified files, design/ prototypes or test fixtures):
// - in stylesheets (.css, .scss, .sass, .less, .pcss, .postcss), a declaration
//   `overflow-wrap: anywhere` or a Tailwind `@apply ... wrap-anywhere` whose rule selects html,
//   body, :root, the universal selector or a bare pseudo-element (::before, ::after), alone or
//   combined only with each other (`html, body`, `body *`, `*::before`); at-rules (@media, @layer,
//   @supports) are looked through;
// - in markup (.tsx, .jsx, .html, .php, .vue, .svelte, .astro), an <html> or <body> tag whose class
//   has `[overflow-wrap:anywhere]` or `wrap-anywhere`, or whose style sets overflow-wrap anywhere.
// `anywhere` on a specific element (a code, a URL cell) is fine and not reported. A global use kept
// on purpose says why with "bedrock: overflow-wrap ok: <reason>" on its line or the line above.
// Global CSS written in theme.json or injected from code is checked in review. No dependencies.
import { execFileSync } from "node:child_process";
import { existsSync, readFileSync, statSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

const STYLE = /\.(css|scss|sass|less|pcss|postcss)$/i;
const MARKUP = /\.(tsx|jsx|html?|php|vue|svelte|astro)$/i;
const SKIP =
  /(^|\/)(node_modules|vendor|vendored|dist|build|out|\.next|\.wp-site|coverage|target|\.git|design|fixtures|testdata|__fixtures__)\/|\.min\.css$/i;
const WAIVER = /bedrock:\s*overflow-wrap ok:\s*[^\s<]/;
const ANYWHERE_DECL = /^overflow-wrap\s*:\s*anywhere\b/i;
const APPLY_ANYWHERE = /^@apply\b[^;]*?(?<![\w-])wrap-anywhere(?![\w-])/i;
const MAX_BYTES = 2 * 1024 * 1024;

/** Files the check reads: { style, markup } among tracked or untracked paths. */
export function cssFiles(files) {
  const keep = files.filter((f) => !SKIP.test(f));
  return { style: keep.filter((f) => STYLE.test(f)), markup: keep.filter((f) => MARKUP.test(f)) };
}

/** The text with /* *\/ comments (and // line comments in SCSS/Less) blanked, newlines kept. */
function blankComments(text, lineComments) {
  let out = text.replace(/\/\*[\s\S]*?\*\//g, (m) => m.replace(/[^\n]/g, " "));
  // A "//" counts only at the start of a line or after a space or ; { }, so url(https://...) stays.
  if (lineComments)
    out = out.replace(
      /(^|[\s;{}])\/\/[^\n]*/g,
      (m, lead) => lead + " ".repeat(m.length - lead.length),
    );
  return out;
}

/** Top-level comma split (commas inside parentheses stay). */
function splitList(s) {
  const parts = [];
  let depth = 0;
  let cur = "";
  for (const ch of s) {
    if (ch === "(") depth++;
    else if (ch === ")") depth--;
    if (ch === "," && depth === 0) {
      parts.push(cur);
      cur = "";
    } else cur += ch;
  }
  parts.push(cur);
  return parts.map((p) => p.trim()).filter(Boolean);
}

/** True when one compound selector (no combinator) selects the whole page: html, body, :root, *, ::before. */
function globalCompound(c) {
  const where = /^:(where|is)\((.*)\)$/i.exec(c);
  if (where) return splitList(where[2]).every(globalComplex);
  // Pseudo-elements and the state of the page (a class on html or body) keep it global.
  return /^(html|body|\*|:root|::?(before|after|backdrop|marker))(?![\w-])/i.test(c);
}

/** True when a complex selector is made only of global compounds (`body *`, `html > body`). */
function globalComplex(sel) {
  // Split on combinators and spaces outside parentheses (`:where(html, body)` stays one token).
  const tokens = [];
  let depth = 0;
  let cur = "";
  for (const ch of sel) {
    if (ch === "(" || ch === "[") depth++;
    else if (ch === ")" || ch === "]") depth--;
    if (depth === 0 && /[\s>+~]/.test(ch)) {
      if (cur) tokens.push(cur);
      cur = "";
    } else cur += ch;
  }
  if (cur) tokens.push(cur);
  return tokens.length > 0 && tokens.every(globalCompound);
}

/** True when a selector list has a global selector (`html, body`, `*`, `.x, body`). */
export function globalSelector(prelude) {
  return splitList(prelude.replace(/\s+/g, " ")).some(globalComplex);
}

/** Lines of `overflow-wrap: anywhere` (or @apply wrap-anywhere) under a global selector in a stylesheet. */
export function globalAnywhereInCss(text, file = "x.css") {
  const raw = text.split(/\r?\n/);
  const code = blankComments(text, /\.(scss|less)$/i.test(file));
  const stack = [];
  const found = [];
  let buf = "";
  let start = 0;
  const flush = (end) => {
    const decl = buf.trim();
    if (decl && (ANYWHERE_DECL.test(decl) || APPLY_ANYWHERE.test(decl))) {
      const rule = [...stack].reverse().find((p) => !p.startsWith("@"));
      if (rule !== undefined && globalSelector(rule)) {
        const offset = start + buf.length - buf.trimStart().length;
        const line = code.slice(0, offset).split("\n").length;
        const near = [raw[line - 1], raw[line - 2]].some((l) => l && WAIVER.test(l));
        if (!near) found.push(line);
      }
    }
    buf = "";
    start = end + 1;
  };
  for (let i = 0; i < code.length; i++) {
    const ch = code[i];
    if (ch === "{") {
      stack.push(buf.trim());
      buf = "";
      start = i + 1;
    } else if (ch === "}") {
      flush(i);
      stack.pop();
    } else if (ch === ";") flush(i);
    else buf += ch;
  }
  return found;
}

const TAG = /<(html|body)\b/gi;
const TAG_ANYWHERE =
  /\[overflow-wrap:anywhere\]|(?<![\w-])wrap-anywhere(?![\w-])|overflow-?wrap["']?\s*:\s*["']?anywhere/i;

/** Lines of an <html> or <body> tag that sets overflow-wrap anywhere (class or style). */
export function globalAnywhereInMarkup(text) {
  const raw = text.split(/\r?\n/);
  const found = [];
  for (const m of text.matchAll(TAG)) {
    // The tag runs to the first ">" that is not part of "=>" (JSX attributes may hold arrows).
    let end = m.index + m[0].length;
    while (end < text.length && !(text[end] === ">" && text[end - 1] !== "=")) end++;
    if (!TAG_ANYWHERE.test(text.slice(m.index, end))) continue;
    const line = text.slice(0, m.index).split("\n").length;
    const near = [raw[line - 1], raw[line - 2]].some((l) => l && WAIVER.test(l));
    if (!near) found.push(line);
  }
  return found;
}

/** Problems for a list of files: [{ file, line }]. read(path) returns the text or null. */
export function check(files, read) {
  const { style, markup } = cssFiles(files);
  const problems = [];
  for (const file of style) {
    const text = read(file);
    if (text !== null)
      for (const line of globalAnywhereInCss(text, file)) problems.push({ file, line });
  }
  for (const file of markup) {
    const text = read(file);
    if (text !== null)
      for (const line of globalAnywhereInMarkup(text)) problems.push({ file, line });
  }
  return problems;
}

function readText(path) {
  if (!existsSync(path) || !statSync(path).isFile() || statSync(path).size > MAX_BYTES) return null;
  return readFileSync(path, "utf8");
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  let files = [];
  try {
    // Untracked files too, unless ignored: a run before git add checks what the PR will (#196).
    files = execFileSync("git", ["ls-files", "-co", "--exclude-standard", "-z"], {
      encoding: "utf8",
      maxBuffer: 64 * 1024 * 1024,
    })
      .split("\0")
      .filter(Boolean);
  } catch {
    console.log("Not a git checkout: nothing to check.");
    process.exit(0);
  }
  const problems = check(files, readText);
  for (const p of problems)
    console.log(
      `::error file=${p.file},line=${p.line}::overflow-wrap: anywhere on html, body or every element: it also shrinks the min-content size of every box, so narrow grid and flex columns break ordinary words mid-word. Use overflow-wrap: break-word globally and anywhere only on the element that must shrink (a code, a URL cell), or add "bedrock: overflow-wrap ok:" followed by the reason`,
    );
  if (problems.length === 0) console.log("No global overflow-wrap: anywhere.");
  process.exit(problems.length ? 1 : 0);
}
