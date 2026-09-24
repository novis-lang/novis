// `bun nv links [path...]`: every link in the repository that does not resolve, that names a file
// whose case does not match the one on disk, or that is written in the wrong form for its file.
//
//     bun nv links                   every tracked .md, .rs, .nvs, .nvst and .py file
//     bun nv links docs/decisions    only the tracked files whose path starts with one of these
//
// A gate: it exits 1 on any finding. A link's form depends on whether a renderer resolves it, and
// `docs/agent/conventions.md` § *Citing a document* is the home of that rule:
//
// - A markdown file is rendered by GitHub and by the website, which resolve a link against the file,
//   so its links are relative (`../decisions/0067.md`).
// - A source file (`.rs`, `.nvs`, `.nvst`) is rendered by nothing, so its links are absolute from the
//   repository root (`/docs/decisions/0067.md`). Two shapes there are not paths and are skipped: a
//   rustdoc intra-doc link has no `/`, and a link to a rustdoc page ends in `.html`.
// - A `.py` file is read for bare path mentions only, anchored at a top-level directory. A link in a
//   tool is usually text it emits into a generated page, and resolves from that page, not from here.
//
// The five kinds of finding:
//
// - `missing`   the target does not exist.
// - `case`      the target exists, but not with the spelling the link used. Windows and macOS resolve
//               a path case-insensitively, so this passes on the author's machine and 404s on GitHub.
// - `relative`  a source file wrote `../docs/…` where the root-absolute form belongs.
// - `absolute`  a markdown file wrote `/docs/…`, which resolves to the site root on GitHub.
// - `retired`   a `.py` file's prose names a repository path that is not on disk. A line carrying
//               `check-links:retired` (the file is gone on purpose), `check-links:written` (a tool
//               creates it on demand) or `check-links:subject` (the line is about the spelling itself)
//               is passed over.
//
// A link inside a code span or a markdown fence is an example, not a link. A fragment is checked as
// far as its file. A file announcing `GENERATED FILE` in its first lines is skipped: the website's
// rulebook pages rewrite every citation into a site route, and their sources are checked instead.

import { existsSync, readdirSync, readFileSync } from "node:fs";
import { dirname, extname, join, resolve, sep } from "node:path";
import { tracked } from "../lib/git.ts";
import { ROOT, rel } from "../lib/paths.ts";

export const summary = "every link that does not resolve, is mis-cased or has the wrong form: nv links [path...]";

/** Inline links and images alike: `](target)`. A target holds no whitespace and no `)`. */
const LINK_RE = /\]\(([^)\s]+)\)/g;
const SKIP_SCHEMES = ["http://", "https://", "mailto:", "ftp://", "data:", "#"];

/** Rendered by a git host and by the website: links stay relative to the file. */
export const DOC_EXTS = [".md"];
/** Rendered by nothing: links are absolute from the repository root. */
export const SOURCE_EXTS = [".rs", ".nvs", ".nvst"];
/** Read for bare path mentions only, never for link syntax. */
export const MENTION_EXTS = [".py"];

/** The top-level directories a mention is anchored to, so a bare `mod.rs` never counts as one. */
const MENTION_TOPS = ["docs", "crates", "tools", "tests", "benches", "examples", "editors", "fuzz", "website"];
// A Unicode word boundary on each side, as Python's `\b` draws it for a `str` pattern.
const MENTION_RE = new RegExp(
  `(?<![\\p{L}\\p{N}_])(?:${MENTION_TOPS.join("|")})/[A-Za-z0-9_./-]*` +
    `\\.(?:md|rs|py|toml|json|nvs|nvst|sh|yml|yaml|mjs)(?![\\p{L}\\p{N}_])`,
  "gu",
);
/** A mention that is a shape rather than a path: `docs/decisions/NNNN.md`, `docs/plan/mN.md`. */
const MENTION_PLACEHOLDER = /NNNN|\bmN\b|<|\{|\*|\?/;
/** A line carrying one of these names a path that is absent on purpose. See the header. */
const MENTION_SKIPS = ["check-links:retired", "check-links:written", "check-links:subject"];

const CODE_SPAN_RE = /`[^`]*`/g;
const FENCE_RE = /^\s*(```|~~~)/;
const GENERATED_MARKER = "GENERATED FILE";
const GENERATED_HEAD_LINES = 15;

