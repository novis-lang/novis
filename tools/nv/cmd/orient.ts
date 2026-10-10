// `bun nv orient`: a loop session's step 1 in one call, narrowed to the current goal. The pack is the
// run marker and the driver's last verdict, the handoff's state and current item, the code at every
// `path:line` the item names, the goal's standing decisions, then what the goal's `context` selects —
// rules, decision-record sections, map lines, convention shapes, playbook traps, plan fields and
// milestones — and last the next free numbers and the closing block with the wrap skeleton. Every
// section but the traps is `cmd/pack.ts`'s, which `bun nv pack` prints alone, and this file prints
// into its output buffer. Every part is sliced out of its live file when the pack is built, so
// nothing in it is a copy.
//
// The goal and its handoff are records, `data/goals/<slug>.json` and `<slug>.handoff.json`, under
// `data/goals/side/` for a side goal, and the standing decisions are read from the goal's prose. The
// goal's `context` is the base selection, and the stage the handoff's next group names adds its own
// `context` on top: an overlay only ever adds, so a stage that forgot an entry prints what the base
// prints. A failing acceptance check is found among the record's checks by the label the ledger
// quotes. The goal's position is `N of M` in `data/chain.json`.
//
// The pack exists to save a session turns, not bytes: the item's anchors are printed inline because
// each one replaces a `peek` call, and the traps are narrowed to the item's own paths, or to the goal's
// `context.modules` when the item names none, because a trap about a file the session never opens
// replaces no call at all. `--audit` appends what each section
// cost, and never exits non-zero over a size.
//
//   bun nv orient              the pack
//   bun nv orient --audit      and what each section cost, in bytes and approximate tokens
//   bun nv orient --item N     pin checklist item N instead of the first unticked one
//   bun nv orient --stage N    apply stage N's context instead of the one the handoff names
//   bun nv orient --full       ignore the narrowing and print every module
//   bun nv orient --traps <path>...  the traps section alone, ranked from the whole playbook against those paths
//   bun nv orient --goal SLUG  read the context of goal SLUG instead of the live goal's

import { existsSync } from "node:fs";
import { join } from "node:path";
import { chainGoals, goalPlan, liveGoal, sideGoal } from "../lib/chain.ts";
import { ROOT } from "../lib/paths.ts";
import { run as runProc } from "../lib/proc.ts";
import { pyRepr } from "../lib/py.ts";
import { readPlaybook } from "../lib/playbook-fragments.ts";
import { load } from "../lib/store.ts";
import { goal as goalType, sideGoal as sideGoalType } from "../schema/goal.ts";
import { handoff as handoffType, sideHandoff as sideHandoffType } from "../schema/handoff.ts";
import {
  begin,
  type BookRule,
  closeLedger,
  CONVENTIONS,
  costs,
  DECISIONS,
  emit,
  type GoalValue,
  manifest,
  nbytes,
  normalize,
  printed,
  read,
  type RecordCheck,
  rulebook,
  runAdrs,
  runMap,
  runMilestones,
  runNumbers,
  runPlan,
  runRules,
  runShapes,
  section,
  sectionOpening,
  SHAPE_IMPLIES,
  sliceSection,
  STAGE_FIELDS,
  stripFrontmatter,
  stripLinks,
  titleMatches,
  tool,
  warn,
} from "./pack.ts";

export const summary = "the loop session's orientation pack, narrowed to the live goal: nv orient [--audit] [--item N] [--stage N] [--full] [--goal SLUG] | --traps <path>...";

const PLAYBOOK = "docs/agent/playbook";
const RUNNING = ".loop/running";
const INTERRUPTED = ".loop/interrupted.json";
const LEDGER = ".loop/log.md";
const DOCGATE = ".loop/doc-gate.json";
const OWNERGATE = ".loop/owner-gate.json";
const CALIBRATION = "tools/data/calibration.json";

const TRAPS_TITLE = "THE TRAPS THAT APPLY HERE";

/**
 * How many of the item's path-promoted traps print whole. The rest keep their one-line lead-in, so
 * nothing goes out of reach; the traps are the pack's largest section, and ranking puts the bullets
 * naming the item's own files first. A handoff reporting a trap it needed and found only in the
 * lead-in list is the signal this is too low.
 */
const PROMOTED_WHOLE = 20;

/**
 * The traps for reading a failing acceptance check. When the driver's verdict names a check that should
 * already pass they are listed one lead-in each, with the selector that prints one; otherwise a single
 * line says they are held back, whatever the manifest names.
 */
const TRIAGE = "Tooling > a goal record's*";

/** A `path:line` anchor in a checklist item, which the pack expands into a window of the file. */
export const ANCHOR_RE = /\b((?:crates|extensions|tools|tests|benches|examples|fuzz|docs|editors|website|apps)\/[\w./-]+\.\w+):(\d+)\b/g;

/** A path a checklist item names, file or directory, which the traps are narrowed to. */
const ITEM_PATH_RE = /\b((?:crates|extensions|tools|tests|benches|examples|fuzz|docs|editors|website|apps)\/[\w./-]*[\w-])/g;

/** Lines printed either side of an anchor: a signature and the top of a body. */
const ANCHOR_CONTEXT = 12;

/** Lines of the failing check's block printed, and of each sibling's, and how many siblings. */
const CHECK_BLOCK_LINES = 60;
const SIBLING_BLOCK_LINES = 14;
const STAGE_SIBLINGS = 12;

/** Bytes per token when no calibration is on disk, and calls per session, for `--audit`. */
const BYTES_PER_TOKEN_FALLBACK = 2.5;
const CALLS_PER_SESSION = 98;

interface HandoffValue {
  state: string;
  next: {
    stage: number | null;
    title?: string;
    files: string[];
    note?: string;
    items: { done: boolean; text: string }[];
    after?: string;
  };
  backlog: string[];
}

/** Where the goal this pack is for is read from. */
interface Sources {
  slug: string;
  /** The goal's own prose, which the pack prints the standing decisions from. */
  md: string;
  /** The goal's record. */
  record: string;
  /** The goal's handoff record. */
  handoff: string;
  side: boolean;
  /** `N of M` on the chain, or null for a side goal. */
  position: string | null;
}

// ----------------------------------------------------------------------------- output

let trapsDetail: string[] = [];

async function git(...args: string[]): Promise<string> {
  const r = await runProc(["git", ...args]);
  return r.code === 0 ? r.stdout.trim() : "";
}

