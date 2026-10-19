// The gates beside lint that every project shares: links, the README's layout table, and prose. Each
// ports its namesake in `crates/looper-core/src/gates.rs`, with the same messages. Lint reads the
// headings here too.
//
// Each gate reads files and reports issues, as lint does, and never rewrites a file. The files a gate
// reads are the ones git lists as tracked, or untracked and not ignored (`listed`), so build output
// and scratch are never read.
//
// - `links` reads the inline links and reference definitions, outside code, of the files `[links]
//   paths` names, `*.md` by default. A link with a scheme such as `https:` is skipped. A relative target is resolved from the file's
//   directory, and one that starts with `/` from the root. It reports a target that does not exist, a
//   target whose case differs from the name on disk, and a `#anchor` that names no heading of the
//   target. An anchor is a heading's GitHub slug: lower case, punctuation dropped, spaces as `-`, and
//   `-1`, `-2` on a repeated heading.
// - `layout` compares the table under the README's `## Layout` with the top-level entries. A row names
//   its paths in backticks in its first cell, and may name several. An entry whose name starts with
//   `.` needs no row. It reports an entry no row names, and a row none of whose paths exists.
// - `prose` checks the shape of the Markdown prose, never its meaning: a sentence over `SENTENCE_MAX`
//   words, a dash that joins two sentences, and a banned word. The banned words are the quoted ones on
//   the *No filler* line of the `docs/agent/prose.md` template, and the project's `[prose] banned`.
//   Code, front matter, tables, HTML comments and link targets are not prose. Text in double quotes is
//   a mention. It reads the files `[prose] paths` names, and by default only the ones that differ
//   from `HEAD` or are untracked (`touched`), so old text never blocks a change.
//
// A line is prose unless it is in a fence, the front matter or an HTML comment. A heading counts only
// on a prose line. Each gate holds one file's text and one listing of the tree at a time, and caches
// nothing.

import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, join } from "node:path";
import type { Config } from "../goal/config.ts";
import type { Issue } from "../goal/lint.ts";
import { lines as textLines } from "../goal/model.ts";
import { CORE, profile } from "../sync/template.ts";

/** A sentence holds at most this many words. */
export const SENTENCE_MAX = 25;
/** The template that lists the shared banned words. */
export const PROSE_TEMPLATE = "docs/agent/prose.md";
/** The README at the root, which holds the layout table. */
export const README = "README.md";

/** The NUL-separated paths `git <args>` prints at `root`, or `undefined` when it does not run or fails. */
function gitFiles(root: string, args: string[]): string[] | undefined {
  let ran: ReturnType<typeof Bun.spawnSync>;
  try {
    ran = Bun.spawnSync(["git", ...args], { cwd: root, stdin: "ignore", stdout: "pipe", stderr: "ignore" });
  } catch {
    return undefined;
  }
  if (ran.exitCode !== 0) return undefined;
  return (ran.stdout as Buffer)
    .toString("utf8")
    .split("\0")
    .filter((f) => f !== "");
}

function sortedUnique(files: string[]): string[] {
  return [...new Set(files)].sort((a, b) => (a < b ? -1 : a > b ? 1 : 0));
}

/**
 * The files git lists at `root` for `pathspecs`, tracked or untracked and not ignored, sorted. A tracked
 * file deleted from the disk is not listed. An issue says why when git does not run.
 */
export function listed(root: string, pathspecs: string[]): string[] | Issue {
  const files = gitFiles(root, ["ls-files", "-z", "-co", "--exclude-standard", "--", ...pathspecs]);
  if (files === undefined) return { file: ".", line: undefined, message: "`git ls-files` did not run, so the tree was not read" };
  return sortedUnique(files.filter((f) => isFile(join(root, f))));
}

/** The files that differ from `HEAD` or are untracked and not ignored, for `pathspecs`. In a repository with no commit, every listed file. */
export function touched(root: string, pathspecs: string[]): string[] | Issue {
  const changed = gitFiles(root, ["diff", "--name-only", "-z", "HEAD", "--", ...pathspecs]);
  if (changed === undefined) return listed(root, pathspecs);
  const untracked = gitFiles(root, ["ls-files", "-z", "-o", "--exclude-standard", "--", ...pathspecs]) ?? [];
  return sortedUnique([...changed, ...untracked].filter((f) => isFile(join(root, f))));
}

