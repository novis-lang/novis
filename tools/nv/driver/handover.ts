// Two fields of a goal's record that tie the run to a tool outside this tree: `waitFor`, a command that
// must pass before the driver starts a session on the goal, and `handover: "looper"`, which ends this
// driver's run for good once the goal is reached and starts looper's main loop in its place.
//
// **The wait gate.** While `waitFor.argv` exits non-zero, or prints none of a `want` string on stdout or
// stderr, the turn starts no session: it says once what it waits for, puts it on the status line, and runs
// the command again every `everyMin` minutes. A wait spends no session and counts no stall, since nothing
// ran that could commit. The console stays live through it: `r` or `.loop/retry` runs the command now,
// `s` or `.loop/stop` ends the run, `p` or `.loop/pause` holds it as between two sessions, and `h` has no
// session to halt. A gate that opens lets the turn go on to its session as if it had never waited.
//
// **The handover.** A reached goal that says `handover: "looper"` is not switched from: the driver deletes
// its prose, record and handoff record and `data/chain.json`, commits those paths alone, then runs `looper
// loop add` and `looper loop start` in the tree. `loop add` refuses a loop that already exists, so its
// failure is reported and the start runs anyway, and only the start's exit decides. The run then ends
// with a verdict `driver/respawn.ts` does not restart. A start that fails holds the run with looper's
// error; the run carries `handing` in `.loop/run.json`, so the turn after a lifted hold retries the start
// alone and never a session, since the chain it would read is gone. `NOVIS_LOOPER` names the looper to
// run, split on whitespace, in place of the one on `PATH`, which is how a test hands it a stub.
//
// The handover refuses while `data/chain.json` names any goal but its own: deleting it would drop the rest
// of the chain, which the goal's own work moves into looper's format first.

import { existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { goalFiles } from "../lib/chain.ts";
import { ROOT } from "../lib/paths.ts";
import { run as runProc, type RunResult } from "../lib/proc.ts";

/** A goal's `waitFor`: the command, the text it must print, and the minutes between two runs of it. */
export interface WaitFor {
  argv: string[];
  want?: string[];
  everyMin: number;
}

export const CHAIN_FILE = "data/chain.json";
/** The environment variable that names the looper to run in place of the one on `PATH`. */
export const LOOPER_ENV = "NOVIS_LOOPER";
/** The subject of the commit that hands the run to looper. */
export const HANDOVER_SUBJECT = "chore(goals): the loop moves onto looper";

const GATE_TIMEOUT_MS = 10 * 60 * 1000;
const LOOPER_TIMEOUT_MS = 2 * 60 * 1000;

type Run = (argv: string[], opts: { cwd?: string; timeoutMs?: number }) => Promise<RunResult>;

/** The command as one line, for the console and the ledger. */
export function shownArgv(argv: string[]): string {
  return argv.map((a) => (/[\s"]/.test(a) ? JSON.stringify(a) : a)).join(" ");
}

function firstLine(r: RunResult): string {
  return (
    `${r.stderr}\n${r.stdout}`
      .split(/\r?\n/)
      .map((l) => l.trim())
      .find(Boolean) ?? ""
  );
}

/** The problems with a goal's `waitFor` and `handover` that the schema's types do not catch; `side` is a goal off the chain. */
export function waitAndHandoverProblems(where: string, goal: { waitFor?: WaitFor; handover?: string }, side: boolean): string[] {
  const out: string[] = [];
  const w = goal.waitFor;
  if (w !== undefined) {
    if (w.argv.length === 0 || w.argv[0]!.trim() === "") out.push(`${where}: \`waitFor.argv\` is empty, so there is no command to wait on`);
    if (!Number.isInteger(w.everyMin) || w.everyMin < 1) out.push(`${where}: \`waitFor.everyMin\` is ${w.everyMin}; it is a whole number of minutes, 1 or more`);
    if ((w.want ?? []).some((x) => x === "")) out.push(`${where}: \`waitFor.want\` holds an empty string, which every output contains`);
  }
  if (goal.handover !== undefined && side) out.push(`${where}: a side goal never hands the run over; \`handover\` belongs to a goal on the chain`);
  return out;
}

/** Runs the gate's command once. Returns "" when the gate is open, or why it is shut. */
export async function gateShut(w: WaitFor, root: string = ROOT, run: Run = runProc): Promise<string> {
  const shown = shownArgv(w.argv);
  let r: RunResult;
  try {
    r = await run(w.argv, { cwd: root, timeoutMs: GATE_TIMEOUT_MS });
  } catch (e) {
    return `\`${shown}\` did not start: ${(e as Error).message.split("\n")[0]}`;
  }
  if (r.timedOut) return `\`${shown}\` timed out`;
  if (r.code !== 0) return `\`${shown}\` exited ${r.code}${firstLine(r) ? `: ${firstLine(r).slice(0, 200)}` : ""}`;
  const out = `${r.stdout}\n${r.stderr}`;
  const missing = (w.want ?? []).filter((x) => !out.includes(x));
  if (missing.length > 0) return `\`${shown}\` passed and printed no ${missing.map((m) => JSON.stringify(m)).join(", ")}`;
  return "";
}

/** What `awaitGate` needs of the console; the driver passes `CONTROL`, `wait` and `holdPause`, a test its own. */
export interface GateConsole {
  /** Runs the gate's command once: "" when open, or why it is shut. */
  check: () => Promise<string>;
  /** Sleeps up to `seconds`, ending early once `until` is true. */
  sleep: (seconds: number, until: () => boolean) => Promise<void>;
  /** Why the run should end now, or "". */
  stopReason: () => string;
  /** Whether a hold is asked for. */
  paused: () => boolean;
  /** Holds while a hold stands: "" when it lifts, or the stop that ended it. */
  hold: () => Promise<string>;
  /** Whether `r` or `.loop/retry` asked for a run now; consumes the request. */
  takeRetry: () => boolean;
  /** Whether a retry or a stop is waiting. */
  pending: () => boolean;
  /** A line for the console and the ledger, said when the wait begins, changes or ends. */
  note: (line: string) => void;
  /** The status line's phase while the wait lasts; "" clears it. */
  phase: (text: string) => void;
  /** Wall-clock seconds, for the wait's length. */
  now: () => number;
  /** Puts the wait under `r`, or takes it out again. */
  park: (on: boolean) => void;
}

/**
 * Waits until goal `slug`'s gate opens. Returns "" once it is open, or the reason the run ends instead: a
 * stop pressed or written during the wait.
 */
export async function awaitGate(slug: string, w: WaitFor, c: GateConsole): Promise<string> {
  const shown = shownArgv(w.argv);
  const began = c.now();
  let last = "";
  let checks = 0;
  c.park(true);
  try {
    for (;;) {
      const stop = c.stopReason();
      if (stop) return `${stop}, while goal \`${slug}\` waited for \`${shown}\``;
      if (c.paused()) {
        const held = await c.hold();
        if (held) return `${held}, while goal \`${slug}\` waited for \`${shown}\``;
      }
      const why = await c.check();
      checks++;
      if (why === "") {
        if (checks > 1) c.note(`wait gate open after ${Math.round((c.now() - began) / 60)} min and ${checks} runs: goal \`${slug}\` starts its session`);
        return "";
      }
      if (why !== last) {
        c.note(`goal \`${slug}\` waits, and no session starts: ${why}. It runs again every ${w.everyMin} min; r runs it now, s ends the run`);
        last = why;
      }
      c.phase(`waiting for \`${shown}\``);
      await c.sleep(w.everyMin * 60, () => c.pending() || c.paused());
      c.takeRetry();
    }
  } finally {
    c.park(false);
    c.phase("");
  }
}

/** The looper to run: `NOVIS_LOOPER` split on whitespace, or `looper` from `PATH`. */
export function looperArgv(env: Record<string, string | undefined> = process.env): string[] {
  const words = (env[LOOPER_ENV] ?? "").trim().split(/\s+/).filter(Boolean);
  return words.length > 0 ? words : [Bun.which("looper") ?? "looper"];
}

/** The goals `data/chain.json` names, or null when it is absent or unreadable. */
function chainGoalsIn(root: string): string[] | null {
  try {
    const goals = JSON.parse(readFileSync(join(root, CHAIN_FILE), "utf8")).goals;
    return Array.isArray(goals) ? goals.map(String) : [];
  } catch {
    return null;
  }
}

/**
 * Deletes goal `slug`'s files and `data/chain.json`, and commits each deleted path that git tracks, those
 * alone. A path already gone is skipped, so a second call after a failed commit commits what the first
 * deleted. Returns "" when it is done, or why it is not.
 */
export async function retireChain(slug: string, root: string = ROOT, run: Run = runProc): Promise<string> {
  const goals = chainGoalsIn(root);
  if (goals !== null) {
    const others = goals.filter((g) => g !== slug);
    if (others.length > 0) return `${CHAIN_FILE} still names ${others.map((g) => `\`${g}\``).join(", ")}, so deleting it would drop them; goal \`${slug}\` moves them into looper's format and leaves only itself on the chain`;
  }
  const paths = [...goalFiles(slug), CHAIN_FILE];
  for (const p of paths) rmSync(join(root, p), { force: true });
  const listed = await run(["git", "ls-files", "--", ...paths], { cwd: root });
  if (listed.code !== 0) return `git ls-files failed: ${firstLine(listed)}`;
  const gone = listed.stdout.split(/\r?\n/).filter((p) => p !== "" && !existsSync(join(root, p)));
  if (gone.length === 0) return "";
  const msg = join(root, ".agent-tmp", "handover-commit.txt");
  mkdirSync(dirname(msg), { recursive: true });
  writeFileSync(msg, `${HANDOVER_SUBJECT}\n\nGoal \`${slug}\` is reached, so its prose, its record, its handoff record and\n${CHAIN_FILE} are deleted, and looper's main loop runs the goals from here.\n`);
  try {
    const committed = await run(["git", "commit", "-q", "-F", msg, "--", ...gone], { cwd: root });
    if (committed.code !== 0) return `the handover's commit failed: ${firstLine(committed)}`;
  } finally {
    rmSync(msg, { force: true });
  }
  return "";
}

/**
 * `looper loop add`, then `looper loop start`, in the tree. Returns "" once the start has passed, or the
 * error that holds the run, which names what `loop add` said when it failed too.
 */
export async function startLooper(root: string = ROOT, looper: string[] = looperArgv(), run: Run = runProc, say: (line: string) => void = () => {}): Promise<string> {
  const call = async (verb: string) => {
    const argv = [...looper, "loop", verb];
    try {
      return await run(argv, { cwd: root, timeoutMs: LOOPER_TIMEOUT_MS });
    } catch (e) {
      return { argv, code: -1, stdout: "", stderr: `did not start: ${(e as Error).message.split("\n")[0]}`, timedOut: false, aborted: false };
    }
  };
  const added = await call("add");
  const addSaid = added.code === 0 ? "" : `\`${shownArgv(added.argv)}\` exited ${added.code}${firstLine(added) ? `: ${firstLine(added)}` : ""}`;
  say(added.code === 0 ? "looper: the main loop is added" : `looper: ${addSaid} -- a loop that already exists is fine, so the start runs anyway`);
  const started = await call("start");
  if (started.code === 0) {
    say("looper: the main loop is started");
    return "";
  }
  const startSaid = `\`${shownArgv(started.argv)}\` exited ${started.code}${started.timedOut ? " (timed out)" : ""}${firstLine(started) ? `: ${firstLine(started)}` : ""}`;
  return addSaid ? `${startSaid}; before it, ${addSaid}` : startSaid;
}
