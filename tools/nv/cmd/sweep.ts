// `bun nv sweep`: what looper runs after every session as `looper.toml`'s `after_session` hook.
//
//     bun nv sweep [--keep-runs <n>]
//
// looper runs the live goal's acceptance checks itself after every session (`[loop] sweep = "every"`), so
// this command runs none of them, and it pays no floor debt: `bun nv debt` does that, once per goal, as
// a floor check (`cmd/debt.ts`). Each run does two things, in this order.
//
// **It sweeps the disk** (`sweepDisk`), the whole of `nv disk --clean`, while nothing else is building.
// That includes `target/deps` and `target/incremental` every time. What it deletes there is already
// dead or past the newest caches per crate, so the next session's build stays as warm, and waiting for
// a low disk would let `target/` fill most of the drive first.
//
// **It reads the newest nightly CI run back** (`readNightly`): the newest completed `workflow_dispatch`
// run of `ci.yml` on `main`, which the cron on `landing` dispatches. It asks `gh`, and it only reads.
//
// What it found goes into `.loop/last-sweep.md`, the file looper writes from its own sweep just before
// this hook runs, and which its orient prints under "Fix these first". This command owns the lines that
// start with `NIGHTLY_LINE`, each job the nightly run failed, and with `HOOK_LINE`, why the hook could
// not run. Each run replaces both sets and leaves looper's lines alone, and it removes the file when no
// line is left. A nightly run `gh` cannot read is one `NIGHTLY_LINE` that says so.
//
// Under looper, which names an event file in `LOOPER_EVENTS`, the run writes looper's progress events
// (`lib/events.ts`), and outside it writes none: one item each for the disk sweep and the nightly read,
// `planned`, and `done`.
//
// It always exits 0, because looper pauses the loop on a hook that fails: the file is the verdict.
// Under a side loop in the root (`LOOPER_LOOP_KIND=root-side`) it does nothing, because the main loop's
// run of it sweeps that tree and writes that file.

import { existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { ev } from "../lib/events.ts";
import { ROOT } from "../lib/paths.ts";
import { run as proc } from "../lib/proc.ts";
import { clean, freedLines, freeGb, human, KEEP_RUNS, total } from "./disk.ts";

/** The file looper writes with its sweep's reds, repo-relative, to which this command adds its own. */
export const LAST_SWEEP = ".loop/last-sweep.md";
/** How the line saying why the hook could not run starts in `LAST_SWEEP`. */
export const HOOK_LINE = "- after-session hook: ";
/** How each line about the nightly CI run starts in `LAST_SWEEP`. Every line with neither prefix is looper's. */
export const NIGHTLY_LINE = "- nightly CI: ";

/** The progress items of the run's two parts. */
export const DISK_ITEM = "sweep:disk";
export const NIGHTLY_ITEM = "sweep:nightly";

export const summary = `looper's hook after each session: sweeps the disk, reads the newest nightly CI run, and writes what is red into ${LAST_SWEEP}: nv sweep [--keep-runs <n>]`;

const USAGE = "usage: bun nv sweep [--keep-runs <n>]";

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
  /** `gh` with `args`. It throws when `gh` cannot start. */
  gh(args: string[]): Promise<GhRun>;
  /** The disk sweep's one-line summary. */
  disk(keepRuns: number, say: (line: string) => void): Promise<string>;
}

// ---- the disk ----------------------------------------------------------------------------------------

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

/** What the command reads besides the seams: how it reports itself, and looper's `LOOPER_LOOP_KIND`. */
export interface SweepCmdSeams extends SweepSeams {
  voice: SweepVoice;
  loopKind: string;
}

/** Parses, sweeps the disk, reads the nightly run and writes their lines into `.loop/last-sweep.md`;
 * returns 0 whatever it found. */
export async function sweep(args: string[], over: Partial<SweepCmdSeams> = {}): Promise<number> {
  const s: SweepSeams = { ...REAL, ...over };
  const voice = over.voice ?? PRINTED;
  // A side loop in the root shares the tree with the main loop, whose own hook
  // sweeps the disk and owns `.loop/last-sweep.md`.
  if ((over.loopKind ?? process.env.LOOPER_LOOP_KIND) === "root-side") {
    voice.step("nv sweep: a side loop in the root leaves the sweep to the main loop");
    return 0;
  }
  let keepRuns = KEEP_RUNS;
  let bad = "";
  for (let i = 0; i < args.length && bad === ""; i++) {
    const a = args[i]!;
    const value = args[i + 1];
    if (a === "--keep-runs" && value !== undefined && /^\d+$/.test(value)) keepRuns = Number(args[++i]);
    else bad = a === "--keep-runs" ? `${a} needs a value` : `unexpected argument ${a}`;
  }
  ev.add(DISK_ITEM, { name: "disk sweep", kind: "step" });
  ev.add(NIGHTLY_ITEM, { name: "nightly CI read", kind: "step" });
  ev.planned();
  let red = false;
  if (bad !== "") {
    console.error(`${USAGE}\nnv sweep: error: ${bad}`);
    writeOwnLines(s.root, HOOK_LINE, [`could not run: \`bun nv sweep\` was started with a bad argument: ${bad}`]);
    ev.end(DISK_ITEM, { result: "skipped", why: bad });
    red = true;
  } else {
    writeOwnLines(s.root, HOOK_LINE, []);
    ev.start(DISK_ITEM);
    const disk = await s.disk(keepRuns, voice.note);
    voice.step(disk);
    const failed = disk.startsWith("disk: the sweep failed");
    ev.end(DISK_ITEM, failed ? { result: "red", why: disk } : {});
    red = failed;
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
