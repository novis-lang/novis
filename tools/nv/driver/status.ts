// What `nv loop` says about a goal: the status line's words, the key row's layout, and the goal table `[g]`
// and `bun nv loop --goal` print. `driver/console.ts` paints the row and the key row, in its colours. Everything here is a pure function of a goal plan, the check results and
// one session's event stream, so a recorded stream reproduces what a run showed.
//
// The row reads `goal {slug}/{stage} | {x}% | {in}in/{out}out | {calls} tool calls | session {n} | {doing}`:
//   - `{stage}` is the lowest stage with a check that is not green, and `done` when there is none;
//   - `{x}` is the share of the goal's own checks that are green, with the floor stage left out, since
//     the floor is every walked goal's checks and would start every goal near 100%;
//   - `{in}` is the context of the session's latest turn, its input, cache-write and cache-read tokens,
//     and `{out}` is what the session has written, summed over its messages; a subagent's turns are
//     skipped, because the session's window holds only that call's one result;
//   - `{doing}` is the latest tool call's `description` for a shell tool, `<tool> <path>` for a file
//     tool and the tool's name otherwise, or the driver's own phase between sessions. After it, behind
//     ` > `, comes what each running `nv` command says it is doing (`lib/progress.ts`).
// A shell call whose command is a check's `argv` sets that check green when it exits 0 and red when it
// does not, until the next acceptance sweep sets every check. A row wider than the terminal is cut at
// its end, and the window title is the same row.

import { rel, ROOT } from "../lib/paths.ts";
import { wrap } from "../lib/py.ts";

/** The floor stage's title: every walked goal's checks, carried as the goal's own first stage. */
export const FLOOR = "floor";

export interface PlanStage {
  number: number;
  title: string;
  /** One sentence saying what the stage does; the table shows the title when it is absent. */
  summary?: string;
}

export interface PlanCheck {
  id: string;
  stage: number;
  argv?: string[];
  kind?: string;
  name?: string;
  file?: string;
}

export interface Plan {
  slug: string;
  stages: PlanStage[];
  checks: PlanCheck[];
}

/** Each check's last known result, by id: true for green. A check with no entry is not green. */
export type Results = Map<string, boolean>;

const SHELLS = new Set(["Bash", "PowerShell"]);
const FILE_TOOLS = new Set(["Read", "Edit", "Write", "NotebookEdit"]);

/** `84213` is `84.2k`: a context is five or six digits, read against a ceiling quoted in thousands. */
export function ktok(tokens: number): string {
  return tokens >= 1000 ? `${(tokens / 1000).toFixed(1)}k` : String(tokens);
}

/** A command's words with their quotes removed; enough to compare a command with a check's `argv`. */
export function words(command: string): string[] {
  const out: string[] = [];
  for (const m of command.trim().matchAll(/"((?:[^"\\]|\\.)*)"|'([^']*)'|(\S+)/g)) out.push(m[1] ?? m[2] ?? m[3]!);
  return out;
}

const sameArgv = (a: string[], b: string[]) => a.length === b.length && a.every((w, i) => w === b[i]);

/** The goal's own stages, which are every stage but the floor. */
const own = (plan: Plan) => new Set(plan.stages.filter((s) => s.title !== FLOOR).map((s) => s.number));

/** The lowest stage with a check that is not green, or null when every check is green. */
export function currentStage(plan: Plan, results: Results): number | null {
  const red = plan.checks.filter((c) => results.get(c.id) !== true).map((c) => c.stage);
  return red.length ? Math.min(...red) : null;
}

/** The share of the goal's own checks that are green, from 0 to 100. */
export function percent(plan: Plan, results: Results): number {
  const mine = own(plan);
  const checks = plan.checks.filter((c) => mine.has(c.stage));
  if (checks.length === 0) return 100;
  return Math.floor((checks.filter((c) => results.get(c.id) === true).length * 100) / checks.length);
}

/**
 * One session's figures, fed its stream-json events one at a time. The figures outlive the session:
 * `begin` is called when the next one reports, so the row between sessions shows the last one's.
 */
export class Session {
  number = 0;
  context = 0;
  calls = 0;
  doing = "";
  /** Output tokens per message id: a message arrives once per content block, each repeating its usage. */
  private written = new Map<string, number>();
  /** Tool call id to the checks its command runs. */
  private pending = new Map<string, string[]>();

  constructor(
    private readonly plan: Plan,
    private readonly results: Results,
  ) {}

  begin(number: number): void {
    this.number = number;
    this.context = 0;
    this.calls = 0;
    this.doing = "";
    this.written.clear();
    this.pending.clear();
  }

  get out(): number {
    let sum = 0;
    for (const n of this.written.values()) sum += n;
    return sum;
  }

  /** The driver's own phase between sessions: `acceptance sweep 31/58`, `usage wall, 12:03 left`, `held`. */
  phase(text: string): void {
    this.doing = text;
  }

  feed(e: Record<string, any>): void {
    if (e.parent_tool_use_id) return;
    const message = e.message ?? {};
    if (e.type === "assistant") {
      const usage = message.usage ?? {};
      const context = ["input_tokens", "cache_creation_input_tokens", "cache_read_input_tokens"].reduce((n, k) => n + Number(usage[k] ?? 0), 0);
      if (context) this.context = context;
      if (usage.output_tokens) this.written.set(String(message.id ?? this.written.size), Number(usage.output_tokens));
      for (const block of message.content ?? []) if (block.type === "tool_use") this.call(block);
    } else if (e.type === "user") {
      for (const block of Array.isArray(message.content) ? message.content : []) {
        if (block.type !== "tool_result") continue;
        for (const id of this.pending.get(block.tool_use_id) ?? []) this.results.set(id, block.is_error !== true);
        this.pending.delete(block.tool_use_id);
      }
    }
  }