// ------------------------------------------------------------------------ the goal

function sources(goalFlag: string | null): Sources | null {
  const side = sideGoal();
  if (side !== null) {
    return { slug: side, md: `docs/agent/goals/side/${side}.md`, record: `data/goals/side/${side}.json`, handoff: `data/goals/side/${side}.handoff.json`, side: true, position: null };
  }
  const goals = chainGoals();
  const slug = goalFlag ?? liveGoal(goals)?.slug;
  if (slug === undefined) return null;
  const at = goals.findIndex((g) => g.slug === slug);
  return {
    slug,
    md: goals[at]?.md ?? `docs/agent/goals/${slug}.md`,
    record: `data/goals/${slug}.json`,
    handoff: `data/goals/${slug}.handoff.json`,
    side: false,
    position: at < 0 ? null : `${at + 1} of ${goals.length}`,
  };
}

function goalRecord(src: Sources): GoalValue | null {
  const stored = load<any>(src.side ? sideGoalType : goalType).find((g) => g.id === src.slug);
  return stored ? (stored.value as GoalValue) : null;
}

function handoffRecord(src: Sources): HandoffValue | null {
  const stored = load<any>(src.side ? sideHandoffType : handoffType).find((h) => h.id === src.slug);
  return stored ? (stored.value as HandoffValue) : null;
}

// ------------------------------------------------------------------ the driver's verdict

/** `(session, failure)` of the driver's last acceptance run in the ledger; `failure` is "" when it passed. */
function lastAcceptance(): [string, string] | null {
  const text = read(LEDGER);
  if (!text) return null;
  let session = "";
  let cost = false;
  let fail = "";
  let found: [string, string] | null = null;
  for (const line of text.split("\n")) {
    const entry = /^- (\d{4}) /.exec(line);
    if (entry) {
      [session, cost, fail] = [entry[1]!, false, ""];
      continue;
    }
    const body = line.trim();
    // A scoped sweep that came back green is followed by the sweep over the whole plan, whose own
    // `goal cost:` line is the verdict; the scoped one says only that the second sweep started.
    if (body.startsWith("goal cost:") && !body.includes("(scoped; opening the floor gate)")) cost = true;
    else if (body.startsWith("goal check:")) fail = body.slice("goal check:".length).trim();
    else if (fail && (body.startsWith("also red:") || body.startsWith("(and "))) fail += "\n" + body;
    if (cost && session) found = [session, fail];
  }
  return found;
}

/** Whether the last verdict names a check that should already pass, which is what the triage traps help read. */
function triageApplies(stage: number | null): boolean {
  const verdict = lastAcceptance();
  if (verdict === null || !verdict[1]) return false;
  const fail = verdict[1];
  const m = /\[(\d+)([^\]]*)\]:/.exec(fail);
  if (m === null) return !fail.includes(" is missing -- ");
  if (m[1]!.startsWith("0")) return true;
  return stage === null || Number(m[1]) < stage;
}

/** A goal-end gate's `{failed, session}` file, as (session, finding), or null when it is green. */
function gateFailure(path: string): [string, string] | null {
  let state: unknown;
  try {
    state = JSON.parse(read(path));
  } catch {
    return null;
  }
  if (typeof state !== "object" || state === null || Array.isArray(state)) return null;
  const s = state as Record<string, unknown>;
  const failed = String(s.failed ?? "").trim();
  if (!failed) return null;
  return [String(s.session ?? "").trim() || "an earlier session", failed];
}

/** A stage's label as the ledger writes it: its number and title. */
function stageLabel(goal: GoalValue, n: number): string {
  const s = goal.stages.find((x) => x.number === n);
  return s ? `${s.number} ${s.title}` : String(n);
}

/**
 * The check(s) the ledger's `goal check:` line opens with. The driver labels a check by its name, its
 * file or its id, then its stage label in brackets, so each of those is tried.
 */
export function locateCheck(goal: GoalValue, fail: string): RecordCheck[] {
  if (!fail) return [];
  return (goal.checks ?? []).filter((c) => {
    const label = stageLabel(goal, c.stage);
    return [c.name, c.file, c.id].some((n) => typeof n === "string" && fail.startsWith(`${n} [${label}]`));
  });
}

/** A check as the pack prints it: its fields as JSON, one per line, capped at `max` lines. */
function checkLines(c: RecordCheck, max: number): string[] {
  const lines = JSON.stringify(c, null, 2).split("\n");
  return lines.length <= max ? lines : [...lines.slice(0, max), `... ${lines.length - max} more line(s)`];
}

function emitCheckBlock(src: Sources, goal: GoalValue, fail: string): void {
  const hits = locateCheck(goal, fail);
  if (hits.length === 0) return;
  if (hits.length > 1) {
    emit();
    emit(`${hits.length} checks carry that exact label, so which of them failed does not follow`);
    emit(`from the ledger line: ${hits.map((c) => `\`${c.id}\``).join(", ")} in ${src.record}.`);
    emit("The one naming what failed above is yours.");
    return;
  }
  const c = hits[0]!;
  emit();
  emit(`That check is \`${c.id}\` in ${src.record}, and it is printed here in full -- it is`);
  emit("what this session exists to turn green, so do not go and find it:");
  emit();
  for (const line of checkLines(c, CHECK_BLOCK_LINES)) emit(`  ${line}`);
  emitStageSiblings(goal, c);
}

/** The rest of the failing check's stage: what closing the stage means. */
function emitStageSiblings(goal: GoalValue, hit: RecordCheck): void {
  const label = stageLabel(goal, hit.stage);
  if (hit.stage === 0) return;
  const siblings = (goal.checks ?? []).filter((c) => c.stage === hit.stage && c.id !== hit.id);
  if (siblings.length === 0) return;
  emit();
  emit(`The rest of stage ${pyRepr(label)} -- ${siblings.length} more check(s). The stage goes green`);
  emit("only when these do too, so they are what closing the one above actually means:");
  for (const c of siblings.slice(0, STAGE_SIBLINGS)) {
    emit();
    for (const line of checkLines(c, SIBLING_BLOCK_LINES)) emit(`  ${line}`);
  }
  if (siblings.length > STAGE_SIBLINGS) emit(`  ... and ${siblings.length - STAGE_SIBLINGS} more check(s) in this stage.`);
}

// ------------------------------------------------------------------------- sections

