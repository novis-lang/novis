// A project's `looper.toml` and the machine's `machine.toml`, read as `Config` and `Machine` in
// `crates/looper-core/src/model.rs` read them. `docs/design.md` § *`looper.toml`* documents every key.
//
// - `Bun.TOML` parses the text. A reader per table then checks each key in the file's order, and fills
//   every key the file leaves out with looper's default.
// - A key no table knows, a value of the wrong type and a missing required key are refused with
//   looper's message. The message names the line and column, which the kit finds in the text itself,
//   because `Bun.TOML` keeps no positions.
// - A key keeps its name from the file, so `[loop] max_stalls` is `config.loop.max_stalls`.
// - The checks that span keys then run in looper's order: verify, merge, gates, lanes, commits, and
//   last the `[[guard.deny]]` patterns.

import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { toRegex } from "./glob.ts";
import {
  argvs,
  bool,
  debug,
  isMap,
  list,
  type Point,
  type Reader,
  Refusal,
  type Seg,
  str,
  strings,
  strMap,
  table,
  u16,
  u32,
  u64,
  usize,
  variants,
  wrongType,
} from "./reader.ts";

/** A `looper.toml` or `machine.toml` the kit refuses, with looper's message. */
export class ConfigError extends Error {
  constructor(
    readonly path: string,
    readonly detail: string,
  ) {
    super(`${path}: ${detail}`);
  }
}

// ---------------------------------------------------------------------------------------------------
// The tables

export interface Config {
  project: Project;
  agent: Agent;
  budget: Budget;
  loop: LoopSettings;
  verify: Verify;
  checks: Checks;
  hooks: Hooks;
  guard: Guard;
  gates: Gates;
  links: Links;
  prose: Prose;
  prompts: Prompts;
  orient: Orient;
  wrap: Wrap;
  sync: SyncSettings;
  commits: Commits;
  jobs: Jobs;
  merge: Merge;
  /** `[[launch]]`: the project's launchers. */
  launch: Launch[];
}

export interface Project {
  name: string;
  /** The stack profiles `looper init` wrote, such as `rust` and `bun`. */
  stacks: string[];
  goals_dir: string;
  playbook: string;
  /** The directory with one file per playbook bullet. Unset, the playbook is the one file. */
  playbook_dir: string | undefined;
  /** The harvest gate of retirement. `harvest = false` reads as `null`. */
  harvest: Harvest | null;
  /** Whether retirement moves a goal's checks into the floor. */
  carry_to_floor: boolean;
  scratch: string;
  /** The architecture doc the agents block points at. Empty, the block names none. */
  design: string;
}

/** Where a reached goal's findings go: a file, and a heading in it with `{slug}` in place of the slug. */
export interface Harvest {
  file: string;
  section: string;
}

export interface Agent {
  adapter: string;
  model: string;
  effort: string;
  permission_mode: string;
  tools: string[];
}

/** Token budgets of one session. `soft` is deprecated: it still loads, and nothing reads it. */
export interface Budget {
  hard: number;
  kill: number | undefined;
  /** The ceiling of an interactive session and of each subagent. Unset, it is `hard`. */
  interactive: number | undefined;
  /** The context past which a session takes no new slice. */
  slice: number;
  soft?: number;
}

export type Sweep = "done" | "every";

export interface LoopSettings {
  max_stalls: number;
  max_retries: number;
  repairs_per_goal: number;
  done_retries: number;
  max_limit_wait_s: number;
  session_timeout_min: number;
  spin_limit: number;
  /** The GiB free on the worktree's volume below which no session starts. 0 turns the gate off. */
  min_free_gb: number;
  sweep: Sweep;
  /** The worktrees side loops take turns in, at least 1. */
  lanes: number;
}

/** The steps `looper verify` runs: `run`, a list of argvs, or `[[verify.step]]` tables, never both. */
export interface Verify {
  run: string[][];
  step: Step[];
}

export interface Step {
  /** Letters, digits, `-`, `_` and `.`. */
  name: string;
  run: string[];
  /** The root-relative paths and globs the step reads. With none, it reads the whole tree. */
  inputs: string[];
  /** Whether the step takes a heavy-job slot. */
  heavy: boolean;
}

