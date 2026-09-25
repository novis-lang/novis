// `bun nv plan`: the read-only modes over the implementation plan. `--show M8` prints a milestone's
// scope file whole, `--show M8:lead` its first paragraph and `--show M8:verify` its `**Verify:**`
// paragraph, its acceptance test. `--get <Field>` prints one field of the status block, unwrapped.
// `--past` prints one line per milestone the program is behind, and whether it is complete. `--stale`
// lists the sentences that defer work to a goal the chain has already walked. `--check` reports the
// status fields' sizes, and fails when a milestone record disagrees with its scope file or the chain.
//
// The roster and the status block are read from the `milestone` and `plan_status` records under
// `data/plan/`. A milestone's scope is prose, `docs/plan/<id>.md`, and its H1 is where the body starts.

import { existsSync, readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { chainGoals, liveGoal, walkedGoals } from "../lib/chain.ts";
import { ROOT } from "../lib/paths.ts";
import { load, pathOf } from "../lib/store.ts";
import { milestone as milestoneType } from "../schema/milestone.ts";
import { planStatus } from "../schema/plan-status.ts";
import { collect, FIRST_FUTURE_MILESTONE, registers } from "./owners.ts";

export const summary = "the plan, read-only: nv plan --show M8[:lead|:verify] | --get <Field> | --past | --stale | --check";

const PLAN = "docs/implementation-plan.md";

/** A milestone id; its number is its place in the program, and the suffix does not move it. */
const MILESTONE = /^M(\d+)[A-Z]?$/;

/** A milestone file's H1: `# M4S — The `Core` API contract and its pure half (~5 weeks)`. */
export const H1 = /^#\s+(M\d+[A-Z]?)\s*—\s*(.*)$/;

/** The status block's field names as the plan writes them, and each one's key in the record. */
export const FIELDS: [string, string][] = [
  ["Status", "status"],
  ["Done", "done"],
  ["On disk", "onDisk"],
  ["Toolchain", "toolchain"],
  ["ADR slices landed", "adrSlicesLanded"],
  ["Open now", "openNow"],
  ["Blocking", "blocking"],
];

export interface Entry {
  id: string;
  rel: string;
  record: string;
}

/** Every milestone, in the table's order. */
export function milestones(): Entry[] {
  return load(milestoneType)
    .sort((a, b) => a.value.order - b.value.order)
    .map((m) => ({ id: m.id, rel: `docs/plan/${m.id.toLowerCase()}.md`, record: pathOf(milestoneType, m.id) }));
}

/** `'x'`, the way Python's `repr` prints a string with no quote in it. */
/** One row of the plan's milestone table, as written: its four cells and its line. */
export interface TableRow {
  line: number;
  carried: string;
  id: string;
  link: string;
  builds: string;
  loopDays: string;
}

/** A row of the milestone table: `| <carried by> | [M4](plan/m4.md) | <what it builds> | <loop-days> |`. */
const TABLE_ROW = /^\|\s*([^|]*?)\s*\|\s*\[(M\d+[A-Z]?)\]\(([^)]*)\)\s*\|\s*(.*?)\s*\|\s*([^|]*?)\s*\|\s*$/;

/** Every row of the milestone table in `text`, in the order written. */
export function tableRows(text: string): TableRow[] {
  const rows: TableRow[] = [];
  text.split(/\r?\n/).forEach((line, i) => {
    const m = TABLE_ROW.exec(line);
    if (m) rows.push({ line: i + 1, carried: m[1]!, id: m[2]!, link: m[3]!, builds: m[4]!, loopDays: m[5]! });
  });
  return rows;
}

/** A milestone record's fields the table writes, and the goals on the chain that carry it. */
export interface RowSource {
  id: string;
  title: string;
  estimate?: string;
  loopDays: string;
  state: string;
  backlog?: number;
  carriers: string[];
}

/**
 * The row the table owes a milestone. **Carried by** is `done` for a finished one, else the goals that
 * carry it by slug (``goal `a` `` or ``goals `a`, `b` ``), else where it stands on its own: `ongoing` or
 * `backlog N`. **What it builds** is the record's title with its estimate in brackets.
 */