async function runMarker(src: Sources, goal: GoalValue): Promise<void> {
  section("RUN", "git, .loop/running, .loop/interrupted.json, .loop/log.md and the failing check itself");
  if (existsSync(join(ROOT, RUNNING))) {
    emit("A LOOP DRIVER HOLDS THIS TREE. Its sessions edit these files on nearly every");
    emit("iteration; do not start a by-hand pass over shared files while this says so.");
    for (const line of read(RUNNING).replace(/\n+$/, "").split("\n")) emit(`  ${line}`);
    emit();
  }
  if (existsSync(join(ROOT, INTERRUPTED))) {
    let cut: Record<string, unknown> = {};
    try {
      const parsed = JSON.parse(read(INTERRUPTED));
      if (parsed && typeof parsed === "object") cut = parsed;
    } catch {
      cut = {};
    }
    const files = (Array.isArray(cut.files) ? cut.files : []).map(String).filter((f) => f.trim());
    emit(`THE PREVIOUS SESSION WAS CUT OFF at ${cut.when ?? "an unrecorded time"} --`);
    emit(`${cut.why ?? "reason unrecorded"}.`);
    emit("Everything it committed stands; one commit per slice is what buys that. The paths");
    emit("below are the slice it was in the MIDDLE of.");
    if (cut.swept) {
      emit(`THE DRIVER COMMITTED THEM as ${String(cut.head ?? "").slice(0, 9)}, so the tree you`);
      emit("open is clean. That commit has NOT been verified. Read it first, then continue");
      emit("it, amend it or revert it -- do not start new work on top of it, and do not");
      emit("assume the handoff describes it, because it was never written.");
    } else {
      emit("They are UNCOMMITTED. Read them first and either finish that slice or revert");
      emit("it -- do not start new work on top of it, and do not assume the handoff");
      emit("describes them, because it was never written.");
    }
    for (const f of files.slice(0, 20)) emit(`  ${f}`);
    if (files.length > 20) emit(`  ... and ${files.length - 20} more`);
    emit();
  }
  const branch = (await git("rev-parse", "--abbrev-ref", "HEAD")) || "(unknown)";
  const changed = (await git("status", "--short")).split("\n").filter((l) => l.trim());
  emit(`branch ${branch}, ${changed.length} path(s) with uncommitted changes`);
  emit(`head   ${(await git("log", "-1", "--oneline")) || "(no commits)"}`);
  const verdict = lastAcceptance();
  if (verdict !== null) {
    const [session, fail] = verdict;
    emit();
    if (!fail) {
      emit(`The driver's last acceptance check, after session ${session}, passed whole.`);
    } else {
      emit(`THE DRIVER'S LAST ACCEPTANCE CHECK FAILED, after session ${session}:`);
      for (const ln of fail.split("\n")) emit(`  ${ln}`);
      emit("The run ends only when every check in the goal's record passes, and nothing else");
      emit("shows a session this one -- the driver writes it to the ledger and moves on.");
      emit("The first line is the EARLIEST-STAGE failing check, so it is the one that can be");
      emit("closed without three other stages landing first. On the sweep a goal is reached");
      emit("on, every other red check follows on an `also red:` line of its own: close all");
      emit("of them this session, because the next sweep is the same length whatever is left.");
      emit();
      emit("Two failures read differently, and getting them the wrong way round is the");
      emit("expensive mistake here:");
      emit("  * A check whose artefact IS NOT WRITTEN YET -- a test that 'did not run', a");
      emit("    fixture whose member does not exist, an `unrecognized subcommand` -- is an");
      emit("    item still open. It is the ordinary state of a goal in progress. It does");
      emit("    NOT outrank the handoff's next group; if the group below is the work that");
      emit("    leads to it, do the group.");
      emit("  * A check that USED TO PASS is a regression and outranks new work outright.");
      emit("The ledger in .loop/log.md says which: the same line repeating session after");
      emit("session is the first kind, and it is not an alarm.");
      emitCheckBlock(src, goal, fail);
    }
  }
  const doc = gateFailure(DOCGATE);
  if (doc !== null) {
    emit();
    emit(`THE RUSTDOC GATE IS RED, as of session ${doc[0]}:`);
    emit(`  ${doc[1]}`);
    emit("The driver runs it only on a sweep where every acceptance check passed, and the goal is");
    emit("not reached while it is red. `bun nv verify --doc` is the whole check, and");
    emit("rustdoc names the file and the line. Fix every finding, run `--doc` until it is green,");
    emit("and say so in the handoff.");
  }
  const owner = gateFailure(OWNERGATE);
  if (owner !== null) {
    emit();
    emit(`THE OWNER GATE IS RED, as of session ${owner[0]}:`);
    emit(`  ${owner[1]}`);
    emit("The driver runs it only on a sweep where every acceptance check passed, and the goal is");
    emit(`not reached while a gap still names it. \`${tool("owners")} --closes <slug>\` and`);
    emit(`\`${tool("playbook")} --closes <slug>\` list each one. Build it and delete its item,`);
    emit("strike it as a stated bound in the module's own prose, or re-tag it to a milestone whose");
    emit("plan states the scope -- a tag is not a build -- then say so in the handoff.");
  }
}

/** The group's lead line, as the handoff writes it. */
function leadLine(next: HandoffValue["next"]): string {
  if (next.title === undefined) return next.note ?? "";
  let s = `**${next.stage === null ? "" : `Stage ${next.stage}: `}${next.title}**`;
  if (next.files.length > 0) s += ` — one file set: ${next.files.map((f) => `\`${f}\``).join(", ")}.`;
  if (next.note) s += next.files.length > 0 ? ` ${next.note}` : ` — ${next.note}`;
  return s;
}

/** Prints the handoff's state and its current item, and returns the item's text. */
function runState(src: Sources, h: HandoffValue | null, itemIndex: number | null): string {
  if (h === null) {
    warn(`${src.handoff} is missing or unreadable -- there is no state to hand over`);
    return "";
  }
  section("WHERE THE WORK STANDS", `${src.handoff} (## State, and the current item in full)`);
  emit("## State");
  emit();
  emit(h.state.trimEnd());
  emit();
  emit("## Next group");
  emit();
  emit(leadLine(h.next));
  const items = h.next.items;
  if (items.length === 0) {
    warn(`${src.handoff}'s \`## Next group\` has no \`- [ ]\` checklist items`);
    return "";
  }
  const unticked = items.flatMap((it, i) => (it.done ? [] : [i]));
  if (unticked.length === 0) {
    emit();
    emit("Every item in the group is ticked. The handoff's next-group line is the goal now;");
    emit("if it names nothing further, pick from ## Backlog and say so in the handoff.");
    return "";
  }
  let pick = itemIndex === null ? unticked[0]! : itemIndex - 1;
  if (pick < 0 || pick >= items.length) {
    warn(`--item ${itemIndex} is out of range: the group has ${items.length} item(s)`);
    pick = unticked[0]!;
  }
  const current = `- [${items[pick]!.done ? "x" : " "}] ${items[pick]!.text}`;
  emit();
  emit(`-- YOUR ITEM (${pick + 1} of ${items.length}), in full:`);
  emit();
  emit(current);
  const rest = unticked.filter((i) => i !== pick);
  if (rest.length > 0) {
    emit();
    emit("-- the rest of the group, one line each. Take a second only if it touches files you");
    emit("   have already loaded and the first left you well short of the ceiling:");
    for (const i of rest) emit(`   [${i + 1}] ${[...stripLinks(items[i]!.text.trim())].slice(0, 150).join("")}`);
  }
  return current;
}