/** What every check, hook and verify step gets. `hide_windows` is `[jobs] hide_windows`, copied here. */
export interface Checks {
  timeout: number;
  env: Record<string, string>;
  hide_windows: boolean;
}

export interface Hooks {
  before_run: Hook[];
  before_session: Hook[];
  after_session: Hook[];
  before_sweep: Hook[];
  after_retire: Hook[];
}

export type OnFail = "pause" | "warn";

/** One hook, written as an argv or as a table `{ run, timeout, on_fail, detach }`. */
export interface Hook {
  run: string[];
  /** In seconds. Without one, `[checks] timeout`. */
  timeout: number | undefined;
  on_fail: OnFail;
  detach: boolean;
}

export interface Guard {
  deny: DenyRule[];
}

/** A shell command the guard refuses, and what the agent is told. */
export interface DenyRule {
  pattern: string;
  reason: string;
}

/** The project's own command for a gate. An empty argv keeps looper's check. */
export interface Gates {
  links: string[];
  layout: string[];
  prose: string[];
  guard: string[];
  json: string[];
}

/** The files `links` reads. A pathspec such as `:(exclude)tests/fixtures/**` leaves a tree out. */
export interface Links {
  paths: string[];
}

export interface Prose {
  banned: string[];
  scope: string;
  paths: string[];
}

export interface Prompts {
  session: string | undefined;
  repair: string | undefined;
}

export interface Orient {
  extra: string[][];
  /** The most playbook bullets TRAPS prints. */
  traps: number;
}

export interface Wrap {
  section: WrapSection[];
  checks: string[][];
}

export interface WrapSection {
  name: string;
  run: string[];
}

export interface SyncSettings {
  skip: string[];
}

export interface Commits {
  subject_max: number;
  types: string[];
}

export interface Jobs {
  heavy: string[];
  hide_windows: boolean;
  /** Patterns, in Rust's `regex` syntax, of Bash commands whose windows stay where they open. */
  show_windows: string[];
  /** Seconds a Bash call may wait with no output and no CPU before the hang watch kills it; 0 never. */
  hang_stall_s: number;
  /** Seconds a Bash call may use the CPU with no output before the hang watch kills it; 0 never. */
  hang_spin_s: number;
  /** Patterns, in Rust's `regex` syntax, of Bash commands the hang watch leaves alone. */
  hang_exempt: string[];
}

export interface Merge {
  generated: string[];
  render: string[][];
}

export type LaunchKind = "service" | "task";
export type LaunchWindow = "show" | "hide";

/** One `[[launch]]` table. A missing `name`, `title`, `about` or `run` reads as empty, for lint to name. */
export interface Launch {
  name: string;
  title: string;
  about: string;
  run: string[];
  build: string[];
  cwd: string | undefined;
  env: Record<string, string>;
  open: string | undefined;
  kind: LaunchKind;
  args: boolean;
  port: number | undefined;
  ready: string | undefined;
  heavy: boolean;
  window: LaunchWindow;
  stop_grace: number;
  copy: boolean;
}

/** `machine.toml` in looper's home: the settings of the whole machine. */
export interface Machine {
  slots: Slots;
}

export interface Slots {
  /** From 1 to `MAX_SLOTS`. */
  heavy: number;
}

/** The types a project that sets none allows. */
export const COMMIT_TYPES = ["feat", "fix", "docs", "refactor", "perf", "test", "chore", "build"];

/** The scope of the prose rules a project that sets none has. */
export const PROSE_SCOPE = "Every sentence";

/** The gates whose command `[gates] json` may name. */
export const JSON_GATES = ["links", "layout", "prose"];

/** The most heavy-job slots `machine.toml` may name. */
export const MAX_SLOTS = 64;

/** The file's name in looper's home. */
export const MACHINE_FILE = "machine.toml";

/** The most launchers a project declares. */
export const MAX_LAUNCHERS = 32;

/** The most characters a launcher's `about` has. */
export const ABOUT_MAX = 200;

