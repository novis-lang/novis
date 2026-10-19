// The landing: what a side loop's worker does once its branch has caught up with the main branch (the
// driver's `land` outcome). It lands the branch on the main branch with no person, pausing the project's
// loops that work in the project root around the merge (decisions 0016 and 0021). It ports
// `crates/looper/src/cmd/worker/landing.rs`, with a state file where looper has a store row.
//
// # The steps
//
// The worker names the state `landing` throughout. `.looper/landing.json` in the project root is the
// lock, and its `step` names how far the landing got. A worker restarted mid-landing goes on from that
// step, and each step is safe to run again.
//
// 1. **Lock.** It takes the file under the project's lane mutex (`locked` in `lanes.ts`). While another
//    side loop holds it, it waits, and checks again every `LOCK_RECHECK_MS` or on a `retry-now`.
// 2. **`pause`.** It asks each other loop of the project whose worker runs in the project root to pause,
//    with a `pause` control by `looper` whose payload is `landing <loop>`. It notes each one in the file
//    before it asks. The fast-forward in the root is what the pause protects, so a loop in a lane or a
//    worktree of its own runs on. An idle loop, a loop in state `landing`, and a loop a pause already
//    stands on are left alone too.
// 3. **`wait`.** It waits until each loop it asked is paused, idle, or landing itself (`settled`),
//    reading their folders every `WAIT_POLL_MS`, with no time limit. A loop a person resumed ends the
//    landing with a hold that names it. A loop whose pause something else lifted is asked to pause
//    again. Then, when the main branch has moved, it runs the catch-up again (`Driver.catchUp`) while
//    the others stay paused. A landing session or a hold from it ends the landing, and the worker takes
//    it.
// 4. **`land`.** In the project root, on the main branch, it runs `git merge --ff-only side/<slug>`. A
//    root on another branch, a fast-forward git refuses, or a lane with uncommitted files ends the
//    landing with a hold.
// 5. **`finish`.** A loop in a lane, `<scratch>/worktrees/lane-<n>`, hands it back (`handBack` in
//    `lanes.ts`), and the file names the side loop the lane went to. A loop in a worktree of its own
//    runs `git worktree remove` on it. Then `git branch -d` deletes the branch. Each git command that
//    fails is a `warn` control by `looper`, and a lane it cannot hand back stays with the loop, so no
//    work is lost.
// 6. **`resume`.** It resumes each loop it paused whose pause it still owns, and starts the side loop
//    the lane went to unless its worker runs. Then it marks the loop landed (`markLanded`), so it leaves
//    the list, and deletes the file.
//
// Every way a landing ends after step 2 resumes the others and deletes the file: a hold, a stop, and a
// new catch-up that did not come back green. A hold resets the round count, so a resume starts afresh.
//
// # The bounds it keeps
//
// - The landing runs the catch-up again at most `MAX_ROUNDS` times while the others are paused, counted
//   across each return to the catch-up, until the loop holds. Past that it holds. The catch-up before
//   the landing, and the one after a landing session, are not counted.
// - The file names at most every loop of the project, and the wait reads each one's controls once per
//   poll. A worker compacts its controls past `CONTROLS_BYTES`.
//
// # Known gaps
//
// - A worker killed inside `handBack`, after the lane went on and before the file names where, leaves
//   that side loop holding the lane with no worker. `loop start` starts it.

import { existsSync, rmSync } from "node:fs";
import { basename, dirname, join } from "node:path";
import { readState, writeState } from "../store/state.ts";
import type { Side } from "./catch-up.ts";
import { appendControl, isPerson, liftedBy, readControls, standingPause } from "./controls.ts";
import type { Driver, Outcome } from "./driver.ts";
import { handBack, locked, loops, loopsDir, markLanded } from "./lanes.ts";
import { git } from "./turn.ts";
import { STATE_FILE, type WorkerState, workerRuns } from "./worker.ts";

/** The landing's state file in the project's `.looper/`. */
export const LANDING_FILE = "landing.json";
/** How long a side loop waits for another side loop's landing before it checks the lock again, in ms. */
export const LOCK_RECHECK_MS = 60_000;
/** How often the wait reads the states of the loops it paused, in ms. */
export const WAIT_POLL_MS = 1_000;
/** The most catch-ups the landing runs while the others are paused before the loop holds. */
export const MAX_ROUNDS = 3;

/** The steps a landing names, in order. */
export type Step = "pause" | "wait" | "land" | "finish" | "resume";

