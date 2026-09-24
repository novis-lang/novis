// `bun nv owners`: every known gap, and who owns it. A gap is a record, `data/gaps/<crate>/<slug>.json`,
// naming exactly one owner: a goal by slug, or a milestone by id. This command classifies each owner
// against the chain and the plan, and `--check` is the gate over that classification.
//
// The kinds, and what `--check` does with each:
//
//   milestone  a milestone at M9 or later that the plan's table has not marked done. The one owner the
//              gate accepts: scheduled work is not an unclosed hole, and `--deferrals` holds each one to
//              the scope its own `docs/plan/<id>.md` states.
//   past       a milestone before M9. Everything before M9 is complete, so the tag names owed work
//              whose carrier has been and gone.
//   goal       a live goal. A goal is a schedule rather than an owner: it walks and is retired, and the
//              gap outlives it.
//   retired    a goal whose record carries no checks, which is how a goal the driver will not reach
//              again is written. Nothing on the chain closes the gap.
//   broken     an owner that resolves to no goal or milestone, or to a milestone that is done.
//   unowned    the retired word `unowned`, refused by name so the reader knows the edit is to choose an
//              owner rather than to fix a typo.
//   untagged   no owner at all.
//
// A gap leaves the gate one of three ways: built and its record deleted, struck as a stated bound in
// the module's own prose, or deferred to a milestone whose plan states the scope. `--closes <slug>` is
// the gate a goal meets at its end: it exits 1 while any gap names that goal, because a tag is not a
// build.
//
// `--registers` counts every place owed work is written down: the gap records, the refusal runs in
// `docs/agent/carried-refusals.md`, the keys in `crates/nvs-stdlib/tests/*-outstanding.txt`, the
// guard names in `docs/agent/guard-name-debt.md`, and the playbook bullets whose `until` names a state
// of the tree. It takes their counts and nothing else, since each has a gate over its own discipline.
// A module doc heading that words owed work outside a gap block is listed too, as a warning.
//
//   bun nv owners                 the roster, by kind
//   bun nv owners --check         the gate; exit 1 with a line per refused gap
//   bun nv owners --closes SLUG   exit 1 while any gap names that goal
//   bun nv owners --deferrals     each milestone owner against its plan file's scope
//   bun nv owners --registers     one line per register, and how many items each holds
//   bun nv owners --untagged | --unowned   only the gaps of that kind
//   bun nv owners --json          every kind, every register with its owners, and the first future milestone
//
// `--untagged-is-an-error`, `--reasons` and `--past-is-an-error` are accepted and do nothing: `--check`
// refuses all three on its own, and a carried floor check still passes them.

import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { ROOT } from "../lib/paths.ts";
import { load } from "../lib/store.ts";
import { gap as gapType } from "../schema/gap.ts";
import { goal as goalType, sideGoal } from "../schema/goal.ts";
import { milestone as milestoneType } from "../schema/milestone.ts";
import { playbookBullet } from "../schema/playbook.ts";

export const summary = "who owns each known gap, and the gate over it: nv owners [--check | --closes <slug> | --registers | ...]";

/** The first milestone a gap may be deferred to. Everything before it is complete. */
export const FIRST_FUTURE_MILESTONE = 9;

const UNOWNED = "unowned";
const MILESTONE = /^M(\d+)[A-Z]?$/;

const CARRIED_GAPS = "docs/agent/carried-gaps.md";
const CARRIED_REFUSALS = "docs/agent/carried-refusals.md";
const GUARD_DEBT = "docs/agent/guard-name-debt.md";
const RATCHETS = "crates/nvs-stdlib/tests/*-outstanding.txt";
const SOURCES = "crates/*/src/**/*.rs";

export interface Gap {
  slug: string;
  module: string;
  title: string;
  text: string;
  owner: string;
  why?: string;
}

export type Kind = "goal" | "milestone" | "past" | "unowned" | "untagged" | "broken" | "retired";
export type Kinds = Record<Kind, Gap[]>;

/** Every kind `--check` refuses: all but `milestone`. */
export const REFUSED: Kind[] = ["broken", "untagged", "unowned", "past", "goal", "retired"];

/** The label each kind is counted under, one count per line so a `want` of `0` cannot match `10`. */
const LABELS: [Kind, string][] = [
  ["goal", "goal-owned"], ["milestone", "milestone-owned"], ["past", "past-milestone"],
  ["unowned", "unowned"], ["untagged", "untagged"], ["broken", "broken-tag"], ["retired", "retired-owner"],
];

