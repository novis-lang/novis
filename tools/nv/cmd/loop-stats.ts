// `bun nv loop-stats`: what the last run's sessions actually cost, measured out of `.loop/logs/*.log`.
//
//     bun nv loop-stats                      per session, the constants, the head, the drift, the projection
//     bun nv loop-stats --attribute          where the context went, by what fetched it, and what was delegated
//     bun nv loop-stats --guard              the guard's denials per session and per rule
//     bun nv loop-stats --calibrate [--write]  bytes per token for the orientation pack, regressed
//     bun nv loop-stats --json               every session's measurements as one object
//     bun nv loop-stats --run <stamp>        any of the above over one run's logs
//
// Every number the loop's design rests on -- how long a session takes, how much of it is fixed cost,
// how fast context grows, how many slices fit under the ceiling -- is a measurement of this repository
// on this machine with this model, not a constant, and all three change. So this re-derives them from
// the NDJSON transcripts rather than restating them, and `docs/agent/loop-authoring.md` § *Measure
// first* makes running it the first step of setting any new goal. It stores no facts and enforces
// nothing: if its projection disagrees with what the session prompt says, the prompt is stale.
//
// Per session it measures:
//
//   calls       tool calls, the unit a session's wall clock is proportional to
//   calls/msg   tool calls per assistant message; 1.00 means no message carried two
//   cmd/call    commands per shell call. A `;`/`&&` chain is batching too, so read the two together
//   head        calls before the first edit: orientation, paid once per session
//   work        calls between the first edit and the tail: the part that ships
//   tail        calls after the last edit: collecting the verification and applying the wrap
//   ctx         context at the first assistant message and the largest after it
//   compaction  a context drop, which is the failure a group must stay under
//   guard       the guard's denials, by rule. A habit the guard exists to break shows as a rule whose
//               count rises again after it had fallen