export function expectedRow(s: RowSource): Omit<TableRow, "line"> {
  const cited = s.carriers.map((g) => `\`${g}\``).join(", ");
  const carried =
    s.state === "done" ? "done" : s.carriers.length === 1 ? `goal ${cited}` : s.carriers.length > 1 ? `goals ${cited}` : s.backlog !== undefined ? `backlog ${s.backlog}` : s.state;
  return {
    carried,
    id: s.id,
    link: `plan/${s.id.toLowerCase()}.md`,
    builds: s.estimate === undefined ? s.title : `${s.title} (${s.estimate})`,
    loopDays: s.loopDays,
  };
}

function repr(s: string): string {
  return s.includes("'") && !s.includes('"') ? `"${s}"` : `'${s}'`;
}

/** A milestone file's text below its H1, with the blank lines at either end cut off. */
export function bodyOf(entry: Entry): string {
  const text = readFileSync(join(ROOT, entry.rel), "utf8").replace(/\r\n/g, "\n");
  const lines = text.split("\n");
  const at = lines.findIndex((l) => H1.test(l));
  return (at < 0 ? text : lines.slice(at + 1).join("\n")).replace(/^\n+|\n+$/g, "");
}

/** The first paragraph: what the milestone is, before the detail. */
export function leadParagraph(entry: Entry): string {
  const buf: string[] = [];
  for (const raw of bodyOf(entry).split("\n")) {
    if (!raw.trim()) {
      if (buf.length > 0) break;
      continue;
    }
    buf.push(raw.trim());
  }
  return buf.join(" ");
}

/** The `**Verify:**` or `**Verified:**` paragraph, or `""` when the milestone has none. */
export function verifyParagraph(entry: Entry): string {
  const lines = bodyOf(entry).split("\n");
  const at = lines.findIndex((l) => l.startsWith("**Verify:**") || l.startsWith("**Verified:**"));
  if (at < 0) return "";
  const buf: string[] = [];
  for (const line of lines.slice(at)) {
    if (!line.trim()) break;
    buf.push(line.trim());
  }
  return buf.join(" ");
}

function show(spec: string): number {
  const colon = spec.indexOf(":");
  const id = (colon < 0 ? spec : spec.slice(0, colon)).trim().toUpperCase();
  const part = colon < 0 ? "" : spec.slice(colon + 1);
  const index = milestones();
  const entry = index.find((m) => m.id === id);
  if (!entry) {
    console.log(`nv plan: no milestone ${repr(colon < 0 ? spec : spec.slice(0, colon))}. The index has: ` +
      index.map((m) => m.id).join(", "));
    return 1;
  }
  if (!existsSync(join(ROOT, entry.rel))) {
    console.log(`nv plan: ${entry.rel} does not exist (the record is ${entry.record})`);
    return 1;
  }
  if (part === "" || part === "all") {
    const text = readFileSync(join(ROOT, entry.rel), "utf8").replace(/\r\n/g, "\n");
    console.log(`-- ${entry.rel}\n\n${text.replace(/\n+$/, "")}`);
  } else if (part === "lead") {
    console.log(leadParagraph(entry));
  } else if (part === "verify") {
    const got = verifyParagraph(entry);
    if (!got) {
      console.log(`nv plan: ${entry.id} has no \`**Verify:**\` paragraph`);
      return 1;
    }
    console.log(got);
  } else {
    console.log(`nv plan: unknown part ${repr(part)} -- use \`:lead\`, \`:verify\`, or no suffix`);
    return 2;
  }
  return 0;
}

function get(name: string): number {
  const status = load(planStatus)[0]!.value as Record<string, string>;
  const hit = FIELDS.find(([field]) => field.toLowerCase() === name.toLowerCase());
  if (!hit) {
    console.log(`nv plan: no field ${repr(name)}. The block has: ${FIELDS.map(([f]) => f).join(", ")}`);
    return 1;
  }
  console.log(status[hit[1]]);
  return 0;
}

/** Python's `textwrap.fill` with `break_long_words` and `break_on_hyphens` off. */
function wrap(text: string, width: number, first: string, rest: string): string {
  const chunks = text.match(/\s+|\S+/g) ?? [];
  const lines: string[] = [];
  let i = 0;
  while (i < chunks.length) {
    const indent = lines.length > 0 ? rest : first;
    const room = width - indent.length;
    if (lines.length > 0 && !chunks[i]!.trim()) i++;
    const line: string[] = [];
    let len = 0;
    while (i < chunks.length && len + chunks[i]!.length <= room) {
      len += chunks[i]!.length;
      line.push(chunks[i++]!);
    }
    if (i < chunks.length && chunks[i]!.length > room && line.length === 0) line.push(chunks[i++]!);
    if (line.length > 0 && !line[line.length - 1]!.trim()) line.pop();
    if (line.length > 0) lines.push(indent + line.join(""));
  }
  return lines.join("\n");
}

