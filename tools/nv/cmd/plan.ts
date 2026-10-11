// `bun nv plan`: the modes over the implementation plan. `--show M8` prints a milestone's scope file
// whole, `--show M8:lead` its first paragraph and `--show M8:verify` its `**Verify:**` paragraph, its
// acceptance test. `--get <Field>` prints one field of the status block, unwrapped. `--past` prints one
// line per milestone the program is behind, and whether it is complete. `--check` reports the status
// fields' sizes, and fails when a milestone record disagrees with its scope file or the chain.
//
// `--apply <wrap-file | ->` is the one mode that writes. It takes a wrap file's `## plan:`, `## plan-edit:`
// and `## milestone:` sections, checks them with `planChecks`, and writes them only when every one is
// sound; `--dry` checks and writes nothing. A session runs it on its wrap file before `looper wrap`,
// which does not read those sections.
//
// The roster and the status block are read from the `milestone` and `plan_status` records under
// `data/plan/`. A milestone's scope is prose, `docs/plan/<id>.md`, and its H1 is where the body starts.
// Each reader and writer takes the repository root, so a test runs them over a scratch tree.

import { existsSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { chainGoals, liveGoal, sideGoal } from "../lib/chain.ts";
import { ROOT } from "../lib/paths.ts";
import { pyRepr } from "../lib/py.ts";
import { load, pathOf, write as writeRecord } from "../lib/store.ts";
import { head, inOrder, nbytes, normalize, occurrences, parseEdits, parseWrap, PLAN_KINDS, type Section, thousands } from "../lib/wrap.ts";
import { record } from "../lib/written.ts";
import { milestone as milestoneType } from "../schema/milestone.ts";
import { planStatus } from "../schema/plan-status.ts";
import { collect, FIRST_FUTURE_MILESTONE, registers } from "./owners.ts";

export const summary = "the plan: nv plan --show M8[:lead|:verify] | --get <Field> | --past | --check | --apply <wrap-file | -> [--dry]";

export const PLAN = "docs/implementation-plan.md";

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
  ["Open now", "openNow"],
  ["Blocking", "blocking"],
];

export interface Entry {
  id: string;
  rel: string;
  record: string;
}