/** The placeholders a launcher's `run`, `build`, `env` and `open` may hold. */
export const PLACEHOLDERS = ["{port}", "{args}"];

/** The value of `ready` that marks a launch ready when its first window shows. */
export const READY_WINDOW = "window";

/** The keys `looper.toml` still accepts and looper no longer reads, as `table.key`, with what to do. */
export const DEPRECATED: readonly (readonly [string, string])[] = [
  ["budget.soft", "Delete it, and set `[budget] slice` to where a session takes no new slice."],
];

// ---------------------------------------------------------------------------------------------------
// The defaults

/** Every key of `looper.toml` at looper's default. */
export function defaultConfig(): Config {
  return {
    project: {
      name: "",
      stacks: [],
      goals_dir: "docs/goals",
      playbook: "docs/agent/playbook.md",
      playbook_dir: undefined,
      harvest: defaultHarvest(),
      carry_to_floor: true,
      scratch: ".agent-tmp",
      design: "docs/design.md",
    },
    agent: {
      adapter: "claude",
      model: "opus",
      effort: "",
      permission_mode: "auto",
      tools: ["Bash", "Edit", "Write", "Read", "Grep", "Glob", "Agent", "Monitor", "TaskStop"],
    },
    budget: { hard: 250_000, kill: undefined, interactive: undefined, slice: 200_000 },
    loop: {
      max_stalls: 10,
      max_retries: 3,
      repairs_per_goal: 3,
      done_retries: 3,
      max_limit_wait_s: 6 * 3600,
      session_timeout_min: 240,
      spin_limit: 20,
      min_free_gb: 10,
      sweep: "done",
      lanes: 2,
    },
    verify: { run: [], step: [] },
    checks: { timeout: 600, env: {}, hide_windows: true },
    hooks: { before_run: [], before_session: [], after_session: [], before_sweep: [], after_retire: [] },
    guard: { deny: [] },
    gates: { links: [], layout: [], prose: [], guard: [], json: [] },
    links: { paths: ["*.md"] },
    prose: { banned: [], scope: PROSE_SCOPE, paths: ["*.md"] },
    prompts: { session: undefined, repair: undefined },
    orient: { extra: [], traps: 20 },
    wrap: { section: [], checks: [] },
    sync: { skip: [] },
    commits: { subject_max: 71, types: [...COMMIT_TYPES] },
    jobs: { heavy: [], hide_windows: true, show_windows: [], hang_stall_s: 90, hang_spin_s: 600, hang_exempt: [] },
    merge: { generated: [], render: [] },
    launch: [],
  };
}

export function defaultHarvest(): Harvest {
  return { file: "docs/findings.md", section: "## {slug}" };
}

/** A `[[launch]]` table with every key at its default. */
export function defaultLaunch(): Launch {
  return {
    name: "",
    title: "",
    about: "",
    run: [],
    build: [],
    cwd: undefined,
    env: {},
    open: undefined,
    kind: "service",
    args: false,
    port: undefined,
    ready: undefined,
    heavy: true,
    window: "show",
    stop_grace: 5,
    copy: true,
  };
}

/** `machine.toml` at looper's default. */
export function defaultMachine(): Machine {
  return { slots: { heavy: 2 } };
}

// ---------------------------------------------------------------------------------------------------
// What a table offers

/** The environment a project's command runs with: this process's, `env` over it, and `LOOPER_WINDOWS=show` unless windows are hidden. */
export function checkEnv(checks: Checks): Record<string, string | undefined> {
  const env: Record<string, string | undefined> = { ...process.env, ...checks.env };
  if (!checks.hide_windows) env.LOOPER_WINDOWS = "show";
  return env;
}

/** Every verify step in order. An argv of the `run` form is a step named by its place, from `1`. */
export function verifySteps(verify: Verify): Step[] {
  const unnamed = verify.run.map((run, i) => ({ name: String(i + 1), run, inputs: [], heavy: true }));
  return [...unnamed, ...verify.step];
}

/** Whether a glob of `[merge] generated` matches the root-relative `path`. */
export function mergeDeclares(merge: Merge, path: string): boolean {
  return merge.generated.some((g) => toRegex(g)?.test(path) ?? false);
}