/** The sentence a refused gap gets when its kind has only one way of being wrong. */
const WHY: Partial<Record<Kind, string>> = {
  goal: "names a goal, and a goal is a schedule rather than an owner: it walks and is retired, " +
    "and the gap outlives it. Build the item, strike it as a stated bound, or defer it to a " +
    "milestone whose plan states the scope",
  retired: "names a goal that is retired, so nothing on the chain will reach this item. Build " +
    "it, strike it as a stated bound, or defer it to a milestone whose plan states the scope",
};

/** Every gap record, ordered by module and then slug. */
export function collect(): Gap[] {
  return load(gapType)
    .map((r) => ({
      slug: r.id.slice(r.id.indexOf("/") + 1),
      module: r.value.module,
      title: r.value.title,
      text: r.value.text,
      owner: r.value.milestone ?? r.value.goal ?? "",
    }))
    .sort((a, b) => cmp(a.module, b.module) || cmp(a.slug, b.slug));
}

function cmp(a: string, b: string): number {
  return a < b ? -1 : a > b ? 1 : 0;
}

/** Sorts every gap into its kind, with the reason for each kind that has more than one. */
export function classify(found: Gap[]): Kinds {
  // A side goal owns a gap exactly as a chain goal does, and its end gate asks `--closes` of its slug.
  const goals = new Map([...load(goalType), ...load(sideGoal)].map((g) => [g.id, g.value]));
  const plan = new Map(load(milestoneType).map((m) => [m.id, m.value]));
  const out: Kinds = { goal: [], milestone: [], past: [], unowned: [], untagged: [], broken: [], retired: [] };
  for (const gap of found) {
    const owner = gap.owner;
    const m = MILESTONE.exec(owner);
    if (!owner) out.untagged.push({ ...gap, why: "no owner" });
    else if (owner === UNOWNED) {
      out.unowned.push({
        ...gap,
        why: "`unowned` is a retired owner kind; a gap is owned by a live goal on the chain or by a " +
          `milestone at M${FIRST_FUTURE_MILESTONE} or later whose plan states the scope`,
      });
    } else if (m) {
      const row = plan.get(owner);
      if (!row) out.broken.push({ ...gap, why: `no milestone ${owner} in the plan's table` });
      else if (Number(m[1]) < FIRST_FUTURE_MILESTONE) {
        out.past.push({
          ...gap,
          why: `${owner} is behind the program; only M${FIRST_FUTURE_MILESTONE} and later is a ` +
            "deferral, so this is owed by a goal or nobody",
        });
      } else if (row.state === "done") {
        out.broken.push({ ...gap, why: `milestone ${owner} is done; a gap it did not close is owned by a goal or by nobody` });
      } else out.milestone.push(gap);
    } else {
      const g = goals.get(owner);
      if (!g) out.broken.push({ ...gap, why: `no goal \`${owner}\` in data/goals/` });
      else if (g.checks.length === 0) out.retired.push(gap);
      else out.goal.push(gap);
    }
  }
  return out;
}

/** Every module doc heading that words owed work while opening no gap block. */
function outsideBlocks(): { file: string; line: number; lead: string }[] {
  const DOC = /^\s*\/\/!(?: ?(.*))?$/;
  const HEADING = /^#{1,6}\s+(\S.*?)\s*$/;
  const GAPS = /^Known gaps?\b/i;
  // Word-bounded, because `lowers` and `borrowed` both contain `owe`.
  const OWED = /\b(?:not(?:\s+\w+){0,3}\s+yet|owe[sd]?|owing|still missing|not armed|gaps?)\b/i;
  const found = [];
  for (const file of [...new Bun.Glob(SOURCES).scanSync({ cwd: ROOT })].map((f) => f.replaceAll("\\", "/")).sort()) {
    const lines = readFileSync(join(ROOT, file), "utf8").split("\n");
    for (let n = 0; n < lines.length; n++) {
      const doc = DOC.exec(lines[n]!);
      const h = doc ? HEADING.exec(doc[1] ?? "") : null;
      if (h && !GAPS.test(h[1]!) && OWED.test(h[1]!)) found.push({ file, line: n + 1, lead: oneLine(h[1]!) });
    }
  }
  return found;
}

function oneLine(text: string): string {
  return text.replace(/\s+/g, " ").trim();
}

/** The first `n` characters of `text`, counted as code points. */
function cut(text: string, n: number): string {
  return Array.from(text).slice(0, n).join("");
}

interface Register {
  name: string;
  where: string;
  what: string;
  /** The owner each item names, "" when the register carries none. */
  owners: string[];
}

