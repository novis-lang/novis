// The one verify run of a tree, which every `verify` of that tree waits on. It ports
// `crates/looper/src/cmd/verify/shared.rs`, with the runs in the project's own data folder.
//
// - A tree is a checkout root: the main checkout or a worktree. Its runs live in its own
//   `.looper/verify/`, so verifies of two trees run apart.
// - A request holds the tree's named mutex while it reads `current`, the id of the tree's newest run,
//   and acts on it:
//   - with no run going, it starts one: it makes the run's folder, spawns its supervisor, and names the
//     run in `current`;
//   - with one going, it joins it, and prints `joined verify job <id>, started by <session>`. It leaves
//     a request in the run's `requests/`, so the supervisor selects again (`SharedWatch`);
//   - with one going whose supervisor died, it takes the run over: it spawns a new supervisor on the
//     same folder, which runs the verify again. Each unit the dead one recorded green is kept, so the
//     new one runs only the rest. A run whose supervisor died `MAX_TAKEOVERS` times ends red instead;
//   - with one going that already has `MAX_WAITERS` waiters, or whose `all` differs from its own, it
//     waits for that run to end and asks again.
// - A run's folder holds `run.json`, the `RunFile`; `job`, its supervisor's job id; `progress.log`,
//   every line the run prints; `waiters/<pid>.json`, one per waiter; `requests/`, the requests the
//   supervisor has not read yet; `supervisor.log`, what stopped its supervisor; and `result.json`, the
//   `Ended`, written once when the run ends.
// - The supervisor is `verify --supervise <run folder>`, spawned detached, out of the caller's job when
//   that job allows it, so the run outlives the session that started it. It is the runner of a
//   `verify` job, and each step's job runs below it. Each unit's job kills its tree when its runner
//   dies, so the supervisor's death ends every unit it runs.
// - Each waiter prints `progress.log` from its start, then gives the run's end. It first waits for the
//   supervisor to exit, so a finished verify leaves no process in the tree.
// - `statusJob` names the job of the tree's newest run without joining it, for `verify --status`. `awaitGoing` waits for
//   that run to end without joining it, for `wrap`.
// - A run with no live waiter for `ABANDONED_MS` is cancelled: its supervisor says so and exits, which
//   kills its unit jobs.
//
// # The bounds it keeps
//
// - A run has at most `MAX_WAITERS` waiters, and its `waiters/` holds one file per live waiter: the
//   supervisor deletes a dead waiter's file each time it counts them.
// - `requests/` holds only the requests that came since the supervisor last read it. It reads and
//   deletes them every `POLL_MS`, and keeps a count and the time of the newest.
// - The supervisor holds one unit in flight at a time, with the digests of the files it rests on.
// - A tree keeps its current run and each run that ended within `KEEP_MS`. A request deletes the rest.
// - `progress.log` holds one line per step and per red unit, and a red step's last `TAIL_LINES` log
//   lines. It adds one line per restart, and each restart takes a request or one of at most
//   `FINAL_SELECTIONS` final selections.
// - A waiter reads `progress.log` from where it left off, `CHUNK` bytes at a time, and holds at most
//   `MAX_LINE` bytes of a line that has not ended.