type GoalState = "walked" | "live" | "ahead";

/**
 * Every milestone before the first future one, and whether it is complete: every goal carrying it has
 * walked, and no register still tags an item to it. A milestone at or past that line is left out,
 * since ahead of the program is not a state this answers.
 */
function pastState(): { id: string; complete: boolean; tagged: number; carried: [string, GoalState][] }[] {
  const goals = chainGoals();
  const live = liveGoal(goals);
  const walked = walkedGoals(goals, live);
  const tagged = new Map<string, number>();
  for (const reg of registers(collect())) {
    for (const owner of reg.owners) tagged.set(owner, (tagged.get(owner) ?? 0) + 1);
  }
  const out = [];
  for (const m of milestones()) {
    const num = MILESTONE.exec(m.id);
    if (!num || Number(num[1]) >= FIRST_FUTURE_MILESTONE) continue;
    const slugs = goals.filter((g) => g.milestone === m.id).map((g) => g.slug);
    const n = tagged.get(m.id) ?? 0;
    out.push({
      id: m.id,
      complete: n === 0 && slugs.every((s) => walked.has(s)),
      tagged: n,
      carried: slugs.map((s): [string, GoalState] => [s, walked.has(s) ? "walked" : live?.slug === s ? "live" : "ahead"]),
    });
  }
  return out;
}

function past(): number {
  const rows = pastState();
  const live = liveGoal();
  console.log(`nv plan --past: the ${rows.length} milestone(s) the index holds before M${FIRST_FUTURE_MILESTONE}, ` +
    "against the chain and every register nv owners reads");
  console.log("  complete = every goal carrying it has walked, and nothing is still tagged to it" +
    (live ? `; live at goal \`${live.slug}\`` : ""));
  for (const row of rows) {
    const carried = row.carried.map(([slug, state]) => `\`${slug}\` ${state}`).join(", ");
    console.log(wrap(`${row.id.padEnd(4)} ${(row.complete ? "complete" : "open").padEnd(8)} ` +
      `carried by ${carried || "no goal on the chain"}, ${row.tagged} item(s) still tagged to it`, 98, "  ", " ".repeat(16)));
  }
  console.log("\nA count above zero names work no closure goal has taken yet: `bun nv owners --check` lists the " +
    "items behind it, each to be closed or re-owned.");
  console.log(`${rows.filter((r) => r.complete).length} of ${rows.length} past milestone(s) complete`);
  return 0;
}

/** A goal citation in the house spelling: ``goal `surface``` or ``goals `a`, `b` and `c```. */
const GOAL_CITE = /\bgoals?\s+((?:`[a-z0-9][a-z0-9-]*`(?:\s*(?:,\s*and|,|and)\s*)?)+)/gi;
const SLUG = /`([a-z0-9][a-z0-9-]*)`/g;

/** The tense that turns a citation into a deferral: a walked goal that *will* do something. */
const FUTURE = /\b(?:will|waits|until|arrives|scheduled)\b|still owed|not yet|the one that|finishes it/i;

/** A sentence boundary: terminal punctuation, whitespace, then something a sentence can open with. */
const SENTENCE = /(?<=[.!?])\s+(?=[A-Z`*\[("])/;

type Block = [number, string][];

/** The text as blank-line-separated blocks of `[line number, text]`, fenced code blanked. */
function proseBlocks(text: string): Block[] {
  const blocks: Block[] = [];
  let block: Block = [];
  let fenced = false;
  text.split("\n").forEach((line, i) => {
    let raw = line;
    if (raw.trimStart().startsWith("```")) {
      fenced = !fenced;
      raw = "";
    } else if (fenced) raw = "";
    if (raw.trim()) block.push([i + 1, raw.trim()]);
    else if (block.length > 0) {
      blocks.push(block);
      block = [];
    }
  });
  if (block.length > 0) blocks.push(block);
  return blocks;
}

