// `bun nv sweep`: what looper runs after every session as `looper.toml`'s `after_session` hook.
//
//     bun nv sweep [--goal <slug>] [--keep-runs <n>]
//
// looper runs the live goal's acceptance checks itself after every session (`[loop] sweep = "every"`), so
// this command runs none of them. Each run does three things, in this order.
//
// **It pays floor debt for at most `DEBT_BUDGET_MS`.** The debt is what the selection store holds that no
// session runs: every bench owed or red, and every proof program, case, test executable, tools test file,
// plan check and Linux leg that is owed. `nv verify` and `bun nv affected --run` mark owed each atom a
// change reached that they did not run (`select/record.ts` `advance`). The red atoms go first, then the
// owed ones, each list by its last run, oldest first (`debtTurns`). One runner takes each turn, and the
// turns go in the order of their first atoms: `select/full.ts` `fullRun` with `only` for every kind but
// the benches and the legs, `cmd/benches.ts` `benchTurn` for the benches, and `cmd/legs.ts` `legsOver`
// for the legs. The legs run over `--goal`'s goal, else the one `LOOPER_GOAL` names, else the current one
// (`lib/chain.ts` `namedPlan`). The budget is asked before each turn, and each runner asks it again before
// each atom starts. An atom that started runs to its end, and what the budget did not reach stays owed for
// the next run. It prints what it paid and what debt is left, by kind.
//
// **It sweeps the disk**, as `nv disk --clean` does: the build is warm, and nothing else is building.
//
// **It reads the newest nightly CI run back** (`readNightly`): the newest completed `workflow_dispatch`
// run of `ci.yml` on `main`, which the cron on `landing` dispatches. It asks `gh`, and it only reads.
//
// What it found goes into `.loop/last-sweep.md`, the file looper writes from its own sweep just before
// this hook runs, and which its orient prints under "Fix these first". This command owns the lines that
// start with `FLOOR_LINE`, each red the debt run found, and with `NIGHTLY_LINE`, each job the nightly run
// failed. Each run replaces both sets with what it found and leaves looper's lines alone, and it removes
// the file when no line is left. A run that cannot work, because of a bad argument or a runner that threw,
// writes why as a `FLOOR_LINE`; a nightly run `gh` cannot read is one `NIGHTLY_LINE` that says so.
//
// Under looper, which names an event file in `LOOPER_EVENTS`, the run writes looper's progress events
// (`lib/events.ts`), and outside it writes none. It declares the debt's item, `DEBT_ITEM`, and under it
// one item per debt atom in the order the turns pay them, with the atom's id and the debt kind; then one
// item each for the disk sweep and the nightly read, and `planned`. A runner tells it of each atom it
// starts on its own: a test executable, a tools test file, a plan check or a bench. The Linux legs start
// with their turn, and a case or a proof program, which runs in a batch, only ends. An atom's item ends
// when its run is recorded (`store.ts` `onRecorded`), red when it ran red. One paid with no recorded run
// ends with its verdict once the turns are over, and one the budget did not reach ends `skipped`. The
// run ends with `done`. What it spends: one appended line per item event.
//
// It always exits 0, because looper pauses the loop on a hook that fails: the file is the verdict.

import { existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { namedPlan } from "../lib/chain.ts";
import { type EndFields, ev } from "../lib/events.ts";
import { ROOT } from "../lib/paths.ts";
import { run as proc } from "../lib/proc.ts";
import { LEGS, legId } from "../select/checks.ts";
import { fullRun } from "../select/full.ts";
import { type AtomRow, onRecorded, SelectStore } from "../select/store.ts";
import { benchTurn } from "./benches.ts";
import { clean, freedLines, freeGb, human, KEEP_RUNS, total } from "./disk.ts";
import { type LegsCmdSeams, legsOver } from "./legs.ts";

/** The file looper writes with its sweep's reds, repo-relative, to which this command adds its own. */
export const LAST_SWEEP = ".loop/last-sweep.md";
/** How each line about the floor debt starts in `LAST_SWEEP`. */
export const FLOOR_LINE = "- floor debt: ";
/** How each line about the nightly CI run starts in `LAST_SWEEP`. Every line with neither prefix is looper's. */
export const NIGHTLY_LINE = "- nightly CI: ";
/**
 * How long one run pays floor debt, in milliseconds. Ten minutes keeps the hook short beside the session it
 * follows, so the loop is not held between sessions, and is long enough for a bench or a pool of proof
 * programs to finish. Debt larger than the budget is paid over the runs that follow.
 */
export const DEBT_BUDGET_MS = 10 * 60_000;

/** The progress items of the run's three parts; each debt atom's item is its atom id, under `DEBT_ITEM`. */
export const DEBT_ITEM = "sweep:debt";
export const DISK_ITEM = "sweep:disk";
export const NIGHTLY_ITEM = "sweep:nightly";

export const summary = `looper's hook after each session: pays owed and red atoms for at most ${DEBT_BUDGET_MS / 60_000} minutes, sweeps the disk, reads the newest nightly CI run, and writes what is red into ${LAST_SWEEP}: nv sweep [--goal <slug>] [--keep-runs <n>]`;

const USAGE = "usage: bun nv sweep [--goal <slug>] [--keep-runs <n>]";

/** A kind of floor debt: an atom kind, or `leg` for the atom of a Linux leg. */
export type DebtKind = "bench" | "proof" | "case" | "test" | "nvtest" | "check" | "nv" | "leg";

/** One atom of the floor debt. */
export interface DebtAtom {
  id: string;
  kind: DebtKind;
  verdict: "red" | "owed";
  /** When it last ran, in milliseconds since the epoch; 0 for never. */
  lastRun: number;
}

/** What runs a turn of the debt: `fullRun`, `benchTurn` or `legsOver`. */
export type Runner = "full" | "bench" | "leg";

/** The atoms one runner pays, in the order they run. */
export interface DebtTurn {
  runner: Runner;
  ids: string[];
}

/** How the command reports itself: the command prints it, and a test silences it. */
export interface SweepVoice {
  /** A step of the run. */
  step(line: string): void;
  /** A line under a step. */
  note(line: string): void;
}

/** What `gh` printed, and how it exited. */
export interface GhRun {
  code: number;
  stdout: string;
  stderr: string;
}

/** Everything the command reads or starts, replaced under test. Each one left out is the real one. */
export interface SweepSeams {
  root: string;
  /** The clock the budget is read on, in milliseconds since the epoch. */
  now(): number;
  /** The floor debt the store holds now. */
  debt(): DebtAtom[];
  /** Runs one turn, no atom of it starting at or after `deadline`: a red line, or "". `slug` is the goal
   * the legs run over, `namedPlan`'s when unset. `onStart` is told of each atom as the runner starts it. */
  pay(turn: DebtTurn, deadline: number, slug: string | undefined, note: (line: string) => void, onStart?: (id: string) => void): Promise<string>;
  /** `gh` with `args`. It throws when `gh` cannot start. */
  gh(args: string[]): Promise<GhRun>;
  /** The disk sweep's one-line summary. */
  disk(keepRuns: number, say: (line: string) => void): Promise<string>;
}

/** What the debt run runs over. */
export interface DebtInput {
  /** The goal whose fixtures and suites the legs run, `--goal`'s; `namedPlan` decides when unset. */
  slug?: string;
  keepRuns: number;
}

/** What the debt run did. */
export interface DebtOutcome {
  /** The atoms of the debt that ran. */
  paid: DebtAtom[];
  /** The debt the store holds after the run. */
  left: DebtAtom[];
  /** Each red line a turn returned. */
  reds: string[];
}

// ---- the debt ----------------------------------------------------------------------------------------

const LEG_IDS = new Set<string>(LEGS.map(legId));

/** Each atom of `rows` that is floor debt: a bench owed or red, and an owed atom of the other kinds. */
export function debtOf(rows: Pick<AtomRow, "id" | "kind" | "verdict" | "lastRun">[]): DebtAtom[] {
  const out: DebtAtom[] = [];
  for (const a of rows) {
    let kind: DebtKind | null = null;
    if (a.kind === "heavy") kind = LEG_IDS.has(a.id) ? "leg" : null;
    else if (a.kind === "bench" || a.kind === "proof" || a.kind === "case" || a.kind === "test" || a.kind === "nvtest" || a.kind === "check" || a.kind === "nv") kind = a.kind;
    if (kind === null) continue;
    if (a.verdict === "owed" || (a.verdict === "red" && kind === "bench")) out.push({ id: a.id, kind, verdict: a.verdict, lastRun: a.lastRun });
  }
  return out;
}

function runnerOf(kind: DebtKind): Runner {
  return kind === "bench" ? "bench" : kind === "leg" ? "leg" : "full";
}

/**
 * The turns that pay `debt`: the red atoms first, then the owed ones, each list ordered by last run,
 * oldest first, then by id. Within each list the atoms of one runner form one turn, and the turns go in
 * the order of their first atoms.
 */
export function debtTurns(debt: DebtAtom[]): DebtTurn[] {
  const byAge = (a: DebtAtom, b: DebtAtom) => a.lastRun - b.lastRun || (a.id < b.id ? -1 : a.id > b.id ? 1 : 0);
  const turns: DebtTurn[] = [];
  for (const verdict of ["red", "owed"] as const) {
    const byRunner = new Map<Runner, string[]>();
    for (const a of debt.filter((d) => d.verdict === verdict).sort(byAge)) {
      const r = runnerOf(a.kind);
      (byRunner.get(r) ?? byRunner.set(r, []).get(r)!).push(a.id);
    }
    for (const [runner, ids] of byRunner) turns.push({ runner, ids });
  }
  return turns;
}

/** The atoms of `before` that ran: gone from the debt, or with a newer run or another verdict. */
export function paidOf(before: DebtAtom[], after: DebtAtom[]): DebtAtom[] {
  const now = new Map(after.map((a) => [a.id, a]));
  return before.filter((a) => {
    const b = now.get(a.id);
    return b === undefined || b.lastRun !== a.lastRun || b.verdict !== a.verdict;
  });
}

/** `atoms` counted by kind, as `3 proof, 1 bench`, most first. */
export function byKind(atoms: DebtAtom[]): string {
  const n = new Map<DebtKind, number>();
  for (const a of atoms) n.set(a.kind, (n.get(a.kind) ?? 0) + 1);
  return [...n].sort((a, b) => b[1] - a[1] || (a[0] < b[0] ? -1 : 1)).map(([k, c]) => `${c} ${k}`).join(", ");
}

function minutes(ms: number): string {
  const s = Math.max(0, Math.round(ms / 1000));
  return `${Math.floor(s / 60)}m${String(s % 60).padStart(2, "0")}s`;
}

/**
 * Pays the floor debt for at most `DEBT_BUDGET_MS`, sweeps the disk, and writes the reds after
 * `FLOOR_LINE`. No turn starts once the budget is spent, and each runner is handed the deadline.
 */
export async function payDebt(i: DebtInput, voice: SweepVoice, s: SweepSeams): Promise<DebtOutcome> {
  const before = s.debt();
  const started = s.now();
  const deadline = started + DEBT_BUDGET_MS;
  const reds: string[] = [];
  const turns = debtTurns(before);
  // The plan: the debt's item and an item per atom under it, in the order the turns pay them, then the
  // disk sweep and the nightly read.
  const planned = new Set(turns.flatMap((t) => t.ids));
  const kindOf = new Map(before.map((a) => [a.id, a.kind]));
  ev.add(DEBT_ITEM, { name: "floor debt", kind: "step" });
  for (const id of planned) ev.add(id, { parent: DEBT_ITEM, name: id, kind: kindOf.get(id) ?? "atom" });
  ev.add(DISK_ITEM, { name: "disk sweep", kind: "step" });
  ev.add(NIGHTLY_ITEM, { name: "nightly CI read", kind: "step" });
  ev.planned();
  ev.start(DEBT_ITEM);
  // An atom's item ends when its run is recorded, whichever runner records it.
  const ended = new Set<string>();
  const end = (id: string, f: EndFields) => {
    if (!planned.has(id) || ended.has(id)) return;
    ended.add(id);
    ev.end(id, f);
  };
  onRecorded((id, verdict) => end(id, verdict === "red" ? { result: "red", why: "ran red" } : {}));
  const onStart = (id: string) => {
    if (planned.has(id) && !ended.has(id)) ev.start(id);
  };
  try {
    if (before.length === 0) voice.step("floor debt: none");
    else {
      voice.step(`floor debt: ${before.length} atom(s), ${byKind(before)}; paying for at most ${minutes(DEBT_BUDGET_MS)}, the red ones first, then the oldest owed`);
      for (const turn of turns) {
        if (s.now() >= deadline) {
          voice.step("floor debt: the budget is spent, so the rest waits for the next run");
          break;
        }
        const kinds = byKind(before.filter((a) => turn.ids.includes(a.id)));
        voice.step(`floor debt: paying ${kinds}, by ${turn.runner === "full" ? "the full run's runners" : turn.runner === "bench" ? "the bench turn" : "the Linux legs"}`);
        reds.push(await s.pay(turn, deadline, i.slug, voice.note, onStart));
      }
    }
  } finally {
    onRecorded(null);
  }
  const left = before.length === 0 ? [] : s.debt();
  const paid = paidOf(before, left);
  if (before.length > 0) {
    voice.step(`floor debt: paid ${paid.length}${paid.length > 0 ? ` (${byKind(paid)})` : ""} in ${minutes(s.now() - started)}; left ${left.length}${left.length > 0 ? ` (${byKind(left)})` : ""}`);
  }
  // An atom paid without a recorded run, such as a bench a skip proved, ends with its verdict now, and an
  // atom the budget did not reach ends skipped.
  const after = new Map(left.map((a) => [a.id, a]));
  const paidIds = new Set(paid.map((a) => a.id));
  for (const id of planned) {
    if (paidIds.has(id)) end(id, after.get(id)?.verdict === "red" ? { result: "red", why: "ran red" } : {});
    else end(id, { result: "skipped", why: "the budget did not reach it, so it stays owed for the next run" });
  }
  const found = reds.filter((r) => r !== "");
  ev.end(DEBT_ITEM, found.length > 0 ? { result: "red", why: found[0]!.split("\n")[0]! } : {});
  ev.start(DISK_ITEM);
  const disk = await s.disk(i.keepRuns, voice.note);
  voice.step(disk);
  ev.end(DISK_ITEM, disk.startsWith("disk: the sweep failed") ? { result: "red", why: disk } : {});
  writeOwnLines(s.root, FLOOR_LINE, found.flatMap(redsOf));
  return { paid, left, reds: found };
}

/** The reds a turn's line names: its first line, and each `also red:` line after it. */
function redsOf(red: string): string[] {
  const [first, ...rest] = red.split("\n");
  return [first!.trim(), ...rest.filter((l) => l.trim().startsWith("also red: ")).map((l) => l.trim().slice("also red: ".length))];
}

// ---- the real runners --------------------------------------------------------------------------------

function storeDebt(): DebtAtom[] {
  const store = new SelectStore();
  try {
    return debtOf(store.atoms());
  } finally {
    store.close();
  }
}

/**
 * The atoms of `ids` that are not benches or legs, run by `fullRun` in their order until `deadline`, each
 * recorded. Returns a red line naming the atoms that ran red, or the covws build that failed, and "" when
 * there is neither.
 */
async function fullTurn(ids: string[], deadline: number, note: (line: string) => void, onStart: (id: string) => void): Promise<string> {
  const store = new SelectStore();
  try {
    const r = await fullRun(store, { only: ids, deadline, say: note, onStart });
    if (r.buildFailed) return "owed atoms: not run, the covws build failed";
    const red = [...r.ran].filter(([, v]) => v === "red").map(([id]) => id);
    note(`owed atoms: ${r.ran.size} ran, ${red.length} red, in ${r.seconds}s`);
    if (red.length === 0) return "";
    return `owed atoms: ${red.length} ran red: ${red.slice(0, 5).join(", ")}${red.length > 5 ? ", ..." : ""}`;
  } finally {
    store.close();
  }
}

/**
 * The Linux legs over `namedPlan`'s goal, or a red line saying no goal is there to read them from. With
 * `full`, both legs run whatever changed; without it, a leg runs when the store selects it, owed or red
 * among the reasons. `over` replaces what `legsOver` reads or starts, under test.
 */
export async function legsTurn(slug: string | undefined, note: (line: string) => void, full: boolean, over: Partial<LegsCmdSeams> = {}): Promise<string> {
  const found = (over.plan ?? namedPlan)(slug);
  if (found === null) return `Linux legs: not run, ${slug ? `no goal record for \`${slug}\`` : "no goal is named or current"}`;
  return legsOver(found.goal, full, { say: note, ...over });
}

async function payTurn(turn: DebtTurn, deadline: number, slug: string | undefined, note: (line: string) => void, onStart: (id: string) => void = () => {}): Promise<string> {
  if (turn.runner === "bench") return benchTurn(note, false, { ids: turn.ids, deadline, onStart });
  if (turn.runner === "leg") {
    // The legs run inside one call, which says nothing of each, so each starts with it.
    for (const id of turn.ids) onStart(id);
    return legsTurn(slug, note, false);
  }
  return fullTurn(turn.ids, deadline, note, onStart);
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
  now: () => Date.now(),
  debt: storeDebt,
  pay: payTurn,
  gh: async (args) => {
    const r = await proc(["gh", ...args], { timeoutMs: 60_000 });
    return { code: r.code, stdout: r.stdout, stderr: r.stderr };
  },
  disk: (keepRuns, say) => sweepDisk(keepRuns, say),
};

// ---- the nightly CI run ------------------------------------------------------------------------------

/** The nightly run as `ci.yml` makes it: a `workflow_dispatch` run on `main`, which the cron on
 * `landing` starts. A push to `main` is a `push` run, so the event tells the two apart. */
const NIGHTLY = ["--workflow", "ci.yml", "--branch", "main", "--event", "workflow_dispatch"];
/** How many of the newest nightly runs are read to find the newest completed one. */
const NIGHTLY_LOOKBACK = 5;
/** A job conclusion that is no failure. */
const PASSED = new Set(["success", "skipped", "neutral"]);

/** The newest completed nightly run: its URL, and one line per job it failed, none when it is green. */
export interface Nightly {
  url: string;
  failed: string[];
}

function ghError(what: string, r: GhRun): string {
  const why = `${r.stderr}\n${r.stdout}`.split("\n").map((l) => l.trim()).find((l) => l !== "") ?? `exit ${r.code}`;
  return `\`gh ${what}\` exited ${r.code}: ${why}`;
}

