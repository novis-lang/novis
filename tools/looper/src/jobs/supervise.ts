// The supervisor of one launch: `launch supervise <name>`, which `startLaunch` spawns (`launch.ts`).
// It ports the supervisor of `crates/looper/src/cmd/launch.rs`. It runs with no window, out of any job
// that allows it to leave, so the end of the session that started it leaves the launch running. It
// holds the launcher's named mutex for its whole life, so a second supervisor of the launcher refuses.
// It is the runner of a `launch` job, and:
//
// - takes the port from `--port`, then the launcher's last chosen one, then its `port`, and the args
//   from `--args`, then its last chosen ones (`ports.ts`);
// - refuses a port that a live launch, looper's own server or a listening process holds, naming the
//   owner and the next free port. It holds the port lock from the check until its `launch.json` names
//   the port, and remembers the port and args it was given;
// - names its job in `start.json` in the launcher's folder;
// - takes one heavy slot from looper's service when the launcher is `heavy`, as a heavy Bash call does;
// - runs `build`, when the launcher has one, and ends with its code when it is red;
// - runs `run` from a copy in the launcher's folder when its program lies under the project's
//   `target/`, unless `copy = false`;
// - frees the slot once the launch is ready: a line of its output matched `ready`, or its first window
//   showed. A line is matched without its terminal escapes, so a coloured line matches as it reads.
//   With no `ready` it keeps the slot until the launch ends. Past `READY_BYTES` of output with no
//   match, the launch counts as running, and the slot is freed;
// - on a stop, sends Ctrl+Break to the program's process group, and kills the job's tree once
//   `stop_grace` seconds have passed;
// - writes the job's exit when `run` ends, and kills what `run` left behind.
//
// It turns Windows' critical-error dialogs off for itself and every program it starts. A program that
// fails to start then exits with its code into the log, where it would otherwise wait behind a box.
//
// A launch hides its windows when the launcher says `window = "hide"`, or when it starts in a loop's
// session, which `LOOPER_LOOP` names. `[jobs] hide_windows = false` shows them all the same.
//
// # The bounds it keeps
//
// - The ready matcher reads at most `READY_BYTES` of output, and holds at most `LINE_MAX` bytes of a line
//   that has not ended.
// - The launcher's folder holds the supervisor's log, `start.json` and one copy.
// - The output moves one pipe read at a time, and the log keeps its bound of three parts (`log.ts`).

import { appendFileSync, copyFileSync, mkdirSync } from "node:fs";
import { basename, isAbsolute, join, resolve } from "node:path";
import { type Config, type Launch as Launcher, READY_WINDOW, ruleRegex } from "../goal/config.ts";
import { ctrlBreak, type Mutex, noErrorBoxes, openMutex, spawnTree, type Tree } from "../os/index.ts";
import { writeState } from "../store/state.ts";
import { type Launch, liveLaunches, launchDir, type Phase, type Refusal, type Request, SESSION_VAR, stopAsked, SUPERVISOR_LOG, TICK_MS, tell, writeLaunch } from "./launch.ts";
import { LogWriter } from "./log.ts";
import { type Chosen, clashText, lockPorts, readTaken, remember, remembered } from "./ports.ts";
import { program } from "./program.ts";
import { type Lease, looperHome, takeSlots } from "./slots.ts";
import { PUMP_MS } from "./spawn.ts";
import { EXIT, type JobExit, type JobRecord, jobName, KILLED_CODE, LOOP_VAR, PARENT_VAR, RECORD, type Store, storeOf, WINDOWS_VAR } from "./store.ts";

/** The most output the ready matcher reads. */
export const READY_BYTES = 1 << 20;
/** The longest unended line the ready matcher holds before it matches it as it is. */
export const LINE_MAX = 64 * 1024;
/** The most reads drained once a program's own process has exited. */
const DRAIN_READS = 16;

/** What a chunk of output told the ready matcher. */
type Heard = { port: number | null } | "unmatched" | null;

/** The ready matcher: it reads lines of the first `READY_BYTES` of output, and says once when one matched `re`. */
export class ReadyLines {
  private line: number[] = [];
  private read = 0;
  private done = false;

  constructor(private readonly re: RegExp) {}

  /** Feeds `chunk`, and says what it found: a match with the port its `port` group named, the bound passed, or nothing yet. */
  feed(chunk: Uint8Array): Heard {
    if (this.done) return null;
    this.read += chunk.length;
    for (const b of chunk) {
      if (b !== 0x0a && this.line.length < LINE_MAX) {
        this.line.push(b);
        continue;
      }
      const text = Buffer.from(plain(this.line)).toString("utf8").trimEnd();
      this.line = [];
      const m = this.re.exec(text);
      if (m) {
        this.done = true;
        const port = Number(m.groups?.port);
        return { port: Number.isInteger(port) && port > 0 && port < 65_536 ? port : null };
      }
    }
    if (this.read > READY_BYTES) {
      this.done = true;
      return "unmatched";
    }
    return null;
  }
}

