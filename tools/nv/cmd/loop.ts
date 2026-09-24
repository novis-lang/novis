// `bun nv loop --list`: the live goal's acceptance plan, read from `docs/agent/loop-goal.toml` while the
// Python driver runs it and from the goal's record under `data/goals/` once that file is gone. One
// line per check, `[<stage number> <stage title>]`, its kind, its name and what it runs, then
// `list: N check(s) match` last. The live goal is the slug the driver's pointer names (`lib/state.ts`), and its
// record carries the floor as its own first stage, so the plan printed is the whole plan the driver runs.
//
// Three filters narrow it, and a check is printed when it matches every one given:
//   - `--stage <label>` — the stage's number, or the start of its label `8 the driver` or of its title;
//   - `--name <text>` — text inside the check's name, case-insensitive;
//   - `--feature <id>` — a proofs check over that feature: its name is `proofs: <id>` or `dossier: <id>`,
//     with or without a `(1/3)` part, or its `argv` passes `<id>` to `--group` or `--id`.
// A check with no name of its own is named by its `file`, so `--name` reaches a fixture's check too.
//
// `bun nv loop --goal` prints the goal table `[g]` prints, one row per stage, without a run. Its results
// are the Python driver's memo `.loop/goal-green.json`, read by `memoResults`, and its session number is
// `.loop/run.json`'s `index`, the number that driver gives its latest session. No session runs under
// this command, so the table's last line names none. The table is as wide as the terminal, or 100
// columns when the output is not one, and it draws its lines in box-drawing characters on Windows and
// under a UTF-8 locale.
//
// `bun nv loop --run` takes the same three filters, at least one of them, and runs each check they
// select once, through `driver/accept.ts`'s `Sweep`. It prints one line per check, `ok`, `FAIL` with the
// line the ledger would quote, or `SHORT` for a suite below its `minPassing`, then `run: N green, M red`.
// What each process is doing goes to stderr as it starts. A session proves its own check this way, one
// command at a time, rather than by starting a sweep.
//
// `bun nv loop --goal-only` is the acceptance sweep, `driver/accept.ts`'s `acceptance`: every check in
// the tiers' order, the memo `.loop/accept-green.json` answering each check still green over its inputs,
// and a stop at the first red. It ends on `NOT GREEN: <the red check's line>` or `GOAL REACHED`. `--full`
// consults no memo, and `--collect` runs past every red and names each one after the first on an
// `also red:` line. The three filters narrow it to the checks they select, and a narrowed sweep that
// passes ends on `GREEN` instead, since it has not asked the whole plan. A check's key is `nv why`'s,
// taken over the tree as the sweep begins.
//
// `bun nv loop` with none of those modes is one turn of the driver: one session, its acceptance sweep and
// its ledger lines in `.loop/log.md`, then exit 75, which asks `tools/respawn.py` for the next turn. It
// takes `--model` (`opus`), `--effort`, `--permission-mode` (`bypassPermissions`), `--max-sessions`,
// `--max-stalls` (10), `--max-retries` (3) and `--max-limit-wait` (21600 seconds). The run is the one `NOVIS_LOOP_RUN` names, and `.loop/run.json` carries its session
// count from turn to turn in the shape `loop.py` writes, so a run the Python driver started goes on here
// with its numbering. A turn refuses a tree whose `.loop/running` names another run. With no
// `NOVIS_LOOP_RUN`, nothing waits to start a next turn, so the turn is a run of one session.
//
// The session is `driver/launch.ts`'s: the session prompt with `nv orient`'s pack behind it, down
// `claude -p`'s stdin as stream-json. Each event repaints `driver/status.ts`'s status row when stdout is a
// terminal. A session that ends without wrapping has its own uncommitted paths swept into a wip commit,
// and a session the usage wall refused or the CLI dropped is run again inside the same turn, after the
// wall reopens or a backoff: `driver/sweep.ts` says how. The acceptance sweep behind it is the goal's own checks, with the carried floor and the release checks
// held in nine turns of ten (`FLOOR_GATE_EVERY`, counted in `.loop/accept-floor.json`). A scoped sweep
// that is green with checks held runs again over the whole plan, collecting every red, since a goal is
// never reached on a held floor. The run ends on a reached goal, a `BLOCKED` status, `--max-stalls`
// sessions in a row without a commit, `--max-sessions`, `--max-retries` non-zero CLI exits in a row,
// `MAX_WALLS` refused sessions in a row, or a wall further out than `--max-limit-wait`.
//
// Not here yet, and each is `loop.py`'s until it is: the chain switch after a reached goal, the overload
// retries that cost no attempt, rejoining a dropped stream with `--resume`, the hold and the keys, the repair and DONE-claim sessions, the doc and owner gates at a goal's end,
// the checkpoint's optimization pass, and the disk and context sweeps.
//
// `--list` and `--goal` exit 0 when the plan is read, whether or not a check matches. `--run` and
// `--goal-only` exit 0 when every check they reached is green and 1 when one is not. Each exits 2 on a
// bad argument or when there is no live goal to read. A turn exits 75 when the run goes on and 0 when it
// ends.

import { existsSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { head } from "../lib/git.ts";
import { run as runProc } from "../lib/proc.ts";
import { AGAIN, LOGDIR, RUN_ENV, RUNDIR, launch, ledger, loadRun, openingLine, runName, saveRun, type LaunchOptions, type RunState } from "../driver/launch.ts";
import { chainGoals, liveGoal } from "../lib/chain.ts";
import { installedGoal } from "../import/goals.ts";
import type { Unread } from "../import/lib.ts";
import { ROOT } from "../lib/paths.ts";
import type { RecordType } from "../lib/schema.ts";
import { loadFile } from "../lib/store.ts";
import { goal as goalType } from "../schema/goal.ts";
import { goalTable, memoResults, Session, statusRow, title, type Results } from "../driver/status.ts";
import { MAX_WALLS, type RateLimit, type Swept, Touched, backoff, hms, markInterrupted, readLimit, standingLimit, waitOutLimit, wallAfter } from "../driver/sweep.ts";
import { ENV as WRITES_ENV } from "../lib/written.ts";
import { type AcceptanceResult, type Check, GreenMemo, Sweep, acceptance, isFloor, isRelease, tiers } from "../driver/accept.ts";
import { checkName, loadRecords, units } from "../keys/checks.ts";
import { metadata } from "../keys/graph.ts";
import { keyOf } from "../keys/key.ts";
import { Tree } from "../keys/tree.ts";

export const summary = "the loop driver: one turn with no mode, or the live goal's plan: nv loop --list|--run|--goal-only [--full] [--collect] [--stage <label>] [--name <text>] [--feature <id>] | --goal";

/** The Python driver's acceptance list, the plan until the cutover deletes it. */
const LEGACY = "docs/agent/loop-goal.toml";
/** The Python driver's memo of green verdicts, and what one of its turns leaves for the next. */
const MEMO = ".loop/goal-green.json";
const RUN = ".loop/run.json";
/** This driver's memo of green verdicts, which `--goal-only` reads and writes. */
const GREEN = ".loop/accept-green.json";

type Goal = typeof goalType extends RecordType<infer T> ? T : never;

const USAGE =
  "bun nv loop [--model <m>] [--effort <e>] [--permission-mode <p>] [--max-sessions <n>] [--max-stalls <n>] [--max-retries <n>] [--max-limit-wait <seconds>] |--list|--run|--goal-only [--full] [--collect] [--stage <label>] [--name <text>] [--feature <id>] | --goal";

/** The session prompt every turn's session opens with. */
const PROMPT = "docs/agent/session-prompt.md";
/** Turns since the sweep last ran the carried floor and the release checks. */
const FLOOR_GATE = ".loop/accept-floor.json";
/**
 * One sweep in this many runs the carried floor and the release checks. A regression there waits at most
 * this many sessions to be named, and every sweep between costs the goal's own checks alone.
 */
const FLOOR_GATE_EVERY = 10;

interface Filters {
  stage?: string;
  name?: string;
  feature?: string;
  full?: boolean;
  collect?: boolean;
}

function parse(args: string[]): Filters | null {
  const out: Filters = {};
  for (let i = 1; i < args.length; i += 2) {
    const flag = args[i]!;
    if (args[0] === "--goal-only" && (flag === "--full" || flag === "--collect")) {
      out[flag === "--full" ? "full" : "collect"] = true;
      i--;
      continue;
    }
    const value = args[i + 1];
    if (value === undefined) return null;
    if (flag === "--stage") out.stage = value;
    else if (flag === "--name") out.name = value;
    else if (flag === "--feature") out.feature = value;
    else return null;
  }
  return out;
}

/** The check's name as the driver prints it: its own, or its fixture's run line. */
function nameOf(c: Check): string {
  return c.name ?? `run ${[...(c.args ?? []), c.file ?? ""].join(" ")}`.trimEnd();
}

function stageMatches(label: string, number: number, want: string): boolean {
  // A bare number names that stage alone, so `1` never reaches stages 10 to 12 through the prefix.
  const w = want.trim().toLowerCase();
  if (/^\d+$/.test(w)) return w === String(number);
  const l = label.toLowerCase();
  return l.startsWith(w) || l.slice(String(number).length + 1).startsWith(w);
}

function featureMatches(c: Check, id: string): boolean {
  const named = /^(?:proofs|dossier): (.+?)(?: \(\d+\/\d+\))?$/.exec(c.name ?? "");
  if (named?.[1] === id) return true;
  const argv = c.argv ?? [];
  return argv.some((a, i) => (a === "--group" || a === "--id") && argv[i + 1] === id);
}

/** One check as `loop.py --list` printed it: what a session reads to decide whether a `want` is producible. */
function line(c: Check, label: string): string {
  const head = `  [${label}] ${c.kind.padEnd(11)}`;
  if (c.file !== undefined) {
    // A check's own `args` go ahead of the fixture, which is the order the driver runs them in.
    return `${head} nvs ${["run", ...(c.args ?? []), c.file].join(" ")}`;
  }
  if (c.kind === "command") {
    const note = c.setup ? "  (setup: runs before the fixtures)" : c.overlap ? "  (overlap: runs beside the sweep)" : c.memoize ? "  (memoized against the tree)" : "";
    return `${head} ${nameOf(c)}: ${(c.argv ?? []).join(" ")} in ${c.cwd ?? "."}${note}`;
  }
  const driver = c.kind === "nvs-suite" ? "nvs" : "cargo";
  return `${head} ${nameOf(c)}: ${driver} ${(c.args ?? []).join(" ")}`;
}

/**
 * The live goal's plan, or why it cannot be read. While the Python driver runs, `docs/agent/loop-goal.toml`
 * is the list it runs and the sessions edit, so the plan is imported from it the way `nv import` would;
 * once that file is gone, the record under `data/goals/` is the plan.
 */
function planOf(slug: string, md: string): Goal | string {
  if (existsSync(join(ROOT, LEGACY))) {
    const unread: Unread[] = [];
    const value = installedGoal(ROOT, slug, md, unread);
    const issues = value === null ? [] : goalType.schema.validate(value);
    if (value === null || unread.length > 0 || issues.length > 0) {
      const why = unread[0] ? `${unread[0].path}: ${unread[0].reason}` : `it fails the goal schema: ${JSON.stringify(issues[0])}`;
      return `${LEGACY} does not import: ${why}`;
    }
    return value as Goal;
  }
  const path = `data/goals/${slug}.json`;
  const rec = loadFile(goalType, path);
  return rec.issues.length > 0 ? `${path} fails its schema; \`bun nv check\` names how` : rec.value;
}

/** The live goal, its plan and the chain's length, or null once the reason it has none is printed. */
function livePlan(): { live: NonNullable<ReturnType<typeof liveGoal>>; goal: Goal; total: number } | null {
  const goals = chainGoals();
  const live = liveGoal(goals);
  if (live === null) {
    console.error("nv loop: the driver's pointer names no goal on the chain, and docs/agent/loop-goal.md is a copy of none, so there is no plan to read");
    return null;
  }
  const goal = planOf(live.slug, live.md!);
  if (typeof goal === "string") {
    console.error(`nv loop: ${goal}`);
    return null;
  }
  return { live, goal, total: goals.length };
}

/** `path`'s JSON object, or an empty one when it is absent or not an object. */
function jsonObject(path: string): Record<string, unknown> {
  try {
    const v = JSON.parse(readFileSync(join(ROOT, path), "utf8"));
    return typeof v === "object" && v !== null && !Array.isArray(v) ? v : {};
  } catch {
    return {};
  }
}

function goalView(): number {
  const found = livePlan();
  if (found === null) return 2;
  const { live, goal, total } = found;
  const plan = { slug: live.slug, stages: goal.stages, checks: goal.checks as Check[] };
  const green = jsonObject(MEMO).green;
  const results = memoResults(typeof green === "object" && green !== null ? (green as Record<string, unknown>) : {}, plan.checks);
  const session = new Session(plan, results);
  session.begin(Number(jsonObject(RUN).index ?? 0) || 0);
  const locale = process.env.LC_ALL || process.env.LC_CTYPE || process.env.LANG || "";
  const utf8 = process.platform === "win32" || /utf-?8/i.test(locale);
  const width = process.stdout.columns ?? 100;
  for (const row of goalTable({ plan, results, session, position: live.num, total, commits: [], width, utf8 })) console.log(row);
  return 0;
}

/** The live goal's checks that match every filter given, with its stage labels, or null once the reason is printed. */
function selected(filters: Filters): { live: NonNullable<ReturnType<typeof liveGoal>>; goal: Goal; total: number; shown: Check[]; labelOf: (n: number) => string } | null {
  const found = livePlan();
  if (found === null) return null;
  const { live, goal } = found;
  const labels = new Map(goal.stages.map((s) => [s.number, `${s.number} ${s.title}`]));
  const labelOf = (n: number) => labels.get(n) ?? String(n);
  if (filters.stage !== undefined && !goal.stages.some((s) => stageMatches(labelOf(s.number), s.number, filters.stage!))) {
    console.error(`nv loop: no stage of \`${live.slug}\` is \`${filters.stage}\`; its stages are ${[...labels.values()].map((l) => `\`${l}\``).join(", ")}`);
    return null;
  }
  const shown = (goal.checks as Check[]).filter(
    (c) =>
      (filters.stage === undefined || stageMatches(labelOf(c.stage), c.stage, filters.stage)) &&
      (filters.name === undefined || nameOf(c).toLowerCase().includes(filters.name.toLowerCase())) &&
      (filters.feature === undefined || featureMatches(c, filters.feature)),
  );
  return { ...found, shown, labelOf };
}

function list(filters: Filters): number {
  const found = selected(filters);
  if (found === null) return 2;
  const { live, goal, total, shown, labelOf } = found;
  const path = existsSync(join(ROOT, LEGACY)) ? LEGACY : `data/goals/${live.slug}.json`;
  const checks = goal.checks as Check[];

  console.log(`${path}: ${checks.length} checks, ${goal.files.length} fixtures, goal \`${live.slug}\` (${live.num} of ${total})`);
  for (const c of shown) {
    console.log(line(c, labelOf(c.stage)));
    // Named cases are the half of a suite check a session acts on: the ones not yet on disk are the worklist.
    const cases = c.cases ?? [];
    if (cases.length > 0) {
      const absent = cases.filter((p) => !existsSync(join(ROOT, p)));
      console.log(`      ${cases.length} named case(s), ${absent.length} not written yet`);
      for (const p of absent) console.log(`        - ${p}`);
    }
  }
  const narrowed = filters.stage !== undefined || filters.name !== undefined || filters.feature !== undefined;
  if (!narrowed) console.log(`  valgrind sweep over every fixture except: ${[...(goal.env.valgrind?.skip ?? [])].sort().join(", ") || "nothing"}`);
  console.log(`list: ${shown.length} ${shown.length === 1 ? "check matches" : "checks match"}`);
  return 0;
}