/** Each gate a command is set for, as `[gate, argv]`. */
export function gatesSet(gates: Gates): [string, string[]][] {
  const all: [string, string[]][] = [
    ["links", gates.links],
    ["layout", gates.layout],
    ["prose", gates.prose],
    ["guard", gates.guard],
  ];
  return all.filter(([, argv]) => argv.length > 0);
}

/** Whether the command of `gate` takes `--json`. */
export function takesJson(gates: Gates, gate: string): boolean {
  return gates.json.includes(gate);
}

/** The ceiling of an interactive session and of each subagent. */
export function interactiveBudget(budget: Budget): number {
  return budget.interactive ?? budget.hard;
}

/** Each prompt file named, as `[which, path]`, the session's first. */
export function promptsNamed(prompts: Prompts): [string, string][] {
  const all: [string, string | undefined][] = [
    ["session", prompts.session],
    ["repair", prompts.repair],
  ];
  return all.filter((p): p is [string, string] => p[1] !== undefined);
}

/**
 * A `[[guard.deny]]` pattern, written in the syntax of Rust's `regex`, as a JavaScript regex. A leading
 * `(?flags)` becomes the regex's flags, and `(?P<name>` becomes `(?<name>`. It throws as `new RegExp`
 * does on a pattern that forms none.
 */
export function ruleRegex(pattern: string): RegExp {
  let flags = "";
  const source = pattern
    .replace(/^\(\?([ims]+)\)/, (_, f: string) => {
      flags = f;
      return "";
    })
    .replaceAll("(?P<", "(?<");
  return new RegExp(source, flags);
}

// ---------------------------------------------------------------------------------------------------
// Reading

/** Parses `looper.toml`'s text, refusing what looper's `Config::parse` refuses. */
export function parseConfig(path: string, text: string): Config {
  const cfg = readToml(path, text, configReader);
  cfg.checks.hide_windows = cfg.jobs.hide_windows;
  const problem = verifyProblem(cfg.verify) ?? mergeProblem(cfg.merge) ?? gatesProblem(cfg.gates);
  if (problem !== undefined) throw new ConfigError(path, problem);
  if (cfg.loop.lanes === 0) throw new ConfigError(path, "[loop] lanes needs at least 1 lane");
  if (cfg.commits.subject_max === 0 || cfg.commits.types.length === 0) {
    throw new ConfigError(path, "[commits] needs a subject_max above 0 and at least one type");
  }
  for (const rule of cfg.guard.deny) {
    try {
      ruleRegex(rule.pattern);
    } catch (e) {
      throw new ConfigError(path, `guard.deny pattern ${debug(rule.pattern)}: ${(e as Error).message}`);
    }
  }
  for (const [key, patterns] of [["hang_exempt", cfg.jobs.hang_exempt], ["show_windows", cfg.jobs.show_windows]] as const) {
    for (const pattern of patterns) {
      try {
        ruleRegex(pattern);
      } catch (e) {
        throw new ConfigError(path, `jobs.${key} pattern ${debug(pattern)}: ${(e as Error).message}`);
      }
    }
  }
  return cfg;
}

/** Reads `<root>/looper.toml`, or the defaults when the project has none. */
export function loadConfig(root: string): Config {
  const path = join(root, "looper.toml");
  if (!existsSync(path)) return defaultConfig();
  return parseConfig(path, readFileSync(path, "utf8"));
}

/** The `DEPRECATED` keys `text` sets, in their order there. A text that is not TOML sets none. */
export function deprecatedKeys(text: string): (readonly [string, string])[] {
  let table: Record<string, unknown>;
  try {
    table = Bun.TOML.parse(text) as Record<string, unknown>;
  } catch {
    return [];
  }
  return DEPRECATED.filter(([path]) => {
    const [head, key] = path.split(".") as [string, string];
    const t = table[head];
    return isMap(t) && key in t;
  });
}

