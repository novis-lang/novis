// The `sand` format, which falling-sand keeps and kick-vst copies, read as
// `crates/looper-core/src/import/sand.rs` reads it: `data/chain.json`, `data/floor.json`, and per goal
// a record `data/goals/<slug>.json`, a handoff `data/goals/<slug>.handoff.json` and its prose
// `docs/agent/goals/<slug>.md`. The mapping is `docs/upgrade-guide.md` § *Mapping notes per project*.
//
// - **The chain** is `goals`, with `live` first and a goal pinned `position: last` behind the rest.
// - **A goal's front matter** comes from its record. The title is the record's title before its first
//   `: `, the summary the first sentence of the prose's `## The target`, and `context.modules` the
//   context, per stage too.
// - **A stage is `done`** when its number is below the stage the handoff's next group names, because the
//   format kept no stage state of its own: its driver judged a stage by its checks. Every stage is open
//   when the handoff names none.
// - **A check** keeps its id, stage, name, cwd, exit, timeout, reach and both `*_contains`.
//   `cargo-named` runs `cargo` with its `args` but `-p` or `--package`, and keeps its `tests`. When no
//   `--test`, `--lib`, `--bin` or `--example` is left, the tests follow `--` as filters.
// - **`want`** was judged over stdout and stderr, so it becomes `output_ordered`, or `output_contains`
//   in a `contains` check. In an `exact` check it becomes `stdout_exact`.
// - **A check that runs `bun <tool> goal harvested`** is dropped, because looper's harvest gate replaces
//   it. A floor check loses its stage, which meant nothing there.
// - **The prose** keeps every section's text verbatim, its headings one level deeper: `## The target`,
//   `## Why here` and any section looper has no place for go under `## Why`; each `## Stage <n> — …`
//   becomes `### <n>. <title>` under `## Stages`, unless the record has no such stage, when it goes
//   under `## Why` too; `## Standing decisions` stays. The paragraph before the first section, which
//   points at the record, is dropped.
// - **The handoff** becomes `## Handoff`. An open item without a `path:line` anchor gets the first file
//   of its group's file set as one.
// - **The playbook** is already in looper's shape and is left where it is.
// - Every JSON file is read as serde reads it: an unknown key or a value of the wrong type refuses it.
//
// Known gap: a `min-bytes` check, which counted a stream's bytes, becomes a `TODO`.

import { readFileSync } from "node:fs";
import { join } from "node:path";
import { type Check, emptyCheck, FORMAT, type Goal, GoalError, type Mapping, normalize, Refused, type Stage } from "../goal/model.ts";
import { bool, list, type Reader, Refusal, type Seg, str, strings, table, u32, u64 } from "../goal/reader.ts";
import { anchors, sectionsOf } from "../goal/sections.ts";
import type { Import } from "./import.ts";

/** `inner`, or `undefined` for a JSON `null`, as serde reads an `Option`. */
export function opt<T>(inner: Reader<T>): Reader<T | undefined> {
  return { expects: inner.expects, read: (v, at) => (v === null ? undefined : inner.read(v, at)) };
}

interface ChainJson {
  live: string;
  goals: string[];
}

const chainJson = table<ChainJson>("ChainJson", { live: str, goals: strings }, () => ({ live: "", goals: [] }), [
  "live",
  "goals",
]);

/** A sand check, its keys as the JSON spells them. */
export interface CheckJson {
  id: string;
  kind: string;
  stage: number;
  name?: string;
  argv?: string[];
  args?: string[];
  cwd?: string;
  tests?: string[];
  stream?: string;
  want?: string[];
  exit?: string;
  stdoutContains?: string[];
  stderrContains?: string[];
  minBytes?: number;
  timeoutSec?: number;
  reach?: boolean;
}

export const checkJson = table<CheckJson>(
  "CheckJson",
  {
    id: str,
    kind: str,
    stage: u32,
    name: opt(str),
    argv: opt(strings),
    args: opt(strings),
    cwd: opt(str),
    tests: opt(strings),
    stream: opt(str),
    want: opt(strings),
    exit: opt(str),
    stdoutContains: opt(strings),
    stderrContains: opt(strings),
    minBytes: opt(u64),
    timeoutSec: opt(u64),
    reach: opt(bool),
  },
  () => ({ id: "", kind: "", stage: 0 }),
  ["id", "kind", "stage"],
);

const floorJson = table<{ checks: CheckJson[] }>("FloorJson", { checks: list(checkJson) }, () => ({ checks: [] }), ["checks"]);