function runAnchors(src: Sources, item: string): void {
  const seen = new Map<string, [string, number]>();
  for (const m of item.matchAll(ANCHOR_RE)) {
    const path = m[1]!.replace(/\\/g, "/");
    const key = `${path}:${m[2]}`;
    if (!seen.has(key)) seen.set(key, [path, Number(m[2])]);
  }
  if (seen.size === 0) return;
  const windows: [string, number, number][] = [];
  const missing: string[] = [];
  for (const [path, line] of seen.values()) {
    const body = read(path);
    if (!body) {
      missing.push(`${path}:${line}`);
      continue;
    }
    const lines = body.split("\n");
    if (line > lines.length) {
      missing.push(`${path}:${line} (the file has ${lines.length} lines)`);
      continue;
    }
    const lo = Math.max(1, line - ANCHOR_CONTEXT);
    const hi = Math.min(lines.length, line + ANCHOR_CONTEXT);
    const last = windows[windows.length - 1];
    if (last && last[0] === path && lo <= last[2] + 1) {
      last[2] = Math.max(last[2], hi);
      continue;
    }
    windows.push([path, lo, hi]);
  }
  if (windows.length === 0 && missing.length === 0) return;
  section("THE CODE YOUR ITEM ANCHORS", `${windows.length} window(s) at the \`path:line\` the item names -- do not peek these again`);
  windows.sort((a, b) => (a[0] < b[0] ? -1 : a[0] > b[0] ? 1 : a[1] - b[1] || a[2] - b[2]));
  for (const [path, lo, hi] of windows) {
    const lines = read(path).split("\n");
    emit(`----- ${path}:${lo}-${hi}`);
    for (let n = lo; n <= hi; n++) emit(`${String(n).padStart(5)}  ${lines[n - 1]}`);
    emit();
  }
  for (const gone of missing) warn(`${src.handoff}'s item anchors ${gone}, which does not resolve -- the anchor is stale`);
}

function runStandingDecisions(src: Sources): void {
  const text = read(src.md);
  if (!text) {
    warn(`${src.md} is missing -- the goal has no prose`);
    return;
  }
  section("THE GOAL'S STANDING DECISIONS", `${src.md} (pre-authorized, never re-opened)`);
  const block = sectionOpening(text, "standing decisions");
  if (block) emit(block);
  else warn(`${src.md} has no \`## Standing decisions\` section -- loop-authoring.md § 4`);
}

// ------------------------------------------------------------------------- the traps

export interface Bullet {
  lead: string;
  /** The bullet as Markdown, `- **lead** body [until: kind arg]`. */
  text: string;
}

export interface BookSection {
  title: string;
  bullets: Bullet[];
}

/**
 * The playbook's sections in order, each with its bullets by id, read from the fragment files. `skip`
 * leaves out the bullets of those fragment files, which is how a retirement asks what a selector
 * reaches once they are deleted.
 */
export function playbookBook(root: string = ROOT, skip: ReadonlySet<string> = new Set()): BookSection[] {
  const { sections, bullets: all } = readPlaybook(root);
  const bullets = all.filter((b) => !skip.has(b.from)).sort((a, b) => (a.id < b.id ? -1 : 1));
  return sections.map((s) => ({
    title: s.title,
    bullets: bullets
      .filter((b) => b.id.startsWith(`${s.id}/`))
      .map(({ value: v }) => ({ lead: v.lead, text: `- **${v.lead}**${v.body ? ` ${v.body}` : ""} [until: ${v.until.kind} ${v.until.arg}]` })),
  }));
}

export interface BookBullet {
  section: string;
  lead: string;
  text: string;
  /** `Section > words`, the fewest words of the lead-in that name this bullet and nothing else in its section. */
  selector: string;
}

/**
 * Every bullet in the book, each with the selector that names it alone. A key is tested as a substring of
 * each neighbour's lead-in, which is stricter than `sliceBullets`' prefix match, so it resolves to one bullet.
 */
export function allBullets(book: BookSection[]): BookBullet[] {
  return book.flatMap((s) =>
    s.bullets.map((b) => {
      const words = normalize(b.lead).split(" ");
      const peers = s.bullets.filter((p) => p !== b).map((p) => normalize(p.lead));
      let key = normalize(b.lead);
      for (let n = 2; n <= words.length; n++) {
        const k = words.slice(0, n).join(" ");
        if (!peers.some((p) => p.includes(k))) {
          key = k;
          break;
        }
      }
      return { section: s.title, lead: b.lead, text: b.text, selector: `${s.title} > ${key}` };
    }),
  );
}

function findSection(book: BookSection[], head: string): BookSection | undefined {
  const key = normalize(head);
  return book.find((s) => titleMatches(s.title, key));
}

/**
 * One `[context] playbook` selector -> the bullets it names. `Tooling` is a whole section,
 * `Tooling > a lead` the bullets in it whose lead-in opens with those words, and a bare lead the
 * same across every section. A trailing `*` claims a family and reads the same.
 */
