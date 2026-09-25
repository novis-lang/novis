// `bun nv orient`: a loop session's step 1 in one call, narrowed to the current goal. The pack is the
// run marker and the driver's last verdict, the handoff's state and current item, the code at every
// `path:line` the item names, the goal's standing decisions, then what the goal's `context` selects —
// rules, decision-record sections, map lines, convention shapes, playbook traps, plan fields and
// milestones — and last the next free numbers and the closing block with the wrap skeleton. Every
// part is sliced out of its live file when the pack is built, so nothing in it is a copy.
//
// The goal and its handoff are records. The goal's `context` is the base selection, and the stage
// the handoff's next group names adds its own `context` on top: an overlay only ever adds, so a
// stage that forgot an entry prints what the base prints. While the Python writers still keep
// `docs/agent/loop-goal.toml`, `docs/agent/handoff.md`, the plan's status block and
// `docs/agent/playbook/` current, each of those records is read through its importer from that file,
// so the pack is never older than the tree. The goal's position is `N of M` in `data/chain.json`.
//
// The pack exists to save a session turns, not bytes: the item's anchors are printed inline because
// each one replaces a `peek` call, and the traps are narrowed to the item's own paths because a trap
// about a file the item never opens replaces no call at all. `--audit` appends what each section
// cost, and never exits non-zero over a size.
//
//   bun nv orient              the pack
//   bun nv orient --audit      and what each section cost, in bytes and approximate tokens
//   bun nv orient --item N     pin checklist item N instead of the first unticked one
//   bun nv orient --stage N    apply stage N's context instead of the one the handoff names
//   bun nv orient --full       ignore the narrowing and print every module
//   bun nv orient --traps <path>...  the traps section alone, ranked from the whole playbook against those paths
//   bun nv orient --goal SLUG  read the context of goal SLUG instead of the live goal's

import { existsSync, readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { chainGoals, liveGoal } from "../lib/chain.ts";
import { tracked } from "../lib/git.ts";
import { ROOT } from "../lib/paths.ts";
import { run as runProc } from "../lib/proc.ts";
import { fill, pyRepr } from "../lib/py.ts";
import { load } from "../lib/store.ts";
import { goalValue, handoffValue } from "../import/goals.ts";
import type { Unread } from "../import/lib.ts";
import { plan as planImporter } from "../import/plan.ts";
import { playbook as playbookImporter } from "../import/playbook.ts";
import { goal as goalType, sideGoal as sideGoalType } from "../schema/goal.ts";
import { handoff as handoffType, sideHandoff as sideHandoffType } from "../schema/handoff.ts";
import { planStatus } from "../schema/plan-status.ts";
import { playbookBullet, playbookSection } from "../schema/playbook.ts";
import { rule as ruleType } from "../schema/rule.ts";
import { topic as topicType } from "../schema/topic.ts";
import { bodyOf, leadParagraph, milestones, verifyParagraph } from "./plan.ts";

export const summary = "the loop session's orientation pack, narrowed to the live goal: nv orient [--audit] [--item N] [--stage N] [--full] [--goal SLUG] | --traps <path>...";

const LIVE = { md: "docs/agent/loop-goal.md", toml: "docs/agent/loop-goal.toml", handoff: "docs/agent/handoff.md" };
const SIDE_ENV = "NOVIS_SIDE_GOAL";
const PLAYBOOK = "docs/agent/playbook";
const CONVENTIONS = "docs/agent/conventions.md";
const RUNNING = ".loop/running";
const INTERRUPTED = ".loop/interrupted.json";
const LEDGER = ".loop/log.md";
const DOCGATE = ".loop/doc-gate.json";
const OWNERGATE = ".loop/owner-gate.json";
const DIAGNOSTICS = "crates/nvs-diagnostics/src/lib.rs";
const DECISIONS = "docs/decisions";
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
 * The traps for reading a failing acceptance check. They print whole when the driver's verdict names
 * a check that should already pass, and as one line otherwise, whatever the manifest names.
 */
const TRIAGE = "Tooling > a loop-goal.toml*";

/** A `path:line` anchor in a checklist item, which the pack expands into a window of the file. */
export const ANCHOR_RE = /\b((?:crates|tools|tests|benches|examples|fuzz|docs|editors)\/[\w./-]+\.\w+):(\d+)\b/g;

/** A path a checklist item names, file or directory, which the traps are narrowed to. */
const ITEM_PATH_RE = /\b((?:crates|tools|tests|benches|examples|fuzz|docs|editors)\/[\w./-]*[\w-])/g;

/** Lines printed either side of an anchor: a signature and the top of a body. */
const ANCHOR_CONTEXT = 12;

/** The longest list printed whole; a longer one prints as a count. */
const NAMED_BEFORE_COUNT = 6;

/** Lines of the failing check's block printed, and of each sibling's, and how many siblings. */
const CHECK_BLOCK_LINES = 60;
const SIBLING_BLOCK_LINES = 14;
const STAGE_SIBLINGS = 12;

/** How many files the manifest's patterns outside `crates/` and `editors/` may list. */
const NAMED_FILES_MAX = 40;

/** How long a summary derived from a file that is not a crate module may be. */
const SUMMARY_MAX = 200;

/** Bytes per token when no calibration is on disk, and calls per session, for `--audit`. */
const BYTES_PER_TOKEN_FALLBACK = 2.5;
const CALLS_PER_SESSION = 98;

/** Shapes that only make sense together: a decision is the record and the rule's fragment. */
const SHAPE_IMPLIES: [string, string][] = [["A decision record", "A rule fragment"]];

/** The plan's status fields as the plan writes them, and each one's key in the record. */
const PLAN_FIELDS: [string, string][] = [
  ["Status", "status"],
  ["Done", "done"],
  ["On disk", "onDisk"],
  ["Toolchain", "toolchain"],
  ["ADR slices landed", "adrSlicesLanded"],
  ["Open now", "openNow"],
  ["Blocking", "blocking"],
];

interface Context {
  modules?: string[];
  rules?: string[];
  adrs?: string[];
  shapes?: string[];
  playbook?: string[];
  plan?: string[];
  milestones?: string[];
}

export interface GoalValue {
  context: Context;
  stages: { number: number; title: string; summary?: string; context?: Context }[];
}

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
  /** The goal's own prose, which the importer reads stage summaries from. */
  md: string;
  /** The prose the pack prints the standing decisions from. */
  print: string;
  toml: string;
  handoff: string;
  side: boolean;
  /** `N of M` on the chain, or null for a side goal. */
  position: string | null;
}

