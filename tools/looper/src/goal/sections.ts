// The prose of a goal file, read and never rewritten, as `crates/looper-core/src/sections.rs` reads it:
// its `##` sections, the `### <id>. <title>` stage headings, the Handoff's `###` subsections, checklist
// items and anchors.
//
// - A heading is any line of exactly `level` hashes, a space and a name, code fences included.
// - `stageOutline` reads each stage's `**Does:**` text and items for the API. A stage's items are its
//   boxes at the shallowest indent. A box indented deeper is a checkpoint of the item above it, and
//   `stageItems` counts items only.
// - The outline keeps the bounds its constants name. A text's bound is in UTF-8 bytes, as looper's is.

/** The `##` sections a goal file holds, in this order. */
export const SECTIONS = ["Why", "Standing decisions", "Stages", "Handoff"] as const;

/** The `###` subsections of `## Handoff`. */
export const HANDOFF_SECTIONS = ["State", "Next", "Backlog"] as const;

/**
 * The most non-blank lines `### State` holds. Every turn of a session re-reads the pack, so a State that
 * logs each finished stage costs every later turn.
 */
export const STATE_MAX = 20;

/** A repo-rooted `path:line`: a path with a directory, or a root file with an extension (`Cargo.toml:1`). */
export const ANCHOR =
  /(?:[A-Za-z0-9_.@-]+\/)+[A-Za-z0-9_.@-]*[A-Za-z0-9_-]:[0-9]+|\b[A-Za-z0-9_@-]+(?:\.[A-Za-z0-9_-]+)*\.[A-Za-z0-9]+:[0-9]+/g;

const STAGE_HEADING = /^### ([0-9]+)\. (.+)$/;

/** One heading and the text below it, up to the next heading of the same level. */
export interface Section {
  name: string;
  /** The 1-based line of the heading within the text that was split. */
  line: number;
  /** The text between this heading and the next, trimmed. */
  text: string;
  /** The 1-based line within the text that was split on which `text` starts. */
  text_line: number;
}

/** Splits markdown at headings of exactly `level` hashes. The text must have LF line endings. */
export function sectionsOf(text: string, level: number): Section[] {
  const prefix = `${"#".repeat(level)} `;
  const marks: { name: string; line: number; start: number; end: number }[] = [];
  let start = 0;
  text.split("\n").forEach((line, i) => {
    if (line.startsWith(prefix) && line.length > prefix.length) {
      marks.push({ name: line.slice(prefix.length).trim(), line: i + 1, start, end: start + line.length });
    }
    start += line.length + 1;
  });
  return marks.map((m, i) => {
    const raw = text.slice(m.end, marks[i + 1]?.start ?? text.length);
    const lead = raw.slice(0, raw.length - raw.trimStart().length);
    return { name: m.name, line: m.line, text: raw.trim(), text_line: m.line + count(lead, "\n") };
  });
}

/** The section of that name. When a name appears twice, the last one holds. */
export function section(sections: Section[], name: string): Section | undefined {
  return sections.findLast((s) => s.name === name);
}

/** The section names in the order they first appear, each once. */
export function names(sections: Section[]): string[] {
  return [...new Set(sections.map((s) => s.name))];
}

/** A `### <id>. <title>` heading under `## Stages`. */
export interface StageHeading {
  id: number;
  title: string;
  /** The 1-based line within the text that was searched. */
  line: number;
}

export function stageHeadings(stagesText: string): StageHeading[] {
  const out: StageHeading[] = [];
  stagesText.split("\n").forEach((line, i) => {
    const c = STAGE_HEADING.exec(line);
    const id = c ? stageId(c[1] as string) : undefined;
    if (c && id !== undefined) out.push({ id, title: (c[2] as string).trim(), line: i + 1 });
  });
  return out;
}

/** A checklist line, `- [ ] …` or `- [x] …`, at any indent. */
export interface Item {
  done: boolean;
  /** The whole line, as written. */
  text: string;
  /** The 1-based line within the text that was searched. */
  line: number;
}

export function items(text: string): Item[] {
  const out: Item[] = [];
  text.split("\n").forEach((line, i) => {
    const rest = line.trimStart();
    if (rest.startsWith("- [ ]")) out.push({ done: false, text: line, line: i + 1 });
    else if (rest.startsWith("- [x]")) out.push({ done: true, text: line, line: i + 1 });
  });
  return out;
}

/**
 * How many of stage `id`'s items a goal file's body ticks, and how many it has, read from the stage's
 * `### <id>. <title>` section under `## Stages`. Its checkpoints are not counted.
 */
export function stageItems(body: string, id: number): [number, number] | undefined {
  const stages = section(sectionsOf(body, 2), "Stages");
  if (!stages) return undefined;
  const stage = sectionsOf(stages.text, 3).find((s) => s.name.startsWith(`${id}. `));
  if (!stage) return undefined;
  const all = nest(boxes(stage.text));
  return [all.filter(([i]) => i.done).length, all.length];
}

/** The most items a goal's outline holds, across its stages. */
export const OUTLINE_ITEMS = 500;
/** The most checkpoints an outline holds under one item. */
export const OUTLINE_CHECKPOINTS = 12;
/** The longest an item's or a `**Does:**` text is in an outline, in bytes. */
export const ITEM_TEXT = 400;
/** The longest a checkpoint's text is in an outline, in bytes. */
export const CHECKPOINT_TEXT = 200;