/** Parses `machine.toml`'s text, refusing a slot count outside 1 to `MAX_SLOTS`. */
export function parseMachine(path: string, text: string): Machine {
  const machine = readToml(path, text, machineReader);
  const heavy = machine.slots.heavy;
  if (heavy < 1 || heavy > MAX_SLOTS) {
    throw new ConfigError(path, `[slots] heavy must be from 1 to ${MAX_SLOTS}, not ${heavy}`);
  }
  return machine;
}

/** Reads `<home>/machine.toml`, or the defaults when there is none. */
export function loadMachine(home: string): Machine {
  const path = join(home, MACHINE_FILE);
  if (!existsSync(path)) return defaultMachine();
  return parseMachine(path, readFileSync(path, "utf8"));
}

function verifyProblem(verify: Verify): string | undefined {
  if (verify.run.length > 0 && verify.step.length > 0) {
    return "verify has both `run` and `[[verify.step]]`; keep one form";
  }
  const seen = new Set<string>();
  for (const step of verifySteps(verify)) {
    if (!/^[A-Za-z0-9_.-]+$/.test(step.name)) {
      return `verify step name ${debug(step.name)} must be letters, digits, \`-\`, \`_\` or \`.\``;
    }
    if (seen.has(step.name)) return `verify step name ${debug(step.name)} is taken twice`;
    seen.add(step.name);
    if (step.run.length === 0) return `verify step ${debug(step.name)} has no \`run\``;
  }
  return undefined;
}

function mergeProblem(merge: Merge): string | undefined {
  const glob = merge.generated.find((g) => g === "" || toRegex(g) === undefined);
  if (glob !== undefined) return `[merge] generated glob ${debug(glob)} is not a glob`;
  if (merge.render.some((argv) => argv.length === 0)) return "[merge] render holds an empty command";
  return undefined;
}

function gatesProblem(gates: Gates): string | undefined {
  const wrong = gates.json.find((g) => !JSON_GATES.includes(g));
  if (wrong === undefined) return undefined;
  return `[gates] json names ${debug(wrong)}, and only ${JSON_GATES.join(", ")} take --json`;
}

// ---------------------------------------------------------------------------------------------------
// The readers of each table

const harvestTable = table<Harvest>("Harvest", { file: str, section: str }, defaultHarvest);

/** `[project] harvest`: `false` is `null`, and a table is the harvest it names. */
const harvest: Reader<Harvest | null> = {
  expects: "false or a table { file, section }",
  read(v, at) {
    if (v === false) return null;
    if (v === true) throw new Refusal(at, `invalid value: boolean \`true\`, expected ${harvest.expects}`, "value");
    return isMap(v) ? harvestTable.read(v, at) : wrongType(v, harvest.expects, at);
  },
};

const project = table<Project>(
  "Project",
  {
    name: str,
    stacks: strings,
    goals_dir: str,
    playbook: str,
    playbook_dir: str,
    harvest,
    carry_to_floor: bool,
    scratch: str,
    design: str,
  },
  () => defaultConfig().project,
);

const agent = table<Agent>(
  "Agent",
  { adapter: str, model: str, effort: str, permission_mode: str, tools: strings },
  () => defaultConfig().agent,
);

const budget = table<Budget>(
  "Budget",
  { hard: u64, kill: u64, interactive: u64, slice: u64, soft: u64 },
  () => defaultConfig().budget,
);

const loopSettings = table<LoopSettings>(
  "LoopSettings",
  {
    max_stalls: u32,
    max_retries: u32,
    repairs_per_goal: u32,
    done_retries: u32,
    max_limit_wait_s: u64,
    session_timeout_min: u64,
    spin_limit: u32,
    min_free_gb: u64,
    sweep: variants<Sweep>(["done", "every"]),
    lanes: u32,
  },
  () => defaultConfig().loop,
);

const step = table<Step>(
  "Step",
  { name: str, run: strings, inputs: strings, heavy: bool },
  () => ({ name: "", run: [], inputs: [], heavy: true }),
  ["name", "run"],
);

const verify = table<Verify>("Verify", { run: argvs, step: list(step) }, () => defaultConfig().verify);

const checks = table<Checks>("Checks", { timeout: u64, env: strMap }, () => defaultConfig().checks);