interface ContextJson {
  modules?: string[];
}

const contextJson = table<ContextJson>("ContextJson", { modules: opt(strings) }, () => ({}));

interface StageJson {
  number: number;
  title: string;
  summary?: string;
  context?: ContextJson;
}

const stageJson = table<StageJson>(
  "StageJson",
  { number: u32, title: str, summary: opt(str), context: opt(contextJson) },
  () => ({ number: 0, title: "" }),
  ["number", "title"],
);

interface GoalJson {
  title: string;
  position?: string;
  context: ContextJson;
  stages: StageJson[];
  checks: CheckJson[];
}

const goalJson = table<GoalJson>(
  "GoalJson",
  { title: str, position: opt(str), context: contextJson, stages: list(stageJson), checks: list(checkJson) },
  () => ({ title: "", context: {}, stages: [], checks: [] }),
  ["title", "stages", "checks"],
);

interface ItemJson {
  done: boolean;
  text: string;
}

interface NextJson {
  stage?: number;
  title?: string;
  files: string[];
  note?: string;
  items: ItemJson[];
  after?: string;
}

/** A goal's handoff, which nv keeps in the same shape. */
export interface HandoffJson {
  state: string;
  next: NextJson;
  backlog: string[];
}

const itemJson = table<ItemJson>("ItemJson", { done: bool, text: str }, () => ({ done: false, text: "" }), ["done", "text"]);

const nextJson = table<NextJson>(
  "NextJson",
  { stage: opt(u32), title: opt(str), files: strings, note: opt(str), items: list(itemJson), after: opt(str) },
  () => ({ files: [], items: [] }),
  ["files", "items"],
);

export const handoffJson = table<HandoffJson>(
  "HandoffJson",
  { state: str, next: nextJson, backlog: strings },
  () => ({ state: "", next: { files: [], items: [] }, backlog: [] }),
  ["state", "next", "backlog"],
);

/** One goal's record as `goalOf` takes it, with its checks already mapped. */
export interface GoalRecord {
  title: string;
  context: string[];
  stages: RecordStage[];
  checks: Check[];
  extra: Mapping | undefined;
}

export interface RecordStage {
  number: number;
  title: string;
  summary: string | undefined;
  context: string[] | undefined;
  extra: Mapping | undefined;
}

/** The file at `rel` under `root` with LF line endings, or `undefined` when there is none. */
export function readText(root: string, rel: string): string | undefined {
  try {
    return normalize(readFileSync(join(root, rel), "utf8"));
  } catch (e) {
    if ((e as NodeJS.ErrnoException).code === "ENOENT") return undefined;
    throw new GoalError(join(root, rel), undefined, (e as Error).message);
  }
}

function pathText(at: Seg[]): string {
  let out = "";
  for (const seg of at) out += typeof seg === "number" ? `[${seg}]` : out === "" ? seg : `.${seg}`;
  return out;
}

/** The JSON file at `rel` read with `reader`, or `undefined` when there is none. */
export function readJson<T>(root: string, rel: string, reader: Reader<T>): T | undefined {
  const text = readText(root, rel);
  if (text === undefined) return undefined;
  let doc: unknown;
  try {
    doc = JSON.parse(text);
  } catch (e) {
    throw new GoalError(rel, undefined, (e as Error).message);
  }
  try {
    return reader.read(doc, []);
  } catch (e) {
    if (!(e instanceof Refusal)) throw e;
    const where = pathText(e.point === "key" ? e.at.slice(0, -1) : e.at);
    throw new GoalError(rel, undefined, where === "" ? e.message : `${where}: ${e.message}`);
  }
}

