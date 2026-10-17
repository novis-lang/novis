// `bun nv owners`: every known gap and every designed rule, and who owns each. A gap is a record,
// `data/gaps/<crate>/<slug>.json`, and a designed rule is a rule record, `data/rules/<topic>/<slug>.json`,
// whose `status` is `designed`. Each names one owner: a goal by slug, or a milestone by id. This command
// classifies each owner against the chain, the goal files and the plan, and `--check` is the gate over
// both classifications and over the prose scan below.
//
// The kinds a gap's owner falls into, and what `--check` does with each:
//
//   milestone  a milestone with a record under `data/plan/milestones/`. The plan holds only the
//              milestones still ahead, since a finished one's record and file are deleted, so this is
//              scheduled work and not an unclosed hole; `--deferrals` holds each one to the scope its own
//              `docs/plan/<id>.md` states.
//   live       the goal `docs/goals/README.md`'s chain names live, or the side goal this process runs. Accepted,
//              because the sweep's owner gate (`--closes`) will not let that goal walk while the gap
//              names it, so the gap cannot outlive its owner.
//   goal       a goal further down the chain. Accepted for the same reason: it becomes live before it
//              is reached, and its `--closes` gate holds the gap then. Deleting its file makes the
//              owner `broken`, which this gate refuses.
//   broken     an owner that resolves to no goal or milestone record, or a goal file that is neither on
//              the chain nor running.
//   unowned    the retired word `unowned`, refused by name so the reader knows the edit is to choose an
//              owner rather than to fix a typo.
//   untagged   no owner at all.
//
// A gap leaves the gate one of three ways: built and its record deleted, struck as a stated bound in
// the module's own prose, or deferred to a milestone whose plan states the scope. `--closes <slug>` is
// the gate a goal meets at its end: it exits 1 while any gap names that goal, because a tag is not a
// build.
//
// A designed rule's owner falls into the same kinds with one difference: any goal whose file is under
// `docs/goals/`, live or not, is an accepted owner, because deleting that file makes the owner `broken`.
// So a designed rule is accepted when it names such a goal or a milestone with a record, and refused
// when it is `broken` or `untagged`. The roster and `--json` report the
// designed rules, `--check` exits 1 while any is refused, and `--closes <slug>` exits 1 while any names
// that goal, because a designed rule is reached when it ships.
//
// `--registers` counts every place owed work is written down: the gap records, the refusal runs in
// `docs/agent/carried-refusals.md`, the keys in `crates/nvs-stdlib/tests/*-outstanding.txt`, the
// guard names in `docs/agent/guard-name-debt.md`, and the playbook bullets whose `until` names a state
// of the tree. It takes their counts and nothing else, since each has a gate over its own discipline.
// The prose scan finds every gap that lives only in prose: a module doc heading that words owed work
// outside a gap block, a `Known gaps` block that is not the one pointer line to its module's records,
// and a numbered `gap N` reference. It reads Rust sources and tests, crate manifests and conformance
// cases. The roster lists its findings, `--prose` lists them alone, and both `--prose` and `--check`
// exit 1 while any remains, because no owner tag reaches a gap written only in prose.
//
//   bun nv owners                 the roster, by kind, gaps and then designed rules
//   bun nv owners --check         the gate; exit 1 with a line per refused gap, designed rule or prose gap
//   bun nv owners --closes SLUG   exit 1 while any gap or designed rule names that goal, each listed
//   bun nv owners --deferrals     each milestone owner against its plan file's scope
//   bun nv owners --registers     one line per register, and how many items each holds
//   bun nv owners --prose         every gap that lives only in prose; exit 1 while any remains
//   bun nv owners --untagged | --unowned   only the gaps of that kind
//   bun nv owners --json          every kind, the designed rules by kind, and every register with its
//                                 owners
//
// `--untagged-is-an-error` and `--reasons` are accepted and do nothing: `--check` refuses both on its
// own.