// ----------------------------------------------------------------------------- output

let out: string[] = [];
let ledger: [string, number][] = [];
let trapsDetail: string[] = [];

function emit(line = ""): void {
  out.push(line);
}

function warn(msg: string): void {
  emit("");
  emit(`!! nv orient: ${msg}`);
}

function nbytes(text: string): number {
  return Buffer.byteLength(text, "utf8");
}

function closeLedger(): void {
  const last = ledger[ledger.length - 1];
  if (last) ledger[ledger.length - 1] = [last[0], nbytes(out.slice(last[1]).join("\n"))];
}

function section(title: string, source: string): void {
  closeLedger();
  emit();
  emit(`== ${title}`);
  emit(`-- source: ${source}`);
  emit();
  ledger.push([title, out.length]);
}

/** A repo-relative file's text with every line ending read as `\n`, or "" when it cannot be read. */
function read(path: string): string {
  try {
    return readFileSync(join(ROOT, path), "utf8").replace(/\r\n?/g, "\n");
  } catch {
    return "";
  }
}

/** The command that runs tool `name`: its `nv` subcommand once one exists, the Python tool until then. */
function tool(name: string): string {
  return existsSync(join(ROOT, "tools", "nv", "cmd", `${name}.ts`)) ? `bun nv ${name}` : `python tools/${name}.py`;
}

async function git(...args: string[]): Promise<string> {
  const r = await runProc(["git", ...args]);
  return r.code === 0 ? r.stdout.trim() : "";
}

/** `[0002](../decisions/0002.md)` -> `0002`. */
function stripLinks(text: string): string {
  return text.replace(/\[([^\]]*)\]\([^)]*\)/g, "$1");
}

/** A Python list of strings, the way the pack has always quoted a manifest field. */
function pyList(items: string[]): string {
  return `[${items.map(pyRepr).join(", ")}]`;
}

function indentedFill(text: string, indent: string, first = indent): string {
  return fill(text, 100, { initialIndent: first, subsequentIndent: indent, breakLongWords: false, breakOnHyphens: false });
}

// --------------------------------------------------------------------- markdown slicing

interface MdHeading {
  index: number;
  level: number;
  title: string;
}

