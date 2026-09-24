// The gaps' legacy home: every `# Known gaps` block in a crate's module doc, read the way
// `tools/owners.py` reads it. Each item becomes `data/gaps/<crate>/<slug>.json`.
//
// A block opens on a `# Known gaps` heading, which runs to the next heading of the same or a higher
// level, or on a `**Known gaps**` bold run, which runs to the next bold run or heading. Its items are
// its numbered or bulleted lines, and a block that lists none is one item. An item's owner is its
// trailing `— owner: <who>` line: a milestone tag (`M10`) or a goal slug.
//
// An item's `title` is the bold run it opens on, or its first sentence when it opens on none, or the
// heading's own words after `Known gap:`; `text` is the rest, unwrapped. The slug is the title's
// first words, lower case, past a leading article, and grows a word at a time until it is unique in
// its crate: it is written once and never derived again, so a later edit to the title keeps it.
//
// A gap is also cited by its position, `nvs-ir`'s known gap 7, and a position is what the cutover
// removes. `citations` finds each such citation in a text file and names the gap it points at today,
// if it can tell the module and that module has an item with that number.

import { existsSync } from "node:fs";
import { join } from "node:path";
import { gap } from "../schema/gap.ts";
import { text, unwrap, walk, type Importer, type ImportResult } from "./lib.ts";