import { existsSync, mkdirSync, readdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { basename, join } from "node:path";
import { ROOT } from "../lib/paths.ts";
import { run as runProc } from "../lib/proc.ts";
import { ArgError, fixed, parseArgs } from "../lib/py.ts";

export const summary =
  "what the loop's sessions cost, out of .loop/logs: nv loop-stats [--attribute|--guard|--calibrate [--write]|--json] [--run <stamp>]";

const USAGE =
  "usage: bun nv loop-stats [--run RUN] [--attribute] [--guard] [--context-ceiling N] [--json] [--calibrate] [--write]";

const LOGDIR = join(ROOT, ".loop", "logs");
const CALIBRATION = join(ROOT, "tools", "data", "calibration.json");

/**
 * A tail begins after the last edit, and a verification call is the fallback boundary for a session that
 * edited nothing. `verify.py` stays in the list because the logs of older runs name it.
 */
const VERIFY_MARKERS = ["nv verify", "verify.py", "tools/verify"];
const MUTATORS = ["Edit", "Write", "NotebookEdit"];
const SHELLS = ["Bash", "PowerShell"];

/** A top-level command separator inside one shell call. A pipe is one command, and `||` is not counted as two. */
const SEPARATOR_RE = /(?<!\|)(?:;|&&)(?!\|)/g;

/** The text of a call that applied a splice patch. `splice.py` is the Python tool older logs name. */
const SPLICE_MARKERS = ["nv splice", "splice.py"];

/**
 * A shell call carrying file content, which AGENTS.md rule 1 forbids: a heredoc, a `sed -i`, or a
 * `>`/`>>` redirect to a path. `2>&1`, `>&2` and `/dev/null` redirect a stream, not content, and are
 * not counted. None of these fires `MUTATORS`, so without this count a session patching the tree
 * through the shell reads as one that made no edits at all.
 */
const SHELL_WRITE_RE = /<<-?\s*['"]?\w+|\bsed\s+(?:[^|;&]*\s)?-i\b|(?<![0-9&<>])>>?\s*(?!&|\/dev\/null)[^\s|;&<>]+/;

/**
 * The context a session must stay under. It is a quality limit, not a capacity one: an agent starts
 * missing what it has read well before its window is full. The window the transcripts report is read
 * only to catch the case where it is smaller than this and capacity binds first.
 */
const CONTEXT_CEILING = 200_000;

/** A shell call running `peek`, whose arguments are the paths it reads. `peek.py` is what older logs name. */
const PEEK_RE = /\bpeek\.py\b|\bnv peek\b/;

/** Where one `peek` target's output begins. Every target opens with this line naming itself. */
const PEEK_SECTION_RE = /^===== (.*)$/gm;

/** A guard denial, as the harness reports a hook that refused a call: `... hook error: guard: <rule>: <why>`. */
const GUARD_RE = /hook error: guard: ([a-z0-9][a-z0-9-]*):/;

/**
 * Where a session's context went, by what put it there. A byte is charged to the call that fetched it,
 * and a bucket is matched in order, first hit wins, against one target at a time: a batched `peek` is
 * split into the paths it names, so a session following AGENTS.md rule 2 does not read as re-reading the
 * pack because its first target was the handoff. `orientation` names the files the pack is built from
 * and nothing else. The goal gets its own bucket, because a session reading it is usually fetching what
 * the pack left out -- the opposite defect from re-reading what it held.
 */
const BUCKETS: [string, string[]][] = [
  ["goal", ["docs/agent/goals/", "data/goals/", "loop-goal"]],
  [
    "orientation",
    ["orient.py", "brief.py", "nv orient", "nv brief", "docs/agent/handoff.md", "docs/agent/playbook", "docs/agent/conventions.md", "AGENTS.md", "CLAUDE.md"],
  ],
  ["process docs", ["docs/agent/"]],
  ["adr", ["docs/adr/", "docs/decisions/"]],
  ["plan + spec", ["implementation-plan", "docs/plan/", "docs/spec/", "plan.py", "nv plan"]],
  ["build + test", ["nv verify", "verify.py", "cargo ", "nvs test", "nvs run", "loop.py", "nv loop"]],
  ["git", ["git "]],
  ["discovery", ["grep", "rg ", "find ", " ls ", "glob", "Glob", "Grep"]],
  ["source", ["crates/", "benches/", "tests/", "examples/", "tools/", "fuzz/"]],
];

/**
 * The pack's growth a session may add before it is a leak. Something a session writes that every later
 * session reads -- the plan's status fields, the playbook -- grows with the number of sessions served,
 * and that is the shape AGENTS.md's priority ordering calls a leak rather than a trade-off. So this is a
 * slope, not a size.
 */
const DRIFT_BYTES_PER_SESSION = 400;

/**
 * A step the slope cannot see. A section added once lifts every later point equally and bends a
 * least-squares line barely at all, so a near-zero slope is not a pack that did not grow. A step is a
 * different defect from a slope: it sits at exactly its size rather than growing on its own.
 */
const STEP_BYTES = 5_000;

type Call = [string, unknown];

interface Subagent {
  agent: string;
  calls: number;
  peak_ctx: number;
}

interface Session {
  log: string;
  calls: number;
  per_message: number;
  per_shell_call: number;
  head: number;
  head_buckets: Record<string, number>;
  work: number;
  tail: number;
  verify_runs: number;
  shell_writes: number;
  commits: number;
  ctx_start: number;
  ctx_end: number;
  pack_bytes: number;
  pack_goal: string;
  compactions: number;
  duration_ms: number | null;
  duration_api_ms: number | null;
  cost_usd: number | null;
  model_usage: Record<string, unknown> | null;
  complete: boolean;
  attribution: Record<string, number>;
  guard: Record<string, number>;
  subagents: Subagent[];
}

interface Totals {
  sessions: number;
  seconds_per_call: number;
  api_share: number;
  ctx_per_call: number;
  ctx_start: number;
  head: number;
  work: number;
  tail: number;
  cost_per_session: number;
}

// ------------------------------------------------------------------------------------------ formatting

/** Python's `format(x, ",.Nf")`: thousands grouped, `digits` after the point. */
function grouped(x: number, digits = 0): string {
  const s = fixed(Math.abs(x), digits);
  const [int, frac] = s.split(".");
  const body = int!.replace(/\B(?=(\d{3})+(?!\d))/g, ",") + (frac !== undefined ? `.${frac}` : "");
  return x < 0 && Number(s) !== 0 ? `-${body}` : body;
}

/** Python's `format(x, "+,.Nf")`. */
const signed = (x: number, digits = 0) => (x < 0 && grouped(x, digits).startsWith("-") ? "" : "+") + grouped(x, digits);

/** Python's `format(x, ".N%")`. */
const pct = (x: number, digits = 0) => `${fixed(x * 100, digits)}%`;

/** Python's `str()` of a float: an integral value keeps its `.0`. */
const pyFloat = (x: number) => (Number.isInteger(x) ? `${x}.0` : String(x));

const round2 = (x: number) => Math.round(x * 100) / 100;

const sum = (xs: number[]) => xs.reduce((a, b) => a + b, 0);

function add(into: Record<string, number>, label: string, n: number): void {
  into[label] = (into[label] ?? 0) + n;
}

// ------------------------------------------------------------------------------------------ one call

function isRecord(x: unknown): x is Record<string, unknown> {
  return typeof x === "object" && x !== null && !Array.isArray(x);
}

/** The searchable text of one tool call: its command, its path, or its whole input. */
function callText([, inp]: Call): string {
  if (!isRecord(inp)) return String(inp);
  const v = inp.command || inp.file_path;
  return v ? String(v) : JSON.stringify(inp);
}

function shellWrite([name, inp]: Call): boolean {
  if (!SHELLS.includes(name) || !isRecord(inp)) return false;
  return SHELL_WRITE_RE.test(String(inp.command ?? ""));
}

function bucketOf(name: string | null, text: string): string {
  if (name !== null && MUTATORS.includes(name)) return "writing";
  // A Windows path is spelled with backslashes -- doubled once `callText` has JSON-encoded it -- and
  // every needle above is spelled with `/`.
  const t = text.replace(/\\+/g, "/");
  for (const [label, needles] of BUCKETS) if (needles.some((nd) => t.includes(nd))) return label;
  return "other";
}

const isPeek = (call: Call) => SHELLS.includes(call[0]) && PEEK_RE.test(callText(call));

/**
 * What one call reads, as the strings `bucketOf` should see: the targets a `peek` call names, or the
 * call's whole text for anything else. `--locate` takes symbols rather than paths, and a `;`/`&&` chain
 * puts other commands' words in the list, so both stay one target. Words are split on whitespace with a
 * quoted run kept whole and its quotes kept, because a PowerShell path's backslashes are separators, not
 * escapes.
 */
function targetsOf(call: Call): string[] {
  const text = callText(call);
  const m = isPeek(call) ? PEEK_RE.exec(text) : null;
  if (!m || new RegExp(SEPARATOR_RE.source).test(text)) return [text];
  const rest = text.slice(m.index + m[0].length).split("|", 1)[0]!;
  if ((rest.match(/"/g)?.length ?? 0) % 2 || (rest.match(/'/g)?.length ?? 0) % 2) return [text];
  const words: string[] = rest.match(/(?:[^\s"']+|"[^"]*"|'[^']*')+/g) ?? [];
  if (words.includes("--locate")) return [text];
  const paths = words.filter((w) => !w.startsWith("-"));
  return paths.length ? paths : [text];
}

/** One call's buckets, each with the share of the call it takes. */
function sharesOf(call: Call): Record<string, number> {
  const targets = targetsOf(call);
  const shares: Record<string, number> = {};
  for (const t of targets) add(shares, bucketOf(call[0], t), 1 / targets.length);
  return shares;
}

/** A tool result's text: a plain string, or the text blocks of a list. */
function plainText(body: unknown): string {
  if (typeof body === "string") return body;
  if (Array.isArray(body)) return body.map((b) => (isRecord(b) && typeof b.text === "string" ? b.text : "")).join("");
  return "";
}

/**
 * Charges one tool result's bytes to the buckets of the call that fetched it. A result that is not a
 * plain string is sized JSON-encoded. A `peek` result opens every target with a `===== <target>` line, so
 * a batched read is charged section by section to the bucket its own header names, and only what sits
 * outside every section is split by the call's shares.
 */
function chargeResult(attribution: Record<string, number>, shares: Record<string, number>, peek: boolean, body: unknown): void {
  const size = typeof body === "string" ? body.length : (JSON.stringify(body) ?? "").length;
  let charged = 0;
  if (peek) {
    const plain = plainText(body);
    const found = [...plain.matchAll(PEEK_SECTION_RE)];
    found.forEach((m, i) => {
      const end = i + 1 < found.length ? found[i + 1]!.index! : plain.length;
      add(attribution, bucketOf(null, m[1]!), end - m.index!);
      charged += end - m.index!;
    });
  }
  const rest = Math.max(size - charged, 0);
  for (const [label, share] of Object.entries(shares)) add(attribution, label, Math.round(rest * share));
}

// ------------------------------------------------------------------------------------------ one session

function readEvents(path: string): Record<string, unknown>[] {
  const out: Record<string, unknown>[] = [];
  for (const raw of readFileSync(path, "utf8").split(/\r?\n/)) {
    const line = raw.trim();
    if (!line) continue;
    try {
      const event = JSON.parse(line);
      if (isRecord(event)) out.push(event);
    } catch {
      // a truncated final line is normal for a killed run
    }
  }
  return out;
}

const contextOf = (usage: Record<string, unknown>) =>
  sum(["input_tokens", "cache_creation_input_tokens", "cache_read_input_tokens"].map((k) => Number(usage[k] ?? 0)));

/**
 * The sessions a transcript delegated to, out of `.loop/logs/<stem>.subagents/`. A subagent's turns are
 * inlined in the parent's stream and `readSession` drops them there, so without this a delegated read is
 * a session that did a great deal with very few calls. If the directory is absent, nothing was delegated
 * or the driver did not capture it, which is why the count is reported apart from the session's own.
 */
function subagentCost(path: string): Subagent[] {
  const dir = path.replace(/\.log$/, ".subagents");
  if (!existsSync(dir) || !statSync(dir).isDirectory()) return [];
  const found: Subagent[] = [];
  for (const file of readdirSync(dir).filter((f) => f.endsWith(".jsonl")).sort()) {
    let calls = 0;
    let peak = 0;
    for (const event of readEvents(join(dir, file))) {
      if (event.type !== "assistant" || !isRecord(event.message)) continue;
      const content = Array.isArray(event.message.content) ? event.message.content : [];
      calls += content.filter((c) => isRecord(c) && c.type === "tool_use").length;
      if (isRecord(event.message.usage)) peak = Math.max(peak, contextOf(event.message.usage));
    }
    if (calls) found.push({ agent: file.replace(/\.jsonl$/, ""), calls, peak_ctx: peak });
  }
  return found;
}

/** One transcript's measurements, or null when it holds no tool call. */
function readSession(path: string): Session | null {
  const calls: Call[] = [];
  const contexts: number[] = [];
  const perMessage: number[] = [];
  let result: Record<string, unknown> | null = null;
  let shellCalls = 0;
  let shellCmds = 0;
  const names = new Map<string, [Record<string, number>, boolean]>();
  const attribution: Record<string, number> = {};
  const guard: Record<string, number> = {};
  let packBytes = 0;
  let packGoal = "";
  for (const event of readEvents(path)) {
    // A subagent's turns are tagged with the tool call that spawned them. They are the parent's context
    // only in the sense that it waited for them, and counted here they would charge it for calls it never
    // made and put a step in its context series that reads as a compaction. `subagentCost` prices them.
    if (event.parent_tool_use_id) continue;
    const kind = event.type;
    if (kind === "loop_pack") {
      packBytes = Number(event.bytes ?? 0);
      packGoal = String(event.goal ?? "");
      continue;
    }
    if (kind === "result") {
      result = event;
      continue;
    }
    const message = isRecord(event.message) ? event.message : {};
    const content = Array.isArray(message.content) ? message.content.filter(isRecord) : [];
    if (kind === "user") {
      // A tool result is the largest thing that enters a conversation, and it is charged to the call
      // that asked for it. A guard denial is a tool result too, and is counted by its rule.
      for (const block of content) {
        if (block.type !== "tool_result") continue;
        const [shares, peek] = names.get(String(block.tool_use_id)) ?? [{ other: 1 }, false];
        chargeResult(attribution, shares, peek, block.content);
        const denied = GUARD_RE.exec(plainText(block.content).slice(0, 200));
        if (denied) add(guard, denied[1]!, 1);
      }
      continue;
    }
    if (kind !== "assistant") continue;
    const toolUses = content.filter((c) => c.type === "tool_use");
    if (toolUses.length) {
      perMessage.push(toolUses.length);
      for (const c of toolUses) {
        const call: Call = [String(c.name), c.input];
        calls.push(call);
        names.set(String(c.id), [sharesOf(call), isPeek(call)]);
        // A `;`/`&&` chain is a batch: one round trip, several commands. Pipes are one command.
        const cmd = isRecord(c.input) ? c.input.command : undefined;
        if (SHELLS.includes(call[0]) && cmd) {
          shellCalls += 1;
          shellCmds += (String(cmd).match(SEPARATOR_RE)?.length ?? 0) + 1;
        }
        // An edit's input is the new code, which is real context the session spent.
        if (MUTATORS.includes(call[0])) add(attribution, "writing", JSON.stringify(c.input ?? {}).length);
      }
    }
    const total = isRecord(message.usage) ? contextOf(message.usage) : 0;
    if (total) contexts.push(total);
  }
  if (!calls.length) return null;

  const texts = calls.map(callText);
  const indices = (pred: (i: number) => boolean) => texts.map((_, i) => i).filter(pred);
  // A splice and a shell write are edits too. Left out, a session that patched the tree only through
  // the shell reads as one long orientation with no work in it.
  const shellWrites = indices((i) => shellWrite(calls[i]!));
  const mutations = [
    ...indices((i) => MUTATORS.includes(calls[i]![0])),
    ...shellWrites,
    ...indices((i) => SPLICE_MARKERS.some((m) => texts[i]!.includes(m))),
  ].sort((a, b) => a - b);
  const verifies = indices((i) => VERIFY_MARKERS.some((m) => texts[i]!.includes(m)));
  // One verification, not one call about one: `--start` and the `--wait` that collects it are one run.
  const runs = verifies.filter((i) => !texts[i]!.includes("--wait"));
  // The wrap is the one spelling that commits, and a hand-rolled `git commit` is counted beside it.
  const commits = indices((i) => /git commit|session\.py --wrap|nv session --wrap/.test(texts[i]!));

  const n = calls.length;
  const head = mutations.length ? mutations[0]! : n;
  // The tail begins after the last edit, not at the first verification: `verify --start` is fired
  // mid-work, so counting from it prices work as fixed cost. A session that verified before it edited
  // anything folds the oddity into `work` rather than reporting a negative phase.
  const tailStart = Math.max(mutations.length ? mutations.at(-1)! + 1 : verifies.length ? verifies[0]! : n, head);

  // Which bucket the head's calls fall in is the question, because the two explanations want opposite
  // fixes: a pack missing what the goal needs is a `[context]` manifest to widen, while a session
  // re-reading what the pack already said is a pack to make more legible. A batched call is split
  // across its targets' buckets, so a head count is a sum of fractions.
  const headBuckets: Record<string, number> = {};
  for (let i = 0; i < head; i++) for (const [label, share] of Object.entries(sharesOf(calls[i]!))) add(headBuckets, label, share);

  let drops = 0;
  for (let i = 1; i < contexts.length; i++) if (contexts[i]! < contexts[i - 1]! * 0.6) drops += 1;

  const r = result ?? {};
  const num = (v: unknown) => (typeof v === "number" ? v : null);
  return {
    log: basename(path),
    calls: n,
    per_message: perMessage.length ? round2(sum(perMessage) / perMessage.length) : 0,
    per_shell_call: shellCalls ? round2(shellCmds / shellCalls) : 0,
    head,
    head_buckets: headBuckets,
    work: tailStart - head,
    tail: n - tailStart,
    verify_runs: runs.length,
    shell_writes: shellWrites.length,
    commits: commits.length,
    ctx_start: contexts[0] ?? 0,
    ctx_end: contexts.length ? Math.max(...contexts) : 0,
    pack_bytes: packBytes,
    pack_goal: packGoal,
    compactions: drops,
    duration_ms: num(r.duration_ms),
    duration_api_ms: num(r.duration_api_ms),
    cost_usd: num(r.total_cost_usd),
    model_usage: isRecord(r.modelUsage) ? r.modelUsage : null,
    complete: result !== null,
    attribution,
    guard,
    subagents: subagentCost(path),
  };
}

// ------------------------------------------------------------------------------------------ constants

/**
 * The window the model actually had, read out of the transcripts' `modelUsage[<model>].contextWindow`
 * rather than assumed. The small side-model a session uses for titles is ignored by how little it read.
 */
function contextWindow(sessions: Session[]): number | null {
  let best = 0;
  for (const s of sessions) {
    for (const usage of Object.values(s.model_usage ?? {})) {
      if (!isRecord(usage)) continue;
      const seen = sum(["inputTokens", "cacheReadInputTokens", "cacheCreationInputTokens"].map((k) => Number(usage[k] ?? 0)));
      if (seen < 100_000) continue;
      best = Math.max(best, Number(usage.contextWindow ?? 0));
    }
  }
  return best || null;
}

/** The constants the projection needs, averaged over whole sessions only. */
function totals(sessions: Session[]): Totals | null {
  const whole = sessions.filter((s) => s.complete && s.duration_ms);
  if (!whole.length) return null;
  const calls = sum(whole.map((s) => s.calls));
  const clock = sum(whole.map((s) => s.duration_ms!));
  const mean = (f: (s: Session) => number) => sum(whole.map(f)) / whole.length;
  return {
    sessions: whole.length,
    seconds_per_call: clock / 1000 / calls,
    api_share: sum(whole.map((s) => s.duration_api_ms ?? 0)) / clock,
    ctx_per_call: sum(whole.map((s) => s.ctx_end - s.ctx_start)) / calls,
    ctx_start: mean((s) => s.ctx_start),
    head: mean((s) => s.head),
    work: mean((s) => s.work),
    tail: mean((s) => s.tail),
    cost_per_session: mean((s) => s.cost_usd ?? 0),
  };
}

/** Least squares over `(x, y)`: slope, intercept and the means, or null when every x is the same. */
function fit(points: [number, number][]): { slope: number; intercept: number; my: number } | null {
  const n = points.length;
  const mx = sum(points.map(([x]) => x)) / n;
  const my = sum(points.map(([, y]) => y)) / n;
  const sxx = sum(points.map(([x]) => (x - mx) ** 2));
  if (!sxx) return null;
  const slope = sum(points.map(([x, y]) => (x - mx) * (y - my))) / sxx;
  return { slope, intercept: my - slope * mx, my };
}

const packPoints = (sessions: Session[]): [number, number][] =>
  sessions.filter((s) => s.pack_bytes && s.ctx_start).map((s) => [s.pack_bytes, s.ctx_start]);

interface Live {
  tokens: number;
  pack: number;
}

let liveMemo: Promise<Live | null> | undefined;

/**
 * What the next session will open at: the regressed fixed floor -- harness prompt, tool schemas,
 * CLAUDE.md, AGENTS.md, none of which moves -- plus the pack on disk now, at the regressed bytes per
 * token. The projection is advice about the run about to happen, and the pack is the one part of the
 * floor anybody changes, so the historical mean would go on producing the old answer for a whole run
 * after a pass that halved it. Null when there are not two distinct pack sizes to regress from.
 */
function liveCtxStart(sessions: Session[]): Promise<Live | null> {
  liveMemo ??= (async () => {
    const points = packPoints(sessions);
    if (new Set(points.map(([p]) => p)).size < 2) return null;
    const line = fit(points);
    if (!line) return null;
    try {
      const r = await runProc([process.execPath, join(ROOT, "tools", "nv", "main.ts"), "orient"], { timeoutMs: 60_000 });
      if (r.code !== 0 || !r.stdout) return null;
      const now = Buffer.byteLength(r.stdout, "utf8");
      return { tokens: line.intercept + line.slope * now, pack: now };
    } catch {
      return null;
    }
  })();
  return liveMemo;
}

/**
 * For a group of N slices: wall clock and tokens against N separate sessions. A turn re-reads the whole
 * context, so a session's token bill goes as calls x mean context, quadratic in its own length -- which
 * is what puts a ceiling on a group.
 */
function project(t: Totals, ceiling: number) {
  const rows = [];
  for (let n = 1; n <= 8; n++) {
    const calls = t.head + t.tail + n * t.work;
    const ctxEnd = t.ctx_start + t.ctx_per_call * calls;
    const tokens = (calls * (t.ctx_start + ctxEnd)) / 2;
    const soloCalls = t.head + t.tail + t.work;
    const soloCtxEnd = t.ctx_start + t.ctx_per_call * soloCalls;
    const soloTokens = (n * soloCalls * (t.ctx_start + soloCtxEnd)) / 2;
    rows.push({
      slices: n,
      calls: Math.round(calls),
      minutes: (calls * t.seconds_per_call) / 60,
      speedup: (n * soloCalls) / calls,
      token_ratio: tokens / soloTokens,
      ctx_end: Math.round(ctxEnd),
      over_ceiling: ctxEnd > ceiling,
    });
  }
  return rows;
}

// ------------------------------------------------------------------------------------------ reports

function renderAttribution(sessions: Session[]): void {
  console.log("\n== WHERE THE CONTEXT WENT  (bytes charged to the call that fetched them)");
  console.log("-- approximate tokens: bytes / 3.6. Read this as shares, not as absolutes.");
  console.log("-- a session's OTHER fixed cost -- the harness prompt, the tool schemas, CLAUDE.md");
  console.log("   and AGENTS.md -- never passes through a tool call, so none of it is below.");
  console.log("   `ctx_start` in the default report is where that floor shows up.");
  const merged: Record<string, number> = {};
  for (const s of sessions) for (const [label, size] of Object.entries(s.attribution)) add(merged, label, size);
  const total = sum(Object.values(merged)) || 1;
  console.log(`\n${"bucket".padEnd(16)}${"bytes".padStart(12)}${"approx tok".padStart(13)}${"share".padStart(8)}`);
  for (const [label, size] of Object.entries(merged).sort((a, b) => b[1] - a[1])) {
    console.log(`${label.padEnd(16)}${grouped(size).padStart(12)}${grouped(size / 3.6).padStart(13)}${pct(size / total).padStart(7)}`);
  }
  console.log(`${"TOTAL".padEnd(16)}${grouped(total).padStart(12)}${grouped(total / 3.6).padStart(13)}${pct(1).padStart(7)}`);

  const share = (label: string) => pct((merged[label] ?? 0) / total).padStart(4);
  console.log("\n   What each share argues for, in the goal's [context] manifest:");
  console.log(`   orientation ${share("orientation")}  -- if this is large, the pack itself is too wide:`);
  console.log("                     narrow `modules`, `playbook` and `shapes`, and check");
  console.log("                     `bun nv orient --audit` for which section carries it.");
  console.log(`   adr         ${share("adr")}  -- if this is large, whole ADRs are being read where`);
  console.log('                     `adrs = ["NNNN §N"]` would have sliced one section.');
  console.log(`   discovery   ${share("discovery")}  -- if this is large, the handoff's checklist items are`);
  console.log("                     missing their file:line anchors, so every session re-derives them.");

  const delegated = sessions.flatMap((s) => s.subagents.map((a) => [s.log, a] as const));
  // The two buckets a subagent may take: finding where something is, and reading a file the session
  // will not edit. The item's own anchors, the code being changed and the verification are the
  // session's own to read.
  const searching = (merged.discovery ?? 0) + (merged.source ?? 0);
  const spawned = new Set(delegated.map(([log]) => log)).size;
  console.log("\n== DELEGATED  (subagent transcripts captured beside the session's own)");
  if (delegated.length) {
    console.log(`\n${"session".padEnd(32)}${"agent".padEnd(26)}${"calls".padStart(7)}${"peak ctx".padStart(11)}`);
    for (const [log, a] of delegated) {
      console.log(`${log.padEnd(32)}${a.agent.padEnd(26)}${String(a.calls).padStart(7)}${grouped(a.peak_ctx).padStart(11)}`);
    }
    console.log(
      "\n   A subagent pays the same startup floor a session does, so a narrow lookup is\n" +
        "   cheap only in the PARENT's context, never in absolute tokens. Peak context here\n" +
        "   is what that floor actually cost.",
    );
  } else {
    console.log(
      "\n   Nothing. Either no session spawned a subagent, or the driver did not capture\n" +
        "   their transcripts -- the two look identical from here.",
    );
  }
  console.log(
    `\n   ${spawned} of ${sessions.length} session(s) delegated anything, against ` +
      `${grouped(searching)} B (${grouped(searching / 3.6)} tok, ${pct(searching / total)} of what was\n` +
      "   fetched) charged to their own windows by discovery and source reads. That is the\n" +
      "   ceiling on what delegation could ever have moved, not a target: a subagent is for a\n" +
      "   search over files this session will NOT open, and its startup floor makes a narrow\n" +
      "   one a loss. The half of it that is the item's own anchors was never delegable.\n" +
      "   docs/agent/session-prompt.md is where the rule and the safety boundary live.",
  );
}

/**
 * The guard's denials, per rule and per session. A denial is a habit the guard exists to break, so what
 * a reader wants is whether a rule's count falls after the rule first fired, and a rule whose later half
 * counts more than its earlier half is flagged as rising.
 */
function renderGuard(sessions: Session[]): void {
  const byRule: Record<string, number> = {};
  for (const s of sessions) for (const [rule, n] of Object.entries(s.guard)) add(byRule, rule, n);
  const total = sum(Object.values(byRule));
  const denied = sessions.filter((s) => Object.keys(s.guard).length);
  console.log(`\n== GUARD  (guard denials by rule, ${sessions.length} session(s) in order, oldest first)`);
  if (!total) {
    console.log("   none: no session here had a call refused by `bun nv guard`.");
    return;
  }
  const count = (from: number, to: number, rule: string) => sum(sessions.slice(from, to).map((s) => s.guard[rule] ?? 0));
  const width = Math.max(16, ...Object.keys(byRule).map((r) => r.length + 2));
  console.log(`\n${"rule".padEnd(width)}${"denials".padStart(8)}${"sessions".padStart(10)}${"earlier".padStart(9)}${"later".padStart(7)}`);
  for (const [rule, n] of Object.entries(byRule).sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0]))) {
    const hit = sessions.filter((s) => s.guard[rule]).length;
    // The window starts at the rule's first denial: a rule the guard gained mid-window has an empty
    // earlier half by construction, which is not a habit coming back.
    const from = sessions.findIndex((s) => s.guard[rule]);
    const mid = from + Math.floor((sessions.length - from) / 2);
    const earlier = count(from, mid, rule);
    const later = count(mid, sessions.length, rule);
    const note = later > earlier ? "  RISING" : "";
    console.log(`${rule.padEnd(width)}${String(n).padStart(8)}${String(hit).padStart(10)}${String(earlier).padStart(9)}${String(later).padStart(7)}${note}`);
  }
  console.log(`${"TOTAL".padEnd(width)}${String(total).padStart(8)}${String(denied.length).padStart(10)}`);
  console.log("\n   `earlier` and `later` split the sessions from a rule's first denial in half. A rule counted");
  console.log("   RISING is a habit that came back: its message is not changing what sessions do,");
  console.log("   so the pack or the prompt is where it is fixed, not the guard.");

  console.log(`\n${"session".padEnd(28)}${"denials".padStart(8)}  by rule`);
  for (const s of denied) {
    const rules = Object.entries(s.guard).sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0]));
    console.log(`${s.log.padEnd(28)}${String(sum(rules.map(([, n]) => n))).padStart(8)}  ${rules.map(([r, n]) => `${r} ${n}`).join(", ")}`);
  }
}

/**
 * Bytes per token for the orientation pack, regressed rather than assumed. The pack is the only part of
 * a session's opening context that moves between sessions, so the slope of `ctx_start` against pack
 * bytes is the ratio, and the intercept is the fixed floor underneath it.
 */
function calibrate(sessions: Session[], write: boolean): void {
  const points = packPoints(sessions);
  const spread = [...new Set(points.map(([p]) => p))];
  if (spread.length < 2) {
    console.log("== CALIBRATION");
    console.log(`   ${points.length} session(s) recorded a pack size, ${spread.length} distinct.`);
    console.log("   Two DIFFERENT pack sizes are the minimum for a slope. The driver records each");
    console.log("   session's pack with a `loop_pack` line; a run over these logs will have them.");
    console.log("   Until then `bun nv orient --audit` uses its default and says so.");
    return;
  }
  const line = fit(points);
  if (!line) return;
  const { slope, intercept, my } = line;
  const ratio = slope > 0 ? 1 / slope : 0;
  const resid = sum(points.map(([p, c]) => (c - (intercept + slope * p)) ** 2));
  const tot = sum(points.map(([, c]) => (c - my) ** 2));
  const r2 = tot ? 1 - resid / tot : 0;
  const callsPerSession = sum(sessions.map((s) => s.calls)) / sessions.length;

  console.log(`== CALIBRATION  (pack bytes -> opening context, regressed over ${points.length} session(s))`);
  console.log(`   bytes per token       ${grouped(ratio, 2)}`);
  console.log(`   fixed floor           ${grouped(intercept)} tokens  (harness prompt + tool schemas + CLAUDE.md + AGENTS.md)`);
  console.log(`   fit                   R^2 ${fixed(r2, 3)} over pack sizes ${grouped(Math.min(...spread))} - ${grouped(Math.max(...spread))} B`);
  console.log();
  console.log("   Every token of the pack is re-billed on every turn of the session, so at the");
  console.log(`   measured ${fixed(callsPerSession, 0)} calls a session, 1,000 bytes of pack is about`);
  console.log(`   ${grouped((1000 / ratio) * callsPerSession)} billed tokens. That is the number to weigh a \`[context]\` selector against.`);

  if (write) {
    mkdirSync(join(ROOT, "tools", "data"), { recursive: true });
    // `calls_per_session` too: `nv orient --audit` reads it before falling back to a constant of its own.
    const record = {
      bytes_per_token: Math.round(ratio * 1000) / 1000,
      floor_tokens: Math.round(intercept),
      r_squared: Math.round(r2 * 10000) / 10000,
      calls_per_session: Math.round(callsPerSession),
      sessions: points.length,
    };
    writeFileSync(CALIBRATION, `${JSON.stringify(record, null, 2)}\n`);
    console.log("\n   written to tools/data/calibration.json -- `bun nv orient --audit` reads it from there.");
  } else {
    console.log("\n   --write records this in tools/data/calibration.json, which is where");
    console.log("   `bun nv orient --audit` looks before falling back to its default.");
  }
}

/**
 * What the calls before the first edit were reading, bucketed. The driver hands a session its pack with
 * the prompt, so the head's floor is zero, and what matters is what the gap was spent on: `orientation`
 * is a session re-reading a source the pack was built from, which is a different defect from reading
 * source it had to look up. Sessions that never edited are left out, since their head is the whole
 * session.
 */
function reportHead(sessions: Session[]): void {
  const heads = sessions.filter((s) => Object.keys(s.head_buckets).length && s.head < s.calls);
  if (!heads.length) return;
  const all: Record<string, number> = {};
  for (const s of heads) for (const [label, n] of Object.entries(s.head_buckets)) add(all, label, n);
  const calls = sum(Object.values(all));
  if (!calls) return;
  const k = heads.length;
  console.log(`\n== WHERE THE HEAD CALLS GO  (${fixed(calls / k, 1)} a session before the first edit, ${k} session(s))`);
  for (const [label, n] of Object.entries(all).sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0]))) {
    console.log(`   ${label.padEnd(16)}${fixed(n / k, 1).padStart(5)} a session${fixed((n / calls) * 100, 0).padStart(5)}%`);
  }
  const reread = all.orientation ?? 0;
  if (reread) {
    const withReread = heads.filter((s) => s.head_buckets.orientation).length;
    console.log(
      `\n   ${fixed(reread, 0)} of those, across ${withReread} of ${k} session(s), RE-READ AN ORIENTATION SOURCE\n` +
        "   -- the pack, the handoff, the playbook, conventions.md, AGENTS.md -- which the\n" +
        "   driver had already piped in ahead of the prompt. Either the `[context]` manifest\n" +
        "   is missing a field the goal needs, or the pack carried it and it was not found.\n" +
        "   Those are opposite fixes, so read the calls in `.loop/logs/` before making one.\n" +
        "   A third case reads like either and is neither: the pack prints the REST of the\n" +
        "   handoff's group one line each, so a session taking a second slice pays one\n" +
        '   `handoff.md:"## Next group"` peek by design. That call is not a defect.',
    );
  }
}