/** What `landing.json` holds. */
export interface LandingState {
  /** The side loop that holds the lock. */
  holder: string;
  step: Step;
  /** The loops the landing asked to pause, by name. */
  paused: string[];
  /** The side loop the landing handed its lane to, which it starts. */
  next: string | null;
}

/** How a landing ended. `again` is the outcome of a catch-up the main branch's move called for, which did not come back green. */
export type Landed = { kind: "done"; why: string } | { kind: "hold"; why: string } | { kind: "again"; outcome: Outcome } | { kind: "stopped" };

/** What the landing waits with: the worker's asks. */
export interface Waits {
  /** Waits until `done` holds or `deadline` passes. True when `done` held. */
  until(done: () => boolean, deadline: number): Promise<boolean>;
  stopped(): boolean;
  /** How many `retry-now` controls the run has had. */
  retries(): number;
}

/** One side loop's landing: where it is, and where it lands. */
export interface LanderOptions {
  /** The loop's name, which the pause's payload names. */
  name: string;
  /** The loop's worktree, empty once its lane is handed back. */
  worktree: string;
  /** The loop's goal, and where it lands. `side.mainRoot` is the project root. */
  side: Side;
  /** Starts the worker of the side loop the landing handed its lane to. */
  start?: (name: string) => void;
}

/** Whether a landing at `step` is past the fast-forward, so a restarted worker ends it with no driver. */
export function pastLand(step: Step | null): boolean {
  return step === "finish" || step === "resume";
}

/** Whether a loop in `state` cannot touch the main branch's tree: it is paused or idle, or it is a side loop waiting for this landing's lock. */
export function settled(state: string): boolean {
  return state === "paused" || state === "idle" || state === "landing";
}

/** The state of the loop whose folder is `dir`: its worker's, or `idle` when no worker runs it. */
export function loopState(dir: string): string {
  if (!workerRuns(dir)) return "idle";
  return readState<WorkerState>(join(dir, STATE_FILE))?.state ?? "idle";
}

export class Lander {
  readonly name: string;
  readonly slug: string;
  readonly worktree: string;
  /** `side/<slug>`. */
  readonly branch: string;
  readonly mainRoot: string;
  readonly mainBranch: string;
  private readonly start: ((name: string) => void) | undefined;
  private readonly file: string;

  constructor(o: LanderOptions) {
    this.name = o.name;
    this.slug = o.side.goal;
    this.worktree = o.worktree;
    this.branch = `side/${o.side.goal}`;
    this.mainRoot = o.side.mainRoot;
    this.mainBranch = o.side.mainBranch;
    this.start = o.start;
    this.file = join(o.side.mainRoot, ".looper", LANDING_FILE);
  }

  /** The payload of each pause the landing asks for. */
  get payload(): string {
    return `landing ${this.name}`;
  }

  /** The step of the landing this loop holds, or null when it holds none. */
  held(): Step | null {
    const s = readState<LandingState>(this.file);
    return s !== undefined && s.holder === this.name ? s.step : null;
  }

  /** Lands the branch, from the step the file names, or from the lock. `rounds.n` counts the catch-ups the landing has run while the others were paused, since the run's last hold. */
  async land(d: Driver, w: Waits, rounds: { n: number }): Promise<Landed> {
    let step = await this.lock(w);
    if (step === null) return { kind: "stopped" };
    if (step === "pause") {
      await this.pauseOthers();
      step = this.step("wait");
    }
    if (step === "wait") {
      for (;;) {
        const resumed = await this.waitPaused(w);
        if (resumed === null) {
          await this.end();
          return { kind: "stopped" };
        }
        if (resumed !== "") return this.abandon(`${resumed} was resumed during the landing of ${this.branch}, so it holds`);
        if (!this.mainMoved()) break;
        rounds.n++;
        if (rounds.n > MAX_ROUNDS) {
          return this.abandon(`landing: ${this.mainBranch} moved under ${MAX_ROUNDS} catch-ups of ${this.branch}, so it holds. A resume tries again.`);
        }
        const outcome = await d.catchUp(this.slug);
        if (outcome.kind !== "land") {
          await this.end();
          return { kind: "again", outcome };
        }
      }
      step = this.step("land");
    }
    if (step === "land") {
      const why = this.dirtyLane() ?? this.fastForward();
      if (why !== null) return this.abandon(why);
      step = this.step("finish");
    }
    return this.finish(step);
  }

