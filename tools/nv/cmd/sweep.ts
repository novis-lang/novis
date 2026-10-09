// `bun nv sweep`: the sweep after a session, the one the loop's turn runs in-process and looper's hook runs
// as a command.
//
//     bun nv sweep [--goal <slug>] [--done] [--keep-runs <n>] [--session <label>]
//
// The goal is `--goal`'s, else the one `LOOPER_GOAL` names, else the current one (`lib/chain.ts`
// `namedPlan`). A goal whose record says `sweep: "done"` is swept only when `--done` says the session
// reported DONE. Otherwise the floor gate's count in the selection store's meta (`FLOOR_META`) says
// whether this sweep opens it, one in `FLOOR_GATE_EVERY`. An open gate runs `bun nv select --full` first,
// then the acceptance sweep over every check; a shut one holds the heavy checks and the Linux legs. A
// scoped sweep that is green with checks held runs again over the whole plan with the gate open, since a
// goal is never reached on a held check, and one goal end in `HEAVY_FULL_EVERY` runs every heavy check and
// leg whatever changed. An open gate then judges the benches (`cmd/benches.ts` `benchTurn`). After every
// sweep the disk is swept, and a sweep that would reach the goal runs the doc and owner gates
// (`accept/gates.ts`). Each step writes the ledger lines `.loop/log.md` carries.
//
// What it found goes to `.loop/last-sweep.md`: one line per red check, its id, its name and its first
// failing line, then any red the full run, the benches or a gate found. A green sweep removes the file,
// because looper's orient prints it under "Fix these first" whenever it exists. A sweep that cannot run,
// over no goal or a step that threw, writes why into the file instead.
//
// It always exits 0, because looper pauses on a hook that fails: the file is the verdict. `--session`
// names the session in the gates' standing verdicts, and is `bun nv sweep` when it is not given.