/**
 * Is the pack growing with the number of sessions? The slope is fitted inside each goal and pooled, so
 * a chain switch -- which installs a new `[context]` manifest and can double the pack without any
 * session writing a byte of it -- is a step between goals rather than a slope through them. Sessions
 * whose pack record names no goal share one unnamed goal.
 */
async function reportDrift(sessions: Session[]): Promise<void> {
  const pts = sessions.flatMap((s, i) => (s.pack_bytes ? [[i, s.pack_bytes, s.pack_goal] as const] : []));
  if (pts.length < 5) return;
  const n = pts.length;
  const runs: [string, [number, number][]][] = [];
  for (const [x, y, goal] of pts) {
    const last = runs.at(-1);
    if (last && last[0] === goal) last[1].push([x, y]);
    else runs.push([goal, [[x, y]]]);
  }
  let num = 0;
  let denom = 0;
  for (const [, run] of runs) {
    const mx = sum(run.map(([x]) => x)) / run.length;
    const my = sum(run.map(([, y]) => y)) / run.length;
    num += sum(run.map(([x, y]) => (x - mx) * (y - my)));
    denom += sum(run.map(([x]) => (x - mx) ** 2));
  }
  if (!denom) return;
  const slope = num / denom;
  const first = pts[0]![1];
  const last = pts.at(-1)![1];
  // What the chain switches moved by themselves: each goal's first pack against the last one the goal
  // before it left.
  const switched = sum(runs.slice(1).map(([, run], i) => run[0]![1] - runs[i]![1].at(-1)![1]));
  console.log("\n== FIXED COST  (the orientation pack, first session to last)");
  console.log(`   ${grouped(first)} -> ${grouped(last)} B, ${signed(slope)} B a session inside a goal`);
  if (runs.length > 1) {
    const names = runs.map(([g]) => (g ? `\`${g}\`` : "(unnamed)")).join(" -> ");
    console.log(
      `   ${signed(switched)} B of that is at ${runs.length - 1} chain switch(es), ${names}:\n` +
        "   a new manifest, authored rather than accumulated, and narrowed only by whoever\n" +
        "   writes the goal.",
    );
  }
  if (slope < DRIFT_BYTES_PER_SESSION) {
    const step = last - first - switched;
    if (step > STEP_BYTES) {
      console.log(
        `   NOT A LEAK, BUT NOT FLAT EITHER: the pack STEPPED ${signed(step)} B (${signed((step / first) * 100)}%) across this\n` +
          `   run while regressing at ${signed(slope)} B a session, so a section was added rather than\n` +
          "   accumulated. `bun nv orient --audit` says which one. Unlike a slope this will\n" +
          "   not grow on its own, so it is a cut to make deliberately or to keep deliberately.",
      );
    } else {
      console.log("   flat enough -- the per-session cost is not growing with the number of sessions.");
    }
    // A transcript records the pack its session already carried, so a pass that adds a section lands
    // on disk a whole run before it can bend this line. Say so where the reader decides whether there
    // is anything to cut.
    const live = await liveCtxStart(sessions);
    if (live && live.pack - last > DRIFT_BYTES_PER_SESSION) {
      console.log(
        `   BUT THE PACK ON DISK IS ${grouped(live.pack)} B, ${signed(live.pack - last)} B past the last one a\n` +
          "   session actually carried. That jump is not in the slope yet, so read this again\n" +
          "   after the next run before concluding the pack is flat.",
      );
    }
    return;
  }
  console.log(
    `   THIS IS A LEAK, NOT A BIG PACK. At ${grouped(slope)} B a session the next ${n} sessions pay\n` +
      `   ${grouped(first + slope * 2 * n)} B each, and every byte is re-billed on every one of a\n` +
      "   session's calls. Something a session WRITES is being read by every session after it.\n" +
      "   `bun nv orient --audit` says which section, and it is usually the plan's status\n" +
      "   block or the playbook: `bun nv plan --check` and `bun nv playbook --check` price\n" +
      "   those two. A status field carrying a record of what landed belongs in `git log`;\n" +
      "   a finding belongs in a playbook bullet no goal ships until its file set implies it.",
  );
}