export function sliceBullets(book: BookSection[], selector: string): [string[], string | null] {
  const gt = selector.indexOf(">");
  let head = (gt < 0 ? selector : selector.slice(0, gt)).trim();
  let lead = gt < 0 ? "" : selector.slice(gt + 1).trim();
  if (!lead) {
    const whole = findSection(book, head);
    if (whole) return [[[`## ${whole.title}`, ...whole.bullets.map((b) => b.text)].join("\n\n")], null];
    [lead, head] = [head, ""];
  }
  const key = normalize(lead);
  const pool = head ? (findSection(book, head)?.bullets ?? []) : book.flatMap((s) => s.bullets);
  const hits = pool.filter((b) => normalize(b.lead).startsWith(key)).map((b) => b.text);
  if (hits.length > 0) return [hits, null];
  return [[], `names ${pyRepr(selector)} and no bullet${head ? ` under ${pyRepr(head)}` : ""} leads with it`];
}

/** A fragment every path has, or one too short to name anything. */
const GENERIC = new Set(["src", "lib", "mod", "main", "crates", "tests", "docs", "tools", "benches", "rs", "md"]);
const usable = (names: Set<string>) => new Set([...names].filter((x) => x.length > 2 && !GENERIC.has(x)));

/** A path's strong spellings (the path, a file's name, its crate) and weak ones (the stem, the parent). */
function spellings(term: string): [Set<string>, Set<string>] {
  const t = term.trim().replace(/\\/g, "/").toLowerCase();
  if (!t.includes("/")) return [usable(new Set([t, t.replaceAll("-", "_"), t.replaceAll("_", "-")])), new Set()];
  const parts = t.split("/").filter(Boolean);
  const last = parts[parts.length - 1]!;
  const strong = new Set([t]);
  if (last.includes(".")) strong.add(last);
  const weak = new Set([last.includes(".") ? last.slice(0, last.lastIndexOf(".")) : last]);
  if (parts.length >= 2 && parts[0] === "crates") {
    strong.add(parts[1]!);
    strong.add(parts[1]!.replaceAll("-", "_"));
  }
  if (parts.length >= 2) weak.add(parts[parts.length - 2]!);
  const s = usable(strong);
  return [s, new Set([...usable(weak)].filter((x) => !s.has(x)))];
}

/** How many of the item's paths a bullet names; a weak spelling counts only beside a strong one. */
function score(body: string, selector: string, terms: string[]): number {
  const hay = `${body} ${selector}`.toLowerCase();
  const strong = terms.filter((t) => [...spellings(t)[0]].some((v) => hay.includes(v)));
  if (strong.length === 0) return 0;
  const weak = terms.filter((t) => !strong.includes(t) && [...spellings(t)[1]].some((v) => hay.includes(v)));
  return strong.length + weak.length;
}

/**
 * `--traps <path>...`: the traps section alone, ranked from the whole playbook rather than a manifest.
 * The best `PROMOTED_WHOLE` print whole and the rest one selector each. Ties keep the book's order.
 */
function runTraps(paths: string[]): void {
  const terms = [...new Set(paths.map((p) => p.replace(/\\/g, "/")))];
  const hits = allBullets(playbookBook())
    .map((b, i) => ({ b, i, s: score(b.text, b.selector, terms) }))
    .filter((r) => r.s > 0)
    .sort((a, b) => b.s - a.s || a.i - b.i)
    .map((r) => r.b);
  section(TRAPS_TITLE, `${PLAYBOOK}/, ranked from the whole playbook against the ${terms.length} path(s) given`);
  if (hits.length === 0) {
    emit("  No bullet names any of these paths.");
    return;
  }
  for (const b of hits.slice(0, PROMOTED_WHOLE)) {
    emit();
    emit(b.text);
  }
  const rest = hits.slice(PROMOTED_WHOLE);
  if (rest.length > 0) {
    emit();
    emit(`-- ${rest.length} more trap(s) name these paths. One line each; \`${tool("playbook")} --show '<selector>'\``);
    emit("   prints one in full:");
    for (const b of rest) emit(`   ${b.selector}`);
  }
}

/** A bullet's lead-in as one listed line's label, links stripped and cut to 110 characters. */
function leadLabel(body: string): string {
  const lead = /^- \*\*(.+?)\*\*/.exec(body.split("\n")[0]!);
  const label = lead ? lead[1]! : body.split("\n")[0]!.slice(2);
  return [...stripLinks(label).replace(/^[* ]+|[* ]+$/g, "")].slice(0, 110).join("");
}