/** Runs the checks the filters select, once each and in the sweep's tiers, and prints one verdict a line. */
async function runChecks(filters: Filters): Promise<number> {
  if (filters.stage === undefined && filters.name === undefined && filters.feature === undefined) {
    console.error("nv loop: --run needs --stage, --name or --feature; the whole plan is the acceptance sweep's to run");
    return 2;
  }
  const found = selected(filters);
  if (found === null) return 2;
  const { shown, labelOf } = found;
  const order = tiers(shown, labelOf).flatMap((t) => t.checks);
  const sweep = new Sweep({ stageLabel: labelOf, onRun: (what) => console.error(`  .. ${what}`) });
  let red = 0;
  for (const c of order) {
    const v = await sweep.check(c);
    if (v.fail !== "") console.log(`  FAIL  ${v.fail}`);
    else if (v.short !== "") console.log(`  SHORT ${v.short}`);
    else console.log(`  ok    ${nameOf(c)} [${labelOf(c.stage)}]`);
    if (v.fail !== "" || v.short !== "") red++;
  }
  console.log(`run: ${order.length - red} green, ${red} red, of ${order.length} ${order.length === 1 ? "check" : "checks"}`);
  return red === 0 ? 0 : 1;
}

/**
 * Each selected check's key over `tree`, by check id. The keys are `nv why`'s units, read from the same
 * plan, so a unit is paired with the goal's check at the same position. A plan the two read differently
 * keys nothing, and every check runs.
 */