const DOC = /^\s*\/\/!(?: ?(.*))?$/;
const HEADING = /^(#{1,6})\s+(\S.*?)\s*$/;
const BOLD = /^\*\*(.+?)(?:\*\*|$)/;
const GAPS = /^Known gaps?\b/i;
const ITEM = /^(?:(\d+)\.|[-*])\s+(\S.*)$/;
const TAG = /^—\s*owner:\s*(\S+)\s*$/;
const TAG_LIKE = /owner:\s*\S/i;
const MILESTONE = /^M\d+[A-Z]?$/;
const SLUG = /^[a-z0-9]+(?:-[a-z0-9]+)*$/;
const SLUG_WORDS = 6;

type Line = [number, string];

/** Every contiguous `//!` run in a file's lines, each line with its 1-based number. */
function docRuns(lines: string[]): Line[][] {
  const runs: Line[][] = [];
  let run: Line[] = [];
  lines.forEach((line, i) => {
    const m = DOC.exec(line);
    if (m) run.push([i + 1, m[1] ?? ""]);
    else if (run.length > 0) {
      runs.push(run);
      run = [];
    }
  });
  if (run.length > 0) runs.push(run);
  return runs;
}

/** Every gap block in one doc run, in the order they open: its line, its title and its body. */
function blocks(run: Line[]): { line: number; title: string; body: Line[] }[] {
  const heads: [number, number, string][] = [];
  const bolds: [number, string, number][] = [];
  run.forEach(([, t], i) => {
    const h = HEADING.exec(t);
    if (h) heads.push([i, h[1]!.length, h[2]!]);
    const b = BOLD.exec(t);
    if (b) bolds.push([i, b[1]!, b[0].length]);
  });
  const stops = [...heads.map((h) => h[0]), ...bolds.map((b) => b[0])].sort((a, b) => a - b);
  const found: { idx: number; title: string; body: Line[] }[] = [];
  heads.forEach(([idx, level, title], n) => {
    if (!GAPS.test(title)) return;
    const next = heads.slice(n + 1).find(([, l]) => l <= level);
    found.push({ idx, title, body: run.slice(idx + 1, next ? next[0] : run.length) });
  });
  for (const [idx, phrase, cut] of bolds) {
    if (!GAPS.test(phrase)) continue;
    const end = stops.find((i) => i > idx) ?? run.length;
    const [line, t] = run[idx]!;
    found.push({ idx, title: phrase, body: [[line, t.slice(cut).replace(/^[ ,.:—-]+/, "")], ...run.slice(idx + 1, end)] });
  }
  return found.sort((a, b) => a.idx - b.idx).map((b) => ({ line: run[b.idx]![0], title: b.title, body: b.body }));
}

interface Item {
  num: number;
  line: number;
  /** The item's lines, its list marker cut off the first. */
  lines: string[];
  /** The heading's words after `Known gap:`, when the block is one item stated in its heading. */
  stated: string;
}

/** The items of one block, and the line of any prose the block opens on before its first item. */
function itemsOf(body: Line[], title: string): { items: Item[]; preamble: number | null } {
  const starts: [number, number][] = [];
  body.forEach(([, t], i) => {
    const m = ITEM.exec(t);
    if (m) starts.push([i, m[1] ? Number(m[1]) : starts.length + 1]);
  });
  if (starts.length === 0) {
    const lines = body.map(([, t]) => t);
    if (lines.join("").trim() === "") return { items: [], preamble: null };
    const colon = title.indexOf(":");
    return { items: [{ num: 1, line: body[0]![0], lines, stated: colon < 0 ? "" : title.slice(colon + 1).trim() }], preamble: null };
  }
  const lead = body.slice(0, starts[0]![0]).find(([, t]) => t.trim() !== "");
  const items = starts.map(([idx, num], n) => {
    const chunk = body.slice(idx, n + 1 < starts.length ? starts[n + 1]![0] : body.length);
    return { num, line: chunk[0]![0], lines: [ITEM.exec(chunk[0]![1])![2]!, ...chunk.slice(1).map(([, t]) => t)], stated: "" };
  });
  return { items, preamble: lead ? lead[0] : null };
}

/** The item's owner and the lines before its tag, or why the tag could not be read. */
function tagOf(lines: string[]): { owner: string; rest: string[] } | { why: string } {
  const full = lines.map((l) => l.trim()).filter((l) => l !== "");
  if (full.length === 0) return { why: "the item is empty" };
  const tagged = full.filter((l) => TAG.test(l));
  if (tagged.length > 1) return { why: "two owner tags in one item" };
  if (tagged.length === 1) {
    if (tagged[0] !== full[full.length - 1]) return { why: "the owner tag is not the item's last line" };
    const at = lines.map((l) => l.trim()).lastIndexOf(tagged[0]!);
    return { owner: TAG.exec(tagged[0]!)![1]!, rest: lines.slice(0, at) };
  }
  return { why: full.some((l) => TAG_LIKE.test(l)) ? "an owner is named but not as a trailing `— owner: <who>` line" : "no owner tag" };
}

/** An item's title and text: its opening bold run, else its first sentence, and what follows. */
export function titled(lines: string[], stated: string): { title: string; text: string } {
  const all = unwrap(lines);
  if (stated !== "") return { title: stated, text: all };
  const bold = /^\*\*([\s\S]+?)\*\*([\s\S]*)$/.exec(all);
  if (bold) return { title: bold[1]!.trim(), text: bold[2]!.replace(/^[\s,;:—-]+/, "").trim() };
  const sentence = /^([\s\S]+?[.!?])\s+(?=[A-Z`*[(§"])([\s\S]*)$/.exec(all);
  return sentence ? { title: sentence[1]!, text: sentence[2]!.trim() } : { title: all, text: "" };
}

/**
 * The title's first `words` words as a slug, without Markdown and past a leading article or a
 * leading `§ N's`, which names a section of a record rather than the gap.
 */
export function slugOf(title: string, words: number = SLUG_WORDS): string {
  const plain = title
    .replace(/\[([^\]]*)\]\([^)]*\)/g, "$1")
    .replace(/^§\s*\d+['’]s\s+/, "")
    .replace(/['’]/g, "")
    .toLowerCase();
  const all = plain.split(/[^a-z0-9]+/).filter((w) => w !== "");
  if (all.length > 1 && ["a", "an", "the"].includes(all[0]!)) all.shift();
  return all.slice(0, words).join("-");
}

/** Where one gap sits in the legacy tree: its module and its number there. */
export interface Position {
  module: string;
  num: number;
  id: string;
}

export function readGaps(root: string): ImportResult & { positions: Position[] } {
  const out: ImportResult & { positions: Position[] } = { records: [], unread: [], files: 0, positions: [] };
  const taken = new Map<string, Set<string>>();
  for (const path of walk(root, "crates", ".rs")) {
    const crate = /^crates\/([^/]+)\/src\//.exec(path)?.[1];
    if (!crate) continue;
    const src = text(root, path);
    if (!/Known gap/i.test(src)) continue;
    let any = false;
    for (const run of docRuns(src.split("\n"))) {
      for (const block of blocks(run)) {
        any = true;
        const { items, preamble } = itemsOf(block.body, block.title);
        if (preamble !== null) out.unread.push({ path, reason: `line ${preamble}: the gap block at line ${block.line} opens on prose no gap holds` });
        for (const item of items) {
          const tag = tagOf(item.lines);
          if ("why" in tag) {
            out.unread.push({ path, reason: `line ${item.line}: gap ${item.num}: ${tag.why}` });
            continue;
          }
          const owner = MILESTONE.test(tag.owner) ? { milestone: tag.owner } : SLUG.test(tag.owner) && tag.owner !== "unowned" ? { goal: tag.owner } : null;
          if (!owner) {
            out.unread.push({ path, reason: `line ${item.line}: gap ${item.num}: its owner \`${tag.owner}\` is neither a milestone nor a goal slug` });
            continue;
          }
          const { title, text: body } = titled(tag.rest, item.stated);
          const used = taken.get(crate) ?? new Set<string>();
          taken.set(crate, used);
          let slug = slugOf(title);
          for (let n = SLUG_WORDS + 1; used.has(slug) && slugOf(title, n) !== slug; n++) slug = slugOf(title, n);
          for (let k = 2; used.has(slug); k++) slug = `${slugOf(title)}-${k}`;
          used.add(slug);
          const id = `${crate}/${slug}`;
          out.records.push({ type: gap, id, value: { module: path, title, text: body, ...owner }, from: path });
          out.positions.push({ module: path, num: item.num, id });
        }
      }
    }
    if (any) out.files++;
  }
  return out;
}

export const gaps: Importer = { name: "gaps", read: (root) => readGaps(root) };

/** One citation of a gap by its position, and the gap it names today, when it can be told. */
export interface Citation {
  path: string;
  line: number;
  num: number;
  /** The module the citation names, or null when its wording does not say. */
  module: string | null;
  /** The gap at that position today, or null when that module has no item with that number. */
  gap: string | null;
}

const CITE = /known[- ]gaps?`?\s+(?:item\s+|§\s*|#)?(\d+)\b/gi;
const PATH = /crates\/[\w.\/-]+?\.rs/g;
const CRATE = /`(nvs[-_][a-z0-9_-]*)((?:::[a-z_][a-z0-9_]*)*)`/g;
const HERE = /\b(?:this|the) module(?:'s| doc)|\bmodule doc\b|\bcrate(?:'s)? docs?'?|\bcrate's own\b/g;

/** The module a Rust path names: `nvs_stdlib::db` is `crates/nvs-stdlib/src/db.rs` or `db/mod.rs`. */
function moduleOf(root: string, crate: string, parts: string[]): string | null {
  const base = `crates/${crate.replace(/_/g, "-")}/src`;
  const candidates = parts.length === 0 ? [`${base}/lib.rs`, `${base}/main.rs`] : [`${base}/${parts.join("/")}.rs`, `${base}/${parts.join("/")}/mod.rs`];
  return candidates.find((c) => existsSync(join(root, c))) ?? null;
}

/** The last match of `re` in `s`, as its index and its value. */
function lastOf(re: RegExp, s: string): RegExpExecArray | null {
  let last: RegExpExecArray | null = null;
  for (const m of s.matchAll(re)) last = m as RegExpExecArray;
  return last;
}

/** Every citation of a gap by position in one file, each resolved against `positions` when it can be. */
export function citations(root: string, path: string, src: string, positions: Position[]): Citation[] {
  const out: Citation[] = [];
  const lines = src.split("\n");
  const own = /^crates\/([^/]+)\//.exec(path)?.[1] ?? null;
  lines.forEach((line, i) => {
    for (const m of line.matchAll(CITE)) {
      const before = `${i > 0 ? lines[i - 1] : ""}\n${line.slice(0, m.index)}`;
      const found: [number, string | null][] = [];
      const p = lastOf(PATH, before);
      if (p) found.push([p.index, p[0]]);
      const c = lastOf(CRATE, before);
      if (c) found.push([c.index, moduleOf(root, c[1]!, c[2]!.split("::").filter((x) => x !== ""))]);
      const h = lastOf(HERE, before);
      if (h && own) found.push([h.index, /crate/.test(h[0]) || !path.includes("/src/") ? moduleOf(root, own, []) : path]);
      const module = found.sort((a, b) => b[0] - a[0])[0]?.[1] ?? null;
      const num = Number(m[1]);
      const hit = module === null ? undefined : positions.find((x) => x.module === module && x.num === num);
      out.push({ path, line: i + 1, num, module, gap: hit?.id ?? null });
    }
  });
  return out;
}