import { appendFileSync, closeSync, existsSync, mkdirSync, openSync, readdirSync, readSync, rmSync, statSync, unlinkSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { asker } from "../cli/claims.ts";
import { loadConfig } from "../goal/config.ts";
import { SESSION_VAR } from "../jobs/launch.ts";
import { EVENTS, EXIT, idMs, type JobExit, type JobRecord, PARENT_VAR, RECORD, storeOf } from "../jobs/store.ts";
import { type Mutex, openMutex, sameProcess, spawnDetached, started } from "../os/index.ts";
import { readState, writeState } from "../store/state.ts";
import { Items } from "./items.ts";
import { type Digest, Tree } from "./select.ts";
import { type Out, runSteps, type Verified, type Watch } from "./verify.ts";

/** The most waiters one run has. A request past them waits for the run to end, then starts its own. */
export const MAX_WAITERS = 64;
/** How long a run goes on with no live waiter before it is cancelled. */
export const ABANDONED_MS = 30_000;
/** How many times a run's supervisor may die and be taken over before the run ends red. */
export const MAX_TAKEOVERS = 3;
/** How long an ended run's folder is kept: a day. */
export const KEEP_MS = 24 * 60 * 60 * 1000;
/** The piece of `progress.log` a waiter reads at a time. */
export const CHUNK = 64 * 1024;
/** The longest unended line a waiter holds before it prints it as it is. */
export const MAX_LINE = 1 << 20;

/** How often a waiter reads the run's progress, and the supervisor its requests. */
const POLL_MS = 100;
/** How often a waiter checks that the supervisor lives, and the supervisor counts its waiters. */
const LIVENESS_MS = 1_000;
/** How long a joined waiter waits for a new supervisor to name its job, and a waiter of an ended run for its supervisor to exit. */
const SETTLE_MS = 10_000;

const CURRENT = "current";
const RUN = "run.json";
const JOB = "job";
const PROGRESS = "progress.log";
const RESULT = "result.json";
const WAITERS = "waiters";
const REQUESTS = "requests";
export const SUPERVISOR_LOG = "supervisor.log";

/** The kit's entry, which the supervisor runs. */
const MAIN = join(import.meta.dir, "..", "..", "main.ts");

/** A process, by its pid and its start time, so a pid the OS gave out again never stands for it. */
interface Pid {
  pid: number;
  started: number;
}

/** A run's `run.json`: what it runs, who started it, and its supervisor now. */
interface RunFile {
  all: boolean;
  /** The session that started it, or the CLI's asker outside a session. */
  session: string;
  supervisor: Pid;
  /** How many times a waiter took it over. */
  takeovers: number;
}

/** A run's `result.json`: its exit code, and its report or why it has none. */
export interface Ended {
  code: number;
  report: Verified | null;
  error: string | null;
}

/** The folder of the runs of the tree at `root`. */
export function verifyDir(root: string): string {
  return join(root, ".looper", "verify");
}

function self(): Pid {
  return { pid: process.pid, started: started(process.pid) ?? 0 };
}

function alive(p: Pid): boolean {
  return p.started !== 0 && sameProcess(p.pid, p.started);
}

function read<T>(path: string): T | undefined {
  try {
    return readState<T>(path);
  } catch {
    return undefined;
  }
}

/** Whether the run in `dir` has a supervisor that lives. */
function supervised(dir: string): boolean {
  const file = read<RunFile>(join(dir, RUN));
  return file !== undefined && alive(file.supervisor);
}

/** The lines of the run in `dir`, appended to its `progress.log`. */
export function progressOut(dir: string): Out {
  const path = join(dir, PROGRESS);
  return { line: (text) => appendFileSync(path, `${text}\n`) };
}

/**
 * Asks for a verify of the tree at `root`, waits for the tree's run, prints its progress through `line`,
 * and gives how it ended.
 */
export async function request(root: string, all: boolean, line: (text: string) => void): Promise<Ended> {
  root = resolve(root);
  mkdirSync(verifyDir(root), { recursive: true });
  const me = self();
  let at: { dir: string; read: number } | null = null;
  for (;;) {
    const taken = await take(root, all, me);
    if ("busy" in taken) {
      await awaitEnd(taken.busy, line);
      continue;
    }
    const { dir, by } = taken;
    try {
      const pos: { read: number } = { read: at !== null && at.dir === dir ? at.read : 0 };
      if ((at === null || at.dir !== dir) && by !== null) line(`joined verify job ${await jobId(dir, true)}, started by ${by}`);
      const ended = await wait(dir, pos, line);
      at = { dir, read: pos.read };
      if (ended === null) continue;
      const file = read<RunFile>(join(dir, RUN));
      const t0 = Date.now();
      while (file !== undefined && alive(file.supervisor) && Date.now() - t0 < SETTLE_MS) await Bun.sleep(POLL_MS);
      return ended;
    } finally {
      rmSync(join(dir, WAITERS, `${me.pid}.json`), { force: true });
    }
  }
}

/** The id of the job of the tree at `root`'s newest run, or null when it has none. */
export function statusJob(root: string): string | null {
  const tree = verifyDir(resolve(root));
  const current = read<string>(join(tree, CURRENT));
  return current === undefined ? null : (read<string>(join(tree, current, JOB)) ?? null);
}

/**
 * Waits for the tree at `root`'s newest run to end or lose its supervisor, without joining it, and says
 * `waiting for verify job <id>` through `line` first. With no run going, it gives at once.
 */
export async function awaitGoing(root: string, line: (text: string) => void): Promise<void> {
  const tree = verifyDir(resolve(root));
  const current = read<string>(join(tree, CURRENT));
  if (current === undefined) return;
  const dir = join(tree, current);
  if (existsSync(join(dir, RESULT)) || !supervised(dir)) return;
  line(`waiting for verify job ${await jobId(dir, true)}`);
  while (!existsSync(join(dir, RESULT)) && supervised(dir)) await Bun.sleep(POLL_MS);
}

/** What a request found for its tree: a run to wait on, with who started it when the request joined it, or a run it may not join. */
type Taken = { dir: string; by: string | null } | { busy: string };

/** Takes the tree's mutex, waiting while another request holds it. */
async function lock(root: string): Promise<Mutex> {
  const key = Bun.hash(root.replaceAll("/", "\\").replace(/\\+$/, "").toLowerCase()).toString(16);
  const m = openMutex(`Local\\looper-kit-verify-${key}`);
  while (!m.tryTake()) await Bun.sleep(POLL_MS / 10);
  return m;
}

/** Under the tree's mutex: starts the tree's run, joins it, or takes it over, and registers `me` as a waiter of it. */
async function take(root: string, all: boolean, me: Pid): Promise<Taken> {
  const m = await lock(root);
  try {
    const tree = verifyDir(root);
    const current = read<string>(join(tree, CURRENT));
    sweep(tree, current);
    const dir = current === undefined ? null : join(tree, current);
    const file = dir === null ? undefined : read<RunFile>(join(dir, RUN));
    if (dir === null || file === undefined || existsSync(join(dir, RESULT))) return start(root, tree, all, me);
    if (!alive(file.supervisor)) {
      const job = read<string>(join(dir, JOB)) ?? "(unnamed)";
      if (file.takeovers >= MAX_TAKEOVERS) {
        const error = `the supervisor of verify job ${job} died ${MAX_TAKEOVERS} times; what stopped it is in ${join(dir, SUPERVISOR_LOG)}`;
        writeState(join(dir, RESULT), { code: 1, report: null, error } satisfies Ended);
      } else {
        file.takeovers++;
        file.supervisor = spawn(root, dir, file.all);
        writeState(join(dir, RUN), file);
        progressOut(dir).line(`verify: took over verify job ${job}, whose supervisor died`);
      }
    } else if (file.all !== all || waiters(dir) >= MAX_WAITERS) {
      return { busy: dir };
    } else {
      ask(dir);
    }
    register(dir, me);
    return { dir, by: file.session };
  } finally {
    m.close();
  }
}

/** Starts a new run of the tree and names it in `current`. The caller holds the tree's mutex. */
function start(root: string, tree: string, all: boolean, me: Pid): Taken {
  const id = `${Date.now()}-${me.pid}`;
  const dir = join(tree, id);
  mkdirSync(join(dir, WAITERS), { recursive: true });
  register(dir, me);
  const file: RunFile = { all, session: session(), supervisor: spawn(root, dir, all), takeovers: 0 };
  writeState(join(dir, RUN), file);
  writeState(join(tree, CURRENT), id);
  return { dir, by: null };
}

/** Spawns the supervisor of the run in `dir`, out of this process's job when the job allows it. */
function spawn(root: string, dir: string, all: boolean): Pid {
  writeFileSync(join(dir, SUPERVISOR_LOG), "");
  const argv = [process.execPath, MAIN, "verify", "--supervise", dir];
  if (all) argv.push("--all");
  const child = spawnDetached(argv, { cwd: root });
  return { pid: child.pid, started: child.started };
}

/** Waits for the run in `dir`, which this request may not join, to end or lose its supervisor. */
async function awaitEnd(dir: string, line: (text: string) => void): Promise<void> {
  line(`verify: waiting for verify job ${await jobId(dir, true)} to end, then starting a run of its own`);
  while (!existsSync(join(dir, RESULT)) && supervised(dir)) await Bun.sleep(LIVENESS_MS);
}

/** Prints the run's progress from `pos.read` on until it ends, and gives how it ended, or null when its supervisor died first. */
async function wait(dir: string, pos: { read: number }, line: (text: string) => void): Promise<Ended | null> {
  const progress = join(dir, PROGRESS);
  let checked = Date.now();
  for (;;) {
    const ended = read<Ended>(join(dir, RESULT));
    printNew(progress, pos, line);
    if (ended !== undefined) return ended;
    if (Date.now() - checked >= LIVENESS_MS) {
      checked = Date.now();
      if (!supervised(dir) && !existsSync(join(dir, RESULT))) return null;
    }
    await Bun.sleep(POLL_MS);
  }
}

/** Prints each whole line of `path` past `pos.read`, and moves `pos.read` past them. A line of `MAX_LINE` bytes is printed as it is. */
export function printNew(path: string, pos: { read: number }, line: (text: string) => void): void {
  let fd: number;
  try {
    fd = openSync(path, "r");
  } catch {
    return;
  }
  try {
    const buf = Buffer.alloc(CHUNK);
    let at = pos.read;
    let held: Buffer[] = [];
    let heldBytes = 0;
    for (;;) {
      const n = readSync(fd, buf, 0, CHUNK, at);
      if (n === 0) break;
      at += n;
      const chunk = buf.subarray(0, n);
      let from = 0;
      while (from < n) {
        const nl = chunk.indexOf(10, from);
        let to = nl < 0 ? n : nl + 1;
        to = Math.min(to, from + MAX_LINE - heldBytes);
        held.push(Buffer.from(chunk.subarray(from, to)));
        heldBytes += to - from;
        from = to;
        if (chunk[to - 1] === 10 || heldBytes >= MAX_LINE) {
          pos.read += heldBytes;
          line(Buffer.concat(held).toString("utf8").replace(/[\r\n]+$/, ""));
          held = [];
          heldBytes = 0;
        }
      }
    }
  } finally {
    closeSync(fd);
  }
}

/** The id of the job of the run in `dir`'s supervisor. With `settle` it waits a while for a supervisor that has just started to name it. */
async function jobId(dir: string, settle: boolean): Promise<string> {
  const t0 = Date.now();
  for (;;) {
    const id = read<string>(join(dir, JOB));
    if (id !== undefined) return id;
    if (!settle || Date.now() - t0 >= SETTLE_MS) return "(unnamed)";
    await Bun.sleep(POLL_MS);
  }
}

/** The session that asks: Claude Code's session id, or the CLI's asker outside a session. */
function session(): string {
  return process.env[SESSION_VAR] || asker();
}

/** Writes `me` into the run's waiters. */
function register(dir: string, me: Pid): void {
  writeState(join(dir, WAITERS, `${me.pid}.json`), me);
}

/** Tells the run's supervisor that a request came, so it selects again. */
function ask(dir: string): void {
  mkdirSync(join(dir, REQUESTS), { recursive: true });
  writeState(join(dir, REQUESTS, `${Date.now()}-${process.pid}.json`), session());
}

/** How many of the run's waiters live. It deletes the file of each one that does not. */
function waiters(dir: string): number {
  let live = 0;
  for (const name of list(join(dir, WAITERS))) {
    if (!name.endsWith(".json")) continue;
    const path = join(dir, WAITERS, name);
    const p = read<Pid>(path);
    if (p !== undefined && alive(p)) live++;
    else rmSync(path, { force: true });
  }
  return live;
}

function list(dir: string): string[] {
  try {
    return readdirSync(dir);
  } catch {
    return [];
  }
}

/** Deletes each run of the tree but `current` that ended, or started, more than `KEEP_MS` ago. */
function sweep(tree: string, current: string | undefined): void {
  const now = Date.now();
  for (const name of list(tree)) {
    if (name === current || idMs(name) === null) continue;
    let at = idMs(name) as number;
    try {
      at = statSync(join(tree, name, RESULT)).mtimeMs;
    } catch {}
    if (now - at > KEEP_MS) rmSync(join(tree, name), { recursive: true, force: true });
  }
}

/**
 * Runs the verify of the tree at `root` as the supervisor of the run in `runDir`: the runner of a
 * `verify` job of its own, which is the parent of each step's job, writing its lines to the run's
 * progress and its end to the run's result. Gives the run's exit code.
 */
export async function supervise(root: string, runDir: string, all: boolean): Promise<number> {
  const out = progressOut(runDir);
  const store = storeOf(root);
  const t0 = Date.now();
  const id = store.create(t0);
  const record: JobRecord = {
    kind: "verify",
    project: root,
    cwd: root,
    command: `looper verify${all ? " --all" : ""}: the run of ${root}`,
    session: process.env[SESSION_VAR] || null,
    agent: null,
    tool_use: null,
    runner: process.pid,
    root: process.pid,
    started: t0,
  };
  writeState(join(store.dir(id), RECORD), record);
  writeState(join(runDir, JOB), id);
  process.env[PARENT_VAR] = id;
  const ending = (ended: Ended): void => {
    writeState(join(store.dir(id), EXIT), { code: ended.code, ended: Date.now() } satisfies JobExit);
    writeState(join(runDir, RESULT), ended);
  };
  const watch = new SharedWatch(join(runDir, REQUESTS), root);
  const abandoned = watchWaiters(runDir, out, ending);
  let ended: Ended;
  try {
    const verified = await runSteps({ root, cfg: loadConfig(root), all, out, watch, items: new Items(join(store.dir(id), EVENTS)) });
    ended = { code: verified.green ? 0 : 1, report: verified, error: null };
  } catch (e) {
    ended = { code: 1, report: null, error: (e as Error).message };
  } finally {
    watch.stop();
    clearInterval(abandoned);
  }
  ending(ended);
  return ended.code;
}

/** Cancels the run once it has had no live waiter for `ABANDONED_MS`: it says so, writes the run's end through `ending`, and exits, which kills every unit job it runs. */
function watchWaiters(runDir: string, out: Out, ending: (ended: Ended) => void): ReturnType<typeof setInterval> {
  let quiet: number | null = null;
  return setInterval(() => {
    if (waiters(runDir) > 0) {
      quiet = null;
      return;
    }
    quiet ??= Date.now();
    if (Date.now() - quiet < ABANDONED_MS) return;
    out.line(`verify: cancelled, as no session waited for it for ${ABANDONED_MS / 1000} s`);
    ending({ code: 1, report: null, error: "the verify run was cancelled: no session waited for it" });
    process.exit(1);
  }, LIVENESS_MS);
}

/** The unit in flight: its files with the digests its run started from, and the files a request found changed, once one did. */
interface Flight {
  files: [string, Digest][];
  changed: string[] | null;
}

/**
 * What the supervisor's steps learn of the requests that join the run. It reads the run's requests
 * every `POLL_MS` and deletes each it reads. A request that finds the files of the unit in flight
 * changed marks the unit killed.
 */
class SharedWatch implements Watch {
  private came = 0;
  private last = 0;
  private flight: Flight | null = null;
  private readonly timer: ReturnType<typeof setInterval>;

  constructor(
    private readonly requestsDir: string,
    private readonly root: string,
  ) {
    this.timer = setInterval(() => this.read(), POLL_MS);
  }

  stop(): void {
    clearInterval(this.timer);
  }

  private read(): void {
    let came = 0;
    for (const name of list(this.requestsDir)) {
      if (!name.endsWith(".json")) continue;
      try {
        unlinkSync(join(this.requestsDir, name));
        came++;
      } catch {}
    }
    if (came === 0) return;
    this.came += came;
    this.last = Date.now();
    const f = this.flight;
    if (f === null || f.changed !== null) return;
    let changed: string[] = [];
    try {
      changed = Tree.list(this.root).changed(f.files);
    } catch {}
    if (changed.length > 0) f.changed = changed;
  }

  requests(): number {
    return this.came;
  }

  fly(files: [string, Digest][]): void {
    this.flight = { files, changed: null };
  }

  killed(): boolean {
    return this.flight?.changed != null;
  }

  land(): string[] {
    const f = this.flight;
    this.flight = null;
    return f?.changed ?? [];
  }

  async quiet(ms: number): Promise<void> {
    let since = Date.now();
    for (;;) {
      if (this.last > since) since = this.last;
      if (Date.now() - since >= ms) return;
      await Bun.sleep(POLL_MS);
    }
  }
}