export type Kind = "missing" | "case" | "relative" | "absolute" | "retired";
export interface Finding {
  line: number;
  target: string;
  kind: Kind;
}

/** Answers whether `target`, read from the directory `base`, is there: `missing`, `case` or null. */
export type Resolver = (base: string, target: string) => "missing" | "case" | null;

const listings = new Map<string, Set<string> | null>();

function listing(dir: string): Set<string> | null {
  let entries = listings.get(dir);
  if (entries === undefined) {
    try {
      entries = new Set(readdirSync(dir));
    } catch {
      entries = null;
    }
    listings.set(dir, entries);
  }
  return entries;
}

/**
 * True when every component of `target`, as written, matches its directory entry exactly. The
 * filesystem's own answer is case-insensitive on Windows and macOS, and a resolved path comes back in
 * the canonical spelling, so this walks the components from a `base` whose case is known good.
 */
function caseExact(base: string, target: string): boolean {
  let current = base;
  for (const part of target.split(/[/\\]/)) {
    if (part === "" || part === ".") continue;
    if (part === "..") {
      current = dirname(current);
      if (!current.startsWith(ROOT)) return true; // out of the repository: not ours to judge
      continue;
    }
    const entries = listing(current);
    if (entries === null) return true;
    if (!entries.has(part)) return false;
    current = join(current, part);
  }
  return true;
}

/** The default resolver: the working tree answers. */
export const onDisk: Resolver = (base, target) => {
  if (!existsSync(join(base, target))) return "missing";
  if (!caseExact(base, target)) return "case";
  return null;
};

/** `%XX` escapes decoded as UTF-8, and a run that does not decode left as it is. */
function unquote(s: string): string {
  return s.replace(/(?:%[0-9A-Fa-f]{2})+/g, (m) => {
    try {
      return decodeURIComponent(m);
    } catch {
      return m;
    }
  });
}

/** Each bare path mention in a tool's text that is not on disk. */
export function deadMentions(text: string): Finding[] {
  const out: Finding[] = [];
  text.split("\n").forEach((line, i) => {
    if (MENTION_SKIPS.some((m) => line.includes(m))) return;
    for (const m of line.matchAll(MENTION_RE)) {
      const target = m[0];
      if (MENTION_PLACEHOLDER.test(target)) continue;
      if (!existsSync(join(ROOT, target))) out.push({ line: i + 1, target, kind: "retired" });
    }
  });
  return out;
}

/**
 * Each finding in `text`, read as a source file when `source` and as markdown otherwise, with `base`
 * the directory its relative links resolve from. It takes text rather than a file so a body not yet
 * written can be checked, and a `resolve` so the tree it is checked against can be another commit's.
 */
export function findingsIn(text: string, source: boolean, base: string, resolveTarget: Resolver = onDisk): Finding[] {
  const out: Finding[] = [];
  let fenced = false;
  text.split("\n").forEach((line, i) => {
    // The fence is markdown's rule. A doc comment's links are prose inside a fence or out of one.
    if (!source && FENCE_RE.test(line)) {
      fenced = !fenced;
      return;
    }
    if (fenced) return;
    const spans = [...line.matchAll(CODE_SPAN_RE)].map((m) => [m.index!, m.index! + m[0].length] as const);
    for (const m of line.matchAll(LINK_RE)) {
      const at = m.index!;
      if (spans.some(([start, end]) => start <= at && at < end)) continue;
      const raw = m[1]!;
      if (SKIP_SCHEMES.some((s) => raw.startsWith(s))) continue;
      let target = unquote(raw.split("#", 1)[0]!);
      if (!target) continue;
      let here: string;
      if (source) {
        if (!target.includes("/") || target.endsWith(".html")) continue;
        if (!target.startsWith("/")) {
          out.push({ line: i + 1, target: raw, kind: "relative" });
          continue;
        }
        here = ROOT;
        target = target.replace(/^\/+/, "");
      } else {
        if (target.startsWith("/")) {
          out.push({ line: i + 1, target: raw, kind: "absolute" });
          continue;
        }
        here = base;
      }
      const kind = resolveTarget(here, target);
      if (kind) out.push({ line: i + 1, target: raw, kind });
    }
  });
  return out;
}