import { existsSync, readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { chainGoals, goalPlan, liveGoal, sideGoal as runningSideGoal } from "../lib/chain.ts";
import { ROOT } from "../lib/paths.ts";
import { load } from "../lib/store.ts";
import { gap as gapType } from "../schema/gap.ts";
import { milestone as milestoneType } from "../schema/milestone.ts";
import { rule as ruleType } from "../schema/rule.ts";

export const summary = "who owns each known gap and designed rule, and the gate over it: nv owners [--check | --closes <slug> | --registers | ...]";

const UNOWNED = "unowned";
/** The playbook's bullets: one file each, under a directory per section. */
const PLAYBOOK = "docs/agent/playbook";
const MILESTONE = /^M(\d+)[A-Z]?$/;

const CARRIED_GAPS = "docs/agent/carried-gaps.md";
const CARRIED_REFUSALS = "docs/agent/carried-refusals.md";
const GUARD_DEBT = "docs/agent/guard-name-debt.md";
const RATCHETS = "crates/nvs-stdlib/tests/*-outstanding.txt";
/** What the prose scan reads: every Rust source and test, every crate manifest, every conformance case. */
const PROSE_SOURCES = ["crates/*/src/**/*.rs", "crates/*/tests/**/*.rs", "crates/*/Cargo.toml", "tests/**/*.nvst"];

export interface Gap {
  slug: string;
  module: string;
  title: string;
  text: string;
  owner: string;
  why?: string;
}

export type Kind = "live" | "goal" | "milestone" | "unowned" | "untagged" | "broken";
export type Kinds = Record<Kind, Gap[]>;

/** Every kind `--check` refuses: all but `milestone`, `live` and `goal`. */
export const REFUSED: Kind[] = ["broken", "untagged", "unowned"];

/** The label each kind is counted under, one count per line so a `want` of `0` cannot match `10`. */
const LABELS: [Kind, string][] = [
  ["live", "live-goal-owned"], ["goal", "goal-owned"], ["milestone", "milestone-owned"],
  ["unowned", "unowned"], ["untagged", "untagged"], ["broken", "broken-tag"],
];

/** The sentence a refused gap gets when its kind has only one way of being wrong. */
const WHY: Partial<Record<Kind, string>> = {};

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

/** The ids of the milestone records, as an owner is classified against them. */
type Plan = Set<string>;

function milestonePlan(root: string = ROOT): Plan {
  return new Set(load(milestoneType, root).map((m) => m.id));
}

/** The kinds a gap and a designed rule share, and the reason for each that is refused. */
interface Resolved {
  kind: "untagged" | "broken" | "milestone" | "goal";
  why?: string;
}

/**
 * Where `owner` stands: a milestone id against `plan`, anything else as a goal slug that `isGoal` knows.
 * A milestone with no record is finished and deleted, or never existed, so naming it is `broken`.
 */
function resolve(owner: string, plan: Plan, isGoal: (slug: string) => boolean): Resolved {
  if (!owner) return { kind: "untagged", why: "no owner" };
  if (!MILESTONE.test(owner)) return isGoal(owner) ? { kind: "goal" } : { kind: "broken", why: `no goal \`${owner}\` in docs/goals/` };
  if (!plan.has(owner)) {
    return { kind: "broken", why: `no milestone ${owner} under data/plan/milestones/; name a goal or a milestone still ahead` };
  }
  return { kind: "milestone" };
}

/** Sorts every gap into its kind, with the reason for each kind that has more than one. */
export function classify(found: Gap[]): Kinds {
  // A side goal owns a gap exactly as a chain goal does, and its end gate asks `--closes` of its slug.
  const plan = milestonePlan();
  const chain = chainGoals();
  const running = new Set([liveGoal(chain)?.slug, runningSideGoal()].filter((s): s is string => !!s));
  const onChain = new Set(chain.map((g) => g.slug));
  const out: Kinds = { live: [], goal: [], milestone: [], unowned: [], untagged: [], broken: [] };
  for (const gap of found) {
    if (gap.owner === UNOWNED) {
      out.unowned.push({
        ...gap,
        why: "`unowned` is a retired owner kind; a gap is owned by a goal on the chain or by a " +
          "milestone whose plan states the scope",
      });
      continue;
    }
    const r = resolve(gap.owner, plan, (slug) => goalPlan(slug) !== null);
    if (r.kind === "goal" && !running.has(gap.owner) && !onChain.has(gap.owner)) {
      out.broken.push({
        ...gap,
        why: `goal \`${gap.owner}\` is on no chain and not running, so no end gate holds the gap; ` +
          "put the goal on the chain, or name a goal that is",
      });
      continue;
    }
    const kind = r.kind === "goal" && running.has(gap.owner) ? "live" : r.kind;
    out[kind].push(r.why ? { ...gap, why: r.why } : gap);
  }
  return out;
}

/** A rule whose `status` is `designed`, and the owner its record names: a milestone id or a goal slug. */
export interface DesignedRule {
  id: string;
  title: string;
  owner: string;
  why?: string;
}

export type RuleKind = Resolved["kind"];
export type RuleKinds = Record<RuleKind, DesignedRule[]>;

/** Every kind the designed rules' gate refuses: all but `goal` and `milestone`. */
export const RULE_REFUSED: RuleKind[] = ["broken", "untagged"];

const RULE_LABELS: [RuleKind, string][] = [
  ["goal", "designed rules owned by a goal file"], ["milestone", "designed rules owned by a milestone"],
  ["broken", "designed rules with a broken owner"], ["untagged", "designed rules naming no owner"],
];

/** Every designed rule, ordered by id. */
export function collectRules(root: string = ROOT): DesignedRule[] {
  return load(ruleType, root)
    .filter((r) => r.value?.status === "designed")
    .map((r) => ({ id: r.id, title: r.value.title, owner: r.value.milestone ?? r.value.goal ?? "" }))
    .sort((a, b) => cmp(a.id, b.id));
}

/**
 * Sorts every designed rule into its kind. Unlike a gap's, a rule's goal need not be running: any goal
 * whose file is under `docs/goals/` is accepted, because deleting that file makes the owner `broken`.
 */
export function classifyRules(found: DesignedRule[], root: string = ROOT): RuleKinds {
  const plan = milestonePlan(root);
  const isGoal = (slug: string) => existsSync(join(root, "docs", "goals", `${slug}.md`));
  const out: RuleKinds = { goal: [], milestone: [], broken: [], untagged: [] };
  for (const rule of found) {
    const r = resolve(rule.owner, plan, isGoal);
    out[r.kind].push(r.why ? { ...rule, why: r.why } : rule);
  }
  return out;
}

/** Where a gap can still live only in prose: one of the three kinds `outsideBlocks` reports. */
type ProseKind = "heading" | "block" | "numbered";
interface Prose { file: string; line: number; kind: ProseKind; lead: string }

/** What each kind of prose gap is, and the edit that clears it. */
const PROSE_WHY: Record<ProseKind, string> = {
  heading: "a doc heading words owed work outside a gap block: make it a record, or rewrite the heading to say what the module does",
  block: "a `Known gaps` block is not the one pointer line to its own module's records, or no record names the module: move its prose into records, or delete the block",
  numbered: "a numbered gap reference: cite the record's slug, or say what the code does without it",
};

/** Every place a gap is written down in prose with no record behind it. Three kinds:
 *
 *  - `heading`: a module doc heading that words owed work while opening no gap block.
 *  - `block`: a `# Known gap(s)` heading in a `//!` or `///` doc comment whose first line is not
 *    `Each gap is a record, and \`bun nv gaps --module <this file>\` lists them.`, or whose file no
 *    record's `module` names.
 *  - `numbered`: `gap N` on any line of a source, test, manifest or conformance case, which cites a
 *    numbered list a record does not keep. */
function outsideBlocks(found: Gap[]): Prose[] {
  const DOC = /^\s*\/\/[!/](?: ?(.*))?$/;
  const HEADING = /^#{1,6}\s+(\S.*?)\s*$/;
  const GAPS = /^Known gaps?\b/i;
  // Word-bounded, because `lowers` and `borrowed` both contain `owe`.
  const OWED = /\b(?:not(?:\s+\w+){0,3}\s+yet|owe[sd]?|owing|still missing|not armed|gaps?)\b/i;
  const NUMBERED = /\bgap\s+\d+\b/i;
  const recorded = new Set(found.map((g) => g.module));
  const pointer = (file: string) => `Each gap is a record, and \`bun nv gaps --module ${file}\` lists them.`;
  const out: Prose[] = [];
  const files = PROSE_SOURCES.flatMap((glob) => [...new Bun.Glob(glob).scanSync({ cwd: ROOT })])
    .map((f) => f.replaceAll("\\", "/"));
  for (const file of [...new Set(files)].sort()) {
    const lines = readFileSync(join(ROOT, file), "utf8").split("\n");
    const rust = file.endsWith(".rs");
    for (let n = 0; n < lines.length; n++) {
      const text = lines[n]!;
      if (NUMBERED.test(text)) out.push({ file, line: n + 1, kind: "numbered", lead: oneLine(text.replace(/^\s*(?:\/\/[!/]?|#)\s?/, "")) });
      const doc = rust ? DOC.exec(text) : null;
      const h = doc ? HEADING.exec(doc[1] ?? "") : null;
      if (!h) continue;
      if (!GAPS.test(h[1]!)) {
        if (text.trimStart().startsWith("//!") && OWED.test(h[1]!)) out.push({ file, line: n + 1, kind: "heading", lead: oneLine(h[1]!) });
        continue;
      }
      let next = n + 1;
      while (next < lines.length && (DOC.exec(lines[next]!)?.[1] ?? "").trim() === "" && DOC.test(lines[next]!)) next++;
      const first = (DOC.exec(lines[next] ?? "")?.[1] ?? "").trim();
      if (first !== pointer(file) || !recorded.has(file)) out.push({ file, line: n + 1, kind: "block", lead: first === "" ? h[1]! : oneLine(first) });
    }
  }
  return out;
}

/** `--prose`: every prose gap, one line each, and exit 1 while there is any. */
function runProse(found: Gap[]): number {
  const prose = outsideBlocks(found);
  for (const kind of Object.keys(PROSE_WHY) as ProseKind[]) {
    const of = prose.filter((p) => p.kind === kind);
    if (of.length === 0) continue;
    console.log(`== ${kind}: ${PROSE_WHY[kind]}`);
    for (const p of of) console.log(`  ${p.file}:${p.line}  ${cut(p.lead, 90)}`);
    console.log("");
  }
  console.log(prose.length === 0 ? "== no gap lives only in prose" : `== ${prose.length} prose gap(s) with no record behind them`);
  return prose.length > 0 ? 1 : 0;
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
/** One empty owner per playbook bullet file, since a bullet names no owner. */
function playbookBullets(): string[] {
  const dirs = readdirSync(join(ROOT, PLAYBOOK), { withFileTypes: true }).filter((d) => d.isDirectory());
  return dirs.flatMap((d) => readdirSync(join(ROOT, PLAYBOOK, d.name)).filter((f) => f.endsWith(".md")).map(() => ""));
}

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
  const all: (Omit<Register, "owners"> & { read: () => string[] })[] = [
    { name: "module docs", where: "data/gaps/", read: () => found.map((g) => g.owner),
      what: "a gap record, naming the goal or milestone that owns it" },
    { name: "carried-refusals.md", where: CARRIED_REFUSALS, read: () => entryLines(CARRIED_REFUSALS, /^9\d\d\. /),
      what: "a run of `nvs-ir` refusal sites, numbered from 900" },
    { name: "outstanding keys", where: RATCHETS, read: ratchetOwners,
      what: "a spec or migration member `registry::CLASSES` does not declare yet, each key naming its owner in a `#` column" },
    { name: "guard-name-debt.md", where: GUARD_DEBT, read: () => entryLines(GUARD_DEBT, /^- \[/),
      what: "a guard test a goal record's check names and the tree does not hold yet" },
    { name: "playbook until", where: `${PLAYBOOK}/`, read: playbookBullets,
      what: "a trap whose `until` names the state of the tree that retires it" },
  ];
  return all.map(({ read, ...r }) => ({ ...r, owners: read() }));
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
  const refused: [Kind, string][] = [["broken", "TAGS THAT DO NOT RESOLVE"]];
  for (const [kind, head] of refused) {
    if (kinds[kind].length === 0) continue;
    out.push("", `== ${head}`);
    for (const g of kinds[kind]) out.push(lineOf(g), `      ${g.why ?? WHY[kind]}`);
  }
  const grouped: [Kind, string, (owner: string) => string][] = [
    ["live", "OWNED BY THE GOAL THAT IS RUNNING, WHICH CANNOT WALK WHILE IT OWNS ONE", (o) => `goal \`${o}\``],
    ["goal", "OWNED BY ANOTHER GOAL ON THE CHAIN", (o) => `goal \`${o}\``],
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
    out.push("", "== GAPS THAT LIVE ONLY IN PROSE, WHERE NO OWNER TAG REACHES THEM");
    for (const s of sections) out.push(`  ${s.file}:${s.line}  ${s.kind}  ${cut(s.lead, 70)}`);
    out.push("  `bun nv owners --prose` says what clears each kind");
  }
  const modules = new Set(found.map((g) => g.module)).size;
  out.push("", `== ${found.length} item(s) across ${modules} module(s)`, ...countLines(kinds));
  out.push(`  prose gaps with no record: ${sections.length}`, "");
  console.log(out.join("\n"));
  reportRegisters(regs);
}

/** The gap records' half of the gate. It reads no other register: a new playbook bullet or
 * outstanding key cannot change it, and `--registers` is where their counts are printed. */
function runCheck(kinds: Kinds): number {
  const bad = REFUSED.flatMap((kind) => kinds[kind].map((g) => ({ ...g, why: g.why ?? WHY[kind]! })))
    .sort((a, b) => cmp(a.module, b.module) || cmp(a.slug, b.slug));
  for (const g of bad) console.log(`${g.module}: gap ${g.slug} -- ${g.why}`);
  if (bad.length > 0) {
    console.log(
      `nv owners: ${bad.length} recorded gap(s) name an owner this gate refuses, each for the reason on ` +
        "its own line. A gap may name a milestone whose plan file states the scope, written as the record's `milestone`, which `bun nv owners --deferrals` holds " +
        "to that scope; or a goal on the chain or running, which `--closes` holds at its end.",
    );
  } else {
    const tagged = kinds.milestone.length + kinds.live.length + kinds.goal.length;
    console.log(
      `nv owners: every one of the ${tagged} tagged gap(s) names an ` +
        "owner the chain or the plan knows -- a milestone still ahead of the program, or a goal on the " +
        "chain or running, whose own end gate refuses to reach it while the gap is open",
    );
  }
  console.log(countLines(kinds).join("\n"));
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
      "each is built and its record deleted, struck as a stated bound, or deferred to a milestone " +
      "whose plan states the scope; a tag is not a build.",
  );
  return 1;
}

function ruleLine(r: DesignedRule): string {
  return `  rule:${r.id}  ${cut(r.title, 78)}`;
}

function ruleCountLines(kinds: RuleKinds): string[] {
  return RULE_LABELS.map(([kind, label]) => `  ${label}: ${kinds[kind].length}`);
}

/** The designed rules' part of the roster: each refused kind with its reasons, then each owner's rules. */
function reportRules(kinds: RuleKinds, found: DesignedRule[]): void {
  const out = ["", "== DESIGNED RULES, AND THE GOAL OR MILESTONE THAT SHIPS EACH"];
  for (const kind of RULE_REFUSED) {
    if (kinds[kind].length === 0) continue;
    out.push(`  ${kind} -- ${kinds[kind].length} rule(s)`);
    for (const r of kinds[kind]) out.push(ruleLine(r), `      ${r.why}`);
  }
  const grouped: [RuleKind, (owner: string) => string][] = [["goal", (o) => `goal \`${o}\``], ["milestone", (o) => o]];
  for (const [kind, name] of grouped) {
    const owners = new Map<string, DesignedRule[]>();
    for (const r of kinds[kind]) owners.set(r.owner, [...(owners.get(r.owner) ?? []), r]);
    for (const [owner, rules] of [...owners].sort((a, b) => cmp(a[0], b[0]))) {
      out.push(`  ${name(owner)} -- ${rules.length} rule(s)`, ...rules.map(ruleLine));
    }
  }
  if (found.length === 0) out.push("  none -- every rule is shipped");
  out.push("", `== ${found.length} designed rule(s)`, ...ruleCountLines(kinds));
  console.log(out.join("\n"));
}

/** The exit code the designed rules' owner gate adds to `--check` and `--closes`: the rules' own verdict. */
function ruleVerdict(code: number): number {
  return code;
}

/** The designed rules' gate: a line per refused rule and the counts. Returns 1 when any is refused. */
function runRuleCheck(kinds: RuleKinds): number {
  const bad = RULE_REFUSED.flatMap((kind) => kinds[kind]).sort((a, b) => cmp(a.id, b.id));
  for (const r of bad) console.log(`rule:${r.id} -- ${r.why}`);
  if (bad.length > 0) {
    console.log(
      `nv owners: ${bad.length} designed rule(s) name no owner this gate accepts. A designed rule names ` +
        "the goal that ships it, as the record's `goal`, whose file is under docs/goals/; or a milestone " +
        "with a record under data/plan/milestones/, as the record's `milestone`.",
    );
  } else console.log(`nv owners: every designed rule names a goal file or a milestone still ahead`);
  console.log(ruleCountLines(kinds).join("\n"));
  return bad.length > 0 ? 1 : 0;
}

/** Goal `slug`'s designed rules. Prints nothing when it owns none, so a gap's last line stays last. */
function runRuleCloses(found: DesignedRule[], slug: string): number {
  const owned = found.filter((r) => r.owner === slug);
  if (owned.length === 0) return 0;
  for (const r of owned) console.log(ruleLine(r));
  console.log(
    `nv owners: goal \`${slug}\` still owns ${owned.length} designed rule(s). A goal is reached when each ` +
      "is shipped and its record says so, or names the goal or milestone that ships it instead.",
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
  const known = new Set(["--check", "--registers", "--deferrals", "--prose", "--untagged", "--unowned", "--json",
    "--untagged-is-an-error", "--reasons"]);
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

  if (has("--json")) {
    const counts = Object.fromEntries(registers(found).map((r) => [r.name, { items: r.owners.length, owners: tally(r.owners) }]));
    const rules = classifyRules(collectRules());
    console.log(JSON.stringify({ ...kinds, designed_rules: rules, registers: counts, sections: outsideBlocks(found) }, null, 2));
    return 0;
  }
  if (has("--registers")) {
    reportRegisters(registers(found));
    return 0;
  }
  if (slug) {
    const gaps = runCloses(found, slug);
    return Math.max(gaps, ruleVerdict(runRuleCloses(collectRules(), slug)));
  }
  if (has("--deferrals")) return runDeferrals(kinds);
  if (has("--prose")) return runProse(found);
  if (has("--check")) {
    const gaps = runCheck(kinds);
    const rules = ruleVerdict(runRuleCheck(classifyRules(collectRules())));
    return Math.max(gaps, rules, runProse(found));
  }
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
  report(kinds, found, registers(found), outsideBlocks(found));
  const rules = collectRules();
  reportRules(classifyRules(rules), rules);
  return 0;
}