const hookTable = table<Hook>(
  "HookTable",
  { run: strings, timeout: u64, on_fail: variants<OnFail>(["pause", "warn"]), detach: bool },
  () => ({ run: [], timeout: undefined, on_fail: "pause", detach: false }),
  ["run"],
);

/** A hook: an argv, which pauses the loop on failure, or a table. */
const hook: Reader<Hook> = {
  expects: "an argv or a table { run, timeout, on_fail, detach }",
  read(v, at) {
    if (Array.isArray(v)) return { run: strings.read(v, at), timeout: undefined, on_fail: "pause", detach: false };
    return isMap(v) ? hookTable.read(v, at) : wrongType(v, hook.expects, at);
  },
};

const hooks = table<Hooks>(
  "Hooks",
  {
    before_run: list(hook),
    before_session: list(hook),
    after_session: list(hook),
    before_sweep: list(hook),
    after_retire: list(hook),
  },
  () => defaultConfig().hooks,
);

const denyRule = table<DenyRule>(
  "DenyRule",
  { pattern: str, reason: str },
  () => ({ pattern: "", reason: "" }),
  ["pattern", "reason"],
);

const guard = table<Guard>("Guard", { deny: list(denyRule) }, () => defaultConfig().guard);

const gates = table<Gates>(
  "Gates",
  { links: strings, layout: strings, prose: strings, guard: strings, json: strings },
  () => defaultConfig().gates,
);

const links = table<Links>("Links", { paths: strings }, () => defaultConfig().links);

const prose = table<Prose>("Prose", { banned: strings, scope: str, paths: strings }, () => defaultConfig().prose);

const prompts = table<Prompts>("Prompts", { session: str, repair: str }, () => defaultConfig().prompts);

const orient = table<Orient>("Orient", { extra: argvs, traps: usize }, () => defaultConfig().orient);

const wrapSection = table<WrapSection>("WrapSection", { name: str, run: strings }, () => ({ name: "", run: [] }));

const wrap = table<Wrap>("Wrap", { section: list(wrapSection), checks: argvs }, () => defaultConfig().wrap);

const sync = table<SyncSettings>("SyncSettings", { skip: strings }, () => defaultConfig().sync);

const commits = table<Commits>("Commits", { subject_max: usize, types: strings }, () => defaultConfig().commits);

const jobsTable = table<Jobs>(
  "Jobs",
  { heavy: strings, hide_windows: bool, show_windows: strings, hang_stall_s: u64, hang_spin_s: u64, hang_exempt: strings },
  () => defaultConfig().jobs,
);

/** What `[jobs] exclusive` is refused with: no job takes every slot (decision 0029). */
export const NO_EXCLUSIVE = "`exclusive` is removed, because a job takes at most one heavy slot. List these commands under `heavy` instead";

/** `[jobs]`, which refuses `exclusive` with `NO_EXCLUSIVE`. */
const jobs: Reader<Jobs> = {
  expects: jobsTable.expects,
  read(v, at) {
    if (isMap(v) && "exclusive" in v) throw new Refusal([...at, "exclusive"], NO_EXCLUSIVE, "key");
    return jobsTable.read(v, at);
  },
};

const merge = table<Merge>("Merge", { generated: strings, render: argvs }, () => defaultConfig().merge);

const launch = table<Launch>(
  "Launch",
  {
    name: str,
    title: str,
    about: str,
    run: strings,
    build: strings,
    cwd: str,
    env: strMap,
    open: str,
    kind: variants<LaunchKind>(["service", "task"]),
    args: bool,
    port: u16,
    ready: str,
    heavy: bool,
    window: variants<LaunchWindow>(["show", "hide"]),
    stop_grace: u64,
    copy: bool,
  },
  defaultLaunch,
);

const configReader = table<Config>(
  "Config",
  {
    project,
    agent,
    budget,
    loop: loopSettings,
    verify,
    checks,
    hooks,
    guard,
    gates,
    links,
    prose,
    prompts,
    orient,
    wrap,
    sync,
    commits,
    jobs,
    merge,
    launch: list(launch),
  },
  defaultConfig,
);