const DECODER = new TextDecoder("utf-8", { fatal: true, ignoreBOM: true });

/** A file's text with its line endings made `\n`, or null when it cannot be read as UTF-8. */
export function readText(path: string): string | null {
  try {
    return DECODER.decode(readFileSync(path)).replace(/\r\n?/g, "\n");
  } catch {
    return null;
  }
}

function walk(dir: string, out: string[]): void {
  for (const e of readdirSync(dir, { withFileTypes: true })) {
    if (e.name === "target") continue;
    const p = join(dir, e.name);
    if (e.isDirectory()) walk(p, out);
    else if (e.isFile()) out.push(p);
  }
}

/** A page a renderer wrote, which announces `GENERATED FILE` in its first lines. */
export function isGenerated(text: string): boolean {
  return text.split("\n", GENERATED_HEAD_LINES).some((l) => l.includes(GENERATED_MARKER));
}

/** The findings in one file, absolute, read by its extension; empty when it is not UTF-8. */
export function fileFindings(path: string, text: string): Finding[] {
  const ext = extname(path);
  return MENTION_EXTS.includes(ext) ? deadMentions(text) : findingsIn(text, SOURCE_EXTS.includes(ext), dirname(path));
}

/** Every tracked file this gate reads, absolute, kept to those under one of `paths` when any is given. */
export async function trackedFiles(paths: string[]): Promise<string[]> {
  let files: string[];
  try {
    files = (await tracked()).map((p) => join(ROOT, p));
  } catch {
    // No git, or not a checkout: the tree itself, minus the build output.
    files = [];
    walk(ROOT, files);
  }
  const exts = [...DOC_EXTS, ...SOURCE_EXTS, ...MENTION_EXTS];
  files = files.filter((f) => exts.includes(extname(f)));
  if (paths.length === 0) return files;
  const prefixes = paths.map((p) => resolve(ROOT, p));
  return files.filter((f) => prefixes.some((pre) => f.startsWith(pre)));
}

/** The order the files are reported in: a path's own order, which ignores case on Windows. */
export function sortKey(path: string): string {
  return process.platform === "win32" ? path.split("/").join(sep).toLowerCase() : path;
}

export async function run(args: string[]): Promise<number> {
  if (args.some((a) => a === "-h" || a === "--help")) {
    console.log(`nv links: ${summary}`);
    return 0;
  }
  const files = await trackedFiles(args.filter((a) => !a.startsWith("-")));
  const ordered = files.map((f) => ({ f, key: sortKey(f) })).sort((a, b) => (a.key < b.key ? -1 : a.key > b.key ? 1 : 0));

  const findings: { rel: string; finding: Finding }[] = [];
  let generated = 0;
  for (const { f } of ordered) {
    const text = readText(f);
    if (text !== null && isGenerated(text)) {
      generated++;
      continue;
    }
    if (text === null) continue;
    for (const finding of fileFindings(f, text)) findings.push({ rel: rel(f), finding });
  }

  const lines: string[] = [];
  for (const { rel: path, finding } of findings) {
    lines.push(`  ${finding.kind.padEnd(8)} ${path}:${finding.line}  ->  ${finding.target}`);
  }
  const checked = files.length - generated;
  const skipped = generated ? `, ${generated} generated page(s) skipped` : "";
  if (findings.length > 0) {
    const counts = new Map<string, number>();
    for (const { finding } of findings) counts.set(finding.kind, (counts.get(finding.kind) ?? 0) + 1);
    const tally = [...counts].sort((a, b) => b[1] - a[1]).map(([kind, n]) => `${n} ${kind}`).join(", ");
    lines.push(
      "",
      `${findings.length} finding(s) across ${checked} file(s)${skipped}: ${tally}.`,
      "A mis-cased link resolves on Windows and macOS and 404s everywhere else.",
      "A source file cites from the repository root (`/docs/…`), a markdown file from itself.",
    );
  } else {
    lines.push(`every link in ${checked} file(s) resolves, with matching case and form${skipped}`);
  }
  process.stdout.write(lines.join("\n") + "\n");
  // A finding is a link that does not resolve, which is a defect rather than a preference.
  return findings.length > 0 ? 1 : 0;
}