/**
 * The newest completed nightly run, read through `gh`, or null when `gh` lists none. A run still in
 * progress is passed over, so the newest completed one still decides. Returns why the run could not be
 * read instead, when `gh` cannot start, fails, or prints what does not parse.
 */
export async function readNightly(gh: SweepSeams["gh"]): Promise<Nightly | null | string> {
  try {
    const list = await gh(["run", "list", ...NIGHTLY, "--limit", String(NIGHTLY_LOOKBACK), "--json", "databaseId,status,conclusion,url"]);
    if (list.code !== 0) return ghError("run list", list);
    const runs = JSON.parse(list.stdout) as { databaseId: number; status: string; conclusion: string; url: string }[];
    const done = runs.find((r) => r.status === "completed");
    if (done === undefined) return null;
    if (done.conclusion === "success") return { url: done.url, failed: [] };
    const view = await gh(["run", "view", String(done.databaseId), "--json", "jobs"]);
    if (view.code !== 0) return ghError("run view", view);
    const jobs = (JSON.parse(view.stdout) as { jobs: { name: string; conclusion: string }[] }).jobs;
    const failed = jobs.filter((j) => !PASSED.has(j.conclusion)).map((j) => `job \`${j.name}\` ended \`${j.conclusion}\``);
    return { url: done.url, failed: failed.length > 0 ? failed : [`the run ended \`${done.conclusion}\``] };
  } catch (e) {
    return `\`gh\` failed: ${(e as Error).message.split("\n")[0]}`;
  }
}