async function checkKeys(goal: Goal, shown: Check[]): Promise<{ keys: Map<string, string | null>; tree: Tree } | string> {
  const graph = await metadata();
  if (!graph) return "`cargo metadata` failed, and every key needs the graph";
  const tree = await Tree.read();
  const records = loadRecords(graph);
  const all = goal.checks as Check[];
  const keys = new Map<string, string | null>();
  if (records.checks.length !== all.length || records.checks.some((raw, i) => checkName(raw) !== checkName(all[i]! as unknown as typeof raw))) {
    console.error("nv loop: the keys read the plan differently from the goal record, so the memo answers nothing this sweep");
    return { keys, tree };
  }
  const unitOf = new Map(units(records).flatMap((u) => (u.check === undefined ? [] : [[u.check, u] as const])));
  const at = new Map(all.map((c, i) => [c.id, records.checks[i]!]));
  for (const c of shown) {
    const u = unitOf.get(at.get(c.id)!);
    try {
      keys.set(c.id, u === undefined ? null : keyOf(u.name, u.parts(tree)));
    } catch {
      keys.set(c.id, null);
    }
  }
  return { keys, tree };
}

/**
 * One acceptance sweep over `checks`, with the memo read before it and written after it. `onDone` is
 * called with each check the memo answers green once the sweep ends, which is how a caller learns the
 * verdicts without a second run.
 */