function runPlaybook(wanted: string[], item: string, stage: number | null, modules: string[]): void {
  if (wanted.length === 0) return;
  const book = playbookBook();
  if (book.every((s) => s.bullets.length === 0)) {
    warn(`${PLAYBOOK}/ holds no bullet`);
    return;
  }
  const selectorOf = new Map(allBullets(book).map((b) => [b.text, b.selector]));
  const [triage] = sliceBullets(book, TRIAGE);
  const needed = triageApplies(stage);

  const picked: [string, string][] = [];
  const seen = new Set(triage.map((b) => b.slice(0, 120)));
  for (const name of wanted) {
    const [found, complaint] = sliceBullets(book, name);
    if (complaint) {
      warn(`[context] playbook ${complaint}`);
      continue;
    }
    for (const body of found) {
      const key = body.slice(0, 120);
      if (seen.has(key)) continue;
      seen.add(key);
      picked.push([name, body]);
    }
  }

  // An item that names no path -- a reached goal has no item at all -- is narrowed to the goal's own
  // modules, so a family selector's bullets about crates this goal never opens stay one line each.
  const itemTerms = [...new Set([...item.matchAll(ITEM_PATH_RE)].map((m) => m[1]!))];
  const byModules = itemTerms.length === 0 && modules.length > 0;
  const terms = byModules ? [...new Set(modules.map((p) => p.replace(/\\/g, "/")))] : itemTerms;
  let whole: [string, string][];
  let listed: [string, string][];
  let spilled: [string, string][] = [];
  if (terms.length > 0) {
    const hits = picked
      .map((p, i) => ({ p, i, s: score(p[1], p[0], terms) }))
      .sort((a, b) => b.s - a.s || a.i - b.i)
      .filter((r) => r.s > 0)
      .map((r) => r.p);
    whole = hits.slice(0, PROMOTED_WHOLE);
    spilled = hits.slice(PROMOTED_WHOLE);
    listed = picked.filter((p) => !whole.includes(p));
  } else {
    [whole, listed] = [picked, []];
  }

  const narrowing = terms.length === 0 ? "" : byModules ? `, then to the goal's ${terms.length} [context] module(s)` : `, then to the ${terms.length} path(s) your item names`;
  section(TRAPS_TITLE, `${PLAYBOOK}/, filtered to [context] playbook${narrowing}`);
  const atWhole = printed().length;
  for (const [, body] of whole) {
    emit();
    emit(body);
  }
  const atListed = printed().length;
  if (listed.length > 0) {
    emit();
    emit(`-- ${listed.length} more trap(s) this GOAL names that your ${byModules ? "goal's modules do" : "ITEM does"} not touch. One line`);
    emit(`   each; \`${tool("playbook")} --show '<selector>'\` prints one in full:`);
    for (const [name, body] of listed) emit(`   ${selectorOf.get(body) ?? name}  --  ${leadLabel(body)}`);
  }
  const atTriage = printed().length;
  if (triage.length > 0 && needed) {
    emit();
    emit("-- The driver's last acceptance check FAILED on a check that should already pass. These");
    emit(`   ${triage.length} trap(s) are for reading one. Read the lead-ins, then print the one that fits`);
    emit(`   with \`${tool("playbook")} --show '<selector>'\`:`);
    for (const body of triage) emit(`   ${selectorOf.get(body) ?? TRIAGE}  --  ${leadLabel(body)}`);
  } else if (triage.length > 0) {
    emit();
    emit(`-- ${triage.length} trap(s) for reading a failing acceptance check are held back: nothing the driver`);
    emit("   reports failing is a floor check or one in a stage already passed.");
    emit(`   \`${tool("playbook")} --show '${TRIAGE}'\` prints them all.`);
  }

  const n = (x: number) => x.toLocaleString("en-US");
  const unnarrowed = nbytes(picked.flatMap(([, b]) => ["", b]).join("\n"));
  const wholeB = nbytes(printed().slice(atWhole, atListed).join("\n"));
  const listedB = nbytes(printed().slice(atWhole, atTriage).join("\n")) - wholeB;
  const triageB = nbytes(printed().slice(atWhole).join("\n")) - wholeB - listedB;
  trapsDetail = [
    `    manifest names ${wanted.length} selector(s) -> ${picked.length} bullet(s), ${n(unnarrowed)} B whole`,
    terms.length > 0
      ? `    ${byModules ? "your ITEM names no path, so the goal's" : "your ITEM's"} ${terms.length} ${byModules ? "module(s)" : "path(s)"} promote ${whole.length + spilled.length}, of which ${whole.length} print whole: ${n(wholeB)} B` +
        (spilled.length > 0 ? ` (${spilled.length} spilled past PROMOTED_WHOLE=${PROMOTED_WHOLE})` : "")
      : `    neither your ITEM nor the goal's [context] modules name a path, so all ${whole.length} are printed in full: ${n(wholeB)} B`,
    `    the other ${listed.length} cost one lead-in line each: ${n(listedB)} B`,
    `    ${triage.length} failing-check trap(s) ` + (needed ? "listed one line each, a check that should pass being red" : "held back") + `: ${n(triageB)} B`,
    `    so narrowing saved ${n(Math.max(unnarrowed - wholeB - listedB, 0))} B -- the item's share`,
    `    is bounded by PROMOTED_WHOLE; trimming the manifest acts on the ${n(unnarrowed)} B`,
  ];
}

// ------------------------------------------------------------------------ the closing

/** The wrap skeleton for this tree, or "" when the writer cannot produce it. */
async function wrapTemplate(): Promise<string> {
  const r = await runProc(["bun", "tools/nv/main.ts", "session", "--template"], { timeoutMs: 60_000 });
  return r.code === 0 ? r.stdout.replace(/^\n+|\n+$/g, "") : "";
}

async function runClosing(): Promise<void> {
  const template = await wrapTemplate();
  const session = tool("session");
  section("WHEN YOU ARE DONE", "AGENTS.md § Session workflow, steps 3-5");
  emit("  bun nv verify --start / --wait    build + test + clippy + fmt, once, at the end");
  if (template) {
    emit("  <Fill in the skeleton below>      plan fields, playbook bullet, handoff, commits, status");
  } else {
    emit(`  ${session} --template a wrap skeleton, already carrying every count the`);
    emit("                                    tree has moved past, the playbook's headings, and");
    emit("                                    the `## commit:` for the docs it writes");
    emit("  <Fill that skeleton in>           plan fields, playbook bullet, handoff, commits, status");
  }
  emit(`  ${session} --wrap F  applies all of it, or refuses and changes nothing`);
  emit();
  if (template) {
    emit("TWO CALLS, and neither is a `--help`, a `--dry-run`, a `grep` of the playbook or a");
    emit(`\`${session} --template\` -- the template IS the skeleton at the end of this section,`);
    emit("generated for this tree at this commit, so its counts and headings are already the");
    emit("current ones. `--wrap` is all-or-nothing: it validates every section before it writes a");
  } else {
    emit("THREE CALLS, and none of them is a `--help`, a `--dry-run` or a `grep` of the playbook.");
    emit("`--template` IS the format, and it answers off the tree what the tail used to re-derive");
    emit("by hand -- so do not grep the plan for a count or the playbook for its headings, they");
    emit("are in it. `--wrap` is all-or-nothing: it validates every section before it writes a");
  }
  emit("byte, so a dry run only buys the same refusal a call earlier. ONE WRAP WRITES THE DOCS");
  emit("AND COMMITS THEM -- anything it wrote that no `## commit:` names joins the last one, so");
  emit("there is no second call for a docs commit and no `git add` by hand.");
  emit();
  emit("WHILE YOU WORK, beside `peek` (AGENTS.md rule 2):");
  emit("  a re: target prints the matching line ALONE -- `re:pat:3`, or --context 3 for the");
  emit("  whole call, is how a heading or a `//!` line comes back with the block under it");
  emit(`  ${tool("gaps").padEnd(34)}the next group, ranked: cases per member per class,`);
  emit("                                    the unasserted error paths -- never an `ls tests/`");
  emit("                                    plus a `grep`");
  emit();
  emit("THE PIPELINE'S `nvs` IS ALREADY BUILT at the commit this session starts from, at");
  emit("`target/covws/<host triple>/debug/nvs`: `bun nv try` runs it. Do not `ls` it first,");
  emit("and let `bun nv verify` rebuild it once you have changed Rust yourself.");
  if (template) {
    emit();
    emit(`THE WRAP SKELETON -- \`${session} --template\` for this tree, so you do not call it.`);
    emit("Fill it in, write it to one file, and hand that file to `--wrap`. Drop any section");
    emit("this session does not owe; `--wrap` says so if you dropped one it needed.");
    emit();
    for (const line of template.split("\n")) emit(line);
  }
}