/** The lines of a markdown file that open an entry, per `pattern`. */
function entryLines(path: string, pattern: RegExp): string[] {
  if (!existsSync(join(ROOT, path))) return [];
  return readFileSync(join(ROOT, path), "utf8").split("\n").filter((l) => pattern.test(l)).map(() => "");
}

/** Every outstanding key in the ratchets, as the owner its `#` column names. */
function ratchetOwners(): string[] {
  const out: string[] = [];
  for (const file of [...new Bun.Glob(RATCHETS).scanSync({ cwd: ROOT })].sort()) {
    for (const line of readFileSync(join(ROOT, file), "utf8").split("\n")) {
      const text = line.trim();
      if (!text || text.startsWith("#")) continue;
      out.push(/^[^#]+?\s*(?:#\s*(\S+))?\s*$/.exec(text)?.[1] ?? "");
    }
  }
  return out;
}

/** Every place owed work is written down, in the order a gap is most often written. */
export function registers(found: Gap[]): Register[] {
  return [
    { name: "module docs", where: "data/gaps/", owners: found.map((g) => g.owner),
      what: "a gap record, naming the goal or milestone that owns it" },
    { name: "carried-refusals.md", where: CARRIED_REFUSALS, owners: entryLines(CARRIED_REFUSALS, /^9\d\d\. /),
      what: "a run of `nvs-ir` refusal sites an earlier milestone left, numbered from 900" },
    { name: "outstanding keys", where: RATCHETS, owners: ratchetOwners(),
      what: "a spec or migration member `registry::CLASSES` does not declare yet, each key naming its owner in a `#` column" },
    { name: "guard-name-debt.md", where: GUARD_DEBT, owners: entryLines(GUARD_DEBT, /^- \[/),
      what: "a guard test `loop-goal.toml` names and the tree does not hold yet" },
    { name: "playbook until", where: "data/playbook/",
      owners: load(playbookBullet).filter((b) => b.value.until.kind !== "reviewed").map(() => ""),
      what: "a trap whose `until` names the state of the tree that retires it" },
  ];
}

function tally(owners: string[]): Record<string, number> {
  const found: Record<string, number> = {};
  for (const o of owners.filter((o) => o).sort()) found[o] = (found[o] ?? 0) + 1;
  return found;
}

function absentLine(): string {
  if (!existsSync(join(ROOT, CARRIED_GAPS))) return `${CARRIED_GAPS}: absent, as the register requires`;
  return `${CARRIED_GAPS}: back on disk, and no register reads it. Make each entry a gap record under ` +
    "`data/gaps/` naming its owner, and delete the file";
}

function reportRegisters(regs: Register[]): void {
  const width = Math.max(...regs.map((r) => r.name.length));
  console.log("== EVERY PLACE THIS REPOSITORY WRITES OWED WORK DOWN");
  for (const r of regs) {
    console.log(`  ${r.name.padEnd(width)}  ${String(r.owners.length).padStart(4)} open item(s)  ${r.where}`);
    console.log(`  ${"".padEnd(width)}       ${r.what}`);
  }
  const total = regs.reduce((n, r) => n + r.owners.length, 0);
  console.log(`\n== ${total} item(s) open across ${regs.length} register(s)`);
  console.log(`   ${absentLine()}`);
}

function registerLine(regs: Register[]): string {
  return `  registers: ${regs.map((r) => `${r.name} ${r.owners.length}`).join(", ")} -- ${regs.length} register(s)`;
}

function lineOf(g: Gap): string {
  return `  ${g.module}  gap ${g.slug}  ${cut(g.title, 78)}`;
}

function byOwner(gaps: Gap[]): [string, Gap[]][] {
  const grouped = new Map<string, Gap[]>();
  for (const g of gaps) grouped.set(g.owner, [...(grouped.get(g.owner) ?? []), g]);
  return [...grouped].sort((a, b) => cmp(a[0], b[0]));
}

function countLines(kinds: Kinds): string[] {
  return LABELS.map(([kind, label]) => `  ${label}: ${kinds[kind].length}`);
}

function report(kinds: Kinds, found: Gap[], regs: Register[], sections: ReturnType<typeof outsideBlocks>): void {
  const out: string[] = ["== ITEMS THAT NAME NOBODY"];
  for (const g of kinds.untagged) out.push(lineOf(g), `      ${g.why}`);
  if (kinds.untagged.length === 0) out.push("  none -- every recorded gap names a goal or a milestone");
  const refused: [Kind, string][] = [
    ["broken", "TAGS THAT DO NOT RESOLVE"],
    ["past", "DEFERRED TO A MILESTONE THE PROGRAM HAS ALREADY PASSED"],
    ["retired", "OWNERS THAT WENT GREEN WITHOUT CLOSING THE GAP"],
  ];
  for (const [kind, head] of refused) {
    if (kinds[kind].length === 0) continue;
    out.push("", `== ${head}`);
    for (const g of kinds[kind]) out.push(lineOf(g), `      ${g.why ?? WHY[kind]}`);
  }
  const grouped: [Kind, string, (owner: string) => string][] = [
    ["goal", "OWNED BY A GOAL ON THE CHAIN", (o) => `goal \`${o}\``],
    ["milestone", "SCHEDULED BY A MILESTONE'S PLAN", (o) => o],
  ];
  for (const [kind, head, name] of grouped) {
    out.push("", `== ${head}`);
    for (const [owner, gaps] of byOwner(kinds[kind])) {
      out.push(`  ${name(owner)} -- ${gaps.length} item(s)`, ...gaps.map(lineOf));
    }
    if (kinds[kind].length === 0) out.push("  none");
  }
  out.push("", "== STILL TAGGED `unowned`, WHICH IS NO LONGER AN OWNER KIND");
  for (const g of kinds.unowned) out.push(lineOf(g), `      ${g.why}`);
  if (kinds.unowned.length === 0) out.push("  none");
  if (sections.length > 0) {
    out.push("", "== OWED WORK UNDER A HEADING OF ITS OWN, WHERE NO OWNER TAG REACHES IT");
    for (const s of sections) out.push(`  ${s.file}:${s.line}  ${cut(s.lead, 78)}`);
    out.push("  make each one a gap record under `data/gaps/` naming its owner, and rewrite the heading " +
      "to say what the module does");
  }
  const modules = new Set(found.map((g) => g.module)).size;
  out.push("", `== ${found.length} item(s) across ${modules} module(s)`, ...countLines(kinds));
  out.push(`  sections outside Known gaps: ${sections.length}`);
  out.push(`  of the ${found.length}, ${kinds.retired.length} owned by a retired goal -- refused by ` +
    "`--check` like every other goal owner, because the chain will not reach it", "");
  console.log(out.join("\n"));
  reportRegisters(regs);
}

function runCheck(kinds: Kinds, regs: Register[]): number {
  const bad = REFUSED.flatMap((kind) => kinds[kind].map((g) => ({ ...g, why: g.why ?? WHY[kind]! })))
    .sort((a, b) => cmp(a.module, b.module) || cmp(a.slug, b.slug));
  for (const g of bad) console.log(`${g.module}: gap ${g.slug} -- ${g.why}`);
  if (bad.length > 0) {
    console.log(
      `nv owners: ${bad.length} recorded gap(s) name an owner this gate refuses, each for the reason on ` +
        `its own line. The one owner a gap may name is a milestone at M${FIRST_FUTURE_MILESTONE} or later ` +
        "whose plan file states the scope, written as the record's `milestone`; `bun nv owners " +
        "--deferrals` is what holds that scope honest.",
    );
  } else {
    console.log(
      `nv owners: every one of the ${kinds.milestone.length} tagged gap(s) names an owner the chain or the ` +
        "plan knows -- a milestone still ahead of the program, which since this gate held whole is the " +
        "only owner it accepts",
    );
  }
  console.log([...countLines(kinds), registerLine(regs)].join("\n"));
  return bad.length > 0 ? 1 : 0;
}

function runCloses(found: Gap[], slug: string): number {
  const owned = found.filter((g) => g.owner === slug);
  if (owned.length === 0) {
    console.log(`nv owners: goal \`${slug}\` owns no module-doc gap`);
    return 0;
  }
  for (const g of owned) console.log(lineOf(g));
  console.log(
    `nv owners: goal \`${slug}\` still owns ${owned.length} module-doc gap(s). A goal is reached when ` +
      `each is built and its record deleted, struck as a stated bound, or deferred to a milestone at ` +
      `M${FIRST_FUTURE_MILESTONE} or later whose plan states the scope; a tag is not a build.`,
  );
  return 1;
}

/** What `--deferrals` reads an item for: a backticked token, or a word long enough to be about it. */
const KEYWORD = /`([^`\s]{3,})`/g;
const WORD = /\b([A-Za-z][A-Za-z0-9_]{5,})\b/g;
const STEM = 6;
/** Words long enough to pass `WORD` that are this repository's prose vocabulary, not a subsystem. */
const PROSE = new Set([
  "because", "instead", "rather", "already", "against", "nothing", "anything", "everything",
  "itself", "second", "single", "whole", "written", "writes", "reading", "carries",
  "answers", "entries", "program", "milestone", "repository", "whatever", "however",
  "therefore", "without", "within", "through", "themselves", "something",
]);

/** What the milestone's plan says that covers this gap, strongest first, or "" when it says nothing. */
function covers(text: string, g: Gap): string {
  const lowered = text.toLowerCase();
  const body = `${g.title} ${g.text}`;
  const crate = g.module.split("/")[1] ?? "";
  for (const path of [g.module, crate, crate.replace(/^nvs-/, "")]) {
    if (path && lowered.includes(path.toLowerCase())) return path;
  }
  for (const m of body.matchAll(KEYWORD)) if (lowered.includes(m[1]!.toLowerCase())) return `\`${m[1]}\``;
  for (const m of body.matchAll(WORD)) {
    const w = m[1]!;
    if (!PROSE.has(w.toLowerCase()) && lowered.includes(w.slice(0, STEM).toLowerCase())) return w;
  }
  return "";
}