async function sweepOver(
  goal: Goal,
  checks: Check[],
  labelOf: (n: number) => string,
  keyed: { keys: Map<string, string | null>; tree: Tree },
  o: { full: boolean; collect: boolean; onDone?: (c: Check, green: boolean) => void },
): Promise<{ result: AcceptanceResult; secs: number }> {
  const memo = GreenMemo.load(join(ROOT, GREEN));
  const key = (c: Check) => keyed.keys.get(c.id) ?? null;
  const started = Date.now();
  const sweep = new Sweep({ stageLabel: labelOf, onRun: (what) => console.error(`  .. ${what}`) });
  let result: AcceptanceResult;
  try {
    result = await acceptance(checks, {
      label: labelOf,
      sweep,
      key,
      memo,
      full: o.full,
      collect: o.collect,
      trace: (c, answered) => console.error(`  ${answered ? "memo" : "check"} ${nameOf(c)} [${labelOf(c.stage)}]`),
    });
  } finally {
    memo.save(join(ROOT, GREEN), new Set((goal.checks as Check[]).map((c) => c.id)));
    keyed.tree.save();
  }
  for (const c of checks) o.onDone?.(c, memo.answers(c, key(c)));
  return { result, secs: Math.round((Date.now() - started) / 1000) };
}

/** The acceptance sweep over the checks the filters select, or over the whole plan. */
async function goalOnly(filters: Filters): Promise<number> {
  const found = selected(filters);
  if (found === null) return 2;
  const { goal, shown, labelOf } = found;
  const narrowed = filters.stage !== undefined || filters.name !== undefined || filters.feature !== undefined;
  const keyed = await checkKeys(goal, shown);
  if (typeof keyed === "string") {
    console.error(`nv loop: ${keyed}`);
    return 2;
  }
  console.log(`running the acceptance sweep over ${shown.length} ${shown.length === 1 ? "check" : "checks"}${filters.full ? " (full: no memo)" : ""}${filters.collect ? " (collecting every red)" : ""}`);
  const { result, secs } = await sweepOver(goal, shown, labelOf, keyed, { full: filters.full === true, collect: filters.collect === true });
  console.log(`cost: ${secs}s, ${result.ran} run, ${result.answered} answered by the memo, of ${shown.length}`);
  if (result.fail !== "") {
    console.log(`NOT GREEN: ${result.fail}`);
    return 1;
  }
  if (narrowed) console.log("GREEN: every check the filters select passes");
  else console.log("GOAL REACHED: every acceptance check passes; the wsl leg and the valgrind sweep are still the Python driver's to run");
  return 0;
}

