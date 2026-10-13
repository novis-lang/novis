// `bun nv sweep`: the floor gate, which looper runs after every session as `looper.toml`'s
// `after_session` hook.
//
//     bun nv sweep [--goal <slug>] [--keep-runs <n>]
//
// looper runs the live goal's acceptance checks itself after every session (`[loop] sweep = "every"`), so
// this command runs none of them. It runs what those checks leave out. Each run counts one session in the
// selection store's meta (`FLOOR_META`), and one run in `FLOOR_GATE_EVERY` opens the floor gate, which
// runs, in order:
//
// - `bun nv select --full` (`select/full.ts` `fullRun`): every atom that is not heavy runs and records,
//   and a red one the selection did not pick is a selection miss;
// - both Linux legs, the WSL leg and the valgrind sweep, over every fixture and suite of the goal,
//   whatever changed, as `bun nv legs --full` runs them (`cmd/legs.ts` `legsOver`). A leg records no
//   footprint of its own, so this run is the safety net for what its selection cannot see. The goal is
//   `--goal`'s, else the one `LOOPER_GOAL` names, else the current one (`lib/chain.ts` `namedPlan`), and
//   only the legs read it;
// - every bench the selection picks, on the release build (`cmd/benches.ts` `benchTurn`).
//
// The count goes back to 0 once the three have run, red or green. A step that throws leaves the count
// where it was, so the next run opens the gate again. Every run then sweeps the disk, as `nv disk
// --clean` does: the build is warm, and nothing else is building.
//
// What the gate found goes into `.loop/last-sweep.md`, the file looper writes from its own sweep just
// before this hook runs, and which its orient prints under "Fix these first". This command owns only the
// lines that start with `FLOOR_LINE`. It replaces them with this run's lines and leaves looper's lines
// alone, and it removes the file when no line is left. A shut gate finds nothing, so it only removes
// lines an earlier run left there. A run that cannot work, because of a bad argument or a step that
// threw, writes why as one of those lines.
//
// It always exits 0, because looper pauses the loop on a hook that fails: the file is the verdict.

import { existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { namedPlan } from "../lib/chain.ts";
import { ROOT } from "../lib/paths.ts";
import { describeFull, fullRun } from "../select/full.ts";
import { SelectStore } from "../select/store.ts";
import { benchTurn } from "./benches.ts";
import { clean, freedLines, freeGb, human, KEEP_RUNS, total } from "./disk.ts";
import { type LegsCmdSeams, legsOver } from "./legs.ts";

/** The file looper writes with its sweep's reds, repo-relative, to which the floor gate adds its own. */
export const LAST_SWEEP = ".loop/last-sweep.md";
/** How each line the floor gate writes into `LAST_SWEEP` starts; every other line is looper's. */
export const FLOOR_LINE = "- floor gate: ";
/** Runs since the floor gate last opened, in the selection store's `meta`. */
export const FLOOR_META = "floor-gate:since";
/**
 * One run in this many opens the floor gate. A regression only the full run, a Linux leg or a bench sees
 * waits at most this many sessions to be named.
 */
export const FLOOR_GATE_EVERY = 10;

export const summary = `the floor gate looper runs after each session: 1 run in ${FLOOR_GATE_EVERY} runs the full selection run, the Linux legs and the benches, every run sweeps the disk, and the reds go into ${LAST_SWEEP}: nv sweep [--goal <slug>] [--keep-runs <n>]`;

const USAGE = "usage: bun nv sweep [--goal <slug>] [--keep-runs <n>]";

/** How the gate reports itself: the command prints it, and a test silences it. */
export interface SweepVoice {
  /** A step of the gate. */
  step(line: string): void;
  /** A line under a step. */
  note(line: string): void;
}

/** Everything the gate reads or starts, replaced under test. Each one left out is the real one. */
export interface SweepSeams {
  root: string;
  /** The count the store's `meta` holds under `key`, or `missing` when it holds none. */
  meta(key: string, missing: number): number;
  setMeta(key: string, count: number): void;
  /** `bun nv select --full`: a red line naming the selection misses, or "". */
  fullRun(note: (line: string) => void): Promise<string>;
  /** Both Linux legs over every fixture and suite of goal `slug`, `namedPlan`'s goal, whatever changed: a red line, or "". */
  legs(slug: string | undefined, note: (line: string) => void): Promise<string>;
  /** The bench turn: a red line, or "". */
  benches(note: (line: string) => void): Promise<string>;
  /** The disk sweep's one-line summary. */
  disk(keepRuns: number, say: (line: string) => void): Promise<string>;
}

/** What the gate runs over. */
export interface FloorInput {
  /** The goal whose fixtures and suites the legs run, `--goal`'s; `namedPlan` decides when unset. */
  slug?: string;
  keepRuns: number;
}

/** What the gate did. */
export interface FloorOutcome {
  /** The gate opened on this run. */
  open: boolean;
  /** Each red line the full run, the legs and the benches returned. */
  reds: string[];
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
 * `bun nv select --full` inside the gate: every atom that is not heavy runs and records, and the store's
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

/**
 * Both legs over `namedPlan`'s goal, whatever changed, or a red line saying no goal is there to read them
 * from. `over` replaces what `legsOver` reads or starts, under test.
 */
export async function legsTurn(slug: string | undefined, note: (line: string) => void, over: Partial<LegsCmdSeams> = {}): Promise<string> {
  const found = (over.plan ?? namedPlan)(slug);
  if (found === null) return `Linux legs: not run, ${slug ? `no goal record for \`${slug}\`` : "no goal is named or current"}`;
  return legsOver(found.goal, true, { say: note, ...over });
}

/** `nv disk --clean`'s sweep. Says each part, and returns the one-line summary. */
async function sweepDisk(keepRuns: number, say: (line: string) => void, root: string = ROOT): Promise<string> {
  const before = freeGb(root);
  let freed: Record<string, number | null>;
  try {
    freed = await clean({ keepRuns });
  } catch (e) {
    const failed = `disk: the sweep failed -- ${(e as Error).message}`;
    say(failed);
    return failed;
  }
  let summary = `disk: freed ${human(total(freed))}, ${before.toFixed(1)}G -> ${freeGb(root).toFixed(1)}G free`;
  for (const line of freedLines(freed)) say(`  ${line}`);
  if (freed["target/deps"] === null) summary += "; deps/ left alone, cargo could not name the live set";
  return summary;
}

const REAL: SweepSeams = {
  root: ROOT,
  meta: metaCount,
  setMeta: setMetaCount,
  fullRun: fullRunTurn,
  legs: legsTurn,
  benches: (note) => benchTurn(note),
  disk: (keepRuns, say) => sweepDisk(keepRuns, say),
};

/** The floor gate after one session: counts it, runs the gate when it opens, sweeps the disk, and writes the reds. */
export async function floorGate(i: FloorInput, voice: SweepVoice, over: Partial<SweepSeams> = {}): Promise<FloorOutcome> {
  const s: SweepSeams = { ...REAL, ...over };
  const since = s.meta(FLOOR_META, FLOOR_GATE_EVERY) + 1;
  const open = since >= FLOOR_GATE_EVERY;
  const reds: string[] = [];
  if (open) {
    voice.step(`floor gate open, 1 run in ${FLOOR_GATE_EVERY}: the full run, the Linux legs and the benches`);
    voice.step("full run: every atom that is not heavy runs and records, and a red one the selection did not pick is a selection miss");
    reds.push(await s.fullRun(voice.note));
    voice.step("Linux legs: the WSL leg and the valgrind sweep over every fixture and suite of the goal, whatever changed");
    reds.push(await s.legs(i.slug, voice.note));
    voice.step("benches: every selected bench, owed, red, new or reached, is judged on the release build and recorded");
    reds.push(await s.benches(voice.note));
  } else {
    voice.step(`floor gate shut: run ${since} of ${FLOOR_GATE_EVERY} since it last opened`);
  }
  s.setMeta(FLOOR_META, open ? 0 : since);
  voice.step(await s.disk(i.keepRuns, voice.note));
  const found = reds.filter((r) => r !== "");
  writeFloorLines(s.root, found.flatMap(redsOf));
  return { open, reds: found };
}

/** The reds a step's line names: its first line, and each `also red:` line after it. */
function redsOf(red: string): string[] {
  const [first, ...rest] = red.split("\n");
  return [first!.trim(), ...rest.filter((l) => l.trim().startsWith("also red: ")).map((l) => l.trim().slice("also red: ".length))];
}

/**
 * Replaces the floor gate's lines in `.loop/last-sweep.md` with `reds`, each after `FLOOR_LINE`, and
 * keeps every other line as looper wrote it. Removes the file when no line is left.
 */
export function writeFloorLines(root: string, reds: string[]): void {
  const path = join(root, LAST_SWEEP);
  const before = existsSync(path) ? readFileSync(path, "utf8").replace(/\r\n/g, "\n") : "";
  const kept = before.split("\n").filter((l) => !l.startsWith(FLOOR_LINE)).join("\n").trimEnd();
  const lines = [...(kept === "" ? [] : [kept]), ...reds.map((r) => `${FLOOR_LINE}${r}`)];
  if (lines.length === 0) {
    rmSync(path, { force: true });
    return;
  }
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, `${lines.join("\n")}\n`);
}

/** How the command reports itself: every line to stdout, which looper keeps in the hook's log. */
const PRINTED: SweepVoice = {
  step: (l) => console.log(l),
  note: (l) => console.log(`   ${l}`),
};

/** What the command reads besides the gate's seams: how it reports itself. */
export interface SweepCmdSeams extends SweepSeams {
  voice: SweepVoice;
}

/** Parses, runs the floor gate and writes its lines into `.loop/last-sweep.md`; returns 0 whatever it found. */
export async function sweep(args: string[], over: Partial<SweepCmdSeams> = {}): Promise<number> {
  const root = over.root ?? ROOT;
  const voice = over.voice ?? PRINTED;
  let slug: string | undefined;
  let keepRuns = KEEP_RUNS;
  for (let i = 0; i < args.length; i++) {
    const a = args[i]!;
    const value = args[i + 1];
    let bad = "";
    if (a === "--goal" && value !== undefined) slug = args[++i];
    else if (a === "--keep-runs" && value !== undefined && /^\d+$/.test(value)) keepRuns = Number(args[++i]);
    else bad = a === "--goal" || a === "--keep-runs" ? `${a} needs a value` : `unexpected argument ${a}`;
    if (bad) {
      console.error(`${USAGE}\nnv sweep: error: ${bad}`);
      writeFloorLines(root, [`could not run: \`bun nv sweep\` was started with a bad argument: ${bad}`]);
      return 0;
    }
  }
  try {
    const out = await floorGate({ ...(slug !== undefined && { slug }), keepRuns }, voice, over);
    voice.step(out.reds.length > 0 ? `RED: ${LAST_SWEEP} names what the floor gate found` : out.open ? "GREEN: the floor gate" : "the floor gate stays shut");
  } catch (e) {
    const why = (e as Error).message.split("\n")[0] ?? String(e);
    console.error(`nv sweep: ${why}`);
    writeFloorLines(root, [`could not run: the gate stopped: ${why}`]);
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