/** `line` without its terminal escapes. A CSI escape, such as a colour, ends at its first byte from `@` to `~`, and any other escape is two bytes. */
export function plain(line: readonly number[]): number[] {
  const text: number[] = [];
  for (let i = 0; i < line.length; i++) {
    const b = line[i] as number;
    if (b !== 0x1b) {
      text.push(b);
      continue;
    }
    i++;
    if (line[i] !== 0x5b) continue;
    for (i++; i < line.length; i++) if ((line[i] as number) >= 0x40 && (line[i] as number) <= 0x7e) break;
  }
  return text;
}

/** `text` with `{port}` and `{args}` filled. */
function fill(text: string, port: number | null, words: readonly string[]): string {
  return text.replaceAll("{port}", port === null ? "" : String(port)).replaceAll("{args}", words.join(" "));
}

/** `argv` with its placeholders filled. An argument that is `{args}` alone becomes the words, and with `append` the words follow an argv that names no `{args}`. */
export function fillArgv(argv: readonly string[], port: number | null, words: readonly string[], append: boolean): string[] {
  let named = false;
  const filled: string[] = [];
  for (const a of argv) {
    if (a === "{args}") {
      named = true;
      filled.push(...words);
    } else {
      named ||= a.includes("{args}");
      filled.push(fill(a, port, words));
    }
  }
  if (append && !named) filled.push(...words);
  return filled;
}

/** Whether `path` lies under the `target/` of the project at `root`. */
function underTarget(root: string, path: string): boolean {
  const spell = (p: string) => p.replaceAll("/", "\\").toLowerCase();
  return spell(path).startsWith(`${spell(resolve(root, "target"))}\\`);
}

/** The name of the mutex of launcher `name` in the project at `root`. */
export function launcherMutex(root: string, name: string): string {
  const key = `${resolve(root).replaceAll("/", "\\").toLowerCase()}|${name}`;
  return `Local\\looper-kit-launch-${Bun.hash(key).toString(16).padStart(16, "0")}`;
}

/** The launcher `name` of `cfg`, or why there is none. */
export function launcher(cfg: Config, name: string): Launcher | string {
  const l = cfg.launch.find((x) => x.name === name);
  if (l) return l;
  const names = cfg.launch.map((x) => x.name);
  return names.length === 0
    ? `no launcher is named \`${name}\`: looper.toml declares no [[launch]]`
    : `no launcher is named \`${name}\`; the launchers are ${names.join(", ")}`;
}

/** The refusal of a start of launcher `name`, which runs already as job `id`. */
export function runsAlready(name: string, id: string): string {
  return `\`${name}\` runs already, as launch job ${id}; stop it first, or restart it`;
}

/** A launch the supervisor runs: its job, its `launch.json`, its log and the program it runs now. */
class Supervised {
  tree: Tree | null = null;
  lease: Lease | null = null;

  constructor(
    readonly store: Store,
    readonly id: string,
    readonly record: JobRecord,
    readonly launch: Launch,
    readonly log: LogWriter,
    readonly graceMs: number,
    readonly env: Record<string, string | undefined>,
    readonly cwd: string,
  ) {}

  get dir(): string {
    return this.store.dir(this.id);
  }

  phase(phase: Phase): void {
    this.launch.phase = phase;
    writeLaunch(this.dir, this.launch);
  }

  say(line: string): void {
    this.log.write(Buffer.from(`looper: ${line}\n`));
  }

  /** Runs `argv` to its end in the job, hidden when the launch is, and gives its exit code. Each chunk of output goes to the log and to `onOutput`, and `onTick` runs each tick until a stop is asked. */
  async run(argv: readonly string[], onOutput?: (chunk: Buffer) => void, onTick?: (tree: Tree) => void): Promise<number> {
    const run = program(argv, this.env);
    const tree = spawnTree(run.argv, { cwd: this.cwd, env: this.env, line: run.line, pipe: true, name: jobName(this.id), group: true, hide: this.launch.hidden });
    this.tree = tree;
    // A watcher that cannot start leaves the windows shown, and the program still runs.
    let watcher: ReturnType<Tree["watchWindows"]> | null = null;
    if (this.launch.hidden) {
      try {
        watcher = tree.watchWindows();
      } catch {}
    }
    this.record.root = tree.pid;
    writeState(join(this.dir, RECORD), this.record);
    const take = (): boolean => {
      const chunk = tree.read();
      if (chunk === null || chunk.length === 0) return false;
      this.log.write(chunk);
      onOutput?.(chunk);
      return true;
    };
    let stopping: number | null = null;
    let killed = false;
    let ticked = 0;
    try {
      for (;;) {
        if (take()) {
          await Bun.sleep(0);
          continue;
        }
        const code = tree.exitCode();
        if (code !== null) {
          for (let n = 0; n < DRAIN_READS && take(); n++) {}
          return code;
        }
        if (Date.now() - ticked >= TICK_MS) {
          ticked = Date.now();
          if (stopping === null && stopAsked(this.dir)) {
            this.phase("stopping");
            try {
              ctrlBreak(tree.pid);
            } catch (e) {
              this.say((e as Error).message);
            }
            stopping = Date.now();
          } else if (stopping !== null && !killed && Date.now() - stopping >= this.graceMs) {
            killed = true;
            this.say(`killed after its grace of ${this.graceMs / 1000} s`);
            tree.kill(KILLED_CODE);
          } else if (stopping === null) onTick?.(tree);
        }
        await Bun.sleep(PUMP_MS);
      }
    } finally {
      watcher?.stop();
      tree.close();
      this.tree = null;
    }
  }