async function renderDefault(sessions: Session[], t: Totals | null, ceilingGiven: number | null): Promise<void> {
  // The driver prunes `.loop/logs` to its newest runs, so the set moves under a reader comparing this
  // count against an earlier report's. Naming the runs makes two reports comparable, or visibly not.
  const runs = [...new Set(sessions.map((s) => s.log.slice(0, s.log.lastIndexOf("-"))))].sort();
  const window = runs.length === 1 ? runs[0] : `${runs[0]} .. ${runs.at(-1)}`;
  console.log(`== PER SESSION  (${sessions.length} transcript(s) in .loop/logs, ${runs.length} run(s): ${window} -- older runs are pruned)`);
  console.log(
    "log".padEnd(28) +
      ["calls", "/msg"].map((h) => h.padStart(6)).join("") +
      "cmd/c".padStart(7) +
      ["head", "work", "tail"].map((h) => h.padStart(6)).join("") +
      ["vfy", "cmt"].map((h) => h.padStart(5)).join("") +
      "ctx end".padStart(10) +
      "min".padStart(7) +
      "$".padStart(8),
  );
  for (const s of sessions) {
    const mins = s.duration_ms ? fixed(s.duration_ms / 60000, 1) : "-";
    const cost = s.cost_usd ? fixed(s.cost_usd, 2) : "-";
    const flag = s.compactions ? "  COMPACTED" : "";
    console.log(
      s.log.padEnd(28) +
        String(s.calls).padStart(6) +
        pyFloat(s.per_message).padStart(6) +
        fixed(s.per_shell_call, 2).padStart(7) +
        [s.head, s.work, s.tail].map((v) => String(v).padStart(6)).join("") +
        [s.verify_runs, s.commits].map((v) => String(v).padStart(5)).join("") +
        grouped(s.ctx_end).padStart(10) +
        mins.padStart(7) +
        cost.padStart(8) +
        flag,
    );
    if (!s.complete) console.log(`${"".padEnd(28)}(incomplete: no result event -- excluded from the constants)`);
  }

  if (!t) {
    console.log("\nno completed session to derive constants from; run the loop once and re-run this");
    return;
  }

  const fixedCost = t.head + t.tail;
  const totalCalls = fixedCost + t.work;
  console.log(`\n== CONSTANTS  (mean over ${t.sessions} complete session(s))`);
  console.log(`   seconds per tool call     ${fixed(t.seconds_per_call, 1)}`);
  console.log(`   share of clock on the API ${fixed(t.api_share * 100, 0)}%`);
  console.log(`   context per tool call     ${grouped(t.ctx_per_call)} tokens`);
  console.log(
    `   fixed cost per session    ${fixed(fixedCost, 0)} of ${fixed(totalCalls, 0)} calls ` +
      `(${fixed((fixedCost / totalCalls) * 100, 0)}%) -- head ${fixed(t.head, 0)} + tail ${fixed(t.tail, 0)}`,
  );
  console.log(`   cost per session          $${fixed(t.cost_per_session, 2)}`);

  reportHead(sessions);
  await reportDrift(sessions);

  // Two kinds of batching, and only the one inside a shell call has been seen here. Reporting only the
  // one across messages said "nothing was ever batched" at sessions that chained dozens of commands.
  const chained = sessions.filter((s) => s.per_shell_call > 1.05);
  if (chained.length) {
    const mean = sum(chained.map((s) => s.per_shell_call)) / chained.length;
    console.log(
      `\n   BATCHING IS IN THE SHELL, NOT ACROSS MESSAGES. ${chained.length} of ${sessions.length} session(s) chained\n` +
        `   commands with \`;\`/\`&&\`, ${fixed(mean, 2)} per shell call, while every message still carried one\n` +
        "   tool call. That is the habit `bun nv peek` and AGENTS.md rule 2 ask for, so the saving is\n" +
        "   taken -- do not read the 1.00 above as one going begging.",
    );
  } else {
    console.log(
      "\n   NOTHING WAS BATCHED, in either sense: no message carried two tool calls and no\n" +
        "   shell call carried two commands. That is the one case where the clock above is\n" +
        "   the worst case and the cheapest saving really is untaken.",
    );
  }

  // AGENTS.md rule 1, measured rather than gated: the risk mostly does not fire -- a heredoc carrying
  // Rust works until an apostrophe in a doc comment ends the quote early -- so a run needs the count,
  // not a gate that would fail a session for a scratch file under `.agent-tmp/`.
  const writers = sessions.filter((s) => s.shell_writes);
  if (writers.length) {
    console.log(
      `\n   THE SHELL IS CARRYING FILE CONTENT. ${sum(writers.map((s) => s.shell_writes))} call(s) across ${writers.length} of\n` +
        `   ${sessions.length} session(s) used a heredoc, a \`>\` redirect or a \`sed -i\` where AGENTS.md rule 1\n` +
        "   asks for Write/Edit or `bun nv splice --patch`. The shell parses apostrophes\n" +
        "   and backticks before it runs anything, so this is the spelling that fails on a doc\n" +
        "   comment rather than on anything the session did wrong.",
    );
  }

  const window2 = contextWindow(sessions);
  let ceiling = ceilingGiven ?? CONTEXT_CEILING;
  let source = ceilingGiven ? "given on the command line" : "the fixed quality ceiling -- an agent degrades well before its window is full";
  if (window2 && window2 < ceiling) {
    ceiling = window2;
    source = `the model's ${grouped(window2)}-token window, smaller than the quality ceiling`;
  }

  const largest = Math.max(...sessions.map((s) => s.ctx_end));
  const over = sessions.filter((s) => s.ctx_end > ceiling);
  const compacted = sessions.filter((s) => s.compactions);
  console.log(`\n== PROJECTION  (ceiling ${grouped(ceiling)}, ${source})`);
  if (over.length) {
    console.log(
      `   ${over.length} of ${sessions.length} session(s) ALREADY FINISHED OVER THE CEILING, doing one\n` +
        `   slice each -- largest ${grouped(largest)}. Until that comes down,\n` +
        "   the lever is reading less per session, not doing more per session.",
    );
  } else {
    console.log(`   largest context any session here reached: ${grouped(largest)}`);
  }
  // The projection is advice about the next run, so it opens where the next session will open.
  const live = await liveCtxStart(sessions);
  if (live && Math.abs(live.tokens - t.ctx_start) > 2_000) {
    console.log(
      `   the pack on disk is now ${grouped(live.pack)} B, so the NEXT session opens at ${grouped(live.tokens)},\n` +
        `   not the ${grouped(t.ctx_start)} these transcripts averaged. Everything below uses the former.`,
    );
    t = { ...t, ctx_start: live.tokens };
  }
  const budget = ceiling - t.ctx_start;
  console.log(
    `   a session starts at ${grouped(t.ctx_start)} (prompt + AGENTS.md + orientation), leaving ${grouped(budget)}\n` +
      `   for growth -- about ${fixed(budget / t.ctx_per_call, 0)} calls at the measured ${grouped(t.ctx_per_call)} tokens a call`,
  );
  if (compacted.length) console.log(`   ${compacted.length} session(s) COMPACTED -- the ceiling is already too high`);
  console.log(
    `   ${"slices".padStart(7)}${"calls".padStart(7)}${"minutes".padStart(9)}${"vs solo".padStart(9)}${"tokens".padStart(9)}${"ctx end".padStart(10)}`,
  );
  const rows = project(t, ceiling);
  for (const r of rows) {
    const note = r.over_ceiling ? "  over ceiling" : "";
    console.log(
      `   ${String(r.slices).padStart(7)}${String(r.calls).padStart(7)}${fixed(r.minutes, 0).padStart(9)}` +
        `${fixed(r.speedup, 2).padStart(8)}x${fixed(r.token_ratio, 2).padStart(8)}x${grouped(r.ctx_end).padStart(10)}${note}`,
    );
  }

  // Three readings of one curve, because they optimise three different things. None is a cap to
  // install: AGENTS.md step 2 takes only the ceiling from here. loop-authoring.md § 1 names the lever.
  const safe = rows.filter((r) => !r.over_ceiling);
  const cheapest = safe.filter((r) => r.token_ratio <= 1.0);
  const knee = safe.slice(1).filter((r, i) => r.speedup - safe[i]!.speedup >= 0.1);
  console.log("\n== GROUP CURVE  what the curve says, not a cap to install");
  if (!safe.length) {
    console.log(
      "   NONE. Even a single slice projects past the ceiling, so there is no group size to\n" +
        "   pick: there is nothing to group, and the work is cutting what a session reads --\n" +
        "   not counting slices. The projection above charges every extra slice a full fresh\n" +
        "   read, which is the right assumption for an unrelated slice and pessimistic for one\n" +
        "   sharing a file set -- so re-run this after the first session that lands well under\n" +
        "   the ceiling.",
    );
    return;
  }
  const reading = (r: (typeof rows)[number]) => `${r.slices} slices -- ${fixed(r.speedup, 2)}x faster, ${fixed(r.token_ratio, 2)}x tokens`;
  if (cheapest.length) console.log(`   cheapest      ${reading(cheapest.at(-1)!)}. Past here a group costs more than separate sessions.`);
  if (knee.length) console.log(`   best value    ${reading(knee.at(-1)!)}. The last slice worth +0.10x; after it the curve flattens.`);
  const last = safe.at(-1)!;
  console.log(`   fastest safe  ${reading(last)}, ending at ${grouped(last.ctx_end)} against the ceiling.`);
  console.log(
    "\n   These are readings of the curve, not caps to install. AGENTS.md § Session\n" +
      "   workflow step 2 holds no slice count -- the 120k gate is the budget, because a\n" +
      "   slice's cost is not fixed and a count prices every slice as the most expensive\n" +
      "   one. What step 2 does take from here is the CEILING, and only after a run that\n" +
      "   changed what a session reads. docs/agent/loop-authoring.md § Measure first says\n" +
      "   which lever each shape wants.",
  );
}