  private call(block: Record<string, any>): void {
    const input = block.input ?? {};
    this.calls++;
    if (SHELLS.has(block.name)) {
      this.doing = String(input.description || block.name);
      const argv = words(String(input.command ?? ""));
      const runs = this.plan.checks.filter((c) => c.argv && sameArgv(c.argv, argv)).map((c) => c.id);
      if (runs.length) this.pending.set(block.id, runs);
    } else if (FILE_TOOLS.has(block.name) && (input.file_path || input.notebook_path)) {
      this.doing = `${block.name} ${rel(String(input.file_path || input.notebook_path), ROOT).replace(/\\/g, "/")}`;
    } else {
      this.doing = String(block.name);
    }
  }
}

/** `text` cut to `width` code points, its end replaced by `…` when it is cut. */
export function cut(text: string, width: number): string {
  const chars = Array.from(text);
  return chars.length <= width ? text : chars.slice(0, Math.max(0, width - 1)).join("") + "…";
}

/** The status row, cut to the terminal's width. `working` is what each running `nv` command says it is
 * doing, the one started first first. */
export function statusRow(plan: Plan, results: Results, s: Session, width: number, working: string[] = []): string {
  const stage = currentStage(plan, results);
  const row = [
    `goal ${plan.slug}/${stage ?? "done"}`,
    `${percent(plan, results)}%`,
    `${ktok(s.context)}in/${ktok(s.out)}out`,
    `${s.calls} tool calls`,
    `session ${s.number}`,
    [s.doing, ...working].filter(Boolean).join(" > "),
  ].join(" | ");
  return cut(row, width);
}

/** The escape sequence that sets the terminal's window title to the row. */
export const title = (row: string) => `\x1b]0;${row}\x07`;

/**
 * The key row under the status row: the keys that do something now, or, while a prompt is typed after
 * `[i]`, the typed text last, so the terminal's cursor stands right after its last character. A typed
 * prompt longer than the row is shown by its tail, since the tail is what is being typed.
 */
export function keyRow(keys: string[], typing: string | null, width: number, utf8 = true): string {
  const sep = utf8 ? " · " : " | ";
  if (typing === null) return cut("  " + keys.join(sep), width);
  const head = `  Enter sends, Esc cancels${sep}prompt> `;
  const room = Math.max(1, width - Array.from(head).length);
  return head + Array.from(typing).slice(-room).join("");
}

export interface TableInput {
  plan: Plan;
  results: Results;
  session: Session;
  /** The goal's place on the chain, printed as `N of M`, or null for a side goal, printed as `side goal`. */
  position: number | null;
  total: number;
  /** The subjects of the session's commits, oldest first. */
  commits: string[];
  width: number;
  utf8: boolean;
}

/** The goal as a table: one row per stage, its state, its green checks of all, and what it does. */
export function goalTable(t: TableInput): string[] {
  const { plan, results, session } = t;
  const dot = t.utf8 ? " · " : " - ";
  const now = currentStage(plan, results);
  const rows = [...plan.stages]
    .sort((a, b) => a.number - b.number)
    .map((st) => {
      const checks = plan.checks.filter((c) => c.stage === st.number);
      const green = checks.filter((c) => results.get(c.id) === true).length;
      const state = green === checks.length ? "done" : st.number === now ? "now" : "ahead";
      return { n: String(st.number), stage: st.title, state, green: String(green), all: String(checks.length), does: st.summary ?? st.title };
    });
  const w = (key: "n" | "stage" | "green" | "all", head = "") => Math.max(head.length, ...rows.map((r) => r[key].length));
  const nw = w("n", "#");
  const sw = w("stage", "stage");
  const gw = w("green");
  const aw = w("all");
  const cw = Math.max("checks".length, gw + 1 + aw);
  const lead = `${" ".repeat(nw + 1)}  ${" ".repeat(sw)}  ${" ".repeat(5)}  ${" ".repeat(cw)}  `;
  const room = Math.max(10, t.width - lead.length);

  const out = [
    cut(`goal ${plan.slug}${dot}${t.position === null ? "side goal" : `${t.position} of ${t.total}`}${dot}${percent(plan, results)}%${dot}session ${session.number}`, t.width),
    `${"#".padStart(nw + 1)}  ${"stage".padEnd(sw)}  ${"state".padEnd(5)}  ${"checks".padEnd(cw)}  what it does`,
  ];
  out.push((t.utf8 ? "─" : "-").repeat(Math.min(t.width, lead.length + room)));
  for (const r of rows) {
    const checks = `${r.green.padStart(gw)}/${r.all.padEnd(aw)}`.padStart(cw);
    const [first = "", ...more] = wrap(r.does, room);
    out.push(`${r.n.padStart(nw + 1)}  ${r.stage.padEnd(sw)}  ${r.state.padEnd(5)}  ${checks}  ${first}`);
    for (const line of more) out.push(lead + line);
  }
  const last = t.commits.at(-1);
  const commits = last === undefined ? "no commits" : `${t.commits.length} commit${t.commits.length === 1 ? "" : "s"}, the last \`${last}\``;
  out.push(cut(`now: ${session.doing || "nothing"}${dot}this session: ${commits}`, t.width));
  return out;
}