  /** Ends the job with `code`, saying who stopped it when someone did. Gives `code`. */
  async finish(code: number): Promise<number> {
    const by = stopAsked(this.dir)?.by;
    if (by !== undefined) this.say(`launch ${this.launch.name} was stopped by ${by}`);
    writeState(join(this.dir, EXIT), { code, ended: Date.now() } satisfies JobExit);
    this.log.close();
    await this.lease?.release();
    return code;
  }
}

/** Takes one heavy slot for the launch, unless a stop is asked meanwhile. A service that cannot give one is said in the log, and the launch runs without. */
async function takeSlot(s: Supervised): Promise<Lease | null> {
  const asking = takeSlots(looperHome(), "one", { kind: "launch", project: s.record.project, command: s.record.command }, (line) =>
    s.say(line.replace(/^looper: /, "")),
  );
  let stopped = false;
  const watch = (async () => {
    while (!stopped && !stopAsked(s.dir)) await Bun.sleep(TICK_MS);
  })();
  const asked = await Promise.race([asking, watch.then(() => null)]);
  stopped = true;
  if (asked === null) return null;
  if ("lease" in asked) return asked.lease;
  if ("failed" in asked) s.say(`no heavy slot: ${asked.failed}`);
  return null;
}

/** Runs launcher `req.name` of the project at `root` as its supervisor, as the head comment says. Gives the launch's exit code. */
export async function supervise(root: string, cfg: Config, req: Request): Promise<number> {
  noErrorBoxes();
  const dir = launchDir(root, req.name);
  mkdirSync(dir, { recursive: true });
  const refuse = (refusal: Refusal): number => {
    appendFileSync(join(dir, SUPERVISOR_LOG), `error: ${refusal.why}\n`);
    tell(dir, { refused: refusal });
    return 1;
  };
  const l = launcher(cfg, req.name);
  if (typeof l === "string") return refuse({ why: l });
  const store = storeOf(root);
  const mutex: Mutex = openMutex(launcherMutex(root, l.name));
  if (!mutex.tryTake()) {
    const live = liveLaunches(store).find((x) => x.launch.name === l.name);
    mutex.close();
    return refuse({ why: live ? runsAlready(l.name, live.id) : `\`${l.name}\` runs already, and is starting` });
  }
  try {
    return await supervised(root, cfg, l, req, store, dir, refuse);
  } finally {
    mutex.close();
  }
}

