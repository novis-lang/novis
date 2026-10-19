// `exec`: the recorded jobs of the project in the current directory, read and ended straight from its
// `.looper/jobs/` (`../jobs/store.ts`), as `crates/looper/src/cmd/exec.rs` prints them.
//
// - `exec list` prints the running jobs, newest first, one line each: the id, the state, the kind, the
//   running time, the project, the session and the command. `--all` adds the ended and lost ones.
//   `--project` keeps the jobs whose project root is the path or ends in the name, `--session` those
//   whose session starts with the text, and `--port <n>` the running ones with a process in their Job
//   Object that listens on TCP port `n`. When looper's service answers, the jobs are followed by
//   each heavy-job slot's holder or `free`, and each asker that waits for a slot (`../jobs/slots.ts`),
//   whatever the filters.
// - `exec log <id>` prints the job's log as bytes. `--from <n>` starts at the 1-based line `n`,
//   `--lines <n>` prints `n` lines, and `--lines` without `--from` prints the last `n`.
// - `exec kill <id>` ends a running job's whole tree, after recording who asked (`asker`).
// - `exec wait <id>` reads the job's folder every `WAIT_EVERY_MS` until it has exited or is lost. It
//   prints the exit code, the running time and the last `WAIT_LINES` lines of the log, and exits with
//   the job's code. A lost job, and a code outside 0 to 255, exit 1.
// - `exec progress <id>` prints the job's progress (`../jobs/progress.ts`): a line of totals, the time
//   left when the steps have a history, then one line per item of the tree, indented by its depth. Each
//   line holds the item's id and name, its state, its count and its time. A child job's items stand
//   under the item that started it.
//
// With `--json`, `list` prints `{"jobs", "slots"}` with `slots` null without the service, `log` `{"id", "lines"}`, `kill` `{"id", "killed", "by",
// "command"}`, `wait` `{"id", "state", "code", "lines"}` and `progress` the `View`. A refusal exits 1.
//
// # The bounds it keeps
//
// - `list` holds one entry per job folder, which the store's sweep bounds.
// - `log --lines` without `--from` holds at most that many lines, and `wait` at most `WAIT_LINES`.
// - `progress` holds one view, within the bounds of `../jobs/progress.ts`.

import { looperHome, type Seen, seenSlots } from "../jobs/slots.ts";
import { type Entry, jobName, type JobState, storeOf } from "../jobs/store.ts";
import { jobPids, listeners } from "../os/index.ts";
import { type Item, MAX_BYTES, MAX_ITEMS, tookMs } from "../jobs/fold.ts";
import { type Lines, readLines } from "../jobs/log.ts";
import { view } from "../jobs/progress.ts";
import { asker } from "./claims.ts";
import { type Command, Refusal, type Report, usage } from "./cli.ts";

/** How often `wait` reads the job's folder. */
export const WAIT_EVERY_MS = 250;
/** The lines of the log `wait` prints. */
export const WAIT_LINES = 40;
/** The longest command `list` prints on its line, in characters. */
const COMMAND_WIDTH = 100;

/** `ms` as the largest whole unit: `42s`, `5m`, `3h` or `2d`. */
export function span(ms: number): string {
  const s = Math.floor(Math.max(0, ms) / 1000);
  if (s < 60) return `${s}s`;
  if (s < 3600) return `${Math.floor(s / 60)}m`;
  if (s < 86_400) return `${Math.floor(s / 3600)}h`;
  return `${Math.floor(s / 86_400)}d`;
}

/** `command` on one line, cut to `COMMAND_WIDTH` characters. */
export function oneLine(command: string): string {
  const flat = command.split(/\s+/).filter((w) => w !== "").join(" ");
  const chars = [...flat];
  return chars.length > COMMAND_WIDTH ? `${chars.slice(0, COMMAND_WIDTH).join("")}…` : flat;
}

/** The last path segment of `path`, or `path` when it has none. */
function base(path: string): string {
  return path.split(/[/\\]/).filter((s) => s !== "").pop() ?? path;
}

/** Whether the project root `root` is `asked`, as a path or as its folder's name. */
function namesProject(root: string, asked: string): boolean {
  const norm = (p: string) => p.replaceAll("\\", "/").replace(/\/+$/, "").toLowerCase();
  const [r, a] = [norm(root), norm(asked)];
  return r === a || r.split("/").pop() === a;
}

/** Takes `--<name> <value>` out of `args`, or gives undefined when it is absent. */
function option(args: string[], name: string): string | undefined {
  const at = args.indexOf(`--${name}`);
  if (at < 0) return undefined;
  const value = args[at + 1];
  if (value === undefined || value.startsWith("--")) throw usage(`\`--${name}\` takes a value`);
  args.splice(at, 2);
  return value;
}

/** Takes the flag `--<name>` out of `args`, and says whether it was there. */
function flag(args: string[], name: string): boolean {
  const at = args.indexOf(`--${name}`);
  if (at >= 0) args.splice(at, 1);
  return at >= 0;
}