  /** Steps 5 and 6, from `step`: the lane is handed back or the worktree goes, the branch goes, the others resume, and the loop leaves the list. A worker restarted in either step runs only these, with no driver. */
  async finish(step: Step): Promise<Landed> {
    if (step === "finish") {
      const warnings: string[] = [];
      const n = laneOf(this.worktree);
      if (n !== null) {
        try {
          const next = await handBack(this.mainRoot, this.name, this.mainBranch);
          if (next !== null) this.write({ ...this.state(), next });
        } catch (e) {
          warnings.push(`the landing kept lane-${n} with its loop: ${(e as Error).message}`);
        }
      } else if (this.worktree !== "" && existsSync(this.worktree)) {
        try {
          git(this.mainRoot, ["worktree", "remove", this.worktree]);
        } catch (e) {
          warnings.push(`the landing left the worktree ${this.worktree} on disk for \`clean\`: ${(e as Error).message}`);
        }
      }
      if (succeeds(this.mainRoot, ["rev-parse", "-q", "--verify", `refs/heads/${this.branch}`])) {
        try {
          git(this.mainRoot, ["branch", "-d", this.branch]);
        } catch (e) {
          warnings.push(`the landing kept the branch ${this.branch}: ${(e as Error).message}`);
        }
      }
      for (const w of warnings) await this.ask(this.name, "warn", w);
      step = this.step("resume");
    }
    await this.resumeOurs();
    const next = this.state().next;
    if (next !== null && this.start !== undefined && !workerRuns(join(loopsDir(this.mainRoot), next))) this.start(next);
    await markLanded(this.mainRoot, this.name);
    await this.giveBack();
    return { kind: "done", why: `${this.branch} landed on ${this.mainBranch}` };
  }

  /** Takes the lock, waiting while another loop holds it. Returns the step the landing is at, or null when a stop was asked while it waited. */
  private async lock(w: Waits): Promise<Step | null> {
    for (;;) {
      const step = await locked(this.mainRoot, () => {
        const s = readState<LandingState>(this.file);
        if (s !== undefined) return s.holder === this.name ? s.step : null;
        this.write({ holder: this.name, step: "pause", paused: [], next: null });
        return "pause" as Step;
      });
      if (step !== null) return step;
      const retry = w.retries();
      await w.until(() => w.stopped() || w.retries() !== retry, Date.now() + LOCK_RECHECK_MS);
      if (w.stopped()) return null;
    }
  }

  /** The file as this loop holds it. */
  private state(): LandingState {
    return readState<LandingState>(this.file) ?? { holder: this.name, step: "pause", paused: [], next: null };
  }

  private write(s: LandingState): void {
    writeState(this.file, s);
  }

  /** Moves the landing to `step`, and returns it. */
  private step(step: Step): Step {
    this.write({ ...this.state(), step });
    return step;
  }

  /** Deletes the file, which gives back the lock. */
  private giveBack(): Promise<void> {
    return locked(this.mainRoot, () => rmSync(this.file, { force: true }));
  }

  /** Asks each other loop of the project whose worker runs in the project root to pause, noting it in the file first. */
  private async pauseOthers(): Promise<void> {
    for (const l of loops(this.mainRoot)) {
      const dir = join(loopsDir(this.mainRoot), l.name);
      const state = loopState(dir);
      const runs = state !== "idle" && state !== "landing" && l.laneWait === null && l.landed === undefined;
      if (l.name === this.name || !runs || !sameDir(l.worktree, this.mainRoot)) continue;
      if (standingPause(readControls(dir)) !== null) continue;
      const s = this.state();
      if (!s.paused.includes(l.name)) this.write({ ...s, paused: [...s.paused, l.name] });
      await this.ask(l.name, "pause", this.payload);
    }
  }

  /** Waits until each loop the landing paused is settled, asking again each one whose pause something other than a person lifted. Returns `""` then, the name of a loop a person resumed, or null on a stop. */
  private async waitPaused(w: Waits): Promise<string | null> {
    for (;;) {
      let waiting = false;
      for (const name of this.state().paused) {
        const dir = join(loopsDir(this.mainRoot), name);
        const controls = readControls(dir);
        if (standingPause(controls) !== null) {
          if (!settled(loopState(dir))) waiting = true;
          continue;
        }
        const by = liftedBy(controls);
        if (by !== null && isPerson(by)) return name;
        await this.ask(name, "pause", this.payload);
        waiting = true;
      }
      if (!waiting) return "";
      await w.until(() => w.stopped(), Date.now() + WAIT_POLL_MS);
      if (w.stopped()) return null;
    }
  }