function runDeferrals(kinds: Kinds): number {
  let missed = 0;
  const out = ["== EVERY GAP DEFERRED TO A MILESTONE AHEAD OF THE PROGRAM"];
  for (const [owner, gaps] of byOwner(kinds.milestone)) {
    const path = `docs/plan/${owner.toLowerCase()}.md`;
    const text = existsSync(join(ROOT, path)) ? readFileSync(join(ROOT, path), "utf8") : "";
    out.push(`  ${owner} -- ${gaps.length} item(s), against ${path}`);
    for (const g of gaps) {
      out.push(lineOf(g));
      const how = text ? covers(text, g) : "";
      if (how) out.push(`      scoped there by ${how}`);
      else {
        missed++;
        out.push(text ? `      ${path} states no scope covering this item` : `      ${path} does not exist`);
      }
    }
  }
  if (kinds.milestone.length === 0) out.push("  none");
  if (missed > 0) {
    out.push("", `== ${missed} deferral(s) name a milestone whose plan does not state the scope. Either the ` +
      "item is owned by a goal on the chain, or the milestone's own file is where the scope belongs -- " +
      "written there for its own sake, never to make a tag pass.");
  } else out.push("", "== every deferral names a future milestone whose plan states the scope");
  console.log(out.join("\n"));
  return missed > 0 ? 1 : 0;
}