/** A count of at least 1 from `value`, or a usage refusal. */
function count(value: string, name: string): number {
  if (!/^\d+$/.test(value) || Number(value) < 1) throw usage(`\`--${name}\` takes a whole number of 1 or more`);
  return Number(value);
}

/** The one id `args` holds and nothing else. */
function onlyId(args: string[], action: string): string {
  const [id, ...rest] = args;
  if (id === undefined || rest.length > 0 || id.startsWith("--")) throw usage(`\`exec ${action}\` takes one job id`);
  return id;
}

function stateName(s: JobState): string {
  return s.state;
}

/** Whether job `e` runs and its Job Object holds one of the pids in `listening`. */
function listensOn(e: Entry, listening: readonly number[]): boolean {
  return e.state.state === "running" && jobPids(jobName(e.id)).some((pid) => listening.includes(pid));
}

async function list(args: string[]): Promise<Report> {
  const all = flag(args, "all");
  const project = option(args, "project");
  const session = option(args, "session");
  const port = option(args, "port");
  if (port !== undefined && (!/^\d+$/.test(port) || Number(port) < 1 || Number(port) > 65_535)) throw usage("`--port` takes a port from 1 to 65535");
  if (args.length > 0) throw usage(`\`exec list\` does not take \`${args[0]}\``);
  const listening = port === undefined ? [] : listeners().filter(([p]) => p === Number(port)).map(([, pid]) => pid);
  const shown = storeOf(process.cwd())
    .list()
    .filter((e) => all || e.state.state === "running")
    .filter((e) => project === undefined || (e.record.project !== null && namesProject(e.record.project, project)))
    .filter((e) => session === undefined || (e.record.session?.startsWith(session) ?? false))
    .filter((e) => port === undefined || listensOn(e, listening));
  const now = Date.now();
  const docs = shown.map((e) => ({
    id: e.id,
    state: stateName(e.state),
    ...e.record,
    exit: e.state.state === "exited" ? e.state.exit.code : null,
    ended: e.state.state === "exited" ? e.state.exit.ended : null,
  }));
  const lines = shown.map((e) => listLine(e, now));
  if (lines.length === 0) lines.push(all ? "No jobs." : "No running jobs.");
  const slots = await seenSlots(looperHome());
  if (slots !== null) lines.push(...slotLines(slots));
  return { doc: { jobs: docs, slots }, text: lines.join("\n") };
}

/** Each slot's holder or `free`, then each asker that waits, as `exec list` prints them under the jobs. */
function slotLines(seen: Seen): string[] {
  const lines: string[] = [];
  for (let index = 0; index < seen.count; index++) {
    const held = seen.held.find((h) => h.index === index);
    lines.push(`slot ${index}  ${held === undefined ? "free" : (held.job ?? `lease ${held.lease}`)}`);
  }
  for (const w of seen.waiting) {
    const project = w.project === null ? "-" : base(w.project);
    lines.push(`waiting  ${w.kind}  ${project}  ${oneLine(w.command)}`);
  }
  return lines;
}

function listLine(e: Entry, now: number): string {
  const r = e.record;
  const state = e.state.state === "exited" ? `exit ${e.state.exit.code}` : e.state.state;
  const until = e.state.state === "exited" ? e.state.exit.ended : now;
  const project = r.project === null ? "-" : base(r.project);
  const session = r.session === null ? "-" : r.session.slice(0, 8);
  return `${e.id}  ${state.padEnd(8)}  ${r.kind.padEnd(7)}  ${span(until - r.started).padStart(6)}  ${project}  ${session}  ${oneLine(r.command)}`;
}

/** The log's chosen lines as text, each without its line break. */
function textLines(dir: string, pick: Lines): string[] {
  const out: string[] = [];
  readLines(dir, pick, (l) => out.push(l.toString("utf8").replace(/\r?\n$/, "")));
  return out;
}

function log(args: string[], json: boolean): Report {
  const from = option(args, "from");
  const lines = option(args, "lines");
  const id = onlyId(args, "log");
  const store = storeOf(process.cwd());
  if (!store.record(id)) throw new Refusal(`no job has the id ${id}`);
  const pick: Lines =
    from !== undefined
      ? { from: count(from, "from"), ...(lines === undefined ? {} : { count: count(lines, "lines") }) }
      : lines !== undefined
        ? { last: count(lines, "lines") }
        : { all: true };
  if (json) return { doc: { id, lines: textLines(store.dir(id), pick) } };
  readLines(store.dir(id), pick, (l) => void process.stdout.write(l));
  return { own: true };
}

function kill(args: string[]): Report {
  const id = onlyId(args, "kill");
  const by = asker();
  const done = storeOf(process.cwd()).kill(id, by);
  if (typeof done === "string") throw new Refusal(`job ${id} was not killed: ${done}`);
  return { doc: { id, killed: true, by, command: done.command }, text: `Killed job ${id}: ${oneLine(done.command)}` };
}