/** A goal's stages as its `## Stages` prose has them, in heading order, cut to the bounds above. */
export interface Outline {
  stages: StageOutline[];
  /** How many items past `OUTLINE_ITEMS` were left out. */
  items_more: number;
}

export interface StageOutline {
  id: number;
  /** The stage's `**Does:**` paragraph without its label, joined onto one line. */
  does: string | undefined;
  items: OutlineItem[];
}

/** A stage's item: a box at the stage's shallowest box indent. */
export interface OutlineItem {
  done: boolean;
  /** Its text without the box, its continuation lines joined. */
  text: string;
  /** The boxes indented under it. */
  checkpoints: Checkpoint[];
  /** How many checkpoints past `OUTLINE_CHECKPOINTS` were left out. */
  checkpoints_more: number;
}

export interface Checkpoint {
  done: boolean;
  text: string;
}

/**
 * The outline of every `### <id>. <title>` stage under the body's `## Stages`. A box indented deeper
 * than the stage's shallowest box is a checkpoint of the nearest item above it.
 */
export function stageOutline(body: string): Outline {
  const out: Outline = { stages: [], items_more: 0 };
  const stages = section(sectionsOf(body, 2), "Stages");
  if (!stages) return out;
  let room = OUTLINE_ITEMS;
  for (const s of sectionsOf(stages.text, 3)) {
    const dot = s.name.indexOf(". ");
    const id = dot < 0 ? undefined : stageId(s.name.slice(0, dot));
    if (id === undefined) continue;
    const all = nest(boxes(s.text));
    out.items_more += Math.max(0, all.length - room);
    const kept: OutlineItem[] = all.slice(0, room).map(([item, checks]) => ({
      done: item.done,
      text: cut(item.text, ITEM_TEXT),
      checkpoints: checks.slice(0, OUTLINE_CHECKPOINTS).map((c) => ({ done: c.done, text: cut(c.text, CHECKPOINT_TEXT) })),
      checkpoints_more: Math.max(0, checks.length - OUTLINE_CHECKPOINTS),
    }));
    room -= kept.length;
    const d = does(s.text);
    out.stages.push({ id, does: d === undefined ? undefined : cut(d, ITEM_TEXT), items: kept });
  }
  return out;
}

/** Every repo-rooted `path:line` anchor in the text. */
export function anchors(text: string): string[] {
  return [...text.matchAll(ANCHOR)].map((m) => m[0]);
}

/** A stage id as Rust's `u64` parse reads it, or `undefined`. */
function stageId(text: string): number | undefined {
  if (!/^\+?[0-9]+$/.test(text)) return undefined;
  const id = Number(text);
  return Number.isSafeInteger(id) ? id : undefined;
}

/** The paragraph that starts with `**Does:**`, without the label, its lines joined. */
function does(text: string): string | undefined {
  const lines = text.split("\n").map((l) => l.trim());
  const at = lines.findIndex((l) => l.startsWith("**Does:**"));
  if (at < 0) return undefined;
  let first = lines[at] as string;
  while (first.startsWith("**Does:**")) first = first.slice("**Does:**".length);
  const rest: string[] = [];
  for (const l of lines.slice(at + 1)) {
    if (l === "") break;
    rest.push(l);
  }
  return [first.trim(), ...rest].join(" ").trim();
}

/** A box line and the lines that continue it. */
interface Boxed {
  indent: number;
  done: boolean;
  text: string;
}

/**
 * Every box in the text, each with its continuation lines: the lines straight after it that are
 * neither blank nor a list entry of their own.
 */
function boxes(text: string): Boxed[] {
  const out: Boxed[] = [];
  let open = false;
  for (const line of text.split("\n")) {
    const rest = line.trimStart();
    const done = rest.startsWith("- [ ]") ? false : rest.startsWith("- [x]") ? true : undefined;
    if (done !== undefined) {
      out.push({ indent: line.length - rest.length, done, text: rest.slice("- [ ]".length).trim() });
      open = true;
    } else if (rest === "" || rest.startsWith("- ") || rest.startsWith("* ") || rest.startsWith("#")) {
      open = false;
    } else if (open && out.length > 0) {
      (out[out.length - 1] as Boxed).text += ` ${rest.trimEnd()}`;
    }
  }
  return out;
}

/**
 * The boxes at the shallowest indent, each with the deeper boxes after it. A deeper box before the
 * first item belongs to none and is dropped.
 */
function nest(all: Boxed[]): [Boxed, Boxed[]][] {
  const top = all.length === 0 ? 0 : Math.min(...all.map((b) => b.indent));
  const out: [Boxed, Boxed[]][] = [];
  for (const b of all) {
    if (b.indent === top) out.push([b, []]);
    else out[out.length - 1]?.[1].push(b);
  }
  return out;
}

/** The text cut to at most `max` UTF-8 bytes on a character boundary, ending in `…` when it was cut. */
function cut(text: string, max: number): string {
  if (Buffer.byteLength(text) <= max) return text;
  const room = max - Buffer.byteLength("…");
  let used = 0;
  let out = "";
  for (const ch of text) {
    used += Buffer.byteLength(ch);
    if (used > room) break;
    out += ch;
  }
  return `${out}…`;
}

function count(text: string, ch: string): number {
  return text.split(ch).length - 1;
}