/** Every milestone, in the table's order. */
export function milestones(root: string = ROOT): Entry[] {
  return load(milestoneType, root)
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
export function bodyOf(entry: Entry, root: string = ROOT): string {
  const text = readFileSync(join(root, entry.rel), "utf8").replace(/\r\n/g, "\n");
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
export function verifyParagraph(entry: Entry, root: string = ROOT): string {
  const lines = bodyOf(entry, root).split("\n");
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

type GoalState = "live" | "ahead";

/**
 * Every milestone before the first future one, and whether it is complete: no goal on the chain carries
 * it, and no register still tags an item to it. The goal switch deletes a reached goal, so a goal still
 * on the chain is live or ahead. A milestone at or past the first future one is left out, since ahead
 * of the program is not a state this answers.
 */
function pastState(): { id: string; complete: boolean; tagged: number; carried: [string, GoalState][] }[] {
  const goals = chainGoals();
  const live = liveGoal(goals);
  const tagged = new Map<string, number>();
  for (const reg of registers(collect(), true)) {
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
      complete: n === 0 && slugs.length === 0,
      tagged: n,
      carried: slugs.map((s): [string, GoalState] => [s, live?.slug === s ? "live" : "ahead"]),
    });
  }
  return out;
}

function past(): number {
  const rows = pastState();
  const live = liveGoal();
  console.log(`nv plan --past: the ${rows.length} milestone(s) the index holds before M${FIRST_FUTURE_MILESTONE}, ` +
    "against the chain and every register nv owners reads");
  console.log("  complete = no goal on the chain carries it, and nothing is still tagged to it" +
    (live ? `; live at goal \`${live.slug}\`` : ""));
  for (const row of rows) {
    const carried = row.carried.map(([slug, state]) => `\`${slug}\` ${state}`).join(", ");
    console.log(wrap(`${row.id.padEnd(4)} ${(row.complete ? "complete" : "open").padEnd(8)} ` +
      `carried by ${carried || "no goal on the chain"}, ${row.tagged} item(s) still tagged to it`, 98, "  ", " ".repeat(16)));
  }
  console.log("\nA count above zero names work no closing goal has taken yet: `bun nv owners --check` lists the " +
    "items behind it, each to be closed or re-owned.");
  console.log(`${rows.filter((r) => r.complete).length} of ${rows.length} past milestone(s) complete`);
  return 0;
}

/** Guidance per status field in bytes, and the multiple of it a growing edit may not cross. */
const FIELD_AIM_FALLBACK = 400;
const FIELD_CEILING_X_FALLBACK = 5;

/** The per-field byte aim and the ceiling a growing edit may not cross, both read out of the plan's leading comment. */
export function fieldLimits(root: string = ROOT): { aim: number; ceiling: number } {
  const planText = readFileSync(join(root, PLAN), "utf8");
  const aimHit = /Aim for ~(\d+) bytes a field/.exec(planText);
  const aim = aimHit ? Number(aimHit[1]) : FIELD_AIM_FALLBACK;
  const xHit = /over (\d+)x that/.exec(planText);
  return { aim, ceiling: aim * (xHit ? Number(xHit[1]) : FIELD_CEILING_X_FALLBACK) };
}

/** The status block's fields in the plan's order, each with its text as one paragraph. */
export function planFields(root: string = ROOT): [string, string][] {
  const status = load(planStatus, root)[0]!.value as Record<string, string>;
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
  console.log("  Every one of these is shipped into every session by nv pack and nv brief.");
  console.log(`  The aim is guidance; the ${ceiling} B ceiling is a gate on GROWTH: nv plan --apply`);
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
          problems.push(`${m.id}: no goal on the chain carries it and no register still tags an item to it, ` +
            `so its record's state is \`done\`\n      record: ${where}\n      \`bun nv plan --past\` is the report`);
        }
      } else if (st && rec.state === "done") {
        const held = st.carried.map(([slug]) => `goal \`${slug}\` is on the chain`);
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
    console.log("  chain: docs/goals/README.md lists no goals, so no milestone's state is checked against it");
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

// ------------------------------------------------------------------------ the wrap's plan sections

/** The plan's line width, the `> ` prefix included. */
const PLAN_WIDTH = 100;
const FIELD_LINE = /^> \*\*([^*:]+):\*\*\s*(.*)$/;
/**
 * A `## plan:` body over `RETYPE_BYTES` whose 8-word runs are at least `RETYPE_OVERLAP` already in the
 * field, and which is not a cut below `RETYPE_SHRINK` of it, is the field typed out again: `## plan-edit:`
 * sends only the sentence that moved.
 */
const RETYPE_BYTES = 1_500;
const RETYPE_OVERLAP = 0.7;
const RETYPE_SHRINK = 0.6;

/** The milestone a `## milestone:` section names, matched on its id whatever its case. */
export function resolveMilestone(arg: string, root: string = ROOT): Entry | null {
  const want = arg.trim().toUpperCase();
  return milestones(root).find((m) => m.id === want) ?? null;
}

/** The fraction of `next`'s 8-word runs that appear verbatim in `old`. */
function verbatimOverlap(next: string, old: string): number {
  const grams = (t: string) => {
    const w = t.split(/\s+/).filter(Boolean);
    const out = new Set<string>();
    for (let i = 0; i + 8 <= w.length; i++) out.add(w.slice(i, i + 8).join("\0"));
    return out;
  };
  const gn = grams(next);
  if (gn.size === 0) return 0;
  const go = grams(old);
  return [...gn].filter((g) => go.has(g)).length / gn.size;
}

/** A refusal only when the edit leaves the field both over the ceiling and bigger than it was. */
function growthRefusal(kind: string, field: string, before: string, after: string, ceiling: number): string | null {
  const was = nbytes(before);
  const now = nbytes(after);
  if (now <= was || now <= ceiling) return null;
  const cut = now - Math.max(ceiling, was);
  const how = kind === "plan-edit" ? "a `--- old` quoting the stale sentence with an empty `--- new` drops it" : "send the replacement shorter";
  return (
    `\`## ${kind}: ${field}\` -- leaves the field at ${thousands(now)} B, +${now - was} B ` +
    `and past its ${thousands(ceiling)} B ceiling. A field is status and is overwritten, not appended ` +
    `to. Cut at least ${cut} B of it in this same section (${how}), or take the new text ` +
    "where it belongs -- a per-file gap to that crate's module doc `# Known gaps`, a trap to " +
    "`## playbook:`, what landed to the commit body. An edit that leaves the field no bigger " +
    "than it is now is always taken."
  );
}

/**
 * What is wrong with the plan, plan-edit and milestone sections among `sections`, each section checked
 * against the field as the sections before it leave it. Every other kind is skipped.
 */
export function planChecks(sections: Section[], root: string = ROOT): string[] {
  const errors: string[] = [];
  const fields = planFields(root);
  const names = new Map(fields.map(([n]) => [n.toLowerCase(), n]));
  const working = new Map(fields.map(([n, t]) => [n.toLowerCase(), t]));
  const { ceiling } = fieldLimits(root);
  const side = sideGoal(root);
  const noField = (s: Section) => `\`## ${s.kind}: ${s.arg}\` -- no such field, and this tool does not add one. The block has: ${[...names.values()].join(", ")}`;

  for (const s of inOrder(sections)) {
    if (!PLAN_KINDS.includes(s.kind)) continue;
    if (side) {
      // The plan's status block and the milestone files are the chain run's state, and a side
      // branch that rewrote them would overwrite main's when it lands.
      errors.push(`\`## ${s.kind}: ${s.arg}\` -- a side run does not write the plan; say what it changed in the handoff, and the landing commit carries it`);
      continue;
    }
    const key = s.arg.toLowerCase();
    if (s.kind === "plan") {
      if (!names.has(key)) {
        errors.push(noField(s));
        continue;
      }
      const next = normalize(s.body);
      const old = working.get(key)!;
      const share = verbatimOverlap(next, old);
      const len = [...next].length;
      if (len > RETYPE_BYTES && share >= RETYPE_OVERLAP && len > [...old].length * RETYPE_SHRINK) {
        errors.push(
          `\`## plan: ${s.arg}\` -- ${thousands(len)} B, and ${Math.round(share * 100)}% of it is already ` +
            "on disk word for word. That is a retype, not a rewrite: use " +
            `\`## plan-edit: ${s.arg}\` with \`--- old\` / \`--- new\` fragments and send only ` +
            `the sentence that moved. (A real rewrite overlaps less than ${Math.round(RETYPE_OVERLAP * 100)}% and is taken as it stands.)`,
        );
        continue;
      }
      const grown = growthRefusal("plan", s.arg, old, next, ceiling);
      if (grown) {
        errors.push(grown);
        continue;
      }
      working.set(key, next);
    } else if (s.kind === "plan-edit") {
      if (!names.has(key)) {
        errors.push(noField(s));
        continue;
      }
      const { pairs, errors: bad } = parseEdits(s.body);
      errors.push(...bad.map((b) => `\`## plan-edit: ${s.arg}\` -- ${b}`));
      let text = working.get(key)!;
      for (const [old, next] of pairs) {
        if (!old) continue;
        const hits = occurrences(text, old);
        if (hits !== 1) {
          const where = hits === 0 ? "is not in that field" : `appears ${hits} times in it`;
          errors.push(
            `\`## plan-edit: ${s.arg}\` -- the \`--- old\` fragment ${where}, so nothing ` +
              "was changed. Quote a longer run, exactly as the field reads (it is one " +
              `paragraph, single-spaced; \`bun nv plan --get ${pyRepr(s.arg)}\` prints ` +
              `it): ${pyRepr(head(old, 70))}`,
          );
          continue;
        }
        text = text.replace(old, () => next);
      }
      const grown = growthRefusal("plan-edit", s.arg, working.get(key)!, text, ceiling);
      if (grown) {
        errors.push(grown);
        continue;
      }
      working.set(key, text);
    } else {
      const entry = resolveMilestone(s.arg, root);
      if (entry === null) {
        errors.push(`\`## milestone: ${s.arg}\` -- the plan's table lists no such milestone, and this tool does not add one. It has: ${milestones(root).map((m) => m.id).join(", ")}`);
      } else if (!existsSync(join(root, entry.rel))) {
        errors.push(`\`## milestone: ${s.arg}\` -- ${entry.rel} does not exist`);
      }
    }
  }
  return errors;
}

/** The status block's fields in the plan's Markdown: name, first line, one past the last line, and the joined text. */
function findFields(lines: string[]): [string, number, number, string][] {
  const fields: [string, number, number, string[]][] = [];
  let started = false;
  for (let i = 0; i < lines.length; i++) {
    const raw = lines[i]!;
    if (raw.startsWith(">")) {
      started = true;
      const m = FIELD_LINE.exec(raw);
      if (m) fields.push([m[1]!.trim(), i, i + 1, [m[2]!.trim()]]);
      else if (fields.length) {
        const body = raw.replace(/^> ?/, "").trim();
        if (body) {
          fields[fields.length - 1]![2] = i + 1;
          fields[fields.length - 1]![3].push(body);
        }
      }
    } else if (started) break;
  }
  return fields.map(([n, a, b, p]) => [n, a, b, p.join(" ").trim()]);
}

/** `**Name:** text` wrapped greedily at word boundaries to the block's width, a long word on a line of its own. */
function renderField(name: string, text: string): string[] {
  const out: string[] = [];
  let line = "";
  for (const word of `**${name}:** ${text}`.split(/\s+/).filter(Boolean)) {
    if (line && [...line].length + 1 + [...word].length > PLAN_WIDTH - 2) {
      out.push(line);
      line = word;
    } else line = line ? `${line} ${word}` : word;
  }
  if (line) out.push(line);
  return out;
}

/**
 * Rewrites one status field to `next`, in the plan's Markdown and in its record, and returns the field's
 * name and its text before. The field is matched by name, whatever its case.
 */
function writeField(arg: string, next: string | ((old: string) => string), dry: boolean, root: string): { name: string; old: string; now: string } {
  const lines = readFileSync(join(root, PLAN), "utf8").split("\n");
  const [name, start, end, old] = findFields(lines).find(([n]) => n.toLowerCase() === arg.toLowerCase())!;
  const now = typeof next === "string" ? next : next(old);
  if (!dry) {
    const rewritten = [...lines.slice(0, start), ...renderField(name, now).map((l) => `> ${l}`), ...lines.slice(end)];
    writeFileSync(join(root, PLAN), rewritten.join("\n"));
    const key = FIELDS.find(([n]) => n === name)?.[1];
    if (key) {
      const status = { ...(load(planStatus, root)[0]!.value as Record<string, string>), [key]: now };
      writeRecord(planStatus, planStatus.name, status as any, root);
    }
  }
  return { name, old, now };
}

export function applyPlan(s: Section, dry: boolean, root: string = ROOT): string {
  const { name, old, now } = writeField(s.arg, normalize(s.body), dry, root);
  return `plan: ${name}  ${nbytes(old)} -> ${nbytes(now)} bytes`;
}

/** One field with the fragments the section names replaced; `planChecks` has proved each `--- old` matches once. */
export function applyPlanEdit(s: Section, dry: boolean, root: string = ROOT): string {
  const { pairs } = parseEdits(s.body);
  const { name, old, now } = writeField(s.arg, (text) => pairs.reduce((t, [was, next]) => t.replace(was, () => next), text), dry, root);
  const { ceiling } = fieldLimits(root);
  const was = nbytes(old);
  const is = nbytes(now);
  let note = `plan-edit: ${name}  ${pairs.length} fragment(s), ${was} -> ${is} bytes`;
  // Only when the field grew and the ceiling a growing edit may not cross is close: this says the next
  // growing edit will be refused, which is the one thing worth knowing ahead of it.
  const headroom = ceiling - is;
  if (is > was && headroom < Math.floor(ceiling / 4)) {
    note +=
      `  (+${is - was} B; ${Math.max(headroom, 0)} B left under the ${ceiling} B` +
      " ceiling -- past it a growing edit is refused, so the next one replaces a" +
      " sentence rather than adding one)";
  }
  return note;
}

export function applyMilestone(s: Section, dry: boolean, root: string = ROOT): string {
  const entry = resolveMilestone(s.arg, root)!;
  const body = s.body.replace(/^\n+|\n+$/g, "");
  let note = `milestone: ${entry.id} in ${entry.rel}, ${nbytes(bodyOf(entry, root))} -> ${nbytes(body)} bytes`;
  if (dry) return note;
  const lines = readFileSync(join(root, entry.rel), "utf8").split("\n");
  const at = lines.findIndex((l) => H1.test(l));
  const top = at < 0 ? [`# ${entry.id}`] : lines.slice(0, at + 1);
  writeFileSync(join(root, entry.rel), `${top.join("\n")}\n\n${body}\n`);
  if (!verifyParagraph(entry, root)) note += "  !! no `**Verify:**` paragraph -- nothing can call this milestone done";
  return note;
}

/**
 * `--apply`: checks the plan sections of the wrap text `text` and, unless `dry`, writes them. Exit 0 with
 * a line per section, or 1 with every problem and nothing written. A wrap with no plan section is 0.
 */
export function apply(text: string, dry: boolean, root: string = ROOT): { code: number; lines: string[] } {
  const parsed = parseWrap(text.replace(/\r\n/g, "\n"));
  const sections = inOrder(parsed.sections.filter((s) => PLAN_KINDS.includes(s.kind)));
  const errors = [...parsed.errors, ...planChecks(sections, root)];
  if (errors.length) return { code: 1, lines: ["nv plan --apply: NOTHING was written:", ...errors.map((e) => `  - ${e}`)] };
  if (!sections.length) return { code: 0, lines: ["nv plan --apply: no `## plan:`, `## plan-edit:` or `## milestone:` section, nothing to write"] };
  const lines = [`nv plan --apply: ${dry ? "would apply" : "applied"} ${sections.length} section(s)`];
  for (const s of sections) {
    lines.push(`  ${s.kind === "plan" ? applyPlan(s, dry, root) : s.kind === "plan-edit" ? applyPlanEdit(s, dry, root) : applyMilestone(s, dry, root)}`);
    if (dry) continue;
    if (s.kind === "milestone") record(join(root, resolveMilestone(s.arg, root)!.rel));
    else record(join(root, PLAN), join(root, pathOf(planStatus, planStatus.name)));
  }
  return { code: 0, lines };
}

const USAGE =
  "usage: bun nv plan --show M8[:lead|:verify] | --get <Field> | --past | --check | --apply <wrap-file | -> [--dry]\n" +
  "  --apply writes a wrap's `## plan:`, `## plan-edit:` and `## milestone:` sections; `-` reads the wrap on stdin";

export async function run(args: string[]): Promise<number> {
  const [flag, value, ...rest] = args;
  if ((flag === "--show" || flag === "--get") && value !== undefined && rest.length === 0) {
    return flag === "--show" ? show(value) : get(value);
  }
  if (flag === "--past" && value === undefined) return past();
  if (flag === "--check" && value === undefined) return check();
  if (flag === "--apply" && value !== undefined && rest.every((a) => a === "--dry")) {
    let text: string;
    if (value === "-") text = await Bun.stdin.text();
    else if (existsSync(value)) text = readFileSync(value, "utf8");
    else {
      console.error(`nv plan --apply: no such file: ${value}`);
      return 2;
    }
    const { code, lines } = apply(text, rest.length > 0);
    for (const line of lines) console.log(line);
    return code;
  }
  console.error(USAGE);
  return 2;
}
