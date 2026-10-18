// `bun nv prose [--all] [<file>...]`: the shape of the Markdown an end user reads. It flags a sentence
// over `SENTENCE_MAX` words, a dash that joins two sentences, and a banned word. It never judges the
// meaning, which AGENTS.md § *Text an end user reads* binds and nothing checks.
//
// The banned words are the quoted ones on the *No filler* line of `docs/agent/prose.md`, which looper
// writes. Code, front matter, tables, HTML comments and link targets are not prose, and text in double
// quotes is a mention, so a banned word in quotes is not flagged.
//
// It reads the files `PATHS` names that differ from `HEAD` or are untracked, so old text never blocks a
// change. `--all` reads every file `PATHS` names, and a file argument reads that file whatever its path.
// It exits 1 on any problem. `looper.toml`'s `[gates] prose` makes `looper prose` run this command, so
// the verdict is the same with looper and without it. The rules are looper's `gates::prose`, ported.

import { readFileSync } from "node:fs";
import { join } from "node:path";
import { ROOT } from "../lib/paths.ts";
import { run as proc } from "../lib/proc.ts";

export const summary = "the shape of the end-user Markdown a change touched: nv prose [--all] [<file>...]";

/** The git pathspecs of the Markdown an end user reads. */
export const PATHS = ["website/**/*.mdx", "website/**/*.md", "**/about.md"];

/** A sentence has at most this many words. */
export const SENTENCE_MAX = 25;

/** The file whose *No filler* line names the banned words. */
const TEMPLATE = "docs/agent/prose.md";

export interface Problem {
  line: number;
  message: string;
}

const CODE_SPAN = /(`+)[^`]*?(`+)/g;
const INLINE_LINK = /!?\[([^\]]*)\]\(\s*<?([^)\s>]*)>?(?:\s+"[^"]*")?\s*\)/g;
const ITEM = /^\s*(?:[-*+]|[0-9]+[.)])\s+/;
const QUOTE = /"[^"]*"|“[^”]*”/g;
const DASH = /—|\s(?:–|--?)\s/;

/** The banned words: each quoted phrase on the *No filler* line of `text`. */
export function bannedIn(text: string): string[] {
  const line = text.split(/\r?\n/).find((l) => l.includes("**No filler.**"));
  return line ? [...line.matchAll(/"([^"]+)"/g)].map((m) => m[1]!) : [];
}

/** Each line of `text` and whether it is prose: not in a fence, front matter or HTML comment. */
function lines(text: string): { number: number; text: string; prose: boolean }[] {
  const out = [];
  let fence: string | null = null;
  let front = text.startsWith("---\n") || text.startsWith("---\r\n");
  let comment = false;
  const raws = text.split("\n");
  if (raws.at(-1) === "") raws.pop();
  for (const [i, line] of raws.entries()) {
    const raw = line.replace(/\r$/, "");
    const trimmed = raw.trimStart();
    let prose = true;
    if (front) {
      prose = false;
      if (i > 0 && raw.trimEnd() === "---") front = false;
    } else if (fence !== null) {
      prose = false;
      if (trimmed.startsWith(fence)) fence = null;
    } else if (trimmed.startsWith("```") || trimmed.startsWith("~~~")) {
      prose = false;
      fence = trimmed.slice(0, 3);
    } else if (comment || trimmed.startsWith("<!--")) {
      prose = false;
      comment = !raw.includes("-->");
    }
    out.push({ number: i + 1, text: raw, prose });
  }
  return out;
}

const escape = (s: string) => s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");