/** Reads the sand project at `root`. */
export function readSand(root: string): Import {
  const chain = readJson(root, "data/chain.json", chainJson);
  if (chain === undefined) throw new Refused("data/chain.json is missing, so this is not a sand project");
  const floor = readJson(root, "data/floor.json", floorJson) ?? { checks: [] };
  const notes: string[] = [];

  const order = chain.goals.includes(chain.live) ? [chain.live] : [];
  order.push(...chain.goals.filter((s) => s !== chain.live));

  const goals: Goal[] = [];
  const last: string[] = [];
  for (const slug of order) {
    const recordPath = `data/goals/${slug}.json`;
    const g = readJson(root, recordPath, goalJson);
    if (g === undefined) throw new Refused(`the chain names ${slug}, and ${recordPath} is missing`);
    if (g.position === "last") last.push(slug);
    const handoff = readJson(root, `data/goals/${slug}.handoff.json`, handoffJson);
    const prose = readText(root, `docs/agent/goals/${slug}.md`) ?? "";
    const owner = `goal ${slug}`;
    const record: GoalRecord = {
      title: g.title,
      context: g.context.modules ?? [],
      stages: g.stages.map((s) => ({
        number: s.number,
        title: s.title,
        summary: s.summary,
        context: s.context?.modules,
        extra: undefined,
      })),
      checks: g.checks.flatMap((c) => checkOf(c, owner, notes) ?? []),
      extra: undefined,
    };
    goals.push(goalOf(slug, record, handoff, prose, notes));
  }
  const chainOrder = [...order.filter((s) => !last.includes(s)), ...last];
  const floorChecks = floor.checks.flatMap((c) => checkOf(c, "the floor", notes) ?? []).map((c) => ({ ...c, stage: undefined }));
  notes.push("the playbook is already in looper's shape: check its path against `[project] playbook`");
  return {
    chain: { looper: FORMAT, chain: chainOrder, floor: floorChecks },
    goals,
    playbook: undefined,
    bullet_files: [],
    notes,
    generated: [],
  };
}

/** One sand check as a looper check, or `undefined` for one looper replaces. */
export function checkOf(c: CheckJson, owner: string, notes: string[]): Check | undefined {
  const argv = c.argv ?? [];
  if (argv[0] === "bun" && argv[2] === "goal" && argv[3] === "harvested") {
    notes.push(`${owner}: dropped check ${c.id}, which looper's harvest gate replaces`);
    return undefined;
  }
  const out: Check = {
    ...emptyCheck(),
    id: c.id,
    stage: c.stage,
    name: c.name,
    cwd: c.cwd,
    exit: c.exit === "nonzero" ? "nonzero" : undefined,
    timeout: c.timeoutSec,
    reach: c.reach,
    stdout_contains: c.stdoutContains,
    stderr_contains: c.stderrContains,
  };
  switch (c.kind) {
    case "cargo-named":
      out.run = cargoRun(c.args ?? [], c.tests ?? []);
      out.tests = c.tests;
      break;
    case "exact":
      out.run = argv;
      out.stdout_exact = c.want?.join("\n");
      break;
    case "command":
    case "ordered":
      out.run = argv;
      out.output_ordered = c.want;
      break;
    case "contains":
      out.run = argv;
      out.output_contains = c.want;
      break;
    default: {
      out.run = ["TODO", ...argv];
      const wanted =
        c.kind === "min-bytes" && c.minBytes !== undefined
          ? `at least ${c.minBytes} bytes of ${c.stream ?? "stdout"}`
          : `a \`${c.kind}\` check`;
      notes.push(`${owner}: check ${c.id} lost its condition, ${wanted}, which looper cannot judge: it is a TODO`);
    }
  }
  return out;
}

/**
 * `cargo` with `args` but the package flag, which a workspace does not need to find a test target. When
 * no target is left, `tests` follow `--` as filters, so the check runs those tests alone.
 */
export function cargoRun(args: string[], tests: string[]): string[] {
  const out = ["cargo"];
  for (let i = 0; i < args.length; i++) {
    const a = args[i] as string;
    if (a === "-p" || a === "--package") i++;
    else if (!a.startsWith("--package=") && !a.startsWith("-p=")) out.push(a);
  }
  const flag = (a: string, f: string) => a === f || a.startsWith(`${f}=`);
  const target = out.some((a) => ["--test", "--lib", "--bin", "--example"].some((f) => flag(a, f)));
  if (!target && tests.length > 0) {
    if (!out.includes("--")) out.push("--");
    out.push(...tests);
  }
  return out;
}

/** The title before the record title's first `: `, with its first letter upper case. */
function titleOf(record: string): string {
  const at = record.indexOf(": ");
  const head = (at < 0 ? record : record.slice(0, at)).trim();
  const [first, ...rest] = [...head];
  return first === undefined ? "" : first.toUpperCase() + rest.join("");
}

/** The first sentence of the text's first paragraph, on one line. */
export function firstSentence(text: string): string {
  const para = (text.split("\n\n")[0] ?? "").split(/\s+/).filter((w) => w !== "").join(" ");
  const at = para.indexOf(". ");
  return at < 0 ? para : para.slice(0, at + 1);
}