/** A block's sentences, each joined onto one line and charged to the line its first character is on. */
function blockSentences(block: Block): [number, string][] {
  const offsets: [number, number][] = [];
  let at = 0;
  for (const [lineno, raw] of block) {
    offsets.push([at, lineno]);
    at += raw.length + 1;
  }
  const text = block.map(([, raw]) => raw).join(" ");
  const out: [number, string][] = [];
  let pos = 0;
  for (const part of text.split(SENTENCE)) {
    let start = text.indexOf(part, pos);
    if (start < 0) start = pos;
    pos = start + part.length;
    let lineno = offsets[0]![1];
    for (const [off, n] of offsets) if (off <= start) lineno = n;
    out.push([lineno, part.trim()]);
  }
  return out;
}

/**
 * Every sentence in the plan that defers work to a goal the chain has already walked: a goal cited by
 * slug, in the future tense. It reads `docs/plan/m*.md` and the status block, whose fields are charged
 * to the line their `> **Field:**` opens on in the rendered plan. It is a lint and always exits 0; the
 * count on the last line is the finding.
 */
function stale(): number {
  const goals = chainGoals();
  const live = liveGoal(goals);
  const walked = walkedGoals(goals, live);
  console.log(`nv plan --stale: docs/plan/m*.md and ${PLAN}'s status block, against the ${walked.size} goal(s) the chain has walked`);
  console.log(live ? `  live: \`${live.slug}\`, ${live.num} of ${goals.length}`
    : "  no live goal on disk -- only the retired entries count as walked");

  const sources: [string, Block[]][] = readdirSync(join(ROOT, "docs/plan"))
    .filter((name) => /^m.*\.md$/.test(name))
    .sort()
    .map((name) => [`docs/plan/${name}`, proseBlocks(readFileSync(join(ROOT, "docs/plan", name), "utf8").replace(/\r\n/g, "\n"))]);
  const planLines = readFileSync(join(ROOT, PLAN), "utf8").replace(/\r\n/g, "\n").split("\n");
  const status = load(planStatus)[0]!.value as Record<string, string>;
  sources.push([PLAN, FIELDS.map(([field, key]): Block => {
    const at = planLines.findIndex((l) => l.startsWith(`> **${field}:**`));
    return [[at + 1, status[key]!]];
  })]);

  let found = 0;
  for (const [rel, blocks] of sources) {
    for (const block of blocks) {
      for (const [lineno, sentence] of blockSentences(block)) {
        const cited = [...sentence.matchAll(GOAL_CITE)]
          .flatMap((m) => [...m[1]!.matchAll(SLUG)].map((s) => s[1]!))
          .filter((s) => walked.has(s));
        const marker = FUTURE.exec(sentence);
        if (cited.length === 0 || !marker) continue;
        found++;
        const names = [...new Set(cited)].map((s) => `\`${s}\``).join(", ");
        const chars = Array.from(sentence);
        console.log(`\n${rel}:${lineno}  ${names}  -- "${marker[0].toLowerCase()}"`);
        console.log(`    ${chars.length <= 160 ? sentence : chars.slice(0, 159).join("") + "…"}`);
      }
    }
  }
  console.log("\nEach is a sentence to re-read against the tree rather than one to delete: the goal has run, so " +
    "either the work landed and the sentence is rewritten to what is true, or it is still owed and the " +
    "sentence names the goal that owns it now.");
  console.log(`sentences deferring to a walked goal: ${found}`);
  return 0;
}

/** Guidance per status field in bytes, and the multiple of it a growing edit may not cross. */
const FIELD_AIM_FALLBACK = 400;
const FIELD_CEILING_X_FALLBACK = 5;

/** The per-field byte aim and the ceiling a growing edit may not cross, both read out of the plan's leading comment. */
export function fieldLimits(): { aim: number; ceiling: number } {
  const planText = readFileSync(join(ROOT, PLAN), "utf8");
  const aimHit = /Aim for ~(\d+) bytes a field/.exec(planText);
  const aim = aimHit ? Number(aimHit[1]) : FIELD_AIM_FALLBACK;
  const xHit = /over (\d+)x that/.exec(planText);
  return { aim, ceiling: aim * (xHit ? Number(xHit[1]) : FIELD_CEILING_X_FALLBACK) };
}

/** The status block's fields in the plan's order, each with its text as one paragraph. */
export function planFields(): [string, string][] {
  const status = load(planStatus)[0]!.value as Record<string, string>;
  return FIELDS.map(([name, key]) => [name, status[key]!.trim()]);
}

/** Python's `f"{x:.0f}"`, which rounds half to even. */
function round0(x: number): string {
  const r = Math.round(x);
  return String(Math.abs(x % 1) === 0.5 && r % 2 !== 0 ? r - 1 : r);
}

