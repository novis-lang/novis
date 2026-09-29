// What a turn does with a session that ended before it wrapped, and with one the account refused.
//
// **The wip sweep.** A session cut off mid-slice has committed every slice it finished, but the one it was
// in the middle of is still in the tree, and the handoff it never reached does not mention it. The next
// session cannot tell those files from the state it was meant to start in, and a run that ends there
// leaves them for nobody. So `markInterrupted` commits them under `SWEEP_SUBJECT`, which `nv orient` and
// `holes` match, and writes `.loop/interrupted.json` naming that commit; `nv orient` puts it at the top of
// the next pack. The commit has not been through `nv verify`, and says so. A commit is recoverable and
// reviewable, and a dirty tree is neither.
//
// It takes the session's own paths and nothing else, because a person works in this tree beside the loop.
// A path is the session's only when something watched it being written: `Touched.note` reads the target
// off every `Write`, `Edit`, `MultiEdit` and `NotebookEdit` call on the event stream, and a tool that
// writes behind a shell call (`nv splice`, the reference generator, `nv proofs`'s blessed `.out` files
// and perf ledger) records its paths in
// `.loop/written.txt` through `lib/written.ts`. Every other dirty path is left where it is and listed as
// `left`. A sweep with nothing of its own to take commits nothing and deletes `interrupted.json`, which
// is how a session that ends clean closes an earlier interruption. A subagent's writes reach neither
// watcher, so they are reported as left rather than taken.
//
// **The usage wall.** A `rate_limit_event` whose status is `rejected` and whose `resetsAt` is still ahead
// is a wall: the session is swept, and the turn sleeps until the window reopens and runs the session
// again. The event is parsed and never grepped, since a healthy one already carries the word `rejected`
// under another key. When a session exits non-zero with the limit named only in the text of its terminal
// `result` event, the wall is taken to reopen `BLIND_WAIT` from now, and the next session's own event
// carries the real deadline. `.loop/limit.json` keeps a wall's deadline across turns, so a driver killed
// during one does not walk straight back into it.
//
// **An overload and a dropped stream.** Two more exits are not crashes. A `529` in the terminal event's
// `api_error_status` is a busy server: the session is swept and run again after `OVERLOAD_BACKOFF`, for as
// long as it takes, costing no retry. A stream that died mid-answer (`streamDropped`) left a healthy session
// whose transcript is whole, so it is rejoined with `claude --resume` and `RESUME_PROMPT`, the tree left as
// it was, up to `MAX_RESUMES` times; after that it is swept like any failed session.

import { existsSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { isAbsolute, join, resolve } from "node:path";
import { run } from "../lib/proc.ts";
import { ROOT, rel } from "../lib/paths.ts";
import { RUNDIR } from "./launch.ts";

/** The subject of the commit a sweep leaves behind, which `nv orient` and `holes` match. */
export const SWEEP_SUBJECT = "wip(loop): the unfinished slice of";
/** The ledger of paths the session's own tools wrote, which `lib/written.ts` appends to. */
export const WRITTEN = `${RUNDIR}/written.txt`;
export const INTERRUPTED = `${RUNDIR}/interrupted.json`;
export const LIMIT = `${RUNDIR}/limit.json`;

/** Walls in a row after which a turn ends the run: an account that stays refused never serves a session. */
export const MAX_WALLS = 8;
/** Seconds to a wall's reopening when the limit was named in text and no event carried a deadline. */
export const BLIND_WAIT = 1800;
/** A usage limit named in a terminal `result` event's text; never matched against a tool result. */
export const LIMIT_TEXT = /usage limit reached|rate_limit_error/i;

/** Seconds to back off after the `fails`th non-zero exit in a row. */
export function backoff(fails: number): number {
  return Math.min(300, 30 * 2 ** fails);
}

/** `1h02m`, `12m05s` or `40s`. */
export function hms(seconds: number): string {
  const s = Math.max(0, Math.round(seconds));
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const p = (n: number) => String(n).padStart(2, "0");
  if (h > 0) return `${h}h${p(m)}m`;
  return m > 0 ? `${m}m${p(s % 60)}s` : `${s}s`;
}

/** Local time as the ledger writes it, `2026-09-24 22:45`, with seconds when `seconds` is set. */
function stamp(d: Date, seconds = false): string {
  const p = (n: number) => String(n).padStart(2, "0");
  const day = `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())} ${p(d.getHours())}:${p(d.getMinutes())}`;
  return seconds ? `${day}:${p(d.getSeconds())}` : day;
}

/** A tool call's `file_path` in the spelling `git status` uses: repo-relative, forward slashes. */
export function repoPath(text: string, root: string = ROOT): string {
  const path = isAbsolute(text) ? rel(resolve(text), root) : text.replace(/\\/g, "/").replace(/^\.\//, "");
  return path.startsWith("../") ? text.replace(/\\/g, "/") : path;
}

/** The dirty paths the session wrote, as opposed to the ones a person has open beside it. */
export class Touched {
  static readonly WRITES = new Set(["Write", "Edit", "MultiEdit", "NotebookEdit"]);
  private written = new Set<string>();

  constructor(private readonly root: string = ROOT) {}

  /** The ledger's absolute path, which the session's environment names. */
  get ledger(): string {
    return join(this.root, WRITTEN);
  }

  /**
   * A session is about to launch: forget the last one's paths and empty the tools' ledger. A rejoined session
   * is the same session, so `carry` keeps both.
   */
  start(carry = false): void {
    if (carry) return;
    this.written.clear();
    try {
      writeFileSync(this.ledger, "");
    } catch {
      // An unwritable ledger costs the sweep the tools' paths, which it then reports as left.
    }
  }

  /** One stream event: every write tool's target in an assistant message. */
  note(e: Record<string, any>): void {
    if (e.type !== "assistant" || !Array.isArray(e.message?.content)) return;
    for (const block of e.message.content) {
      if (block?.type !== "tool_use" || !Touched.WRITES.has(String(block.name))) continue;
      for (const key of ["file_path", "notebook_path"]) {
        const value = block.input?.[key];
        if (typeof value === "string" && value.trim()) this.written.add(repoPath(value, this.root));
      }
    }
  }

  /** Every path either watcher saw. */
  paths(): Set<string> {
    const seen = new Set(this.written);
    try {
      for (const line of readFileSync(this.ledger, "utf8").split("\n")) if (line.trim()) seen.add(line.trim());
    } catch {
      // No ledger means no tool wrote behind a shell call.
    }
    return seen;
  }
}

export interface Entry {
  code: string;
  path: string;
}

/** `git status`'s dirty paths with their two-letter codes; a rename contributes both of its paths. */
export async function dirtyEntries(root: string = ROOT): Promise<Entry[]> {
  const r = await run(["git", "status", "--porcelain=v1", "-z", "--untracked-files=all"], { cwd: root, timeoutMs: 60_000 });
  if (r.code !== 0) return [];
  const fields = r.stdout.split("\0").filter((f) => f !== "");
  const out: Entry[] = [];
  for (let i = 0; i < fields.length; i++) {
    const f = fields[i]!;
    if (f.length < 4) continue;
    const code = f.slice(0, 2);
    out.push({ code, path: f.slice(3) });
    if ((code.includes("R") || code.includes("C")) && i + 1 < fields.length) out.push({ code, path: fields[++i]! });
  }
  return out;
}

export interface Swept {
  /** The session's own paths, committed when `committed` is set. */
  paths: number;
  committed: boolean;
  /** Dirty paths the session never wrote, left in the tree. */
  left: number;
}

/**
 * Commits what session `index` left uncommitted of its own, and records it in `interrupted.json`. `why` is
 * the sentence that says what cut the session off. With nothing of the session's own dirty, it commits
 * nothing and deletes `interrupted.json`.
 */
export async function markInterrupted(index: number, why: string, touched: Touched, root: string = ROOT, now = new Date()): Promise<Swept> {
  const seen = touched.paths();
  const entries = await dirtyEntries(root);
  const ours = entries.filter((e) => seen.has(e.path));
  const theirs = entries.filter((e) => !seen.has(e.path));
  const left = theirs.map((e) => `${e.code} ${e.path}`);
  if (ours.length === 0) {
    rmSync(join(root, INTERRUPTED), { force: true });
    return { paths: 0, committed: false, left: theirs.length };
  }
  const paths = [...new Set(ours.map((e) => e.path))];
  const whose = `session ${String(index).padStart(4, "0")}`;
  const body =
    `${SWEEP_SUBJECT} ${whose}\n\n` +
    `The session ended before it wrapped -- ${why} -- so this is what it had in the\n` +
    `tree at that moment, committed by the driver rather than left for the next one to\n` +
    "find as an unexplained diff. It has NOT been through `nv verify`.\n\n" +
    (left.length > 0
      ? `${left.length} other path(s) were dirty and are NOT in this commit -- the session never\n` +
        "wrote them, so they are somebody else's work and are still in the tree.\n\n"
      : "") +
    "`.loop/interrupted.json` names this commit; `nv orient` puts it at the top of the\n" +
    "next session's pack. Continue it, amend it or revert it -- but read it first.\n";
  const git = (args: string[]) => run(["git", ...args], { cwd: root, timeoutMs: 120_000 });
  const before = (await git(["rev-parse", "HEAD"])).stdout.trim();
  // By path, never `-A`, and `--only`, so a person's own staged edit stays out of the commit. The `add`
  // is still needed: `--only` reaches an untracked path only once something has put it in the index.
  // No `--no-verify`: the commit-msg hook judges this message like any other.
  await git(["add", "--", ...paths]);
  const msg = join(root, RUNDIR, "sweep-msg.txt");
  writeFileSync(msg, body);
  await git(["commit", "-F", msg, "--only", "--", ...paths]);
  rmSync(msg, { force: true });
  const swept = (await git(["rev-parse", "HEAD"])).stdout.trim();
  const committed = swept !== "" && swept !== before;
  // Unstaged again by path, so a failed commit leaves the tree as it was found and a person's staged
  // work as they staged it.
  if (!committed) await git(["reset", "-q", "--", ...paths]);
  writeFileSync(
    join(root, INTERRUPTED),
    `${JSON.stringify({ session: index, when: stamp(now, true), why, head: swept, swept: committed, files: ours.map((e) => `${e.code} ${e.path}`), left }, null, 1)}\n`,
  );
  return { paths: ours.length, committed, left: theirs.length };
}

/** One `rate_limit_event`'s `rate_limit_info`: the last thing a session said about the account. */
export class RateLimit {
  readonly status: string;
  readonly kind: string;
  readonly utilization: unknown;
  /** Epoch seconds, or 0 when no deadline was given. */
  readonly resetsAt: number;

  constructor(info: Record<string, unknown> | null | undefined) {
    const i = info ?? {};
    this.status = String(i.status ?? "");
    this.kind = String(i.rateLimitType ?? "");
    this.utilization = i.utilization;
    const at = Number(i.resetsAt ?? 0);
    this.resetsAt = Number.isFinite(at) ? Math.trunc(at) : 0;
  }

  /** Seconds until the window reopens, or 0 when no deadline was given. */
  left(now = Date.now()): number {
    return this.resetsAt ? this.resetsAt - now / 1000 : 0;
  }

  /**
   * Refused, with a reset still ahead. The CLI leaves the last value standing when a window turns over, so
   * a rejection whose reset has passed is stale and is no wall.
   */
  blocked(now = Date.now()): boolean {
    return this.status === "rejected" && this.left(now) > 0;
  }

  when(): string {
    return this.resetsAt ? stamp(new Date(this.resetsAt * 1000)) : "?";
  }

  describe(now = Date.now()): string {
    const state = this.status === "rejected" ? "is reached" : this.status === "allowed_warning" ? "is close" : "is fine";
    const pct = this.utilization;
    const used = typeof pct === "number" && pct >= 0 && pct <= 1 ? `, ${Math.round(pct * 100)}% used` : "";
    return `the ${(this.kind || "usage").replace(/_/g, "-")} usage window ${state}${used}; it resets at ${this.when()}, ${hms(this.left(now))} from now`;
  }
}

/** The event as a `RateLimit`, when it is a `rate_limit_event`. */
export function readLimit(e: Record<string, any>): RateLimit | null {
  return e.type === "rate_limit_event" ? new RateLimit(e.rate_limit_info) : null;
}

/**
 * The wall a session ended against, or null. `latest` is the last `rate_limit_event` it saw, `code` its
 * exit and `result` its terminal event, whose text is the fallback when no event carried a deadline.
 */
export function wallAfter(latest: RateLimit | null, code: number, result: Record<string, unknown> | null, now = Date.now()): RateLimit | null {
  if (latest?.blocked(now)) return latest;
  if (code !== 0 && result !== null && LIMIT_TEXT.test(JSON.stringify(result))) {
    return new RateLimit({ status: "rejected", resetsAt: Math.trunc(now / 1000) + BLIND_WAIT });
  }
  return null;
}

/** The HTTP statuses a terminal `result` event blames when the server was busy rather than the account refused. */
export const OVERLOAD_STATUS = new Set([529]);
/**
 * Seconds before the nth re-run in a row of an overloaded session; the last entry repeats for as long as the
 * overload lasts. It opens at a minute because the CLI has already fought the same 529 before it exited.
 */
export const OVERLOAD_BACKOFF = [60, 120, 300, 600];
/** Resumes in a row of one dropped session before it is swept and a fresh one started. */
export const MAX_RESUMES = 3;
/** Seconds before the nth resume in a row of a dropped session; the last entry repeats. */
export const RESUME_BACKOFF = [15, 30, 60];
/**
 * What a rejoined session is told in place of the session prompt, which is already the first turn of the
 * transcript it replays: nothing moved under it, and its last call may or may not have landed.
 */
export const RESUME_PROMPT =
  "The connection to the API dropped mid-response and this session was rejoined with `--resume`, so the conversation above is yours and you are continuing it.\n\n" +
  "Nothing moved under you. The working tree is exactly as you left it and the driver committed nothing on your behalf. Your last tool call may or may not have landed -- read back whatever it touched rather than assuming either way.\n\n" +
  "Pick up where you stopped and finish the session the way the prompt at the top told you to, ending with the wrap. Do not re-orient, and do not restart the group.";

/** The nth entry of a backoff table, the last one repeating. */
export function nthWait(table: number[], n: number): number {
  return table[Math.min(Math.max(n, 1), table.length) - 1]!;
}

/** The HTTP status a terminal `result` event blames for the exit, or 0. Read off that one event, never grepped. */
export function apiErrorStatus(result: Record<string, unknown> | null): number {
  const status = Number(result?.api_error_status ?? 0);
  return Number.isInteger(status) ? status : 0;
}

/**
 * Whether the terminal `result` event blames a connection that died mid-answer: it is an error, its
 * `terminal_reason` is `api_error`, and it carries no status, since nothing answered to supply one. A refusal
 * carries a status, and the two need opposite recoveries.
 */
export function streamDropped(result: Record<string, unknown> | null): boolean {
  return result !== null && result.is_error === true && result.terminal_reason === "api_error" && !result.api_error_status;
}

/** Writes the wall's deadline to `.loop/limit.json`. Best effort: a lost file costs one refused session. */
export function rememberLimit(limit: RateLimit, root: string = ROOT, now = new Date()): void {
  try {
    writeFileSync(join(root, LIMIT), `${JSON.stringify({ resets_at: limit.resetsAt, kind: limit.kind, status: limit.status, noted: stamp(now, true) }, null, 1)}\n`);
  } catch {
    // As above.
  }
}

/** The wall `.loop/limit.json` still names, or null once it has turned over, when the file is deleted. */
export function standingLimit(root: string = ROOT, now = Date.now()): RateLimit | null {
  const path = join(root, LIMIT);
  if (!existsSync(path)) return null;
  try {
    const entry = JSON.parse(readFileSync(path, "utf8"));
    const limit = new RateLimit({ status: "rejected", resetsAt: entry?.resets_at, rateLimitType: entry?.kind });
    if (limit.blocked(now)) return limit;
  } catch {
    // An unreadable file is no wall.
  }
  rmSync(path, { force: true });
  return null;
}

/**
 * Sleeps until the wall reopens, with a minute's margin for the server's clock. Returns "" when the run may
 * go on, or the reason it ends: a wall further out than `maxWait` seconds is not slept through.
 */
export async function waitOutLimit(
  limit: RateLimit,
  maxWait: number,
  say: (line: string) => void,
  root: string = ROOT,
  sleep: (ms: number) => Promise<void> = Bun.sleep,
): Promise<string> {
  const left = limit.left() + 60;
  if (left <= 0) {
    rmSync(join(root, LIMIT), { force: true });
    return "";
  }
  if (left > maxWait) {
    return `${limit.describe()} -- further out than --max-limit-wait (${hms(maxWait)}), so the run stops here rather than sleeping through it; every session committed its own slices, and a restart after ${limit.when()} picks up from the handoff`;
  }
  rememberLimit(limit, root);
  say(`       usage wall: ${limit.describe()}; waiting ${hms(left)}`);
  await sleep(left * 1000);
  rmSync(join(root, LIMIT), { force: true });
  return "";
}