async function wait(args: string[]): Promise<Report> {
  const id = onlyId(args, "wait");
  const store = storeOf(process.cwd());
  const record = store.record(id);
  if (!record) throw new Refusal(`no job has the id ${id}`);
  let state = store.state(id, record);
  while (state.state === "running") {
    await Bun.sleep(WAIT_EVERY_MS);
    state = store.state(id, record);
  }
  const lines = textLines(store.dir(id), { last: WAIT_LINES });
  const code = state.state === "exited" ? state.exit.code : null;
  const head =
    state.state === "exited"
      ? `Job ${id} exited with ${state.exit.code} after ${span(state.exit.ended - record.started)}.`
      : `Job ${id} is lost: its runner ended without recording an exit.`;
  const exit = code !== null && Number.isInteger(code) && code >= 0 && code <= 255 ? code : 1;
  return { code: exit, doc: { id, state: stateName(state), code, lines }, text: [head, ...lines].join("\n") };
}

/** `ms` to a tenth of a second under a minute, else in minutes and seconds, or hours and minutes. */
export function took(ms: number): string {
  const whole = Math.max(0, Math.floor(ms));
  const s = Math.floor(whole / 1000);
  const two = (n: number) => String(n).padStart(2, "0");
  if (s < 60) return `${s}.${Math.floor((whole % 1000) / 100)}s`;
  if (s < 3600) return `${Math.floor(s / 60)}m${two(s % 60)}s`;
  return `${Math.floor(s / 3600)}h${two(Math.floor((s % 3600) / 60))}m`;
}

/** One item on one line, as of `now`: its id and name, its state, its count and its time. */
function itemLine(item: Item, now: number): string {
  let line = item.id;
  if (item.name !== null && item.name !== item.id) line += ` (${oneLine(item.name)})`;
  const counted = item.done === null ? "" : item.total === null ? ` ${item.done}` : ` ${item.done}/${item.total}`;
  switch (item.state) {
    case "planned":
    case "kept":
    case "dropped":
      line += `  ${item.state}`;
      break;
    case "waiting":
      line += `  waiting on ${item.wait === null ? "-" : oneLine(item.wait)}`;
      break;
    case "running":
      line += `  running${counted}, ${took(item.started === null ? 0 : now - item.started)}`;
      break;
    case "ended": {
      line += `  ${item.result ?? "green"}${counted}`;
      const ms = tookMs(item);
      if (ms !== null) line += ` in ${took(ms)}`;
      if (item.why !== null) line += `: ${oneLine(item.why)}`;
      break;
    }
  }
  if (item.attempts > 1) line += `, attempt ${item.attempts}`;
  return line;
}

export function progress(args: string[]): Report {
  const id = onlyId(args, "progress");
  const store = storeOf(process.cwd());
  const now = Date.now();
  const record = store.record(id);
  const v = record ? view(store, id, record.project, now) : null;
  if (v === null) throw new Refusal(`no job has the id ${id}`);
  const t = v.totals;
  const counts: [number, string][] = [
    [t.green, "green"],
    [t.red, "red"],
    [t.running, "running"],
    [t.waiting, "waiting"],
    [t.planned, "planned"],
    [t.kept, "kept"],
    [t.skipped, "skipped"],
    [t.cancelled, "cancelled"],
  ];
  const lines = [`Job ${id} ${v.state}: ${t.items} items${counts.filter(([n]) => n > 0).map(([n, name]) => `, ${n} ${name}`).join("")}.`];
  if (v.estimate !== null) {
    lines.push(`About ${took(v.estimate.left_ms)} left, from the history of ${v.estimate.rests_on} of ${v.estimate.items} steps.`);
  }
  if (v.note !== null) lines.push(oneLine(v.note));
  for (const item of v.items) lines.push(`${"  ".repeat(item.depth)}${itemLine(item, now)}`);
  if (v.over > 0) lines.push(`${v.over} more items are past the bound of ${MAX_ITEMS}.`);
  if (v.truncated) lines.push(`The events passed ${MAX_BYTES >> 20} MiB, so the rest was not read.`);
  if (v.skipped_lines > 0) lines.push(`${v.skipped_lines} lines were skipped: too long or not an event.`);
  return { doc: v, text: lines.join("\n") };
}

export const execCommand: Command = {
  name: "exec",
  about: "Lists, prints, kills, waits for and shows the progress of this project's recorded jobs: `exec list|log|kill|wait|progress`.",
  json: true,
  run: (args, ctx) => {
    const [action, ...rest] = args;
    switch (action) {
      case "list":
        return list(rest);
      case "log":
        return log(rest, ctx.json);
      case "kill":
        return kill(rest);
      case "wait":
        return wait(rest);
      case "progress":
        return progress(rest);
      default: {
        const takes = "`exec` takes `list`, `log`, `kill`, `wait` or `progress`";
        throw usage(action === undefined ? takes : `${takes}, and got \`${action}\``);
      }
    }
  },
};