// ----------------------------------------------------------------------------- audit

function audit(): string[] {
  let ratio = BYTES_PER_TOKEN_FALLBACK;
  let calls = CALLS_PER_SESSION;
  let source = `ESTIMATED -- run \`${tool("loop-stats")} --calibrate --write\` after a run to measure it`;
  try {
    const data = JSON.parse(read(CALIBRATION));
    if (typeof data.bytes_per_token === "number") {
      ratio = data.bytes_per_token;
      calls = Number(data.calls_per_session ?? CALLS_PER_SESSION);
      source = `measured: regressed over ${Number(data.sessions ?? 0)} session(s), R^2 ${Number(data.r_squared ?? 0).toFixed(3)}`;
    }
  } catch {
    // The fallback and its label stand.
  }
  const n = (x: number, digits = 0) => x.toLocaleString("en-US", { maximumFractionDigits: digits, minimumFractionDigits: digits });
  const lines = ["", "== WHAT THIS PACK COST", `-- bytes / ${Number(ratio.toPrecision(6))}  (${source})`, ""];
  let total = 0;
  for (const [title, size] of costs()) {
    total += size;
    lines.push(`  ${title.padEnd(44)}${n(size).padStart(8)} B${n(size / ratio).padStart(10)} tok`);
    if (title === TRAPS_TITLE) lines.push(...trapsDetail);
  }
  lines.push(`  ${"TOTAL".padEnd(44)}${n(total).padStart(8)} B${n(total / ratio).padStart(10)} tok`);
  lines.push(
    "",
    "  The driver pipes this to the session, so it enters the context once -- and is",
    "  then re-billed on every turn, because a turn re-reads its whole context. At the",
    `  measured ${calls} calls a session that is about ${n((total / ratio) * calls / 1_000_000, 1)}M billed tokens, so trimming`,
    `  1,000 bytes here is worth about ${n((1000 / ratio) * calls)} of them.`,
    "",
    "  The largest section is usually the traps. A `[context] playbook` entry may name",
    "  one BULLET rather than a whole section -- `\"Tooling > a whole decision record\"` -- which is",
    "  what keeps this from growing every time a trap is written down.",
    "",
    "  Its indented rows split that cost in two, because only one half is yours: the",
    "  manifest's bullets are what a goal author trims, while which of them this ITEM",
    "  promotes to full text moves item to item. A traps row that rose because the",
    "  item touches a well-documented file is not an argument for naming fewer traps.",
    "",
    "  This is a number to look at when you WRITE a goal. It is not a check: nothing",
    "  here exits non-zero over a size (docs/agent/doc-style.md says why).",
  );
  return lines;
}

// ------------------------------------------------------------------- the manifest audit

/** A `rule:<topic>/<slug>` token as the prose writes it. */
const RULE_TOKEN = /rule:([a-z0-9-]+\/[a-z0-9-]+)/g;

/**
 * `{ problems, notes }` for one goal's `context`, over the base and every stage's overlay at once,
 * printing nothing. `where` names the goal in each line, and `prose` is the path of its prose.
 *
 * What gates and what only reports is whether the target can legitimately not exist yet. A `shapes`
 * entry names a heading in a file that is already written, and a `playbook` selector names a bullet
 * that is already there, so each of those that resolves to nothing is a problem. A `rules` or `adrs`
 * entry may name something the goal is about to create, so those are notes, and so are the two
 * shapes a manifest is written in when nobody has read loop-authoring.md § 2. `modules` is not
 * audited: a goal whose first slice creates the crate is the ordinary case, and the pack's own
 * warning is where a pattern that matches nothing shows. A `context` key no stage or goal has is the
 * schema's finding, not this one's.
 */
/** What one call over many goals reads once and hands every goal: the playbook's bullets and the
 * rulebook, each filled the first time a goal needs it. */
export interface ManifestShared {
  book?: BookSection[];
  rules?: Map<string, BookRule>;
}

export function manifestFindings(g: GoalValue, where: string, prose: string | null, shared: ManifestShared = {}): { problems: string[]; notes: string[] } {
  const problems: string[] = [];
  const notes: string[] = [];
  const m = manifest(g, null);
  for (const s of [...g.stages].sort((a, b) => a.number - b.number)) {
    for (const name of STAGE_FIELDS) {
      for (const x of s.context?.[name] ?? []) if (!m[name].includes(x)) m[name].push(x);
    }
  }
  if (!m.present) {
    notes.push(`${where}: has no \`context\`, so a session opens on the unscoped pack`);
    return { problems, notes };
  }

  const conventions = read(CONVENTIONS);
  for (const name of m.shapes) {
    if (sliceSection(conventions, name) === null) {
      problems.push(`${where}: shapes names ${pyRepr(name)}, and conventions.md has no such heading -- the shape is silently not printed`);
    }
  }
  for (const [shape, implied] of SHAPE_IMPLIES) {
    if (m.shapes.includes(shape) && !m.shapes.includes(implied)) problems.push(`${where}: shapes names ${pyRepr(shape)} without ${pyRepr(implied)}`);
  }

  if (m.playbook.length > 0) {
    const book = (shared.book ??= playbookBook());
    for (const selector of m.playbook) {
      const [hits, complaint] = sliceBullets(book, selector);
      if (complaint) problems.push(`${where}: playbook selector ${pyRepr(selector)} -- ${complaint}`);
      else if (hits.length > 1 && !selector.trimEnd().endsWith("*")) {
        problems.push(`${where}: playbook selector ${pyRepr(selector)} opens ${hits.length} bullets' lead-ins -- name one, or end it in \`*\` to take every one of them`);
      }
    }
  }

  for (const entry of m.adrs) {
    const parts = entry.replaceAll("§", " ").split(/\s+/).filter(Boolean);
    if (parts.length === 0) continue;
    const [number, ...rest] = parts as [string, ...string[]];
    const wanted = rest.join(" ");
    const path = `${DECISIONS}/${number}.md`;
    if (!existsSync(join(ROOT, path))) notes.push(`${where}: adrs names ${number}, and docs/decisions/ has no ${number}.md`);
    else if (wanted && sliceSection(stripFrontmatter(read(path)), wanted) === null) {
      notes.push(`${where}: adrs entry ${pyRepr(entry)} matches no heading in ${number}.md -- one entry is ONE section, so \`§§1,4,5\` slices nothing`);
    }
  }

  if (m.rules.length > 0) {
    const book = (shared.rules ??= rulebook());
    for (const entry of m.rules) {
      if (entry.includes("/")) {
        if (!book.has(entry)) notes.push(`${where}: rules names rule:${entry}, and the rulebook has no rule with that id`);
      } else if (![...book.values()].some((r) => r.because.includes(entry))) {
        notes.push(`${where}: rules names ${entry}, and no rule's \`because\` names it`);
      }
    }
    // Stated as "your prose is held to a rule your manifest cannot reach", so it clears the moment it
    // is acted on and never fires on a goal that has no rule to name.
    const text = prose === null ? "" : read(prose);
    if (text) {
      const records = new Set(m.rules.filter((e) => !e.includes("/")));
      const covered = new Set(m.rules.filter((e) => e.includes("/")));
      for (const r of book.values()) if (r.because.length > 0 && records.has(r.because[0]!)) covered.add(r.id);
      const unreached = [...new Set([...text.matchAll(RULE_TOKEN)].map((x) => x[1]!))].filter((id) => book.has(id) && !covered.has(id)).sort();
      if (unreached.length > 0) {
        notes.push(
          `${where}: the prose is held to ${unreached.length} rule(s) no \`rules\` entry reaches -- ${unreached.slice(0, 3).join(", ")}` +
            `${unreached.length > 3 ? ", …" : ""}. Name the two or three each stage is written against as ids; loop-authoring.md § 2`,
        );
      }
    }
  }

  // Only where there is something a stage could take: a process goal whose base is what every stage
  // needs has nothing to narrow, and a note that cannot clear schedules a pass whether or not anything drifted.
  const stages = g.stages.length;
  if (stages >= 3 && m.staged.length === 0 && (m.adrs.length > 0 || m.rules.length >= 5)) {
    notes.push(`${where}: runs ${stages} stages and narrows to none of them -- every session reads all ${stages} stages' rules and record sections (a stage's \`context\`, loop-authoring.md § 2)`);
  }
  return { problems, notes };
}

