// Two fields of a goal's record that tie the run to a tool outside this tree: `waitFor`, a command that
// must pass before the driver starts a session on the goal, and `handover: "looper"`, which ends this
// driver's run for good once the goal is reached and starts looper's main loop in its place.
//
// **The wait gate.** While `waitFor.argv` exits non-zero, misses a `want` string on stdout and stderr, or
// prints a line that holds an `absent` string, the turn starts no session: it says once what it waits for,
// puts it on the status line, and runs the command again every `everyMin` minutes. A wait spends no
// session and counts no stall, since nothing ran that could commit. The console stays live through it: `r` or `.loop/retry` runs the command now,
// `s` or `.loop/stop` ends the run, `p` or `.loop/pause` holds it as between two sessions, and `h` has no
// session to halt. A gate that opens lets the turn go on to its session as if it had never waited.
//
// **The handover.** A reached goal that says `handover: "looper"` is not switched from: the driver deletes
// its prose, record and handoff record and `data/chain.json`, commits those paths alone, then gives looper's
// sessions their permissions: `.claude/settings.json` takes the allow entries `LOOPER_ALLOW` and a
// PreToolUse hook that runs `looper guard`, beside every entry it has, the `bun nv guard` hook among them,
// and is committed by that path alone. A session never edits its own permission file, so the driver does
// it, and an uncommitted edit somebody else left there holds the run, as `allow.ts` leaves one alone. Then
// it runs `looper project add .`, `looper loop add` and `looper loop start` in the tree. Only the start
// decides: the first two are reported when they fail, and a start looper refuses with a message that the
// loop runs `already` counts as started. The run then ends with a verdict `driver/respawn.ts` does not
// restart. A settings step or a start that fails holds the run with its error; the run carries `handing`
// in `.loop/run.json`, so the turn after a lifted hold retries the settings and the start alone and never
// a session, since the chain it would read is gone. Each step skips what is in place already.
//
// **Which looper.** `NOVIS_LOOPER` names the looper to run, split on whitespace, which is how a test hands
// it a stub. Otherwise it is `looper` on the `Path` the user's registry holds now, ahead of the one the run
// inherited: `looper self-update` moves looper to a new directory, and a run that waits for that update
// must find the new copy. A goal with either field has that directory put first on the turn's `PATH`, so
// its sessions run the same looper.
//
// The handover refuses while `data/chain.json` names any goal but its own: deleting it would drop the rest
// of the chain, which the goal's own work moves into looper's format first.

import { existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { goalFiles } from "../lib/chain.ts";
import { ROOT } from "../lib/paths.ts";
import { run as runProc, type RunResult } from "../lib/proc.ts";
import { SETTINGS, withRules } from "./allow.ts";

/** A goal's `waitFor`: the command, the text it must print and must not, and the minutes between two runs of it. */
export interface WaitFor {
  argv: string[];
  want?: string[];
  absent?: string[];
  everyMin: number;
}

export const CHAIN_FILE = "data/chain.json";
/** The environment variable that names the looper to run in place of the one on `PATH`. */
export const LOOPER_ENV = "NOVIS_LOOPER";
/** The subject of the commit that hands the run to looper. */
export const HANDOVER_SUBJECT = "chore(goals): the loop moves onto looper";
/** The allow entries looper's sessions need, which the handover adds to `.claude/settings.json`. */
export const LOOPER_ALLOW = ["Bash(looper:*)", "PowerShell(looper:*)"];
/** The command of looper's PreToolUse hook, which the handover adds beside the `bun nv guard` hook. */
export const LOOPER_GUARD = "looper guard";
/** The subject of the commit that gives looper's sessions their permissions. */
export const SETTINGS_SUBJECT = "build(loop): the allowlist and the guard hook take looper";

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
    if ((w.absent ?? []).some((x) => x === "")) out.push(`${where}: \`waitFor.absent\` holds an empty string, which every line contains`);
  }
  if (goal.handover !== undefined && side) out.push(`${where}: a side goal never hands the run over; \`handover\` belongs to a goal on the chain`);
  return out;
}

/**
 * Runs the gate's command once. A command whose program is `looper` runs the looper `looperArgv` finds,
 * the one the handover starts. Returns "" when the gate is open, or why it is shut.
 */