  /** Whether the main branch holds a commit the side branch does not. */
  private mainMoved(): boolean {
    const out = Bun.spawnSync(["git", "merge-base", "--is-ancestor", this.mainBranch, "HEAD"], { cwd: this.worktree, stdin: "ignore", stdout: "ignore", stderr: "pipe" });
    if (out.exitCode === 0) return false;
    if (out.exitCode === 1) return true;
    throw new Error(`git cannot tell whether ${this.mainBranch} moved: ${out.stderr.toString().trim()}`);
  }

  /** Fast-forwards the main branch to the side branch in the project root. Returns why it cannot, or null. */
  private fastForward(): string | null {
    const out = Bun.spawnSync(["git", "symbolic-ref", "-q", "--short", "HEAD"], { cwd: this.mainRoot, stdin: "ignore", stdout: "pipe", stderr: "ignore" });
    const head = out.exitCode === 0 ? out.stdout.toString().trim() : "";
    if (head !== this.mainBranch) {
      return `landing: the project root ${this.mainRoot} is on ${head === "" ? "no branch" : head}, not ${this.mainBranch}, so ${this.branch} cannot land`;
    }
    try {
      git(this.mainRoot, ["merge", "--ff-only", "-q", this.branch]);
      return null;
    } catch (e) {
      return `landing: git refused to fast-forward ${this.mainBranch} to ${this.branch}: ${(e as Error).message}`;
    }
  }

  /** Why the landing holds rather than land, when the loop works in a lane with uncommitted files, or null. */
  private dirtyLane(): string | null {
    const n = laneOf(this.worktree);
    if (n === null) return null;
    let status: string;
    try {
      status = git(this.worktree, ["status", "--porcelain"]);
    } catch (e) {
      return `landing: git cannot read lane-${n}'s status: ${(e as Error).message}`;
    }
    if (status.trim() === "") return null;
    return `landing: lane-${n} has uncommitted files, so ${this.branch} does not land. Commit or remove them, then resume.`;
  }

  /** Resumes the others, gives back the lock, and holds with `why`. */
  private async abandon(why: string): Promise<Landed> {
    await this.end();
    return { kind: "hold", why };
  }

  /** Resumes the others, then gives back the lock. */
  private async end(): Promise<void> {
    await this.resumeOurs();
    await this.giveBack();
  }

  /** Resumes each loop the landing paused whose pause it still owns. */
  private async resumeOurs(): Promise<void> {
    for (const name of this.state().paused) {
      const p = standingPause(readControls(join(loopsDir(this.mainRoot), name)));
      if (p !== null && p.by === "looper" && p.why === this.payload) await this.ask(name, "resume", "");
    }
  }

  /** Appends a control by `looper` to loop `name`. One the append refuses becomes a warning on this loop. */
  private async ask(name: string, action: string, payload: string): Promise<void> {
    try {
      await appendControl(join(loopsDir(this.mainRoot), name), { by: "looper", action, payload });
    } catch (e) {
      if (action === "warn") throw e;
      await this.ask(this.name, "warn", `the landing's ${action} of loop ${name} was refused: ${(e as Error).message}`);
    }
  }
}

/** The lane `worktree` is, from its path `<scratch>/worktrees/lane-<n>`, or null for a side loop's own worktree. */
function laneOf(worktree: string): number | null {
  if (worktree === "" || basename(dirname(worktree)) !== "worktrees") return null;
  const m = /^lane-(\d+)$/.exec(basename(worktree));
  return m === null ? null : Number(m[1]);
}

/** Whether `a` and `b` name one directory, as Windows compares: with either separator, with or without `\\?\` and a trailing separator, in any case. */
function sameDir(a: string, b: string): boolean {
  const norm = (p: string) =>
    (p.startsWith("\\\\?\\") ? p.slice(4) : p)
      .replaceAll("/", "\\")
      .replace(/\\+$/, "")
      .toLowerCase();
  return norm(a) === norm(b);
}

function succeeds(root: string, args: string[]): boolean {
  return Bun.spawnSync(["git", ...args], { cwd: root, stdin: "ignore", stdout: "ignore", stderr: "ignore" }).exitCode === 0;
}