import { appendFileSync, mkdirSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { type AcceptanceResult, type Check, allReds, heldByGate } from "../accept/accept.ts";
import { docGate, ownerGate, sweepDisk } from "../accept/gates.ts";
import { nameOf, type Progress, stageLabels, sweepOver } from "../accept/sweep.ts";
import { type Goal, namedPlan } from "../lib/chain.ts";
import { ROOT } from "../lib/paths.ts";
import { describeFull, fullRun } from "../select/full.ts";
import { SelectStore } from "../select/store.ts";
import { benchTurn } from "./benches.ts";
import { KEEP_RUNS } from "./disk.ts";

export const summary = "the sweep after a session: floor gate, acceptance sweep, benches, disk and goal-end gates, into .loop/last-sweep.md: nv sweep [--goal <slug>] [--done] [--keep-runs <n>] [--session <label>]";

const USAGE = "usage: bun nv sweep [--goal <slug>] [--done] [--keep-runs <n>] [--session <label>]";

/** What the sweep found, repo-relative; no file is a green sweep. */
export const LAST_SWEEP = ".loop/last-sweep.md";
/** Sweeps since the floor gate last opened, in the selection store's `meta`. */
export const FLOOR_META = "floor-gate:since";
/**
 * One sweep in this many runs the heavy checks and the Linux legs. A regression only they see waits at
 * most this many sessions to be named; every other check runs in the sweep after the change that reached it.
 */
export const FLOOR_GATE_EVERY = 10;
/** Goal ends since the last one that ran every heavy check and Linux leg, in the store's `meta`. */
export const HEAVY_FULL_META = "heavy-full:since";
/**
 * One goal end in this many runs every heavy check and both Linux legs, whatever changed: the safety net
 * for the keys their twins record. Every other goal end runs the ones its change reached, so a heavy
 * check the keys missed waits at most this many goals to be named.
 */
export const HEAVY_FULL_EVERY = 3;

/** How the sweep reports itself: the driver paints it on its console, the command prints it. */
export interface SweepVoice {
  /** A step of the sweep. */
  step(line: string): void;
  /** A line under a step. */
  note(line: string): void;
  /** One line appended to `.loop/log.md`. */
  ledger(line: string): void;
  /** What the sweep is doing now, and how many checks that step has when it counts them. */
  phase(phase: string, total?: number): void;
  /** How an acceptance sweep started at `begun` reports itself; `sweepOver`'s own when omitted. */
  progress?(labelOf: (n: number) => string, begun: number): Progress;
}

/** Everything the sweep reads or starts, replaced under test. Each one left out is the real one. */
export interface SweepSeams {
  root: string;
  /** The count the store's `meta` holds under `key`, or `missing` when it holds none. */
  meta(key: string, missing: number): number;
  setMeta(key: string, count: number): void;
  /** `bun nv select --full`: a red line naming the selection misses, or "". */
  fullRun(note: (line: string) => void): Promise<string>;
  sweep(goal: Goal, checks: Check[], labelOf: (n: number) => string, o: Parameters<typeof sweepOver>[3]): Promise<{ result: AcceptanceResult; secs: number }>;
  /** The bench turn: a red line, or "". */
  benches(note: (line: string) => void): Promise<string>;
  /** The disk sweep's one-line summary. */
  disk(keepRuns: number, say: (line: string) => void): Promise<string>;
  docGate(session: string, say: (line: string) => void): Promise<string>;
  ownerGate(slug: string, session: string, say: (line: string) => void): Promise<string>;
}

/** What a sweep is over. */
export interface SweepInput {
  slug: string;
  goal: Goal;
  /** The session reported DONE. */
  done: boolean;
  /** Names the session in the gates' standing verdicts. */
  session: string;
  keepRuns: number;
  /** Called with each check's verdict once a sweep ends. */
  onDone?: (c: Check, green: boolean) => void;
}

/** What the sweep found. `fail` is the acceptance sweep's verdict with the full run's and the benches' reds folded in. */
export interface SweepOutcome {
  /** The goal is swept only after a DONE, and the session did not report one. */
  skipped: boolean;
  fail: string;
  docsRed: string;
  ownerRed: string;
}

function metaCount(key: string, missing: number): number {
  const store = new SelectStore();
  try {
    const raw = store.meta(key);
    const since = raw === null ? NaN : Number(raw);
    return Number.isInteger(since) && since >= 0 ? since : missing;
  } finally {
    store.close();
  }
}

function setMetaCount(key: string, count: number): void {
  const store = new SelectStore();
  try {
    store.setMeta(key, String(count));
  } finally {
    store.close();
  }
}

/**
 * `bun nv select --full` inside a sweep: every atom that is not heavy runs and records, and the store's
 * tree moves past the change. `fullRun` takes the sweep lock itself while a plan check needs the origin.
 * Returns a red line naming the selection misses, or a covws build that failed, and "" when there is
 * neither.
 */
async function fullRunTurn(note: (line: string) => void): Promise<string> {
  const store = new SelectStore();
  try {
    const r = await fullRun(store, { say: note });
    for (const line of describeFull(r)) note(line);
    if (r.buildFailed) return "full run: the covws build failed";
    if (r.misses.length === 0) return "";
    const named = r.misses.slice(0, 5).map((m) => m.id).join(", ");
    return `full run: ${r.misses.length} selection miss(es), red now and not selected by the change that broke them: ${named}${r.misses.length > 5 ? ", ..." : ""}`;
  } finally {
    store.close();
  }
}

const REAL: SweepSeams = {
  root: ROOT,
  meta: metaCount,
  setMeta: setMetaCount,
  fullRun: fullRunTurn,
  sweep: sweepOver,
  benches: (note) => benchTurn(note),
  disk: (keepRuns, say) => sweepDisk(keepRuns, say),
  docGate: (session, say) => docGate(session, say),
  ownerGate: (slug, session, say) => ownerGate(slug, session, say),
};

const mmss = (secs: number) => `${Math.floor(secs / 60)}:${String(Math.floor(secs % 60)).padStart(2, "0")}`;

/** The sweep after a session over `i.goal`; writes `.loop/last-sweep.md` and returns what it found. */
export async function afterSession(i: SweepInput, voice: SweepVoice, over: Partial<SweepSeams> = {}): Promise<SweepOutcome> {
  const s: SweepSeams = { ...REAL, ...over };
  const all = i.goal.checks as Check[];
  const labelOf = stageLabels(i.goal);
  // A goal whose record says `sweep: "done"` is swept only when a session claims it is reached, so a
  // session that carries on is followed straight by the next one.
  if (i.goal.sweep === "done" && !i.done) {
    voice.step(`acceptance check: not run -- goal \`${i.slug}\` is swept only after a session that reports DONE`);
    return { skipped: true, fail: "", docsRed: "", ownerRed: "" };
  }
  const since = s.meta(FLOOR_META, FLOOR_GATE_EVERY) + 1;
  let open = since >= FLOOR_GATE_EVERY;
  // Every check but a heavy one: what starts is what this session's change reached, carried or the
  // goal's own, and what was owed before it. A setup command is never held, since the fixtures read
  // what it writes.
  const heldBack = open ? [] : all.filter(heldByGate);
  const scoped = all.filter((c) => !heldBack.includes(c));
  // On the floor gate's cadence the full run comes first: every atom that is not heavy runs and records,
  // so the sweep after it reaches the heavy checks, the Linux legs and whatever is red. A selection miss
  // it finds is a red of this sweep.
  let missed = "";
  if (open) {
    voice.step("full run: every atom that is not heavy runs and records, and a red one the selection did not pick is a selection miss");
    voice.phase("full run");
    missed = await s.fullRun(voice.note);
    if (missed) voice.ledger(`       ${missed}`);
  }
  voice.step(`acceptance check: every check a change reached${open ? ", the heavy ones and the Linux legs with them" : ` (heavy checks and the Linux legs held, 1 session in ${FLOOR_GATE_EVERY})`}`);
  const checkedAt = performance.now();
  const progress = voice.progress?.(labelOf, checkedAt);
  const told = { ...(i.onDone && { onDone: i.onDone }), ...(progress && { progress }) };
  voice.phase("acceptance sweep", scoped.length);
  // Every red is collected: a carried check that went red must not keep the goal's own checks behind it
  // from being judged.
  let { result, secs } = await s.sweep(i.goal, scoped, labelOf, { full: false, collect: true, legs: true, gateOpen: open, ...told });
  let cost = `${secs}s over ${result.ran} check(s), ${result.answered} not reached`;
  // Only a sweep that would reach the goal pays for the two goal-end gates. They ask about the goal's own
  // work, which the scoped sweep has just passed, so they run on the gate-open sweep red or green, and one
  // session sees every finding.
  let reaching = result.fail === "";
  if (result.fail === "" && heldBack.length > 0) {
    // On the safety net's cadence of goal ends, every heavy check and leg runs, whatever this goal changed.
    const goalEnds = s.meta(HEAVY_FULL_META, HEAVY_FULL_EVERY) + 1;
    const heavyAll = goalEnds >= HEAVY_FULL_EVERY;
    const which = heavyAll ? `every heavy check and leg, 1 goal end in ${HEAVY_FULL_EVERY}` : "what this goal changed";
    voice.step(`scoped sweep green with ${heldBack.length} check(s) held -- opening the floor gate over ${which} before the goal is reached`);
    voice.ledger(`       goal cost: ${cost}, ${heldBack.length} held (scoped; opening the floor gate${heavyAll ? " over every heavy check" : ""})`);
    open = true;
    reaching = true;
    voice.phase("acceptance sweep, floor gate open", all.length);
    ({ result, secs } = await s.sweep(i.goal, all, labelOf, { full: false, collect: true, legs: true, gateOpen: true, heavyAll, ...told }));
    cost = `${secs}s over ${result.ran} check(s), ${result.answered} not reached`;
    s.setMeta(HEAVY_FULL_META, heavyAll ? 0 : goalEnds);
  }
  // With the floor gate open, every selected bench is judged after the sweep, and a red bench is a red
  // of the sweep, as it is of `bun nv affected --run`.
  let benchRed = "";
  if (open) {
    voice.step("benches: every selected bench, owed, red, new or reached, is judged on the release build and recorded");
    voice.phase("benches, floor gate open");
    benchRed = await s.benches(voice.note);
    if (benchRed) voice.ledger(`       ${benchRed}`);
  }
  s.setMeta(FLOOR_META, open ? 0 : since);
  let fail = result.fail;
  for (const red of [missed, benchRed]) if (red) fail = fail ? allReds([fail, red]) : red;
  voice.step(`acceptance check done in ${mmss((performance.now() - checkedAt) / 1000)}`);
  voice.ledger(`       goal cost: ${cost}${open ? "" : `, ${heldBack.length} held (floor gate shut)`}`);
  // Every sweep: it has just left the build warm, nothing is building, and a day of builds is what fills `target/`.
  voice.phase("cleaning disk");
  voice.ledger(`       ${await s.disk(i.keepRuns, voice.note)}`);
  const docsRed = reaching ? await s.docGate(i.session, voice.step) : "";
  if (docsRed) voice.ledger(`       doc gate: ${docsRed}`);
  const ownerRed = reaching ? await s.ownerGate(i.slug, i.session, voice.step) : "";
  if (ownerRed) voice.ledger(`       owner gate: ${ownerRed}`);
  writeLastSweep(s.root, redLines(all, labelOf, fail, docsRed, ownerRed), i.slug);
  return { skipped: false, fail, docsRed, ownerRed };
}

/**
 * One line per red: a check's id before its verdict line, which names it and says how it failed, and
 * every other red, the full run's, the benches' and the gates', as it stands.
 */
export function redLines(checks: Check[], labelOf: (n: number) => string, fail: string, docsRed: string, ownerRed: string): string[] {
  const out: string[] = [];
  if (fail !== "") {
    const [first, ...rest] = fail.split("\n");
    const reds = [first!, ...rest.filter((l) => l.trim().startsWith("also red: ")).map((l) => l.trim().slice("also red: ".length))];
    for (const red of reds) {
      const c = checks.find((c) => red.startsWith(`${nameOf(c)} [${labelOf(c.stage)}]`));
      out.push(c ? `- \`${c.id}\` ${red.trim()}` : `- ${red.trim()}`);
    }
  }
  if (docsRed) out.push(`- doc gate: ${docsRed}`);
  if (ownerRed) out.push(`- owner gate: ${ownerRed}`);
  return out;
}

/** Writes the reds to `.loop/last-sweep.md`, or removes it when there are none. */
function writeLastSweep(root: string, reds: string[], slug: string): void {
  const path = join(root, LAST_SWEEP);
  if (reds.length === 0) {
    rmSync(path, { force: true });
    return;
  }
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, `# The sweep after the last session is red, goal \`${slug}\`\n\n${reds.join("\n")}\n`);
}