interface TurnFlags extends LaunchOptions {
  maxSessions: number;
  maxStalls: number;
  /** Non-zero CLI exits in a row that end the run. */
  maxRetries: number;
  /** Seconds of usage wall the turn sleeps through; a wall further out ends the run. */
  maxLimitWait: number;
}

function parseTurn(args: string[]): TurnFlags | null {
  const out: TurnFlags = { model: "opus", permissionMode: "bypassPermissions", maxSessions: Infinity, maxStalls: 10, maxRetries: 3, maxLimitWait: 6 * 3600 };
  for (let i = 0; i < args.length; i += 2) {
    const value = args[i + 1];
    if (value === undefined) return null;
    const count = Number(value);
    if (args[i] === "--model") out.model = value;
    else if (args[i] === "--effort") out.effort = value;
    else if (args[i] === "--permission-mode") out.permissionMode = value;
    else if (args[i] === "--max-sessions" && Number.isInteger(count) && count > 0) out.maxSessions = count;
    else if (args[i] === "--max-stalls" && Number.isInteger(count) && count > 0) out.maxStalls = count;
    else if (args[i] === "--max-retries" && Number.isInteger(count) && count > 0) out.maxRetries = count;
    else if (args[i] === "--max-limit-wait" && Number.isFinite(count) && count >= 0) out.maxLimitWait = count;
    else return null;
  }
  return out;
}

const MARKER = `${RUNDIR}/running`;

/** The run `.loop/running` names, or "" when no run holds the tree. */
function holder(): string {
  try {
    return /^run:\s*(\S+)/m.exec(readFileSync(join(ROOT, MARKER), "utf8"))?.[1] ?? "";
  } catch {
    return "";
  }
}

/** Local time as the ledger writes it, `2026-09-24 22:45`. */
function stamp(d = new Date()): string {
  const p = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())} ${p(d.getHours())}:${p(d.getMinutes())}`;
}

/** The run ends: said once in the ledger, the marker removed. Returns the exit code. */
function finish(state: RunState, reason: string): number {
  ledger(`## run ended ${stamp()} -- ${reason}`);
  if (holder() === state.run) rmSync(join(ROOT, MARKER), { force: true });
  saveRun(state);
  console.log(`run done: ${state.served} session(s)`);
  return 0;
}

/** Turns since the floor gate last opened; an unreadable file opens it, the safe direction. */
function floorSince(): number {
  const since = Number(jsonObject(FLOOR_GATE).since);
  return Number.isInteger(since) && since >= 0 ? since : FLOOR_GATE_EVERY;
}