const machineReader = table<Machine>(
  "Machine",
  { slots: table<Slots>("Slots", { heavy: u32 }, () => defaultMachine().slots) },
  defaultMachine,
);

/** Parses `text` with `Bun.TOML` and reads it with `reader`, turning a refusal into looper's message. */
function readToml<T>(path: string, text: string, reader: Reader<T>): T {
  let doc: unknown;
  try {
    doc = Bun.TOML.parse(text);
  } catch (e) {
    throw new ConfigError(path, (e as Error).message);
  }
  try {
    return reader.read(doc, []);
  } catch (e) {
    if (!(e instanceof Refusal)) throw e;
    const span = locate(text, e.at, e.point);
    throw new ConfigError(path, span === undefined ? e.message : snippet(text, span, e.message));
  }
}

// ---------------------------------------------------------------------------------------------------
// Where in the text a refusal points

/** A 0-based line, a 1-based column and a length in characters. */
interface Span {
  line: number;
  column: number;
  length: number;
}

const HEADER = /^\s*\[\[?\s*([\w\-."' ]+?)\s*\]\]?\s*(#.*)?$/;

function headerKeys(line: string): string[] | undefined {
  const m = HEADER.exec(line);
  return m?.[1]?.split(".").map((k) => k.trim().replace(/^["']|["']$/g, ""));
}

/** The `nth` header line whose keys are `keys`, pointing at its last key. */
function findHeader(lines: string[], keys: string[], nth: number): Span | undefined {
  let seen = 0;
  for (let l = 0; l < lines.length; l++) {
    const line = lines[l] as string;
    const found = headerKeys(line);
    if (found?.length !== keys.length || !found.every((k, i) => k === keys[i])) continue;
    if (seen++ < nth) continue;
    const last = keys[keys.length - 1] as string;
    return { line: l, column: line.lastIndexOf(last, line.indexOf("]")) + 1, length: last.length };
  }
  return undefined;
}

/** The first `key =` or `key.` from line `from`, column `col`, before the next header line. */
function findKey(lines: string[], key: string, from: number, col: number): Span | undefined {
  const k = key.replace(/[\\^$.*+?()[\]{}|-]/g, "\\$&");
  const re = new RegExp(`(^|[\\s{,])(${k}|"${k}"|'${k}')\\s*[=.]`);
  for (let l = from; l < lines.length; l++) {
    const line = lines[l] as string;
    if (l > from && headerKeys(line) !== undefined) return undefined;
    const start = l === from ? col : 0;
    const m = re.exec(line.slice(start));
    if (m) return { line: l, column: start + m.index + (m[1] as string).length + 1, length: (m[2] as string).length };
  }
  return undefined;
}

/** A 0-based line and column in the text. */
interface Pos {
  line: number;
  col: number;
}

/** The first position from `at` that is not blank, a comment or a line break, or `undefined` at the end. */
function skipBlank(lines: string[], at: Pos, commas: boolean): Pos | undefined {
  let { line, col } = at;
  while (line < lines.length) {
    const text = lines[line] as string;
    const c = text[col];
    if (c === undefined || c === "#") {
      line++;
      col = 0;
    } else if (c === " " || c === "\t" || (commas && c === ",")) {
      col++;
    } else {
      return { line, col };
    }
  }
  return undefined;
}

/** Where the one value that starts at `at` ends: past its closing quote or bracket, or its last bare character. */
function valueEnd(lines: string[], at: Pos): Pos {
  let { line, col } = at;
  const first = (lines[line] as string)[col];
  if (first !== "[" && first !== "{" && first !== '"' && first !== "'") {
    const text = lines[line] as string;
    while (col < text.length && !/[\s,}\]#]/.test(text[col] as string)) col++;
    return { line, col };
  }
  let depth = 0;
  let quote: string | undefined;
  while (line < lines.length) {
    const text = lines[line] as string;
    for (; col < text.length; col++) {
      const c = text[col] as string;
      if (quote !== undefined) {
        if (c === "\\" && quote === '"') col++;
        else if (c === quote) {
          quote = undefined;
          if (depth === 0) return { line, col: col + 1 };
        }
      } else if (c === '"' || c === "'") {
        quote = c;
      } else if (c === "#") {
        break;
      } else if (c === "[" || c === "{") {
        depth++;
      } else if ((c === "]" || c === "}") && --depth === 0) {
        return { line, col: col + 1 };
      }
    }
    line++;
    col = 0;
  }
  return { line: lines.length - 1, col: (lines[lines.length - 1] as string).length };
}

/** The span from `start` to `end`, cut at the end of `start`'s line. */
function spanOf(lines: string[], start: Pos, end: Pos): Span {
  const stop = end.line === start.line ? end.col : (lines[start.line] as string).length;
  return { line: start.line, column: start.col + 1, length: Math.max(1, stop - start.col) };
}

/** Where the value after the key at `span` starts. */
function valueStart(lines: string[], span: Span): Pos | undefined {
  const eq = (lines[span.line] as string).indexOf("=", span.column - 1 + span.length);
  return eq < 0 ? undefined : skipBlank(lines, { line: span.line, col: eq + 1 }, false);
}

/** The `nth` element of the inline array that starts at `at`. */
function elementSpan(lines: string[], at: Pos, nth: number): Span | undefined {
  if ((lines[at.line] as string)[at.col] !== "[") return undefined;
  let pos = skipBlank(lines, { line: at.line, col: at.col + 1 }, true);
  for (let i = 0; pos !== undefined && (lines[pos.line] as string)[pos.col] !== "]"; i++) {
    const end = valueEnd(lines, pos);
    if (i === nth) return spanOf(lines, pos, end);
    pos = skipBlank(lines, end, true);
  }
  return undefined;
}

/**
 * Where the path `at` stands in the text. Each key is found in turn, as a header or as a `key =` after
 * the one before, and an index as an element of the inline array before it. `point` then picks the
 * key, its value, or the table: a header line whole, or an inline table's value.
 */
function locate(text: string, at: Seg[], point: Point): Span | undefined {
  const lines = text.split("\n").map((l) => l.replace(/\r$/, ""));
  const keys: string[] = [];
  let span: Span | undefined;
  let kind: "header" | "key" | "element" = "key";
  let cursor: Pos = { line: 0, col: 0 };
  for (const [i, seg] of at.entries()) {
    if (typeof seg === "number") {
      let start: Pos | undefined;
      if (span !== undefined && kind === "key") start = valueStart(lines, span);
      if (span !== undefined && kind === "element") start = { line: span.line, col: span.column - 1 };
      const element = start === undefined ? undefined : elementSpan(lines, start, seg);
      if (element !== undefined) {
        span = element;
        kind = "element";
        cursor = { line: element.line, col: element.column - 1 };
      }
      continue;
    }
    keys.push(seg);
    const next = at[i + 1];
    const header = findHeader(lines, keys, typeof next === "number" ? next : 0);
    const found = header ?? findKey(lines, seg, cursor.line, cursor.col);
    if (found === undefined) continue;
    span = found;
    kind = header !== undefined ? "header" : "key";
    cursor = { line: found.line, col: found.column - 1 + found.length };
  }
  if (span === undefined || point === "key" || kind === "element") return span;
  if (kind === "header") {
    if (point === "value") return span;
    const line = lines[span.line] as string;
    const open = line.indexOf("[");
    return { line: span.line, column: open + 1, length: line.lastIndexOf("]") + 1 - open };
  }
  const start = valueStart(lines, span);
  return start === undefined ? span : spanOf(lines, start, valueEnd(lines, start));
}

/** The message as the `toml` crate writes it: the line and column, the line itself, and a caret. */
function snippet(text: string, span: Span, message: string): string {
  const source = (text.split("\n")[span.line] ?? "").replace(/\r$/, "");
  const n = String(span.line + 1);
  const pad = " ".repeat(n.length);
  const caret = `${" ".repeat(span.column - 1)}${"^".repeat(span.length)}`;
  return `TOML parse error at line ${n}, column ${span.column}\n${pad} |\n${n} | ${source}\n${pad} | ${caret}\n${message}\n`;
}