/** Writes why the sweep could not run to `.loop/last-sweep.md`. */
function cannotRun(root: string, why: string): void {
  const path = join(root, LAST_SWEEP);
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, `# The sweep after the last session could not run\n\n- ${why}\n`);
}

/** How the command reports itself: every line to stdout, the ledger's to `.loop/log.md` as well. */
function printed(root: string): SweepVoice {
  return {
    step: (l) => console.log(l),
    note: (l) => console.log(`   ${l}`),
    ledger: (l) => {
      const path = join(root, ".loop", "log.md");
      mkdirSync(dirname(path), { recursive: true });
      appendFileSync(path, `${l}\n`);
      console.log(l);
    },
    phase: () => {},
  };
}

/** What the command reads besides the sweep's seams: the goal's plan and how it reports itself. */
export interface SweepCmdSeams extends SweepSeams {
  plan(slug: string | undefined): { slug: string; goal: Goal } | null;
  voice: SweepVoice;
}

/** Parses, sweeps and writes `.loop/last-sweep.md`; returns 0 whatever it found. */
export async function sweep(args: string[], over: Partial<SweepCmdSeams> = {}): Promise<number> {
  const root = over.root ?? ROOT;
  const voice = over.voice ?? printed(root);
  const plan = over.plan ?? namedPlan;
  let slug: string | undefined;
  let done = false;
  let keepRuns = KEEP_RUNS;
  let session = "bun nv sweep";
  for (let i = 0; i < args.length; i++) {
    const a = args[i]!;
    const value = args[i + 1];
    let bad = "";
    if (a === "--done") done = true;
    else if (a === "--goal" && value !== undefined) slug = args[++i];
    else if (a === "--session" && value !== undefined) session = args[++i]!;
    else if (a === "--keep-runs" && value !== undefined && /^\d+$/.test(value)) keepRuns = Number(args[++i]);
    else bad = a === "--goal" || a === "--session" || a === "--keep-runs" ? `${a} needs a value` : `unexpected argument ${a}`;
    if (bad) {
      console.error(`${USAGE}\nnv sweep: error: ${bad}`);
      cannotRun(root, `\`bun nv sweep\` was started with a bad argument: ${bad}`);
      return 0;
    }
  }
  const found = plan(slug);
  if (found === null) {
    const why = slug ? `no goal record for \`${slug}\`` : "no goal is named or current";
    console.error(`nv sweep: ${why}`);
    cannotRun(root, `\`bun nv sweep\` has no plan to sweep: ${why}`);
    return 0;
  }
  try {
    const out = await afterSession({ slug: found.slug, goal: found.goal, done, session, keepRuns }, voice, over);
    if (!out.skipped) voice.step(out.fail || out.docsRed || out.ownerRed ? `RED: ${LAST_SWEEP} names what is red` : `GREEN: goal \`${found.slug}\``);
  } catch (e) {
    const why = (e as Error).message.split("\n")[0] ?? String(e);
    console.error(`nv sweep: ${why}`);
    cannotRun(root, `the sweep stopped: ${why}`);
  }
  return 0;
}

export async function run(args: string[]): Promise<number> {
  if (args.includes("--help") || args.includes("-h")) {
    console.log(`${USAGE}\n\n${summary}`);
    return 0;
  }
  return sweep(args);
}