// ------------------------------------------------------------------------------------------ entry

export async function run(args: string[]): Promise<number> {
  let parsed;
  try {
    parsed = parseArgs(args, {
      flags: ["--attribute", "--guard", "--json", "--calibrate", "--write"],
      valued: ["--run", "--context-ceiling"],
      order: ["--run", "--attribute", "--guard", "--context-ceiling", "--json", "--calibrate", "--write"],
    });
  } catch (e) {
    if (!(e instanceof ArgError)) throw e;
    console.error(`${USAGE}\nnv loop-stats: error: ${e.message}`);
    return 2;
  }
  const flag = (f: string) => parsed.flags.has(f);
  if (flag("--help")) {
    console.log(`${USAGE}\n\n${summary}`);
    return 0;
  }
  const ceilingText = parsed.values.get("--context-ceiling");
  const ceiling = ceilingText === undefined ? null : Number.parseInt(ceilingText, 10);
  if (ceiling !== null && !Number.isFinite(ceiling)) {
    console.error(`${USAGE}\nnv loop-stats: error: argument --context-ceiling: invalid int value: '${ceilingText}'`);
    return 2;
  }

  if (!existsSync(LOGDIR)) {
    console.error("no .loop/logs -- nothing has been measured yet");
    return 1;
  }
  const stamp = parsed.values.get("--run");
  // `<run>-console.log` is the run's own stamped record of itself, not one session's NDJSON; the other
  // two suffixes are the same kind of file under names an older driver gave it.
  const paths = readdirSync(LOGDIR)
    .filter((f) => f.endsWith(".log") && (!stamp || f.startsWith(`${stamp}-`)))
    .filter((f) => !/-(console|run|supervisor)\.log$/.test(f))
    .sort()
    .map((f) => join(LOGDIR, f));
  const pattern = stamp ? `${stamp}-*.log` : "*.log";
  if (!paths.length) {
    console.error(`no transcripts match ${pattern} in .loop/logs`);
    return 1;
  }
  const sessions = paths.map(readSession).filter((s): s is Session => s !== null);
  if (!sessions.length) {
    console.error(`${paths.length} transcript(s) found, none holding a tool call`);
    return 1;
  }
  const t = totals(sessions);

  if (flag("--json")) console.log(JSON.stringify({ sessions, constants: t }, null, 2));
  else if (flag("--calibrate")) calibrate(sessions, flag("--write"));
  else if (flag("--attribute")) renderAttribution(sessions);
  else if (flag("--guard")) renderGuard(sessions);
  else await renderDefault(sessions, t, ceiling);
  return 0;
}