async function supervised(root: string, cfg: Config, l: Launcher, req: Request, store: Store, dir: string, refuse: (r: Refusal) => number): Promise<number> {
  let ready: RegExp | "window" | null = null;
  if (l.ready === READY_WINDOW) ready = "window";
  else if (l.ready !== undefined) {
    try {
      ready = ruleRegex(l.ready);
    } catch (e) {
      return refuse({ why: `the ready pattern of \`${l.name}\` does not compile: ${(e as Error).message}` });
    }
  }
  const last = remembered(root, l.name) ?? {};
  const args = l.args ? (req.args ?? last.args) : undefined;
  const chosen: Chosen = {};
  const asked = req.port ?? last.port;
  if (asked !== undefined) chosen.port = asked;
  if (args !== undefined) chosen.args = args;
  const port = chosen.port ?? l.port ?? null;
  let lock: Mutex;
  try {
    lock = await lockPorts();
  } catch (e) {
    return refuse({ why: `cannot take the port lock: ${(e as Error).message}` });
  }
  try {
    const clash = port === null ? null : readTaken(store, root).clash(port);
    if (clash !== null) return refuse({ why: clashText(clash), port: clash });
    const words = (chosen.args ?? "").split(/\s+/).filter((w) => w !== "");
    const run = fillArgv(l.run, port, words, l.args);
    const build = fillArgv(l.build, port, words, false);
    const own: Record<string, string> = {};
    for (const [k, v] of Object.entries({ ...cfg.checks.env, ...l.env })) own[k] = fill(v, port, words);
    const cwd = l.cwd === undefined ? root : resolve(root, l.cwd);
    const loop = process.env[LOOP_VAR] || null;
    const hidden = cfg.jobs.hide_windows && (l.window === "hide" || loop !== null);
    const env: Record<string, string | undefined> = { ...process.env, ...own };
    if (hidden) delete env[WINDOWS_VAR];
    else env[WINDOWS_VAR] = "show";

    const started = Date.now();
    const id = store.create(started);
    env[PARENT_VAR] = id;
    const record: JobRecord = {
      kind: "launch",
      project: root,
      cwd,
      command: run.join(" "),
      session: process.env[SESSION_VAR] || null,
      agent: null,
      tool_use: null,
      runner: process.pid,
      root: 0,
      started,
      ...(loop !== null ? { loop } : {}),
    };
    writeState(join(store.dir(id), RECORD), record);
    const launch: Launch = { name: l.name, project: cfg.project.name, root, port, phase: build.length > 0 ? "building" : "starting", ready: null, hidden, loop };
    const s = new Supervised(store, id, record, launch, new LogWriter(store.dir(id)), l.stop_grace * 1000, env, cwd);
    s.phase(launch.phase);
    try {
      remember(root, cfg.launch.map((x) => x.name), l.name, chosen);
    } catch (e) {
      s.say(`cannot remember the port and args: ${(e as Error).message}`);
    }
    lock.close();
    tell(dir, { job: id });
    return await lifetime(s, l, root, dir, run, build, ready);
  } finally {
    lock.close();
  }
}

/** The launch's life once its job exists: the slot, the build, the copy, and the run until it ends. */
async function lifetime(s: Supervised, l: Launcher, root: string, dir: string, run: string[], build: string[], ready: RegExp | "window" | null): Promise<number> {
  try {
    if (l.heavy) s.lease = await takeSlot(s);
    if (stopAsked(s.dir)) return await s.finish(KILLED_CODE);
    await s.lease?.hold(s.id);
    if (build.length > 0) {
      s.say(`build: ${build.join(" ")}`);
      let code: number;
      try {
        code = await s.run(build);
      } catch (e) {
        s.say((e as Error).message);
        code = 1;
      }
      if (code !== 0 || stopAsked(s.dir)) {
        s.say(`the build exited with ${code}, so \`run\` never starts`);
        return await s.finish(code);
      }
      s.phase("starting");
    }
    if (l.copy) copyProgram(s, run, root, dir);
    s.record.command = run.join(" ");
    const matcher = ready instanceof RegExp ? new ReadyLines(ready) : null;
    let heard: Heard = null;
    if (ready === null) s.phase("running");
    const up = async (port: number | null): Promise<void> => {
      // The slot is freed first, so a reader that sees the phase finds the slot free.
      await s.lease?.release();
      s.lease = null;
      if (port !== null) s.launch.port = port;
      s.launch.ready = Date.now();
      s.phase("ready");
    };
    let pending: Promise<void> = Promise.resolve();
    const onOutput = (chunk: Buffer): void => {
      if (matcher && heard === null) heard = matcher.feed(chunk);
    };
    const onTick = (tree: Tree): void => {
      if (s.launch.phase !== "starting") return;
      if (ready === "window" && tree.showsWindow()) pending = up(null);
      else if (heard === "unmatched") {
        s.say(`no line matched \`ready\` in the first ${READY_BYTES} bytes, so the launch counts as running`);
        pending = s.lease?.release().then(() => s.phase("running")) ?? Promise.resolve(s.phase("running"));
        s.lease = null;
      } else if (heard !== null) pending = up(heard.port);
    };
    let code: number;
    try {
      code = await s.run(run, onOutput, onTick);
    } catch (e) {
      s.say((e as Error).message);
      code = 1;
    }
    await pending;
    return await s.finish(code);
  } catch (e) {
    s.say(`the supervisor failed: ${(e as Error).message}`);
    return await s.finish(1);
  }
}

/** Points `run` at a copy of its program in the launcher's folder `dir`, when the program lies under the project's `target/`. A copy that fails is said in the log, and the original runs. */
function copyProgram(s: Supervised, run: string[], root: string, dir: string): void {
  const first = run[0];
  if (first === undefined || !/[/\\]/.test(first)) return;
  const program = isAbsolute(first) ? first : resolve(s.cwd, first);
  if (!underTarget(root, program)) return;
  const copy = join(dir, basename(program));
  try {
    copyFileSync(program, copy);
    run[0] = copy;
  } catch (e) {
    s.say(`cannot copy ${program}, so the original runs: ${(e as Error).message}`);
  }
}