/** Reads the nightly run and replaces the `NIGHTLY_LINE` lines with what it found. Returns the line it
 * said when the run is red or could not be read, and "" otherwise. It never throws. */
export async function nightlyTurn(s: Pick<SweepSeams, "root" | "gh">, voice: SweepVoice): Promise<string> {
  const got = await readNightly(s.gh);
  let lines: string[];
  let red = "";
  if (typeof got === "string") {
    lines = [`could not be read: ${got}`];
    red = `nightly CI: could not be read: ${got}`;
    voice.step(red);
  } else if (got === null) {
    lines = [];
    voice.step("nightly CI: no completed nightly run");
  } else {
    lines = got.failed.map((f) => `${f}: ${got.url}`);
    if (got.failed.length > 0) red = `nightly CI: ${got.failed.length} job(s) red, ${got.url}`;
    voice.step(red || `nightly CI: green, ${got.url}`);
  }
  try {
    writeOwnLines(s.root, NIGHTLY_LINE, lines);
  } catch (e) {
    console.error(`nv sweep: ${LAST_SWEEP} could not be written: ${(e as Error).message}`);
  }
  return red;
}

// ---- the file ----------------------------------------------------------------------------------------

/**
 * Replaces the lines of `.loop/last-sweep.md` that start with `prefix` with `lines`, each after `prefix`,
 * at the end of the file, and keeps every other line as it was. Removes the file when no line is left.
 */