export async function gateShut(w: WaitFor, root: string = ROOT, run: Run = runProc, looper: () => string[] = looperArgv): Promise<string> {
  const shown = shownArgv(w.argv);
  const argv = w.argv[0] === "looper" ? [...looper(), ...w.argv.slice(1)] : w.argv;
  let r: RunResult;
  try {
    r = await run(argv, { cwd: root, timeoutMs: GATE_TIMEOUT_MS });
  } catch (e) {
    return `\`${shown}\` did not start: ${(e as Error).message.split("\n")[0]}`;
  }
  if (r.timedOut) return `\`${shown}\` timed out`;
  if (r.code !== 0) return `\`${shown}\` exited ${r.code}${firstLine(r) ? `: ${firstLine(r).slice(0, 200)}` : ""}`;
  const out = `${r.stdout}\n${r.stderr}`;
  const missing = (w.want ?? []).filter((x) => !out.includes(x));
  if (missing.length > 0) return `\`${shown}\` passed and printed no ${missing.map((m) => JSON.stringify(m)).join(", ")}`;
  const found = out.split(/\r?\n/).find((l) => (w.absent ?? []).some((x) => l.includes(x)));
  if (found !== undefined) return `\`${shown}\` passed and still printed ${JSON.stringify(found.trim().slice(0, 200))}`;
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

/**
 * The `Path` the user's registry holds now, then `inherited`. On Windows `looper self-update` moves looper
 * to a new directory and rewrites the user's `Path`, and a run started before the update would still find
 * the old copy, or none once it is pruned, so the registry is read on every call. Elsewhere it is
 * `inherited` alone.
 */
export function currentPath(inherited: string = process.env.PATH ?? ""): string {
  if (process.platform !== "win32") return inherited;
  try {
    const r = Bun.spawnSync(["reg", "query", "HKCU\\Environment", "/v", "Path"], { stdout: "pipe", stderr: "ignore" });
    const m = /^\s*Path\s+REG_(?:EXPAND_)?SZ\s+(.*)$/im.exec(r.stdout.toString());
    const user = (m?.[1] ?? "").trim().replace(/%([^%]+)%/g, (all, name: string) => process.env[name] ?? all);
    return user ? `${user};${inherited}` : inherited;
  } catch {
    return inherited;
  }
}

/** The looper to run: `NOVIS_LOOPER` split on whitespace, or `looper` on `currentPath`. */
export function looperArgv(env: Record<string, string | undefined> = process.env): string[] {
  const words = (env[LOOPER_ENV] ?? "").trim().split(/\s+/).filter(Boolean);
  return words.length > 0 ? words : [Bun.which("looper", { PATH: currentPath(env.PATH ?? "") }) ?? "looper"];
}

/**
 * Puts the directory of the looper `looperArgv` finds first on this process's `PATH`, so a session and a
 * check the turn starts run the same looper as the gate and the handover. Changes nothing when looper is
 * not found or `NOVIS_LOOPER` names it.
 */
export function putLooperFirst(env: Record<string, string | undefined> = process.env): void {
  if ((env[LOOPER_ENV] ?? "").trim()) return;
  const found = looperArgv(env)[0]!;
  if (!/[\\/]/.test(found)) return;
  const dir = dirname(found);
  const sep = process.platform === "win32" ? ";" : ":";
  const path = env.PATH ?? "";
  if (path.split(sep)[0] !== dir) env.PATH = `${dir}${sep}${path}`;
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

/** The index of the `]` or `}` that closes the bracket opened just before `from`, or -1. */
function closingBracket(text: string, from: number): number {
  let depth = 0;
  for (let i = from; i < text.length; i++) {
    const ch = text[i];
    if (ch === '"') {
      for (i++; i < text.length && text[i] !== '"'; i++) if (text[i] === "\\") i++;
    } else if (ch === "[" || ch === "{") depth++;
    else if (ch === "]" || ch === "}") {
      if (depth === 0) return i;
      depth--;
    }
  }
  return -1;
}

/**
 * The settings text with a PreToolUse entry that runs `looper guard` appended to `hooks.PreToolUse`, on
 * a line of its own in the one-line form the file's entries have, and every other byte kept.
 */
export function withLooperGuard(text: string): string {
  const m = /"PreToolUse"\s*:\s*\[/.exec(text);
  if (!m) throw new Error(`${SETTINGS} has no hooks.PreToolUse array`);
  const open = m.index + m[0].length;
  const close = closingBracket(text, open);
  if (close < 0) throw new Error(`${SETTINGS} has a hooks.PreToolUse array that never closes`);
  const lineStart = text.lastIndexOf("\n", m.index) + 1;
  const outer = /^[ \t]*/.exec(text.slice(lineStart))![0];
  const body = text.slice(open, close).trimEnd();
  const entry = `{ "matcher": "Read|Bash|PowerShell", "hooks": [ { "type": "command", "command": ${JSON.stringify(LOOPER_GUARD)} } ] }`;
  const joined = `${body.trim() === "" ? "" : `${body},`}\n${outer}  ${entry}\n${outer}`;
  const out = text.slice(0, open) + joined + text.slice(close);
  JSON.parse(out);
  return out;
}

interface SettingsShape {
  permissions?: { allow?: string[] };
  hooks?: { PreToolUse?: { hooks?: { command?: string }[] }[] };
}

/**
 * Gives looper's sessions their permissions: adds `LOOPER_ALLOW` and the `looper guard` hook to
 * `.claude/settings.json` where it lacks them, keeping every other entry and the file's layout, and
 * commits that path alone. Returns "" when the file has them, or why it does not. An uncommitted edit to
 * the file is somebody's work, so it is left as it is and the reason says so.
 */
export async function admitLooper(root: string = ROOT, run: Run = runProc): Promise<string> {
  const path = join(root, SETTINGS);
  let text: string;
  let settings: SettingsShape;
  try {
    text = readFileSync(path, "utf8");
    settings = JSON.parse(text) as SettingsShape;
  } catch (e) {
    return `${SETTINGS} could not be read: ${(e as Error).message.split("\n")[0]}`;
  }
  const allow = settings.permissions?.allow ?? [];
  const rules = LOOPER_ALLOW.filter((r) => !allow.includes(r));
  const guarded = (settings.hooks?.PreToolUse ?? []).some((e) => (e.hooks ?? []).some((h) => h.command === LOOPER_GUARD));
  if (rules.length === 0 && guarded) return "";
  const diff = await run(["git", "diff", "--quiet", "HEAD", "--", SETTINGS], { cwd: root });
  if (diff.code !== 0) return `${SETTINGS} has an uncommitted edit, so looper's allow entries and \`${LOOPER_GUARD}\` hook are not added to it; commit or discard that edit, then lift the hold`;
  let next: string;
  try {
    next = withRules(text, rules);
    if (!guarded) next = withLooperGuard(next);
  } catch (e) {
    return (e as Error).message.split("\n")[0]!;
  }
  writeFileSync(path, next);
  const msg = join(root, ".agent-tmp", "handover-settings-msg.txt");
  mkdirSync(dirname(msg), { recursive: true });
  const added = [...rules.map((r) => `\`${r}\``), ...(guarded ? [] : [`the \`${LOOPER_GUARD}\` hook`])];
  writeFileSync(msg, `${SETTINGS_SUBJECT}\n\nAdded by the loop's driver as it hands the run to looper: ${added.join(", ")}.\n`);
  try {
    const committed = await run(["git", "commit", "-q", "-F", msg, "--only", "--", SETTINGS], { cwd: root });
    if (committed.code !== 0) {
      writeFileSync(path, text);
      return `the commit of ${SETTINGS} failed: ${firstLine(committed)}`;
    }
  } finally {
    rmSync(msg, { force: true });
  }
  return "";
}

/** Looper's refusal of something that is in place already: a project, a loop, or a loop that runs. */
const ALREADY = /\balready\b/i;

/**
 * `admitLooper`, then `looper project add .`, `looper loop add` and `looper loop start`, in the tree.
 * Returns "" once the start has passed, or looper refused it because the loop runs already. Otherwise
 * returns the error that holds the run: the settings step's, before looper runs at all, or the start's,
 * which names what each earlier looper step said when it failed too.
 */
export async function startLooper(root: string = ROOT, looper: string[] = looperArgv(), run: Run = runProc, say: (line: string) => void = () => {}): Promise<string> {
  const admitted = await admitLooper(root, run);
  if (admitted) return admitted;
  say(`looper: ${SETTINGS} has looper's allow entries and the \`${LOOPER_GUARD}\` hook`);
  const call = async (...words: string[]) => {
    const argv = [...looper, ...words];
    try {
      return await run(argv, { cwd: root, timeoutMs: LOOPER_TIMEOUT_MS });
    } catch (e) {
      return { argv, code: -1, stdout: "", stderr: `did not start: ${(e as Error).message.split("\n")[0]}`, timedOut: false, aborted: false };
    }
  };
  const said = (r: RunResult) => `\`${shownArgv(r.argv)}\` exited ${r.code}${r.timedOut ? " (timed out)" : ""}${firstLine(r) ? `: ${firstLine(r)}` : ""}`;
  const earlier: string[] = [];
  for (const [words, done] of [
    [["project", "add", "."], "the project is registered"],
    [["loop", "add"], "the main loop is added"],
  ] as const) {
    const r = await call(...words);
    if (r.code === 0) say(`looper: ${done}`);
    else {
      earlier.push(said(r));
      say(`looper: ${said(r)} -- the start runs anyway, and decides`);
    }
  }
  const started = await call("loop", "start");
  const already = started.code !== 0 && ALREADY.test(`${started.stdout}\n${started.stderr}`);
  if (started.code === 0 || already) {
    say(already ? `looper: ${said(started)} -- the main loop runs already` : "looper: the main loop is started");
    return "";
  }
  return earlier.length > 0 ? `${said(started)}; before it, ${earlier.join("; ")}` : said(started);
}