/** Every heading of the text one level deeper, so it nests under the heading the import gives it. */
function demote(text: string): string {
  return text
    .split("\n")
    .map((line) => (line.startsWith("##") && line.replace(/^#+/, "").startsWith(" ") ? `#${line}` : line))
    .join("\n");
}

/** The stage number of a `Stage <n> — <title>` section name. */
function stageSection(name: string): number | undefined {
  if (!name.startsWith("Stage ")) return undefined;
  const digits = /^[0-9]*/.exec(name.slice("Stage ".length))?.[0] ?? "";
  return digits === "" ? undefined : Number(digits);
}

/**
 * One goal from its record, its handoff and its prose. A prose stage section with no stage in the
 * record goes under `## Why` with the other sections.
 */
export function goalOf(slug: string, g: GoalRecord, handoff: HandoffJson | undefined, prose: string, notes: string[]): Goal {
  const sections = sectionsOf(prose, 2);
  const textOf = (name: string) => sections.find((s) => s.name === name)?.text;
  const nextStage = handoff?.next.stage;
  const inRecord = (name: string) => {
    const n = stageSection(name);
    return n !== undefined && g.stages.some((st) => st.number === n);
  };

  let why = "";
  for (const s of sections.filter((s) => s.name !== "Standing decisions" && !inRecord(s.name))) {
    why += `### ${s.name}\n\n${demote(s.text)}\n\n`;
  }
  let stagesText = "";
  const stages: Stage[] = [];
  for (const st of g.stages) {
    const found = sections.find((s) => stageSection(s.name) === st.number);
    let body = found === undefined ? "" : demote(found.text);
    if (st.summary !== undefined && !body.includes("**Does:**")) body = `**Does:** ${st.summary}\n\n${body}`;
    stagesText += `### ${st.number}. ${st.title}\n\n${body.trimEnd()}\n\n`;
    stages.push({
      id: st.number,
      title: st.title,
      status: nextStage !== undefined && st.number < nextStage ? "done" : "open",
      gate: undefined,
      context: st.context,
      extra: st.extra,
    });
  }
  const target = textOf("The target");
  const sentence = target === undefined ? "" : firstSentence(target);
  const summary = sentence === "" ? g.title : sentence;

  const title = titleOf(g.title);
  const decisions = textOf("Standing decisions");
  const body =
    `\n# ${title}\n\n## Why\n\n${why}## Standing decisions\n\n${decisions === undefined ? "" : demote(decisions)}\n\n` +
    `## Stages\n\n${stagesText}## Handoff\n\n${handoffText(slug, handoff, g.stages, notes)}`;
  return {
    slug,
    front: {
      looper: FORMAT,
      title,
      summary,
      context: g.context,
      stages,
      checks: g.checks,
      settings: undefined,
      extra: g.extra,
    },
    body: `${body.trimEnd()}\n`,
    body_line: 1,
  };
}

/** The `## Handoff` section's text: `### State`, `### Next` and `### Backlog`. */
function handoffText(slug: string, h: HandoffJson | undefined, stages: RecordStage[], notes: string[]): string {
  if (h === undefined) return "### State\n\n### Next\n\n### Backlog\n";
  const n = h.next;
  const stageTitle = n.stage === undefined ? undefined : stages.find((s) => s.number === n.stage)?.title;
  const t = n.title ?? stageTitle;
  let lead: string;
  if (n.stage !== undefined && t !== undefined) lead = `**Stage ${n.stage}: ${t}**`;
  else if (n.stage !== undefined) lead = `**Stage ${n.stage}**`;
  else if (t !== undefined) lead = `**${t}**`;
  else lead = "";
  if (n.files.length > 0) lead += `, one file set: ${n.files.map((f) => `\`${f}\``).join(", ")}.`;
  if (n.note !== undefined) lead += ` ${n.note}`;
  let next = `${lead.trim()}\n\n`;
  for (const item of n.items) {
    let text = item.text.replaceAll("\n", "\n  ");
    if (!item.done && anchors(text.split("\n")[0] ?? "").length === 0) {
      const anchor = n.files[0] ?? `docs/goals/${slug}.md`;
      text += ` \`${anchor}:1\``;
      notes.push(`goal ${slug}: a handoff item had no anchor and now points at \`${anchor}:1\``);
    }
    next += `- [${item.done ? "x" : " "}] ${text}\n`;
  }
  if (n.after !== undefined) next += `\n${n.after}\n`;
  const backlog = h.backlog.map((b) => `- ${b.replaceAll("\n", "\n  ")}\n`).join("");
  return `### State\n\n${h.state.trim()}\n\n### Next\n\n${next.trimEnd()}\n\n### Backlog\n\n${backlog}`;
}