/** One turn of the run; the module doc says what it does and what it does not do yet. */
async function turn(f: TurnFlags): Promise<number> {
  const byRespawn = Boolean(process.env[RUN_ENV]);
  const name = process.env[RUN_ENV] || runName();
  const held = holder();
  if (held !== "" && held !== name) {
    console.error(`nv loop: run \`${held}\` holds this tree (${MARKER}); a second driver would edit it beside that one`);
    return 2;
  }
  const { state, fresh } = loadRun(name);
  if (fresh) {
    writeFileSync(join(ROOT, MARKER), `pid:      ${process.pid}\nrun:      ${name}\nstarted:  ${stamp()}\nsessions: ${Number.isFinite(f.maxSessions) ? f.maxSessions : "uncapped"}, model ${f.model}\n`);
    ledger("");
    ledger(`## run started ${stamp()} (max ${Number.isFinite(f.maxSessions) ? `${f.maxSessions} sessions` : "uncapped"}${f.effort ? `, effort ${f.effort}` : ""}, logs ${state.run_id}-*)`);
  }
  if (state.served >= f.maxSessions) return finish(state, `hit --max-sessions (${f.maxSessions})`);
  const found = selected({});
  if (found === null) return 2;
  const { live, goal, labelOf } = found;
  const checks = goal.checks as Check[];
  const plan = { slug: live.slug, stages: goal.stages, checks };
  const results: Results = new Map();
  const green = jsonObject(GREEN).green;
  if (typeof green === "object" && green !== null) for (const id of Object.keys(green)) results.set(id, true);

  const session = new Session(plan, results);
  const tty = process.stdout.isTTY === true;
  const paint = () => {
    if (!tty) return;
    const row = statusRow(plan, results, session, process.stdout.columns ?? 100);
    process.stdout.write(`\r\x1b[2K${row}${title(row)}`);
  };
  const exe = Bun.which("claude") ?? "claude";
  const touched = new Touched();
  const sweptBy = (swept: Swept) =>
    swept.paths === 0
      ? "; the tree is clean"
      : swept.committed
        ? `; ${swept.paths} path(s) swept into a wip commit`
        : `; ${swept.paths} path(s) left uncommitted, the sweep's commit failed`;

  // A session the account refused or the CLI dropped is swept and run again, in this turn, until one is
  // served or the run ends. `index` goes up on every launch, so no log is written twice.
  let wall = standingLimit();
  let walls = 0;
  let fails = 0;
  let index = 0;
  let number = "";
  let base = "";
  for (;;) {
    if (wall !== null) {
      session.phase(`usage wall, ${hms(wall.left())} left`);
      paint();
      const stop = await waitOutLimit(wall, f.maxLimitWait, ledger);
      if (stop) return finish(state, stop);
      wall = null;
    }
    state.index++;
    index = state.index;
    number = String(index).padStart(4, "0");
    saveRun(state);
    const log = join(ROOT, LOGDIR, `${state.run_id}-${number}.log`);
    rmSync(join(ROOT, RUNDIR, "status.txt"), { force: true });
    base = await head();
    console.log(`== session ${index}  ${stamp()}`);
    const oriented = await runProc([process.execPath, join(ROOT, "tools/nv/main.ts"), "orient"], { timeoutMs: 120_000 });
    const pack = oriented.code === 0 ? oriented.stdout : "";
    if (!pack) console.log("orientation pack: `nv orient` failed, so the session runs it itself");
    const prompt = readFileSync(join(ROOT, PROMPT), "utf8");

    session.begin(index);
    touched.start();
    let latest: RateLimit | null = null;
    const launched = await launch(
      [exe],
      f,
      openingLine(prompt, pack),
      log,
      { bytes: Buffer.byteLength(pack), goal: live.slug },
      (e) => {
        latest = readLimit(e) ?? latest;
        touched.note(e);
        session.feed(e);
        paint();
      },
      { [WRITES_ENV]: touched.ledger },
    );
    if (tty) process.stdout.write("\r\x1b[2K");
    const where = log.slice(ROOT.length + 1).replace(/\\/g, "/");

    // The wall is judged before the exit code, because it explains it: a refused session exits non-zero
    // exactly like a crashed one, and neither a retry nor the tree can help with it.
    const limit = wallAfter(latest, launched.code, launched.result);
    if (limit !== null) {
      walls++;
      const swept = await markInterrupted(index, limit.describe(), touched);
      ledger(`- ${number} refused by the usage wall -- ${limit.describe()}${sweptBy(swept)} -- see ${where}`);
      if (walls >= MAX_WALLS) return finish(state, `${walls} sessions in a row were refused by the usage limit`);
      wall = limit;
      continue;
    }
    if (launched.code !== 0) {
      fails++;
      const swept = await markInterrupted(index, `the CLI exited ${launched.code}`, touched);
      ledger(`- ${number} CLI exit ${launched.code} (attempt ${fails}/${f.maxRetries})${sweptBy(swept)} -- see ${where}`);
      if (fails >= f.maxRetries) return finish(state, `claude CLI failed ${fails} times in a row`);
      const back = backoff(fails);
      console.log(`backing off ${hms(back)} before retry ${fails + 1}`);
      await Bun.sleep(back * 1000);
      continue;
    }
    break;
  }
  state.served++;
  saveRun(state);

  const statusPath = join(ROOT, RUNDIR, "status.txt");
  const line = existsSync(statusPath) ? readFileSync(statusPath, "utf8").trim() : "";
  // A session that exits zero without wrapping, cut off by the harness or out of turns, leaves its
  // unfinished slice as surely as a crashed one, and one that wrapped leaves nothing and closes an earlier
  // interruption.
  const swept = await markInterrupted(index, "it exited without wrapping", touched);
  const counted = await runProc(["git", "rev-list", "--count", `${base}..HEAD`]);
  const commits = Number(counted.stdout.trim()) || 0;
  const left = swept.left > 0 ? ` | ${swept.left} path(s) the session never wrote left in the tree` : "";
  const wip = swept.committed ? ` | ${swept.paths} path(s) swept into a wip commit` : swept.paths > 0 ? ` | ${swept.paths} path(s) left uncommitted, the sweep's commit failed` : "";
  ledger(`- ${number} ${commits} commit(s)${wip}${left} | ${line || "(no status written)"}`);

  // The sweep, over the goal's list as the session left it.
  const again = selected({});
  if (again === null) return finish(state, "the goal's plan did not read after the session");
  const all = again.goal.checks as Check[];
  const keyed = await checkKeys(again.goal, all);
  if (typeof keyed === "string") return finish(state, keyed);
  const since = floorSince() + 1;
  let open = since >= FLOOR_GATE_EVERY;
  const heldBack = open ? [] : all.filter((c) => isFloor(c, again.labelOf(c.stage)) || isRelease(c));
  const scoped = all.filter((c) => !heldBack.includes(c));
  session.phase(`acceptance sweep over ${scoped.length} checks`);
  paint();
  const onDone = (c: Check, ok: boolean) => results.set(c.id, ok);
  let { result, secs } = await sweepOver(again.goal, scoped, again.labelOf, keyed, { full: false, collect: false, onDone });
  let cost = `${secs}s over ${result.ran} check(s), ${result.answered} remembered`;
  if (result.fail === "" && heldBack.length > 0) {
    ledger(`       goal cost: ${cost}, ${heldBack.length} held (scoped; opening the floor gate)`);
    open = true;
    ({ result, secs } = await sweepOver(again.goal, all, again.labelOf, keyed, { full: false, collect: true, onDone }));
    cost = `${secs}s over ${result.ran} check(s), ${result.answered} remembered`;
  }
  writeFileSync(join(ROOT, FLOOR_GATE), `${JSON.stringify({ since: open ? 0 : since })}\n`);
  if (tty) process.stdout.write("\r\x1b[2K");
  ledger(`       goal cost: ${cost}${open ? "" : `, ${heldBack.length} held (floor gate shut)`}`);
  if (result.fail === "") {
    ledger(`## goal reached: ${live.slug} -- every check in its acceptance list passes`);
    return finish(state, `goal \`${live.slug}\` reached; switching to the next goal is still loop.py's`);
  }
  ledger(`       goal check: ${result.fail}`);
  if (line.startsWith("BLOCKED")) return finish(state, `blocked on a user decision: ${line}`);
  state.stalls = commits === 0 ? state.stalls + 1 : 0;
  if (state.stalls >= f.maxStalls) return finish(state, `${state.stalls} sessions in a row produced no commit`);
  if (state.served >= f.maxSessions) return finish(state, `hit --max-sessions (${f.maxSessions})`);
  if (!byRespawn) return finish(state, `one turn: ${RUN_ENV} is not set, so nothing starts a next one`);
  saveRun(state);
  return AGAIN;
}

export async function run(args: string[]): Promise<number> {
  if (args.length === 1 && args[0] === "--goal") return goalView();
  const mode = args[0];
  if (mode === undefined || !["--list", "--run", "--goal-only"].includes(mode)) {
    const flags = parseTurn(args);
    if (flags === null) {
      console.error(`usage: ${USAGE}`);
      return 2;
    }
    return turn(flags);
  }
  const filters = mode === "--list" || mode === "--run" || mode === "--goal-only" ? parse(args) : null;
  if (filters === null) {
    console.error(`usage: ${USAGE}`);
    return 2;
  }
  if (mode === "--goal-only") return goalOnly(filters);
  return mode === "--run" ? runChecks(filters) : list(filters);
}