function isFile(path: string): boolean {
  try {
    return statSync(path).isFile();
  } catch {
    return false;
  }
}

/** The text of the file at `path`, or `undefined` when it cannot be read. */
function readText(path: string): string | undefined {
  try {
    return readFileSync(path, "utf8");
  } catch {
    return undefined;
  }
}

/** One line of a Markdown file and whether it is prose. */
interface Line {
  number: number;
  text: string;
  prose: boolean;
}

/** Each line of `text`, marked prose or not. */
function lines(text: string): Line[] {
  const out: Line[] = [];
  let fence: string | undefined;
  let front = text.startsWith("---\n") || text.startsWith("---\r\n");
  let comment = false;
  textLines(text).forEach((raw, i) => {
    const trimmed = raw.trimStart();
    let prose = true;
    if (front) {
      prose = false;
      if (i > 0 && raw.trimEnd() === "---") front = false;
    } else if (fence !== undefined) {
      prose = false;
      if (trimmed.startsWith(fence)) fence = undefined;
    } else if (trimmed.startsWith("```") || trimmed.startsWith("~~~")) {
      prose = false;
      fence = trimmed.slice(0, 3);
    } else if (comment || trimmed.startsWith("<!--")) {
      prose = false;
      comment = !raw.includes("-->");
    }
    out.push({ number: i + 1, text: raw, prose });
  });
  return out;
}