/**
 * The sizes of the status fields, then the milestone records against the prose and the chain. A size is
 * reported and never fails the check; a structural finding does. A record's `title`, with its `estimate`
 * in brackets after it, must be its scope file's H1, and every `docs/plan/m*.md` must have a record. Its
 * `state` must agree with the chain: `done`
 * exactly when `pastState()` calls it complete, `open` with no `backlog` place while a chain goal carries
 * it, and `done`, `ongoing` or a `backlog` place while none does. Last, every row of the plan's milestone
 * table must be the row `expectedRow` builds from its record and the chain, in the records' order.
 */
function check(): number {
  const problems: string[] = [];
  const { aim, ceiling } = fieldLimits();
  const status = load(planStatus)[0]!.value as Record<string, string>;
  const sizes = FIELDS.map(([name, key]): [string, number] => [name, Buffer.byteLength(status[key]!.trim(), "utf8")]);
  const total = sizes.reduce((sum, [, n]) => sum + n, 0);
  console.log(`status block: ${total} bytes across ${sizes.length} fields, aim ~${aim} each (~${aim * sizes.length})`);
  for (const [name, n] of sizes) {
    if (n > ceiling) {
      console.log(`  ${name.padEnd(20)} ${String(n).padStart(6)} bytes   OVER the ${ceiling} B ceiling -- an edit that grows it is refused until it is cut`);
    } else if (n > aim * 1.5) {
      console.log(`  ${name.padEnd(20)} ${String(n).padStart(6)} bytes   ${round0(n / aim)}x the aim, ${ceiling - n} B under the ${ceiling} B ceiling`);
    }
  }
  console.log("  Every one of these is shipped into every session by nv orient and nv brief.");
  console.log(`  The aim is guidance; the ${ceiling} B ceiling is a gate on GROWTH: nv session --wrap`);
  console.log("  refuses an edit that leaves a field both over it and bigger than it was. A shrink");
  console.log("  is always taken, so there is never prose to shave -- a sentence is replaced instead.");

  console.log("\nindex vs disk:");
  const index = milestones();
  const records = new Map(load(milestoneType).map((r) => [r.id, r.value]));
  const seen = new Set<string>();
  for (const m of index) {
    seen.add(m.rel.toLowerCase());
    if (!existsSync(join(ROOT, m.rel))) {
      problems.push(`${m.id}: its record ${m.record} names ${m.rel}, which does not exist`);
      continue;
    }
    const rec = records.get(m.id)!;
    const title = rec.estimate === undefined ? rec.title : `${rec.title} (${rec.estimate})`;
    const first = readFileSync(join(ROOT, m.rel), "utf8").replace(/\r\n/g, "\n").split("\n")[0]!;
    const h1 = H1.exec(first);
    if (!h1) {
      problems.push(`${m.id}: ${m.rel} does not open with \`# ${m.id} — <title>\``);
    } else {
      if (h1[1] !== m.id) problems.push(`${m.id}: ${m.rel} calls itself ${h1[1]}`);
      if (h1[2]!.trim() !== title) {
        problems.push(`${m.id}: the record's title and the file's H1 have drifted apart\n` +
          `      record: ${title}\n      file:   ${h1[2]!.trim()}`);
      }
    }
    if (!verifyParagraph(m)) problems.push(`${m.id}: no \`**Verify:**\` paragraph -- it has no acceptance test`);
  }
  for (const name of readdirSync(join(ROOT, "docs/plan")).filter((n) => /^m.*\.md$/.test(n)).sort()) {
    if (!seen.has(`docs/plan/${name}`.toLowerCase())) {
      problems.push(`docs/plan/${name}: on disk, but no milestone record names it, so nothing links to it`);
    }
  }

  // A goal's milestone is a foreign key, so a dangling one is `nv check`'s finding too. It is repeated
  // here so this check says everything that is wrong with the plan in one place.
  const goals = chainGoals();
  if (goals.length > 0) {
    const ids = new Set(index.map((m) => m.id));
    for (const g of goals) {
      if (g.milestone !== null && !ids.has(g.milestone)) {
        problems.push(`goal \`${g.slug}\`: tagged ${g.milestone}, which is not a milestone record`);
      }
    }
    const past = new Map(pastState().map((row) => [row.id, row]));
    const carriedIds = new Set(goals.flatMap((g) => (g.milestone ? [g.milestone] : [])));
    for (const m of index) {
      const rec = records.get(m.id)!;
      const slugs = goals.filter((g) => g.milestone === m.id).map((g) => g.slug);
      const st = past.get(m.id);
      const where = rec.backlog !== undefined ? `${rec.state}, backlog ${rec.backlog}` : rec.state;
      if (st?.complete) {
        if (rec.state !== "done") {
          problems.push(`${m.id}: every goal carrying it has walked and no register still tags an item to it, ` +
            `so its record's state is \`done\`\n      record: ${where}\n      \`bun nv plan --past\` is the report`);
        }
      } else if (st && rec.state === "done") {
        const held = st.carried.filter(([, state]) => state !== "walked").map(([slug]) => `goal \`${slug}\` has not walked`);
        if (st.tagged) held.push(`${st.tagged} item(s) in the registers still name it`);
        problems.push(`${m.id}: its record says it is finished and \`bun nv plan --past\` does not -- ${held.join("; ")}`);
      } else if (slugs.length > 0) {
        if (rec.state !== "done" && (rec.state !== "open" || rec.backlog !== undefined)) {
          problems.push(`${m.id}: goals ${slugs.map((s) => `\`${s}\``).join(", ")} carry it, so its record is ` +
            `\`open\` with no backlog place, not ${repr(where)}`);
        }
      } else if (rec.state === "open" && rec.backlog === undefined) {
        problems.push(`${m.id}: no chain goal carries it, so its record says where it stands on its own -- ` +
          "`done`, `ongoing` or a `backlog` place, not 'open'");
      }
    }
    const none = goals.filter((g) => g.milestone === null).map((g) => `\`${g.slug}\``);
    const live = liveGoal(goals);
    console.log(`  chain: ${goals.length} goals, ${carriedIds.size} milestone(s) carried` +
      (none.length > 0 ? `, ${none.length} in none (goals ${none.join(", ")})` : "") +
      (live ? `; live at goal \`${live.slug}\`` : "; nothing live"));
  } else {
    console.log("  chain: data/chain.json lists no goals, so no milestone's state is checked against it");
  }

  // The table is the records and the chain written out for a reader, so every cell of every row is
  // compared with them, in the records' order.
  const rows = tableRows(readFileSync(join(ROOT, PLAN), "utf8"));
  const want = index.map((m) => {
    const rec = records.get(m.id)!;
    return expectedRow({ id: m.id, ...rec, carriers: goals.filter((g) => g.milestone === m.id).map((g) => g.slug) });
  });
  if (rows.map((r) => r.id).join(" ") !== want.map((w) => w.id).join(" ")) {
    problems.push(`${PLAN}: the milestone table's rows are ${rows.map((r) => r.id).join(", ") || "none"}, and the records, in order, are ${want.map((w) => w.id).join(", ")}`);
  } else {
    rows.forEach((row, i) => {
      const w = want[i]!;
      for (const cell of ["carried", "link", "builds", "loopDays"] as const) {
        if (row[cell] !== w[cell]) problems.push(`${PLAN}:${row.line}: ${row.id}'s ${cell === "carried" ? "Carried by" : cell === "builds" ? "What it builds" : cell === "loopDays" ? "Loop-days" : "link"} cell has drifted from its record and the chain\n      table:  ${row[cell] || "(empty)"}\n      wanted: ${w[cell]}`);
      }
    });
  }

  if (problems.length > 0) {
    for (const p of problems) console.log(`  !! ${p}`);
    console.log(`\n  ${problems.length} structural finding(s) -- these are what this exits non-zero on. ` +
      "Each is an edit to a milestone record or to the scope file it names.");
    return 1;
  }
  console.log(`  ${index.length} rows, ${index.length} files, titles matching, every one with a ` +
    "`**Verify:**`, every cell agreeing with the chain");
  return 0;
}

export async function run(args: string[]): Promise<number> {
  const [flag, value, ...rest] = args;
  if ((flag === "--show" || flag === "--get") && value !== undefined && rest.length === 0) {
    return flag === "--show" ? show(value) : get(value);
  }
  if (flag === "--past" && value === undefined) return past();
  if (flag === "--stale" && value === undefined) return stale();
  if (flag === "--check" && value === undefined) return check();
  console.error("usage: bun nv plan --show M8[:lead|:verify] | --get <Field> | --past | --stale | --check\n" +
    "  a status field or a milestone is rewritten by `bun nv session --wrap`'s `## plan:` and `## milestone:` sections");
  return 2;
}