export function writeOwnLines(root: string, prefix: string, lines: string[]): void {
  const path = join(root, LAST_SWEEP);
  const before = existsSync(path) ? readFileSync(path, "utf8").replace(/\r\n/g, "\n") : "";
  const kept = before.split("\n").filter((l) => !l.startsWith(prefix)).join("\n").trimEnd();
  const all = [...(kept === "" ? [] : [kept]), ...lines.map((l) => `${prefix}${l}`)];
  if (all.length === 0) {
    rmSync(path, { force: true });
    return;
  }
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, `${all.join("\n")}\n`);
}

// ---- the command -------------------------------------------------------------------------------------

/** How the command reports itself: every line to stdout, which looper keeps in the hook's log. */
const PRINTED: SweepVoice = {
  step: (l) => console.log(l),
  note: (l) => console.log(`   ${l}`),
};

/** What the command reads besides the seams: how it reports itself. */
export interface SweepCmdSeams extends SweepSeams {
  voice: SweepVoice;
}

/** Parses, pays the debt, reads the nightly run and writes their lines into `.loop/last-sweep.md`;
 * returns 0 whatever it found. */
export async function sweep(args: string[], over: Partial<SweepCmdSeams> = {}): Promise<number> {
  const s: SweepSeams = { ...REAL, ...over };
  const voice = over.voice ?? PRINTED;
  let slug: string | undefined;
  let keepRuns = KEEP_RUNS;
  let bad = "";
  for (let i = 0; i < args.length && bad === ""; i++) {
    const a = args[i]!;
    const value = args[i + 1];
    if (a === "--goal" && value !== undefined) slug = args[++i];
    else if (a === "--keep-runs" && value !== undefined && /^\d+$/.test(value)) keepRuns = Number(args[++i]);
    else bad = a === "--goal" || a === "--keep-runs" ? `${a} needs a value` : `unexpected argument ${a}`;
  }
  let red = false;
  if (bad !== "") {
    console.error(`${USAGE}\nnv sweep: error: ${bad}`);
    writeOwnLines(s.root, FLOOR_LINE, [`could not run: \`bun nv sweep\` was started with a bad argument: ${bad}`]);
    red = true;
  } else {
    try {
      const out = await payDebt({ ...(slug !== undefined && { slug }), keepRuns }, voice, s);
      voice.step(out.reds.length > 0 ? `RED: ${LAST_SWEEP} names what the debt run found` : "GREEN: the floor debt this run paid");
      red = out.reds.length > 0;
    } catch (e) {
      const why = (e as Error).message.split("\n")[0] ?? String(e);
      console.error(`nv sweep: ${why}`);
      writeOwnLines(s.root, FLOOR_LINE, [`could not run: the debt run stopped: ${why}`]);
      red = true;
    }
  }
  ev.start(NIGHTLY_ITEM);
  const nightly = await nightlyTurn(s, voice);
  ev.end(NIGHTLY_ITEM, nightly ? { result: "red", why: nightly } : {});
  ev.done(red || nightly ? "red" : "green");
  return 0;
}

export async function run(args: string[]): Promise<number> {
  if (args.includes("--help") || args.includes("-h")) {
    console.log(`${USAGE}\n\n${summary}`);
    return 0;
  }
  return sweep(args);
}