// ---------------------------------------------------------------------------- driver

interface Options {
  audit: boolean;
  item: number | null;
  full: boolean;
  goal: string | null;
  stage: number | null;
}

function parse(args: string[]): Options | string {
  const o: Options = { audit: false, item: null, full: false, goal: null, stage: null };
  for (let i = 0; i < args.length; i++) {
    const [flag, inline] = args[i]!.includes("=") ? args[i]!.split(/=(.*)/s, 2) as [string, string] : [args[i]!, undefined];
    const value = () => inline ?? args[++i];
    if (flag === "--audit") o.audit = true;
    else if (flag === "--full") o.full = true;
    else if (flag === "--goal") {
      const v = value();
      if (!v) return "--goal needs a goal slug";
      o.goal = v;
    } else if (flag === "--item" || flag === "--stage") {
      const v = value();
      if (!v || !/^\d+$/.test(v)) return `${flag} needs a number`;
      if (flag === "--item") o.item = Number(v);
      else o.stage = Number(v);
    } else return `unknown argument ${pyRepr(args[i]!)}`;
  }
  return o;
}

export async function run(args: string[]): Promise<number> {
  if (args[0] === "--traps") {
    if (args.length < 2 || args.slice(1).some((a) => a.startsWith("--"))) {
      console.error(`nv orient: --traps needs one or more paths and nothing else\n${summary}`);
      return 2;
    }
    begin("orient");
    runTraps(args.slice(1));
    process.stdout.write(printed().join("\n").replace(/^\n+/, "") + "\n");
    return 0;
  }
  const opts = parse(args);
  if (typeof opts === "string") {
    console.error(`nv orient: ${opts}\n${summary}`);
    return 2;
  }
  begin("orient");
  trapsDetail = [];

  const src = sources(opts.goal);
  if (src === null) {
    process.stdout.write(`nv orient: data/chain.json names no live goal on the chain, so there is no goal to narrow to.\n\`bun nv brief --where <keyword>\` routes a topic to the file that owns it.\n`);
    return 2;
  }
  const g = goalRecord(src);
  if (g === null) {
    process.stdout.write(`nv orient: goal ${src.slug} has no record, so there is no goal to narrow to.\n\`bun nv brief --where <keyword>\` routes a topic to the file that owns it.\n`);
    return 2;
  }
  const h = handoffRecord(src);
  const stage = opts.stage ?? h?.next.stage ?? null;
  const m = manifest(g, opts.full ? null : stage);
  if (opts.full) m.modules = ["crates/**", "editors/**"];

  emit("Novis -- oriented to the current goal. This is deliberately narrow: it prints what this");
  emit("goal's [context] manifest names and nothing else. `bun nv orient --full` is the wide one.");
  emit(src.position === null ? `Side goal \`${src.slug}\`, off the chain.` : `Goal \`${src.slug}\`, ${src.position} on the chain.`);
  if (m.staged.length > 0) {
    const namedBy = opts.stage !== null ? "--stage" : "the handoff's next group";
    if (m.applied !== null) {
      emit(`Narrowed to STAGE ${m.applied}, which ${namedBy} names: the base [context] plus`);
      emit("that stage's own entries. If you need something it did not print, say so in the");
      emit("handoff naming the field -- see the closing block.");
    } else if (m.stage !== null) {
      emit(`The handoff names stage ${m.stage}, and this goal has no [context.stage.${m.stage}]`);
      emit("table, so only the base [context] applies.");
    } else {
      emit("This goal has per-stage tables and the handoff's `## Next group` names no stage,");
      emit("so only the base [context] applies -- the widest pack this goal can print.");
    }
  }

  // The failing check is looked for in the plan the driver ran, and a side goal's in its own record.
  await runMarker(src, src.side ? g : ((goalPlan(src.slug) as unknown as GoalValue | null) ?? g));
  const item = runState(src, h, opts.item);
  runAnchors(src, item);
  runStandingDecisions(src);
  runRules(m);
  runAdrs(m);
  await runMap(m);
  runShapes(m);
  runPlaybook(m.playbook, item, stage, m.modules);
  runPlan(m);
  runMilestones(m);
  runNumbers();
  await runClosing();
  closeLedger();

  if (!m.present) warn(`${src.record} has no \`context\` at all, so nothing could be selected. docs/agent/loop-authoring.md § 2 has the field list.`);

  let body = printed().join("\n").replace(/^\n+/, "");
  if (opts.audit) body += "\n" + audit().join("\n");
  process.stdout.write(body + "\n");
  return 0;
}