export async function run(args: string[]): Promise<number> {
  const known = new Set(["--check", "--registers", "--deferrals", "--untagged", "--unowned", "--json",
    "--untagged-is-an-error", "--reasons", "--past-is-an-error"]);
  const at = args.indexOf("--closes");
  const slug = at >= 0 ? args[at + 1] : undefined;
  const rest = args.filter((a, i) => i !== at && !(at >= 0 && i === at + 1));
  const stray = rest.filter((a) => !known.has(a));
  if (stray.length > 0 || (at >= 0 && !slug)) {
    console.error(`nv owners: ${stray.length > 0 ? `unknown argument ${stray.join(" ")}` : "--closes needs a goal slug"}`);
    return 2;
  }
  const has = (flag: string) => rest.includes(flag);
  const found = collect();
  const kinds = classify(found);
  const regs = registers(found);

  if (has("--json")) {
    const counts = Object.fromEntries(regs.map((r) => [r.name, { items: r.owners.length, owners: tally(r.owners) }]));
    console.log(JSON.stringify({ ...kinds, registers: counts, sections: outsideBlocks(), first_future_milestone: FIRST_FUTURE_MILESTONE }, null, 2));
    return 0;
  }
  if (has("--registers")) {
    reportRegisters(regs);
    return 0;
  }
  if (slug) return runCloses(found, slug);
  if (has("--deferrals")) return runDeferrals(kinds);
  if (has("--check")) return runCheck(kinds, regs);
  if (has("--untagged")) {
    for (const g of kinds.untagged) console.log(lineOf(g));
    console.log(`\n  ${kinds.untagged.length} of ${found.length} item(s) name nobody`);
    return 0;
  }
  if (has("--unowned")) {
    for (const g of kinds.unowned) console.log(lineOf(g));
    console.log(`\n  ${kinds.unowned.length} item(s) still tagged \`unowned\`, which is no longer an owner kind`);
    return 0;
  }
  report(kinds, found, regs, outsideBlocks());
  return 0;
}