/** Every heading outside a ``` fence, with its 0-based line index. */
function headings(text: string): MdHeading[] {
  const found: MdHeading[] = [];
  let fenced = false;
  text.split("\n").forEach((line, index) => {
    if (line.trimStart().startsWith("```")) fenced = !fenced;
    if (fenced) return;
    const m = /^(#{1,6})\s+(.*?)\s*$/.exec(line);
    if (m) found.push({ index, level: m[1]!.length, title: m[2]! });
  });
  return found;
}

/** `### 2. Both operands ...` -> `2 both operands ...`, so a section is named by number or by words. */
export function normalize(title: string): string {
  let t = title.replace(/[`*_]/g, "").trim().toLowerCase();
  t = t.replace(/^(\d+[a-z]?)\s*[.)]?\s*/, "$1 ");
  return t.replace(/\s+/g, " ").trim();
}

export function titleMatches(title: string, key: string): boolean {
  const norm = normalize(title);
  return norm === key || norm.startsWith(key + " ") || norm.startsWith(key + ".");
}

/** The named heading and its body, up to the next heading at the same or a higher level. */
function sliceSection(text: string, wanted: string): string | null {
  const lines = text.split("\n");
  const hs = headings(text);
  const key = normalize(wanted);
  const at = hs.findIndex((h) => titleMatches(h.title, key));
  if (at < 0) return null;
  const h = hs[at]!;
  const next = hs.slice(at + 1).find((later) => later.level <= h.level);
  return lines.slice(h.index, next ? next.index : lines.length).join("\n").trimEnd();
}

/** The first `##` section whose title opens with `words`. */
function sectionOpening(text: string, words: string): string | null {
  const h = headings(text).find((x) => x.level === 2 && normalize(x.title).startsWith(words));
  return h ? sliceSection(text, h.title) : null;
}

/** A decision record's title, metadata and *In short*: everything before its first `##`. */
function sliceHead(text: string): string {
  const h = headings(text).find((x) => x.level === 2);
  return h ? text.split("\n").slice(0, h.index).join("\n").trimEnd() : text.trimEnd();
}

/** A frozen record's text without the YAML block it opens on. */
function stripFrontmatter(text: string): string {
  if (!text.startsWith("---\n")) return text;
  const end = text.indexOf("\n---\n", 4);
  return end < 0 ? text : text.slice(end + 5).replace(/^\n+/, "");
}

// ------------------------------------------------------------------------ the goal

function sources(goalFlag: string | null): Sources | null {
  const side = (process.env[SIDE_ENV] ?? "").trim();
  if (/^[a-z0-9]+(?:-[a-z0-9]+)*$/.test(side) && existsSync(join(ROOT, "docs/agent/goals/side", `${side}.md`))) {
    const stem = `docs/agent/goals/side/${side}`;
    return { slug: side, md: `${stem}.md`, print: `${stem}.md`, toml: `${stem}.toml`, handoff: `${stem}.handoff.md`, side: true, position: null };
  }
  const goals = chainGoals();
  const live = liveGoal(goals);
  if (!live) return null;
  const position = (slug: string) => {
    const at = goals.findIndex((g) => g.slug === slug);
    return at < 0 ? null : `${at + 1} of ${goals.length}`;
  };
  return {
    slug: goalFlag ?? live.slug,
    md: live.md ?? LIVE.md,
    print: LIVE.md,
    toml: LIVE.toml,
    handoff: LIVE.handoff,
    side: false,
    position: position(goalFlag ?? live.slug),
  };
}

/** The goal record: read from the live acceptance list while the driver keeps it, else the stored one. */
function goalRecord(src: Sources, fromLive: boolean, unread: Unread[]): GoalValue | null {
  if (fromLive && existsSync(join(ROOT, src.toml))) {
    const v = goalValue(ROOT, { slug: src.slug, md: src.md, toml: src.toml, handoff: null }, unread);
    if (v) return v as unknown as GoalValue;
  }
  const stored = load<any>(src.side ? sideGoalType : goalType).find((g) => g.id === src.slug);
  return stored ? (stored.value as GoalValue) : null;
}

/** The handoff record: read from the live handoff while the sessions write it, else the stored one. */
function handoffRecord(src: Sources, unread: Unread[]): HandoffValue | null {
  if (existsSync(join(ROOT, src.handoff))) {
    const v = handoffValue(ROOT, src.handoff, src.slug, unread);
    if (v) return v as unknown as HandoffValue;
  }
  const stored = load<any>(src.side ? sideHandoffType : handoffType).find((h) => h.id === src.slug);
  return stored ? (stored.value as HandoffValue) : null;
}

/** The fields a stage's `context` may add to. */
const STAGE_FIELDS = ["rules", "adrs", "shapes", "playbook", "milestones"] as const;

interface Manifest {
  present: boolean;
  /** Every stage that carries a `context` of its own. */
  staged: number[];
  stage: number | null;
  applied: number | null;
  modules: string[];
  rules: string[];
  adrs: string[];
  shapes: string[];
  playbook: string[];
  plan: string[];
  milestones: string[];
}

function manifest(g: GoalValue, stage: number | null): Manifest {
  const ctx = g.context ?? {};
  const over = stage === null ? undefined : g.stages.find((s) => s.number === stage)?.context;
  const field = (name: keyof Context): string[] => {
    const base = [...(ctx[name] ?? [])].map(String);
    if (over && (STAGE_FIELDS as readonly string[]).includes(name)) {
      for (const x of over[name] ?? []) if (!base.includes(x)) base.push(x);
    }
    return base;
  };
  return {
    present: Object.keys(ctx).length > 0,
    staged: g.stages.filter((s) => s.context !== undefined).map((s) => s.number),
    stage,
    applied: over ? stage : null,
    modules: field("modules"),
    rules: field("rules"),
    adrs: field("adrs"),
    shapes: field("shapes"),
    playbook: field("playbook"),
    plan: ctx.plan ?? ["Open now", "Blocking"],
    milestones: field("milestones"),
  };
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
    if (body.startsWith("goal cost:")) cost = true;
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
  if (m[1]!.startsWith("0") || m[2]!.includes("floor")) return true;
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

interface CheckBlock {
  fields: Map<string, string>;
  top: number;
  last: number;
  lines: string[];
}

/** Every `[[check]]` in an acceptance list, with the comment header written above it. */
function checkBlocks(text: string): CheckBlock[] {
  const lines = text.split("\n");
  const blocks: CheckBlock[] = [];
  lines.forEach((ln, i) => {
    if (ln.trim() !== "[[check]]") return;
    let stop = lines.findIndex((l, j) => j > i && l.startsWith("["));
    if (stop < 0) stop = lines.length;
    let last = stop - 1;
    while (last > i && (!lines[last]!.trim() || lines[last]!.trimStart().startsWith("#"))) last--;
    let top = i;
    let h = i - 1;
    while (h >= 0 && !lines[h]!.trim()) h--;
    while (h >= 0 && lines[h]!.trimStart().startsWith("#")) [top, h] = [h, h - 1];
    const fields = new Map<string, string>();
    for (const body of lines.slice(i, last + 1)) {
      const f = /^(name|file|stage) = "(.*)"\s*$/.exec(body);
      if (f && !fields.has(f[1]!)) fields.set(f[1]!, f[2]!);
    }
    blocks.push({ fields, top, last, lines: lines.slice(top, last + 1) });
  });
  return blocks;
}

function escapeRe(s: string): string {
  return s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

/** The block(s) whose label the ledger's `goal check:` line opens with, matched exactly. */
function locateCheck(toml: string, fail: string): CheckBlock[] {
  const text = read(toml);
  if (!fail || !text) return [];
  return checkBlocks(text).filter((b) => {
    const stage = b.fields.get("stage") ?? "?";
    const name = b.fields.get("name");
    const file = b.fields.get("file");
    if (name !== undefined) return fail.startsWith(`${name} [${stage}]: `);
    if (file !== undefined) return new RegExp(`^\\S+ ${escapeRe(file)} \\[${escapeRe(stage)}\\]: `).test(fail);
    return false;
  });
}

function emitCheckBlock(toml: string, fail: string): void {
  const hits = locateCheck(toml, fail);
  if (hits.length === 0) return;
  if (hits.length > 1) {
    const shown = hits.slice(0, 4);
    const where = shown.map((b) => `'${toml}:${b.top + 1}-${b.last + 1}'`).join(" ");
    const more = hits.length === shown.length ? "" : ` (of ${hits.length}; the rest carry it too)`;
    emit();
    emit(`${hits.length} checks carry that exact label, so which of them failed does not follow`);
    emit(`from the ledger line. \`${tool("peek")} ${where}\`${more}`);
    emit("prints them in one call -- the one naming what failed above is yours.");
    return;
  }
  const b = hits[0]!;
  const anchor = `${toml}:${b.top + 1}-${b.last + 1}`;
  emit();
  emit(`That check is ${anchor}, and it is printed here in full -- it is what this`);
  emit("session exists to turn green, so do not go and find it. Any comment above the");
  emit("`[[check]]` line is the stage's own header, and says what the whole stage is for:");
  emit();
  for (const line of b.lines.slice(0, CHECK_BLOCK_LINES)) emit(line.trim() ? `  ${line}` : "");
  if (b.lines.length > CHECK_BLOCK_LINES) {
    emit(`  ... ${b.lines.length - CHECK_BLOCK_LINES} more line(s) -- \`${tool("peek")} '${anchor}'\` for the whole block.`);
  }
  emitStageSiblings(toml, b);
}

/** The rest of the failing check's stage, one block each: what closing the stage means. */
function emitStageSiblings(toml: string, hit: CheckBlock): void {
  const stage = hit.fields.get("stage") ?? "";
  if (!stage || stage.toLowerCase().includes("floor") || stage.startsWith("0")) return;
  const siblings = checkBlocks(read(toml)).filter((b) => (b.fields.get("stage") ?? "") === stage && b.top !== hit.top);
  if (siblings.length === 0) return;
  emit();
  emit(`The rest of stage ${pyRepr(stage)} -- ${siblings.length} more check(s). The stage goes green`);
  emit("only when these do too, so they are what closing the one above actually means:");
  for (const b of siblings.slice(0, STAGE_SIBLINGS)) {
    emit();
    emit(`  -- ${toml}:${b.top + 1}`);
    const body = b.lines.filter((ln) => !ln.trimStart().startsWith("#"));
    for (const line of body.slice(0, SIBLING_BLOCK_LINES)) emit(line.trim() ? `  ${line}` : "");
    if (body.length > SIBLING_BLOCK_LINES) emit(`  ... ${body.length - SIBLING_BLOCK_LINES} more line(s)`);
  }
  if (siblings.length > STAGE_SIBLINGS) emit(`  ... and ${siblings.length - STAGE_SIBLINGS} more check(s) in this stage.`);
}

// ------------------------------------------------------------------------- sections

async function runMarker(src: Sources): Promise<void> {
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
      emit("The run ends only when every check in loop-goal.toml passes, and nothing else");
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
      emitCheckBlock(src.toml, fail);
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
  const text = read(src.print);
  if (!text) {
    warn(`${src.print} is missing -- the loop has no stated goal`);
    return;
  }
  section("THE GOAL'S STANDING DECISIONS", `${src.print} (pre-authorized, never re-opened)`);
  const block = sectionOpening(text, "standing decisions");
  if (block) emit(block);
  else warn(`${src.print} has no \`## Standing decisions\` section -- loop-authoring.md § 4`);
}

interface BookRule {
  id: string;
  title: string;
  status: string;
  because: string[];
  guardedBy: string[];
  topicOrder: number;
  order: number;
}

function rulebook(): Map<string, BookRule> {
  const topics = load(topicType);
  const place = new Map<string, [number, number]>();
  for (const t of topics) t.value.rules.forEach((id, i) => place.set(id, [t.value.order, i]));
  const out = new Map<string, BookRule>();
  for (const r of load(ruleType)) {
    const [topicOrder, order] = place.get(r.id) ?? [Number.MAX_SAFE_INTEGER, 0];
    out.set(r.id, { id: r.id, title: r.value.title, status: r.value.status, because: r.value.because, guardedBy: r.value.guardedBy, topicOrder, order });
  }
  return out;
}

function capped(label: string, items: string[], tail: string, indent = "  "): void {
  if (items.length === 0) return;
  const rest = items.length > NAMED_BEFORE_COUNT ? `, and ${items.length - NAMED_BEFORE_COUNT} more (${tail})` : "";
  emit(indentedFill(`${label}: ` + items.slice(0, NAMED_BEFORE_COUNT).join(", ") + rest, indent));
}

function runRules(m: Manifest): void {
  section("THE RULES THIS GOAL LIVES INSIDE", `docs/rules/, selected by [context] rules = ${m.rules.length ? pyList(m.rules) : "[]"}`);
  emit("AGENTS.md's priority ordering and its four rules are already in your context. These are");
  emit("the rules this goal's own work sits inside. A rule named by id is printed whole, because");
  emit("the fragment IS the rule; a record number expands to the rules it created or changed.");
  emit(`A short changed list is named whole; a long one is only its count, and \`${tool("brief")} --where\``);
  emit("routes a topic to the chapter that holds the rest.");
  emit();
  if (m.rules.length === 0) {
    warn("[context] rules is empty, so no rule and no decision is named as binding this goal");
    return;
  }
  const book = rulebook();
  const byPlace = (a: BookRule, b: BookRule) => a.topicOrder - b.topicOrder || a.order - b.order;
  for (const entry of m.rules) {
    if (entry.includes("/")) {
      runOneRule(book, entry);
      continue;
    }
    const all = [...book.values()];
    const created = all.filter((r) => r.because[0] === entry).sort(byPlace);
    const changed = all.filter((r) => r.because.slice(1).includes(entry)).sort(byPlace);
    if (created.length === 0 && changed.length === 0) {
      warn(`[context] rules names ${entry}, but no rule's \`because\` names that record and the rulebook has no rule with that id`);
      continue;
    }
    emit(`ADR ${entry} -- ${created.length} rule(s) created, ${changed.length} changed:`);
    for (const r of created) emit(`- \`rule:${r.id}\` -- ${r.title}${r.status === "shipped" ? "" : "  (designed)"}`);
    const guards = new Set(created.flatMap((r) => r.guardedBy));
    if (guards.size > 0) emit(`  guarded by ${guards.size} path(s) (\`${tool("rules")} --show <id>\` lists a rule's own)`);
    if (changed.length > 0 && changed.length <= NAMED_BEFORE_COUNT) {
      emit(indentedFill("changed: " + changed.map((r) => `rule:${r.id}`).join(", "), "  "));
    }
    emit();
  }
}

function runOneRule(book: Map<string, BookRule>, id: string): void {
  const r = book.get(id);
  if (!r) {
    warn(`[context] rules names rule:${id}, and the rulebook has no rule with that id -- \`${tool("rules")} --list\` is every one of them`);
    return;
  }
  emit(`---- rule:${r.id}${r.status === "shipped" ? "" : "  (designed, not yet shipped)"}`);
  emit(`     ${r.title}`);
  if (r.because.length > 0) {
    emit(`     decided in ${r.because.join(", ")}${r.because.length === 1 ? "" : " (first created it, the rest amended it)"}`);
  }
  capped("guarded by", r.guardedBy, `\`${tool("rules")} --show\` lists them all`, "     ");
  emit();
  emit(read(`docs/rules/${id}.md`).replace(/^\n+|\n+$/g, ""));
  emit();
}

function runAdrs(m: Manifest): void {
  if (m.adrs.length === 0) return;
  section("THE ADR SECTIONS IN SCOPE", "docs/decisions/*.md, sliced live -- never a copy");
  emit("A section, not the file. If you need one this does not print, open the record at that");
  emit("heading and add the section to [context] adrs so the next session does not pay twice.");
  for (const entry of m.adrs) {
    const parts = entry.replaceAll("§", " ").split(/\s+/).filter(Boolean);
    if (parts.length === 0) continue;
    const [number, ...rest] = parts as [string, ...string[]];
    const wanted = rest.join(" ");
    const path = `${DECISIONS}/${number}.md`;
    if (!existsSync(join(ROOT, path))) {
      warn(`[context] adrs names ADR ${number}, and docs/decisions/ has no ${number}.md`);
      continue;
    }
    const text = stripFrontmatter(read(path));
    const body = wanted ? sliceSection(text, wanted) : sliceHead(text);
    if (body === null) {
      warn(`ADR ${number} has no section matching ${pyRepr(wanted)} -- it was renamed or renumbered`);
      continue;
    }
    emit();
    emit(`---- ${path}` + (wanted ? `  §${wanted}` : "  (In short)"));
    emit();
    emit(body);
  }
}

// ---------------------------------------------------------------------------- the map

/** Python's `fnmatch.fnmatch`: `*` crosses `/`, and on Windows the match ignores case. */
function fnmatch(name: string, pat: string): boolean {
  let re = "";
  for (let i = 0; i < pat.length; i++) {
    const c = pat[i]!;
    if (c === "*") re += ".*";
    else if (c === "?") re += ".";
    else if (c === "[") {
      const end = pat.indexOf("]", i + 2);
      if (end < 0) re += "\\[";
      else {
        let cls = pat.slice(i + 1, end).replace(/\\/g, "\\\\");
        if (cls.startsWith("!")) cls = "^" + cls.slice(1);
        re += `[${cls}]`;
        i = end;
      }
    } else re += escapeRe(c);
  }
  return new RegExp(`^(?:${re})$`, process.platform === "win32" ? "is" : "s").test(name);
}

function globHits(full: string, pat: string): boolean {
  return fnmatch(full, pat) || fnmatch(full, pat.replace(/\/+$/, "") + "/**");
}

/** `Foo the bar. Then baz.` -> `Foo the bar`. */
function firstSentence(text: string): string {
  const t = stripLinks(text).replace(/\[(`[^`\]]+`)\]/g, "$1");
  return t.split(/(?<=[a-z)`])\.\s+(?=[A-Z[`])/, 1)[0]!.trim().replace(/\.+$/, "");
}

/** The first paragraph of a Rust module's own `//!` block, or "". */
function moduleDoc(text: string): string {
  const para: string[] = [];
  for (const line of text.split("\n")) {
    const m = /^\s*\/\/!\s?(.*)$/.exec(line);
    if (m) {
      const body = m[1]!.trim();
      if (!body) {
        if (para.length > 0) break;
        continue;
      }
      para.push(body);
    } else if (para.length > 0) break;
    else if (!/^\s*(#!\[|\/\/[^!]|\/\/$|$)/.test(line)) break;
  }
  return para.join(" ");
}

/** The first paragraph of a TypeScript file's leading comment, or "". */
function headerDoc(text: string): string {
  const para: string[] = [];
  for (const raw of text.split("\n")) {
    let line = raw.trim();
    if (line.startsWith("/*")) line = line.replace(/^[/*]+/, "").trim();
    else if (line.startsWith("*/")) break;
    else if (line.startsWith("*")) line = line.slice(1).trim();
    else if (line.startsWith("//")) line = line.slice(2).trim();
    else if (para.length > 0 || line) break;
    if (!line) {
      if (para.length > 0) break;
      continue;
    }
    para.push(line);
  }
  return para.join(" ");
}

/** The opening paragraph of a Python file's module docstring, or "". */
function pyDoc(text: string): string {
  const m = /^(?:[ \t]*(?:#[^\n]*)?\n)*[ \t]*[rRuU]?("""|''')([\s\S]*?)\1/.exec(text);
  if (!m) return "";
  const lines = m[2]!.replace(/\t/g, "        ").split("\n");
  const margins = lines.slice(1).filter((l) => l.trim()).map((l) => l.length - l.trimStart().length);
  const margin = margins.length > 0 ? Math.min(...margins) : 0;
  const cleaned = [lines[0]!.trimStart(), ...lines.slice(1).map((l) => l.slice(margin))].join("\n").trim();
  return cleaned.split("\n\n", 1)[0]!.trim();
}

/** A Markdown file's first `#` heading and the opening sentence under it, as one line. */
function mdDoc(text: string): string {
  let head = "";
  const para: string[] = [];
  for (const raw of text.split("\n")) {
    const line = raw.trim();
    if (!head) {
      if (line.startsWith("# ")) head = line.slice(2).trim();
      continue;
    }
    if (line.startsWith("#")) break;
    if (line) para.push(line);
    else if (para.length > 0) break;
  }
  const body = para.length > 0 ? firstSentence(para.join(" ")) : "";
  return head && body ? `${head} -- ${body}` : head || body;
}

/** One line saying what any file in the tree is, or "" for a shape that carries no header. */
function pathSummary(path: string): string {
  const text = read(path);
  const ext = path.slice(path.lastIndexOf("."));
  const fn: Record<string, (t: string) => string> = {
    ".py": (t) => firstSentence(pyDoc(t)),
    ".md": mdDoc,
    ".rs": (t) => firstSentence(moduleDoc(t)),
    ".ts": (t) => firstSentence(headerDoc(t)),
    ".tsx": (t) => firstSentence(headerDoc(t)),
  };
  const f = fn[ext];
  if (!text || !f || !path.slice(path.lastIndexOf("/") + 1).includes(".")) return "";
  const line = f(text).split(/\s+/).filter(Boolean).join(" ");
  return line.length <= SUMMARY_MAX ? line : line.slice(0, SUMMARY_MAX).replace(/[ ,;:-]+$/, "") + "...";
}

/** Every file under `dir` whose name passes `keep`, as paths relative to `dir`, sorted. */
function filesUnder(dir: string, keep: (rel: string) => boolean): string[] {
  let names: string[];
  try {
    names = readdirSync(join(ROOT, dir), { recursive: true }) as string[];
  } catch {
    return [];
  }
  return names.map((n) => n.split("\\").join("/")).filter(keep).sort();
}

/** Every crate module and then every editor module, grouped, each with its header summary. */
function sourceModules(): [string, string, [string, string][]][] {
  const groups: [string, string, [string, string][]][] = [];
  const byKey = (first: string) => (a: [string, string], b: [string, string]) =>
    Number(a[0] !== first) - Number(b[0] !== first) || (a[0] < b[0] ? -1 : a[0] > b[0] ? 1 : a[1] < b[1] ? -1 : a[1] > b[1] ? 1 : 0);
  const packages = (base: string) => {
    try {
      return readdirSync(join(ROOT, base)).filter((d) => existsSync(join(ROOT, base, d, "src"))).sort();
    } catch {
      return [];
    }
  };
  for (const crate of packages("crates")) {
    const entries = filesUnder(`crates/${crate}/src`, (p) => p.endsWith(".rs") && !p.split("/").includes("snapshots"))
      .map((p): [string, string] => [`src/${p}`, firstSentence(moduleDoc(read(`crates/${crate}/src/${p}`)))]);
    if (entries.length > 0) groups.push([crate, `crates/${crate}`, entries.sort(byKey("src/lib.rs"))]);
  }
  for (const pkg of packages("editors")) {
    const entries = filesUnder(`editors/${pkg}/src`, (p) => (p.endsWith(".ts") || p.endsWith(".tsx")) && !p.split("/").includes("node_modules"))
      .map((p): [string, string] => [`src/${p}`, firstSentence(headerDoc(read(`editors/${pkg}/src/${p}`)))]);
    if (entries.length > 0) groups.push([`editors/${pkg}`, `editors/${pkg}`, entries.sort(byKey("src/extension.ts"))]);
  }
  return groups;
}

async function runMap(m: Manifest): Promise<void> {
  section("THE MAP, SCOPED", `what each file the goal names says it is, filtered to [context] modules (${m.modules.length} pattern(s))`);
  const groups = sourceModules();
  if (groups.length === 0) {
    warn("no `crates/*/src/**/*.rs` or `editors/*/src/**/*.ts` found -- those are the source");
    return;
  }
  if (m.modules.length === 0) {
    warn("[context] modules is empty, so the map is not printed at all. A goal that touches code must name the files it touches; `python tools/brief.py` prints all of them.");
    return;
  }
  let shown = 0;
  let total = 0;
  const unmatched = new Set(m.modules);
  for (const [name, prefix, entries] of groups) {
    const keep: [string, string][] = [];
    for (const [within, summary] of entries) {
      total++;
      const hit = m.modules.filter((pat) => globHits(`${prefix}/${within}`, pat));
      if (hit.length > 0) {
        keep.push([within, summary || "(no header doc comment)"]);
        for (const pat of hit) unmatched.delete(pat);
      }
    }
    if (keep.length === 0) continue;
    emit();
    emit(name);
    const width = Math.max(...keep.map(([w]) => w.length));
    for (const [within, summary] of keep) {
      emit(`  ${within.padEnd(width)}  ${summary}`);
      shown++;
    }
  }

  const rest = [...unmatched].sort();
  const named = new Map<string, [string, string][]>();
  const hitPats = new Set<string>();
  let kept = 0;
  for (const path of rest.length > 0 ? await tracked() : []) {
    const matched = rest.filter((pat) => globHits(path, pat));
    if (matched.length === 0) continue;
    for (const pat of matched) hitPats.add(pat);
    kept++;
    if (kept > NAMED_FILES_MAX) continue;
    const slash = path.lastIndexOf("/");
    const group = slash < 0 ? "." : path.slice(0, slash);
    if (!named.has(group)) named.set(group, []);
    named.get(group)!.push([path.slice(slash + 1), pathSummary(path)]);
  }
  const cut = Math.max(0, kept - NAMED_FILES_MAX);
  let extra = 0;
  for (const group of [...named.keys()].sort()) {
    const entries = named.get(group)!.sort((a, b) => (a[0] < b[0] ? -1 : a[0] > b[0] ? 1 : 0));
    extra += entries.length;
    emit();
    emit(group);
    const width = Math.max(...entries.map(([n]) => n.length));
    for (const [n, summary] of entries) emit(summary ? `  ${n.padEnd(width)}  ${summary}` : `  ${n}`);
  }
  if (cut > 0) {
    emit();
    emit(`...and ${cut} more file(s) the manifest's patterns match, not listed. A selector this broad is naming the tree rather than the goal's file set.`);
  }
  emit();
  const also = extra > 0 ? `, plus ${extra} file(s) it names outside them` : "";
  emit(`${shown} of ${total} crate and editor module(s) are in scope${also}. For one that is not,`);
  emit("`python tools/brief.py` prints the whole map -- and if you needed it, the manifest is");
  emit("missing a pattern.");
  for (const pat of rest.filter((p) => !hitPats.has(p))) {
    warn(`[context] modules pattern ${pyRepr(pat)} matches no file in the tree -- it moved, or the glob is wrong. The shape does not have to be Rust: any tracked file resolves.`);
  }
}

function runShapes(m: Manifest): void {
  if (m.shapes.length === 0) return;
  const text = read(CONVENTIONS);
  if (!text) {
    warn(`${CONVENTIONS} is missing`);
    return;
  }
  for (const [shape, implied] of SHAPE_IMPLIES) {
    if (m.shapes.includes(shape) && !m.shapes.includes(implied)) {
      warn(`[context] shapes names ${pyRepr(shape)} without ${pyRepr(implied)}. One decision is both files, in one commit, and \`session.py --wrap\` refuses a rulebook left half-written -- add ${pyRepr(implied)} to the manifest`);
    }
  }
  section("THE SHAPES YOU ARE ABOUT TO WRITE", `${CONVENTIONS}, filtered to [context] shapes`);
  for (const name of m.shapes) {
    const body = sliceSection(text, name);
    if (body === null) {
      warn(`[context] shapes names ${pyRepr(name)}, and ${CONVENTIONS} has no such heading`);
      continue;
    }
    emit();
    emit(body);
  }
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
 * The playbook's sections in order, each with its bullets by id, read through the importer. `skip` leaves
 * out the bullets imported from those fragment files, which is how a retirement asks what a selector
 * reaches once they are deleted.
 */
export function playbookBook(root: string = ROOT, skip: ReadonlySet<string> = new Set()): BookSection[] {
  const got = playbookImporter.read(root);
  const sections = got.records
    .filter((r) => r.type === playbookSection)
    .map((r) => ({ id: r.id, ...(r.value as { title: string; order: number }) }))
    .sort((a, b) => a.order - b.order);
  const bullets = got.records.filter((r) => r.type === playbookBullet && !skip.has(r.from)).sort((a, b) => (a.id < b.id ? -1 : 1));
  return sections.map((s) => ({
    title: s.title,
    bullets: bullets
      .filter((b) => b.id.startsWith(`${s.id}/`))
      .map((b) => {
        const v = b.value as { lead: string; body: string; until: { kind: string; arg: string } };
        return { lead: v.lead, text: `- **${v.lead}**${v.body ? ` ${v.body}` : ""} [until: ${v.until.kind} ${v.until.arg}]` };
      }),
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

function runPlaybook(wanted: string[], item: string, stage: number | null): void {
  if (wanted.length === 0) return;
  const book = playbookBook();
  if (book.every((s) => s.bullets.length === 0)) {
    warn(`${PLAYBOOK}/ holds no bullet`);
    return;
  }
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

  const terms = [...new Set([...item.matchAll(ITEM_PATH_RE)].map((m) => m[1]!))];
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

  section(TRAPS_TITLE, `${PLAYBOOK}/, filtered to [context] playbook` + (terms.length > 0 ? `, then to the ${terms.length} path(s) your item names` : ""));
  const atWhole = out.length;
  for (const [, body] of whole) {
    emit();
    emit(body);
  }
  const atListed = out.length;
  if (listed.length > 0) {
    emit();
    emit(`-- ${listed.length} more trap(s) this GOAL names that your ITEM does not touch. One line`);
    emit(`   each; \`${tool("playbook")} --show '<selector>'\` prints one in full:`);
    for (const [name, body] of listed) {
      const lead = /^- \*\*(.+?)\*\*/.exec(body.split("\n")[0]!);
      const label = lead ? lead[1]! : body.split("\n")[0]!.slice(2);
      emit(`   ${name}  --  ${[...stripLinks(label).replace(/^[* ]+|[* ]+$/g, "")].slice(0, 110).join("")}`);
    }
  }
  const atTriage = out.length;
  if (triage.length > 0 && needed) {
    emit();
    emit("-- The driver's last acceptance check FAILED on a check that should already pass, so");
    emit(`   here in full are the ${triage.length} trap(s) for reading one:`);
    for (const body of triage) {
      emit();
      emit(body);
    }
  } else if (triage.length > 0) {
    emit();
    emit(`-- ${triage.length} trap(s) for reading a failing acceptance check are held back: nothing the driver`);
    emit("   reports failing is a floor check or one in a stage already passed.");
    emit(`   \`${tool("playbook")} --show '${TRIAGE}'\` prints them all.`);
  }

  const n = (x: number) => x.toLocaleString("en-US");
  const unnarrowed = nbytes(picked.flatMap(([, b]) => ["", b]).join("\n"));
  const wholeB = nbytes(out.slice(atWhole, atListed).join("\n"));
  const listedB = nbytes(out.slice(atWhole, atTriage).join("\n")) - wholeB;
  const triageB = nbytes(out.slice(atWhole).join("\n")) - wholeB - listedB;
  trapsDetail = [
    `    manifest names ${wanted.length} selector(s) -> ${picked.length} bullet(s), ${n(unnarrowed)} B whole`,
    terms.length > 0
      ? `    your ITEM's ${terms.length} path(s) promote ${whole.length + spilled.length}, of which ${whole.length} print whole: ${n(wholeB)} B` +
        (spilled.length > 0 ? ` (${spilled.length} spilled past PROMOTED_WHOLE=${PROMOTED_WHOLE})` : "")
      : `    your ITEM names no path, so all ${whole.length} are printed in full: ${n(wholeB)} B`,
    `    the other ${listed.length} cost one lead-in line each: ${n(listedB)} B`,
    `    ${triage.length} failing-check trap(s) ` + (needed ? "printed whole, a check that should pass being red" : "held back") + `: ${n(triageB)} B`,
    `    so narrowing saved ${n(Math.max(unnarrowed - wholeB - listedB, 0))} B -- the item's share`,
    `    is bounded by PROMOTED_WHOLE; trimming the manifest acts on the ${n(unnarrowed)} B`,
  ];
}

// ------------------------------------------------------------------- plan and numbers

function runPlan(m: Manifest): void {
  const status = planImporter.read(ROOT).records.find((r) => r.type === planStatus)?.value as Record<string, string> | undefined;
  if (!status) return;
  const picked = PLAN_FIELDS.filter(([label, key]) => m.plan.includes(label) && status[key] !== undefined);
  if (picked.length === 0) return;
  section("THE PLAN, THE FIELDS THIS GOAL READS", `docs/implementation-plan.md, ${pyList(m.plan)}`);
  for (const [label, key] of picked) emit(indentedFill(`${label}: ${stripLinks(status[key]!)}`, "    ", ""));
  emit();
  emit(`These fields are rewritten by \`${tool("session")} --wrap\`, never edited by hand.`);
}

function runMilestones(m: Manifest): void {
  if (m.milestones.length === 0) return;
  const index = milestones();
  section("THE MILESTONES THIS GOAL IS INSIDE", `docs/plan/, filtered to [context] milestones = ${pyList(m.milestones)}`);
  for (const wanted of m.milestones) {
    const colon = wanted.indexOf(":");
    const spec = colon < 0 ? wanted : wanted.slice(0, colon);
    const part = colon < 0 ? "" : wanted.slice(colon + 1);
    const entry = index.find((e) => e.id === spec.trim().toUpperCase());
    if (!entry) {
      warn(`[context] milestones names ${pyRepr(spec)}, which the plan's table does not list`);
      continue;
    }
    if (!existsSync(join(ROOT, entry.rel))) {
      warn(`[context] milestones names ${pyRepr(spec)}, whose file ${entry.rel} is missing`);
      continue;
    }
    emit();
    if (part === "verify") {
      emit(`-- ${entry.id} acceptance (${entry.rel})`);
      emit(stripLinks(verifyParagraph(entry)));
    } else if (part === "lead") {
      emit(`-- ${entry.id} lead (${entry.rel})`);
      emit(stripLinks(leadParagraph(entry)));
    } else {
      emit(`-- ${entry.id} (${entry.rel})`);
      emit(bodyOf(entry));
    }
  }
  emit();
  emit("`Mn` is the whole milestone; `Mn:lead` and `Mn:verify` are the two paragraphs that");
  emit(`usually answer the question. \`${tool("plan")} --amend Mn --from <file>\` rewrites`);
  emit(`one, and \`${tool("session")} --wrap\` takes a \`## milestone: Mn\` section for the same thing.`);
}

/** The next free diagnostic code in each band, and the next free decision number. */
function runNumbers(): void {
  section("THE NEXT FREE NUMBER", "nvs-diagnostics (every `Code::new`) and docs/decisions/ filenames");
  const text = read(DIAGNOSTICS);
  if (!text) {
    warn(`could not read ${DIAGNOSTICS} -- it is the diagnostic-code registry`);
  } else {
    const legend = new Map<string, string>();
    const highest = new Map<string, number>();
    for (const line of text.split("\n")) {
      const m = /^\/\/\/\s*\|\s*`E(\d{2})xx`\s*\|\s*([^|]+?)\s*\|/.exec(line);
      if (m) legend.set(m[1]!, m[2]!.trim());
    }
    // A retired code is never reused, so the comment naming it holds its band's ceiling up.
    const decl = [/Code::new\("(E(\d{2})\d{2})"\)/g, /`(E(\d{2})\d{2})`[^`]{0,80}?\bretired\b/g, /\bretired\b[^`]{0,80}?`(E(\d{2})\d{2})`/g];
    for (const re of decl) {
      for (const m of text.matchAll(re)) highest.set(m[2]!, Math.max(highest.get(m[2]!) ?? 0, Number(m[1]!.slice(1))));
    }
    if (highest.size === 0) {
      warn(`no \`Code::new("Ennnn")\` declarations in ${DIAGNOSTICS}`);
    } else {
      emit("diagnostic codes -- next free in each band (max + 1; a retired code is never");
      emit("reused, so this is deliberately not the lowest hole):");
      for (const band of [...highest.keys()].sort()) {
        const top = highest.get(band)!;
        const meaning = (legend.get(band) ?? "(no row for this band in that file's legend table)").padEnd(48);
        // Max-plus-one leaves the band at `Enn00`, and a filled band's next code comes from the
        // legend's continuation row rather than from the next band's digits.
        if ((top + 1) % 100 === 0) emit(`  E${band}xx  ${meaning} FULL at E${String(top).padStart(4, "0")}`);
        else emit(`  E${band}xx  ${meaning} next: E${String(top + 1).padStart(4, "0")}`);
      }
      const unlisted = [...legend.keys()].filter((b) => !highest.has(b)).sort();
      if (unlisted.length > 0) emit(`  bands with a legend row but no code yet: ${unlisted.map((b) => `E${b}xx`).join(", ")}`);
    }
  }
  let numbers: number[];
  try {
    numbers = readdirSync(join(ROOT, DECISIONS)).flatMap((f) => {
      const m = /^(\d{4})\.md$/.exec(f);
      return m ? [Number(m[1])] : [];
    });
  } catch {
    warn(`no ${DECISIONS}/ directory`);
    return;
  }
  emit();
  if (numbers.length === 0) {
    warn(`no \`NNNN.md\` files in ${DECISIONS}/`);
    return;
  }
  const top = Math.max(...numbers);
  emit(`ADRs: ${numbers.length} on disk, highest ${String(top).padStart(4, "0")} -- next free is ${String(top + 1).padStart(4, "0")}`);
  emit("Claim it by creating the file, and re-check this immediately before you do: another");
  emit("agent working the same tree derives the same answer from the same directory.");
}

// ------------------------------------------------------------------------ the closing

/** The wrap skeleton for this tree, or "" when the writer cannot produce it. */
async function wrapTemplate(): Promise<string> {
  const session = tool("session");
  const argv = session.startsWith("bun ") ? ["bun", "tools/nv/main.ts", "session", "--template"] : ["python", "tools/session.py", "--template"];
  const r = await runProc(argv, { timeoutMs: 60_000 });
  return r.code === 0 ? r.stdout.replace(/^\n+|\n+$/g, "") : "";
}

async function runClosing(): Promise<void> {
  const template = await wrapTemplate();
  const session = tool("session");
  const peek = tool("peek");
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
  emit("AND COMMITS THEM -- the handoff, the playbook and the plan are on disk before any");
  emit("commit is staged, and anything it wrote that no `## commit:` names joins the last one.");
  emit("There is no second call for a docs commit, and no `git add` by hand.");
  emit();
  emit("Nothing about WHAT you write changes -- the handoff contract, one commit per slice and");
  emit("the fixed plan field set all still hold, and `--wrap` refuses input that breaks them.");
  emit();
  emit("WHILE YOU WORK, read in one call, not fifty:");
  emit(`  ${peek} A.rs:120-160 B.rs:@symbol C.md:"## 4" "crates/**/*.rs:re:pat:3"`);
  emit("  a re: target prints the matching line ALONE -- `re:pat:3`, or --context 3 for the");
  emit("  whole call, is how a heading or a `//!` line comes back with the block under it");
  emit(`  ${`${peek} --locate <symbol> ...`.padEnd(46)}file:line anchors, no bodies`);
  emit(`  ${tool("gaps").padEnd(34)}the next group, ranked: cases per member per class,`);
  emit("                                    the PHP twins with no oracle case, the unasserted");
  emit("                                    error paths -- never an `ls tests/` plus a `grep`");
  emit("The first two take as many targets as you have questions. Measured over one 19-session");
  emit("run, a session issued 38 tool calls and carried 1.97 shell commands in each, so the");
  emit("habit is holding -- keep chaining read-only probes rather than spending a call each.");
  emit();
  emit("`target/debug/nvs.exe` IS ALREADY BUILT at the commit this session starts from -- the");
  emit("driver builds it after every acceptance check. Run it. Do not `ls` it first, and");
  emit("rebuild only once you have changed Rust yourself.");
  emit();
  emit("If this pack did not print something you needed, that is a gap in [context] in");
  emit("docs/agent/loop-goal.toml. Say which field was missing it, in the handoff.");
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
  for (const [title, size] of ledger) {
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
export function manifestFindings(g: GoalValue, where: string, prose: string | null): { problems: string[]; notes: string[] } {
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
    const book = playbookBook();
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
    const book = rulebook();
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
    out = [];
    ledger = [];
    runTraps(args.slice(1));
    process.stdout.write(out.join("\n").replace(/^\n+/, "") + "\n");
    return 0;
  }
  const opts = parse(args);
  if (typeof opts === "string") {
    console.error(`nv orient: ${opts}\n${summary}`);
    return 2;
  }
  out = [];
  ledger = [];
  trapsDetail = [];

  const src = sources(opts.goal);
  if (src === null) {
    process.stdout.write(`nv orient: no goal is installed at ${LIVE.md}, so there is no goal to narrow to.\nRun \`python tools/brief.py\` for the unscoped orientation.\n`);
    return 2;
  }
  const unread: Unread[] = [];
  const g = goalRecord(src, opts.goal === null, unread);
  if (g === null) {
    process.stdout.write(`nv orient: goal ${src.slug} has no record, so there is no goal to narrow to.\nRun \`python tools/brief.py\` for the unscoped orientation.\n`);
    return 2;
  }
  const h = handoffRecord(src, unread);
  const stage = opts.stage ?? h?.next.stage ?? null;
  const m = manifest(g, opts.full ? null : stage);
  if (opts.full) m.modules = ["crates/**", "editors/**"];

  emit("Novis -- oriented to the current goal. This is deliberately narrow: it prints what this");
  emit("goal's [context] manifest names and nothing else. `python tools/brief.py` is the wide one.");
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

  await runMarker(src);
  const item = runState(src, h, opts.item);
  runAnchors(src, item);
  runStandingDecisions(src);
  runRules(m);
  runAdrs(m);
  await runMap(m);
  runShapes(m);
  runPlaybook(m.playbook, item, stage);
  runPlan(m);
  runMilestones(m);
  runNumbers();
  await runClosing();
  closeLedger();

  if (!m.present) warn(`${src.toml} has no [context] block at all, so nothing could be selected. docs/agent/loop-authoring.md § 2 has the field list.`);
  for (const u of unread) warn(`${u.path}: ${u.reason}`);

  let body = out.join("\n").replace(/^\n+/, "");
  if (opts.audit) body += "\n" + audit().join("\n");
  process.stdout.write(body + "\n");
  return 0;
}