/** Every prose problem in one Markdown file's `text`, flagging the words in `banned`. */
export function proseIn(text: string, banned: string[]): Problem[] {
  const words = banned.filter((w) => w.trim()).map((w) => w.trim().split(/\s+/).map(escape).join("\\s+"));
  const bannedRe = words.length ? new RegExp(`\\b(?:${words.join("|")})\\b`, "gi") : null;
  const out: Problem[] = [];
  // A unit is a paragraph, a list item or a heading: its first line and its text, one line per `\n`.
  const units: [number, string][] = [];
  let open = false;
  for (const l of lines(text)) {
    let t = l.text.trim();
    if (!l.prose || !t || t.startsWith("|")) {
      open = false;
      continue;
    }
    t = t.replace(/^>+/, "").trim();
    const clean = t.replace(CODE_SPAN, "x").replace(INLINE_LINK, "$1").replace(ITEM, "");
    if (DASH.test(clean)) out.push({ line: l.number, message: "a dash joins two sentences: write two sentences, or use a colon" });
    if (bannedRe) {
      for (const m of clean.replace(QUOTE, "x").matchAll(bannedRe)) out.push({ line: l.number, message: `a banned word: ${m[0]}` });
    }
    const starts = t.startsWith("#") || ITEM.test(l.text);
    const last = units.at(-1);
    if (open && !starts && last) last[1] += `\n${clean}`;
    else units.push([l.number, clean.replace(/^#+/, "").trim()]);
    open = !t.startsWith("#");
  }
  for (const [first, unit] of units) {
    const chars = [...unit];
    let start = 0;
    let at = 0;
    for (const [k, c] of chars.entries()) {
      at += c.length;
      const next = chars.slice(k + 1).find((n) => !`*_)"'”’`.includes(n));
      const ends = ".!?".includes(c) && (next === undefined || /\s/.test(next));
      if (!ends && k + 1 !== chars.length) continue;
      const sentence = unit.slice(start, at);
      const split = sentence.split(/\s+/).filter((w) => /[\p{L}\p{N}]/u.test(w));
      if (split.length > SENTENCE_MAX) {
        const lead = sentence.length - sentence.trimStart().length;
        const line = first + (unit.slice(0, start + lead).match(/\n/g)?.length ?? 0);
        const shown = sentence.trim().split(/\s+/).slice(0, 8).join(" ");
        out.push({ line, message: `a sentence of ${split.length} words, over ${SENTENCE_MAX}: ${shown} …` });
      }
      start = at;
    }
  }
  return out.sort((a, b) => a.line - b.line);
}

/** The files `git <args>` prints, NUL-separated, or null when git fails. */
async function gitFiles(args: string[]): Promise<string[] | null> {
  const r = await proc(["git", ...args]);
  return r.code === 0 ? r.stdout.split("\0").filter(Boolean) : null;
}

/** The files of `PATHS` that differ from `HEAD` or are untracked, or every one when `all`. */
async function selected(all: boolean): Promise<string[] | null> {
  const listed = () => gitFiles(["ls-files", "-z", "-co", "--exclude-standard", "--", ...PATHS]);
  if (all) return listed();
  const changed = await gitFiles(["diff", "--name-only", "-z", "HEAD", "--", ...PATHS]);
  if (changed === null) return listed();
  const untracked = await gitFiles(["ls-files", "-z", "-o", "--exclude-standard", "--", ...PATHS]);
  return [...new Set([...changed, ...(untracked ?? [])])].sort();
}

export async function run(args: string[]): Promise<number> {
  const all = args.includes("--all");
  const named = args.filter((a) => a !== "--all");
  if (named.some((a) => a.startsWith("-"))) {
    console.error("usage: bun nv prose [--all] [<file>...]");
    return 2;
  }
  const files = named.length ? named : await selected(all);
  if (files === null) {
    console.error("nv prose: `git ls-files` did not run, so the tree was not read");
    return 1;
  }
  const banned = bannedIn(readFileSync(join(ROOT, TEMPLATE), "utf8"));
  let found = 0;
  for (const file of files) {
    let text: string;
    try {
      text = readFileSync(named.length ? file : join(ROOT, file), "utf8");
    } catch {
      if (named.length) {
        console.error(`nv prose: cannot read ${file}`);
        return 2;
      }
      continue;
    }
    for (const p of proseIn(text, banned)) {
      console.log(`${file.replaceAll("\\", "/")}:${p.line}: ${p.message}`);
      found++;
    }
  }
  if (found) return 1;
  console.log(`the prose of ${named.length ? "the named files" : all ? "every end-user Markdown file" : "the touched end-user Markdown files"} is in shape`);
  return 0;
}