const CODE_SPAN = /(`+)[^`]*?(`+)/g;
const HEADING = /^(#{1,6})\s+(.*?)\s*#*\s*$/;
const INLINE_LINK = /!?\[([^\]]*)\]\(\s*<?([^)\s>]*)>?(?:\s+"[^"]*")?\s*\)/g;
const REFERENCE = /^\s{0,3}\[[^\]]+\]:\s+<?([^\s>]+)>?/;

/** One heading of a Markdown file, outside code: its line, its level and its text. */
export interface Heading {
  line: number;
  level: number;
  text: string;
}

/** Every heading of `text` outside fences, front matter and HTML comments. */
export function headings(text: string): Heading[] {
  const out: Heading[] = [];
  for (const l of lines(text)) {
    const c = l.prose ? HEADING.exec(l.text) : null;
    if (c) out.push({ line: l.number, level: (c[1] as string).length, text: c[2] as string });
  }
  return out;
}

/** The anchor GitHub gives a heading whose text is `heading`, before a repeat adds its suffix. */
export function slug(heading: string): string {
  const text = heading.replace(INLINE_LINK, "$1").trim().toLowerCase();
  let out = "";
  for (const c of text) {
    if (c === " ") out += "-";
    else if (/^[\p{L}\p{N}_-]$/u.test(c)) out += c;
  }
  return out;
}

/** Every anchor the headings of `text` give, with `-1`, `-2` on a repeated one. */
function anchors(text: string): Set<string> {
  const seen = new Map<string, number>();
  const out = new Set<string>();
  for (const h of headings(text)) {
    const base = slug(h.text);
    const n = seen.get(base) ?? 0;
    out.add(n === 0 ? base : `${base}-${n}`);
    seen.set(base, n + 1);
  }
  return out;
}

/** Every dead, miscased or unanchored link in the files `[links] paths` names at `root`. An empty list reads none. */
export function links(root: string, cfg: Config): Issue[] {
  if (cfg.links.paths.length === 0) return [];
  const found = listed(root, cfg.links.paths);
  if (!Array.isArray(found)) return [found];
  const out: Issue[] = [];
  for (const file of found) {
    const text = readText(join(root, file));
    if (text === undefined) continue;
    for (const l of lines(text)) {
      if (!l.prose) continue;
      const line = l.text.replace(CODE_SPAN, "");
      const targets = [...line.matchAll(INLINE_LINK)].map((c) => c[2] as string);
      const ref = REFERENCE.exec(line);
      if (ref) targets.push(ref[1] as string);
      for (const target of targets) {
        const message = checkLink(root, file, text, target);
        if (message !== undefined) out.push({ file, line: l.number, message });
      }
    }
  }
  return out;
}

/** What is wrong with the link `target` in `file`, whose text is `text`, or `undefined` when it resolves. */
function checkLink(root: string, file: string, text: string, target: string): string | undefined {
  const colon = target.indexOf(":");
  const scheme = colon > 1 && /^[A-Za-z0-9+.-]+$/.test(target.slice(0, colon));
  if (target === "" || scheme) return undefined;
  const hash = target.indexOf("#");
  const path = (hash < 0 ? target : target.slice(0, hash)).replaceAll("%20", " ");
  const anchor = hash < 0 ? undefined : target.slice(hash + 1);
  let targetText: string | undefined;
  if (path === "") {
    targetText = text;
  } else {
    const parts = path.startsWith("/") ? [] : file.split("/").slice(0, -1);
    for (const c of path.replace(/^\/+/, "").split(/[\\/]/)) {
      if (c === "" || c === ".") continue;
      if (c === "..") {
        if (parts.pop() === undefined) {
          const outside = join(dirname(join(root, file)), path);
          return existsSync(outside) ? undefined : `a dead link: ${target}`;
        }
      } else {
        parts.push(c);
      }
    }
    const disk = onDisk(root, parts);
    if (disk.kind === "missing") return `a dead link: ${target}`;
    if (disk.kind === "miscased") return `a link whose case differs from the file on disk: ${target} is ${disk.real}`;
    targetText = disk.full.endsWith(".md") ? readText(disk.full) : undefined;
  }
  if (anchor !== undefined && anchor !== "" && targetText !== undefined && !anchors(targetText).has(anchor)) {
    return `an anchor that names no heading: ${target}`;
  }
  return undefined;
}

/** Where a path under the root stands on disk. A miscased one exists, and `real` spells it as the disk does. */
type Disk = { kind: "found"; full: string } | { kind: "missing" } | { kind: "miscased"; real: string };

/** Whether `parts` under `root` exists, comparing each part's case with the directory's entry. */
function onDisk(root: string, parts: string[]): Disk {
  let at = root;
  const real: string[] = [];
  let miscased = false;
  for (const part of parts) {
    let names: string[];
    try {
      names = readdirSync(at);
    } catch {
      return { kind: "missing" };
    }
    let name: string | undefined;
    if (names.includes(part)) {
      name = part;
    } else {
      name = names.find((n) => n.toLowerCase() === part.toLowerCase());
      if (name === undefined) return { kind: "missing" };
      miscased = true;
    }
    at = join(at, name);
    real.push(name);
  }
  return miscased ? { kind: "miscased", real: real.join("/") } : { kind: "found", full: at };
}

/** Every top-level entry no row of the README's `## Layout` table names, and every row none of whose paths exists. */
export function layout(root: string): Issue[] {
  const issue = (line: number | undefined, message: string): Issue => ({ file: README, line, message });
  const text = readText(join(root, README));
  if (text === undefined) return [issue(undefined, "there is no README.md to hold the layout table")];
  const rows = layoutRows(text);
  if (rows.length === 0) return [issue(undefined, "there is no table under `## Layout`")];
  const found = listed(root, []);
  if (!Array.isArray(found)) return [found];
  const entries = sortedUnique(found.map((f) => f.split("/")[0] as string).filter((e) => !e.startsWith(".")));
  const named = new Set(rows.flatMap(([, names]) => names));
  const out = entries
    .filter((e) => !named.has(e))
    .map((e) => issue(undefined, `\`${e}\` is at the top level, and no row of \`## Layout\` names it`));
  for (const [line, names] of rows) {
    if (!names.some((n) => existsSync(join(root, n)))) {
      out.push(issue(line, `the row for \`${names.join("`, `")}\` names nothing in the tree`));
    }
  }
  return out;
}

/** Each row of the table under `## Layout`: its line, and the paths its first cell names, each without a trailing `/`. */
function layoutRows(text: string): [number, string[]][] {
  const all = lines(text);
  const start = all.findIndex((l) => l.prose && l.text.trimEnd() === "## Layout");
  if (start < 0) return [];
  const rows: [number, string[]][] = [];
  let header = true;
  for (const l of all.slice(start + 1)) {
    const t = l.text.trim();
    if (l.prose && t.startsWith("## ")) break;
    if (!t.startsWith("|")) {
      if (rows.length > 0) break;
      continue;
    }
    const wasHeader = header;
    header = false;
    if (wasHeader || /^\|[\s:|-]+$/.test(t)) continue;
    const cell = (t.replace(/^\|+/, "").split("|")[0] ?? "").trim();
    const names = [...cell.matchAll(/`([^`]+)`/g)].map((c) => c[1] as string);
    if (names.length === 0) names.push(cell);
    rows.push([
      l.number,
      names.map((n) => n.replace(/^(\.\/)+/, "").replace(/\/+$/, "")).filter((n) => n !== ""),
    ]);
  }
  return rows;
}

/** The shared banned words: each quoted phrase on the *No filler* line of `PROSE_TEMPLATE`. */
export function sharedBanned(): string[] {
  const text = profile(CORE)?.find((t) => t.path === PROSE_TEMPLATE)?.text ?? "";
  const line = textLines(text).find((l) => l.includes("**No filler.**"));
  return line === undefined ? [] : [...line.matchAll(/"([^"]+)"/g)].map((c) => c[1] as string);
}

/** Every prose problem in the project at `root`: in the touched files of `[prose] paths`, or in all of them when `all`. */
export function prose(root: string, cfg: Config, all: boolean): Issue[] {
  const paths = cfg.prose.paths;
  if (paths.length === 0) return [];
  const found = all ? listed(root, paths) : touched(root, paths);
  if (!Array.isArray(found)) return [found];
  const banned = [...sharedBanned(), ...cfg.prose.banned];
  return found.flatMap((f) => {
    const text = readText(join(root, f));
    return text === undefined ? [] : proseIn(f, text, banned);
  });
}

function escapeRegex(s: string): string {
  return s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

const ITEM = /^\s*(?:[-*+]|[0-9]+[.)])\s+/;
const QUOTE = /"[^"]*"|“[^”]*”/g;
const DASH = /—|\s(?:–|--?)\s/;
/** The characters that may close a sentence after its stop. */
const CLOSERS = "*_)\"'”’";

/** Every prose problem in one Markdown file's `text`, flagging the words in `banned`. */
export function proseIn(file: string, text: string, banned: string[]): Issue[] {
  const words = banned
    .filter((w) => w.trim() !== "")
    .map((w) => w.trim().split(/\s+/).map(escapeRegex).join("\\s+"));
  const bannedRe = words.length > 0 ? new RegExp(`\\b(?:${words.join("|")})\\b`, "gi") : undefined;

  const out: Issue[] = [];
  const at = (line: number, message: string) => out.push({ file, line, message });
  // A unit is a paragraph, a list item or a heading: its first line, and its text with one line per `\n`.
  const units: [number, string][] = [];
  let open = false;
  for (const l of lines(text)) {
    let t = l.text.trim();
    if (!l.prose || t === "" || t.startsWith("|")) {
      open = false;
      continue;
    }
    t = t.replace(/^>+/, "").trim();
    let clean = t.replace(CODE_SPAN, "x").replace(INLINE_LINK, "$1");
    clean = clean.replace(ITEM, "");
    if (DASH.test(clean)) at(l.number, "a dash joins two sentences: write two sentences, or use a colon");
    if (bannedRe !== undefined) {
      for (const m of clean.replace(QUOTE, "x").matchAll(bannedRe)) at(l.number, `a banned word: ${m[0]}`);
    }
    const starts = t.startsWith("#") || ITEM.test(l.text);
    const last = units[units.length - 1];
    if (open && !starts && last !== undefined) last[1] += `\n${clean}`;
    else units.push([l.number, clean.replace(/^#+/, "").trim()]);
    open = !t.startsWith("#");
  }
  for (const [first, unit] of units) {
    const chars = Array.from(unit);
    let start = 0;
    let index = 0;
    chars.forEach((c, k) => {
      const end = index + c.length;
      index = end;
      const closed = chars.slice(k + 1).find((n) => !CLOSERS.includes(n));
      const ends = ".!?".includes(c) && (closed === undefined || /\s/.test(closed));
      if (!ends && k + 1 !== chars.length) return;
      const sentence = unit.slice(start, end);
      const all = sentence.split(/\s+/).filter((w) => w !== "");
      const count = all.filter((w) => /[\p{L}\p{N}]/u.test(w)).length;
      if (count > SENTENCE_MAX) {
        const lead = sentence.length - sentence.trimStart().length;
        const line = first + (unit.slice(0, start + lead).match(/\n/g)?.length ?? 0);
        at(line, `a sentence of ${count} words, over ${SENTENCE_MAX}: ${all.slice(0, 8).join(" ")} …`);
      }
      start = end;
    });
  }
  return out.sort((a, b) => (a.line ?? 0) - (b.line ?? 0));
}
