// The `nv` format, mwl's, read as `crates/looper-core/src/import/nv.rs` reads it: `data/chain.json`, per
// goal a record `data/goals/<slug>.json`, a handoff `data/goals/<slug>.handoff.json` and its prose
// `docs/agent/goals/<slug>.md`, and the playbook as one `data/playbook/<section>.json` per section with
// one `data/playbook/<section>/<bullet>.json` per bullet. The mapping is `docs/upgrade-guide.md`
// § *Mapping notes per project*.
//
// The chain, the record, the handoff and the prose are the `sand` format's with more in them, so they
// map as `import-sand.ts` says, with these differences:
//
// - **The prose** opens with front matter and a `#` heading, which are dropped. The text between the
//   heading and the first section is the target: it goes under `## Why` as `### The target`, and the
//   summary is its first sentence.
// - **A side goal**, a record under `data/goals/side/` with its handoff beside it and its prose under
//   `docs/agent/goals/side/`, maps as a chain goal does and stays off the chain.
// - **A goal with `handover`** is the one that moves mwl onto looper. It is not imported, and the chain
//   leaves out its slug.
// - **A goal's `milestone`, `sweep`, `files`, `env` and `waitFor`** are mwl's own, and go into its
//   `extra` under those names. So do its context selectors but `modules`, such as `rules` or `adrs`,
//   under `context`. A stage's selectors go into the stage's `extra` the same way. Every map inside them
//   has its keys sorted, as looper writes them.
// - **An `nvs-suite` check** runs `bun nv suite <trees> --case <case>…`, with its cases as `tests`, when
//   the project has `tools/nv/cmd/suite.ts`. Without it, the check runs `cargo run --quiet --bin nvs --
//   <args>` and wants `passed, 0 failed` on stdout, its cases named in its `name`, and a note says so.
// - **A check with `file`** of kind `min-bytes`, `contains`, `exact` or `ordered` becomes a file check,
//   its `minBytes` as `min_bytes` and its `want` as the matching `output_*` matcher.
// - **Every check runs for up to 1800 seconds,** as under mwl's driver, unless `[checks] timeout` allows
//   that already.
// - **A check's driver options** (`minPassing`, `needs`, `setup`, `memoize`, `overlap`, and `file` on
//   another kind) are dropped, and a note names them.
// - **The floor** is empty: mwl kept none.
// - **The playbook** is one bullet per record, the sections in their `order` and the bullets in file
//   name order: `- **<lead>** <body>`, then the record's files when the text names none of them, then
//   its `until` as the trailer, a `rule` as `test`. A `gone` or `exists` of a text in a file keeps its
//   `path:text` argument.
// - **With `[project] playbook_dir`,** each bullet is the file `<playbook_dir>/<section>/<name>.md`,
//   where `<name>` is the record's file name and `<section>` the kebab-case form of its section's title.
//   The playbook file gets each section's `##` heading it lacks. Without it, the bullets go into the
//   playbook file, and a note says how to set it.
// - **An `until` no trailer can say,** of another kind or with a backtick in it, is left off its bullet,
//   and a note counts them.
//
// Known gap: without `tools/nv/cmd/suite.ts`, an `nvs-suite` check judges the summary line alone, so a
// named case that is missing or skipped no longer fails it.

import { existsSync, readdirSync } from "node:fs";
import { join } from "node:path";
import type { Config } from "../goal/config.ts";
import { type Check, emptyCheck, FORMAT, type Goal, Refused, type Mapping } from "../goal/model.ts";
import { kebab, parseUntil } from "../goal/playbook.ts";
import { bool, isMap, list, type Reader, str, strings, table, u32, u64, wrongType } from "../goal/reader.ts";
import type { Import } from "./import.ts";
import { type CheckJson as SandCheck, checkOf, type GoalRecord, goalOf, handoffJson, opt, readJson, readText } from "./import-sand.ts";

/** Any JSON value, kept as it is. */
const anyValue: Reader<unknown> = { expects: "any value", read: (v) => v };

/** A context: `modules`, and every other selector, such as `rules` or `adrs`, which are mwl's own. */
interface ContextJson {
  modules: string[] | undefined;
  selectors: Record<string, unknown>;
}

const contextJson: Reader<ContextJson> = {
  expects: "struct ContextJson",
  read(v, at) {
    if (!isMap(v)) return wrongType(v, "struct ContextJson", at);
    const { modules, ...selectors } = v;
    return { modules: modules === undefined ? undefined : opt(strings).read(modules, [...at, "modules"]), selectors };
  },
};

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

/** An nv check, its keys as the JSON spells them. */
interface CheckJson {
  id: string;
  kind: string;
  stage: number;
  name?: string;
  argv?: string[];
  args?: string[];
  cwd?: string;
  tests?: string[];
  cases?: string[];
  file?: string;
  stream?: string;
  want?: string[];
  exit?: string;
  stdoutContains?: string[];
  stderrContains?: string[];
  minBytes?: number;
  minPassing?: number;
  needs?: string;
  setup?: boolean;
  memoize?: boolean;
  overlap?: boolean;
}

const checkJson = table<CheckJson>(
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
    cases: opt(strings),
    file: opt(str),
    stream: opt(str),
    want: opt(strings),
    exit: opt(str),
    stdoutContains: opt(strings),
    stderrContains: opt(strings),
    minBytes: opt(u64),
    minPassing: opt(u64),
    needs: opt(str),
    setup: opt(bool),
    memoize: opt(bool),
    overlap: opt(bool),
  },
  () => ({ id: "", kind: "", stage: 0 }),
  ["id", "kind", "stage"],
);

interface GoalJson {
  title: string;
  milestone?: unknown;
  position?: string;
  sweep?: unknown;
  files?: unknown;
  context: ContextJson;
  stages: StageJson[];
  checks: CheckJson[];
  env?: unknown;
  waitFor?: unknown;
  handover?: unknown;
}

const goalJson = table<GoalJson>(
  "GoalJson",
  {
    title: str,
    milestone: opt(anyValue),
    position: opt(str),
    sweep: opt(anyValue),
    files: opt(anyValue),
    context: contextJson,
    stages: list(stageJson),
    checks: list(checkJson),
    env: opt(anyValue),
    waitFor: opt(anyValue),
    handover: opt(anyValue),
  },
  () => ({ title: "", context: { modules: undefined, selectors: {} }, stages: [], checks: [] }),
  ["title", "stages", "checks"],
);

const sectionJson = table<{ title: string; order: number }>(
  "SectionJson",
  { title: str, order: { expects: "i64", read: (v, at) => (Number.isInteger(v) ? (v as number) : wrongType(v, "i64", at)) } },
  () => ({ title: "", order: 0 }),
  ["title", "order"],
);

interface BulletJson {
  lead: string;
  body: string;
  files: string[];
  until: { kind: string; arg: string };
}

const untilJson = table<{ kind: string; arg: string }>("UntilJson", { kind: str, arg: str }, () => ({ kind: "", arg: "" }), [
  "kind",
  "arg",
]);

const bulletJson = table<BulletJson>(
  "BulletJson",
  { lead: str, body: str, files: strings, until: untilJson },
  () => ({ lead: "", body: "", files: [], until: { kind: "", arg: "" } }),
  ["lead", "body", "files", "until"],
);

const chainJson = table<{ live: string; goals: string[] }>(
  "ChainJson",
  { live: str, goals: strings },
  () => ({ live: "", goals: [] }),
  ["live", "goals"],
);

/** How long mwl's driver let a check run, in seconds. */
const MWL_TIMEOUT = 1800;

/** What a check's mapping takes from the project rather than from the check. */
interface Project {
  /** The timeout each check is written with, or `undefined` when `[checks] timeout` covers mwl's already. */
  timeout: number | undefined;
  /** The project has `tools/nv/cmd/suite.ts`, which judges an `nvs-suite` check case by case. */
  suiteTool: boolean;
}

/** The names in `dir` that end in `.json`, sorted, or none when it cannot be read. */
function jsonNames(dir: string): string[] {
  try {
    return readdirSync(dir)
      .filter((n) => n.endsWith(".json"))
      .sort();
  } catch {
    return [];
  }
}

/** Reads the nv project at `root`, configured by `cfg`. */
export function readNv(root: string, cfg: Config): Import {
  const chain = readJson(root, "data/chain.json", chainJson);
  if (chain === undefined) throw new Refused("data/chain.json is missing, so this is not an nv project");
  const notes: string[] = [];
  const project: Project = {
    timeout: cfg.checks.timeout < MWL_TIMEOUT ? MWL_TIMEOUT : undefined,
    suiteTool: existsSync(join(root, "tools/nv/cmd/suite.ts")),
  };

  const order = chain.goals.includes(chain.live) ? [chain.live] : [];
  order.push(...chain.goals.filter((s) => s !== chain.live));

  const goals: Goal[] = [];
  const chainOrder: string[] = [];
  const last: string[] = [];
  for (const slug of order) {
    const path = `data/goals/${slug}.json`;
    const g = readJson(root, path, goalJson);
    if (g === undefined) throw new Refused(`the chain names ${slug}, and ${path} is missing`);
    if (g.handover !== undefined) continue;
    (g.position === "last" ? last : chainOrder).push(slug);
    goals.push(goal(root, "", slug, g, project, notes));
  }
  chainOrder.push(...last);

  const side = jsonNames(join(root, "data/goals/side"))
    .map((n) => n.slice(0, -".json".length))
    .filter((s) => !s.endsWith(".handoff"));
  for (const slug of side) {
    const g = readJson(root, `data/goals/side/${slug}.json`, goalJson);
    if (g === undefined) throw new Refused(`data/goals/side/${slug}.json vanished during the import`);
    if (g.handover === undefined) goals.push(goal(root, "side/", slug, g, project, notes));
  }

  const [playbookText, bulletFiles] = playbook(root, cfg, notes);
  return {
    chain: { looper: FORMAT, chain: chainOrder, floor: [] },
    goals,
    playbook: playbookText,
    bullet_files: bulletFiles,
    notes,
    generated: [],
  };
}

/** The goal `slug` whose files lie under `side`, which is `""` for a chain goal and `side/` for a side goal. */
function goal(root: string, side: string, slug: string, g: GoalJson, project: Project, notes: string[]): Goal {
  const handoff = readJson(root, `data/goals/${side}${slug}.handoff.json`, handoffJson);
  const prose = readText(root, `docs/agent/goals/${side}${slug}.md`) ?? "";
  return goalOf(slug, record(slug, g, project, notes), handoff, proseInSandShape(prose), notes);
}

/** The record in the shape `goalOf` takes, with mwl's own fields and selectors as `extra`. */
function record(slug: string, g: GoalJson, project: Project, notes: string[]): GoalRecord {
  const owner = `goal ${slug}`;
  const fields: [string, unknown][] = [
    ["milestone", g.milestone],
    ["sweep", g.sweep],
    ["files", g.files],
    ["env", g.env],
    ["waitFor", g.waitFor],
  ];
  return {
    title: g.title,
    context: g.context.modules ?? [],
    extra: extra(fields, g.context.selectors),
    stages: g.stages.map((s) => ({
      number: s.number,
      title: s.title,
      summary: s.summary,
      context: s.context?.modules,
      extra: extra([], s.context?.selectors ?? {}),
    })),
    checks: g.checks.flatMap((c) => {
      const out = check(c, project, owner, notes);
      return out === undefined ? [] : [{ ...out, timeout: out.timeout ?? project.timeout }];
    }),
  };
}

/** The value with every map's keys sorted, as serde_json's map holds them. */
function sorted(v: unknown): unknown {
  if (Array.isArray(v)) return v.map(sorted);
  if (!isMap(v)) return v;
  const out: Record<string, unknown> = {};
  for (const k of Object.keys(v).sort()) out[k] = sorted(v[k]);
  return out;
}

/**
 * mwl's own `fields` that are set, then `selectors` under `context`, all under mwl's names, or
 * `undefined` when there are none.
 */
function extra(fields: [string, unknown][], selectors: Record<string, unknown>): Mapping | undefined {
  const context = Object.keys(selectors).length > 0 ? selectors : undefined;
  const out: Mapping = {};
  for (const [name, value] of [...fields, ["context", context] as [string, unknown]]) {
    if (value !== undefined) out[name] = sorted(value);
  }
  return Object.keys(out).length > 0 ? out : undefined;
}

/** One nv check as a looper check, or `undefined` for one looper replaces. */
function check(c: CheckJson, project: Project, owner: string, notes: string[]): Check | undefined {
  const fileKind = ["min-bytes", "contains", "exact", "ordered"].includes(c.kind);
  const options: [string, boolean][] = [
    ["minPassing", c.minPassing !== undefined],
    ["needs", c.needs !== undefined],
    ["setup", c.setup !== undefined],
    ["memoize", c.memoize !== undefined],
    ["overlap", c.overlap !== undefined],
    ["file", c.file !== undefined && !fileKind],
  ];
  const dropped = options.filter(([, set]) => set).map(([name]) => `\`${name}\``);
  if (dropped.length > 0) {
    notes.push(`${owner}: check ${c.id} dropped ${dropped.join(", ")}, which looper has no place for`);
  }
  const base: Check = { ...emptyCheck(), id: c.id, stage: c.stage, name: c.name, cwd: c.cwd };
  if (c.file !== undefined && fileKind) {
    return {
      ...base,
      file: c.file,
      min_bytes: c.kind === "min-bytes" ? c.minBytes : undefined,
      output_contains: c.kind === "contains" ? c.want : undefined,
      output_exact: c.kind === "exact" ? c.want?.join("\n") : undefined,
      output_ordered: c.kind === "ordered" ? c.want : undefined,
    };
  }
  if (c.kind === "nvs-suite" && project.suiteTool) {
    const args = c.args ?? [];
    let skip = 0;
    while (args[skip] === "test") skip++;
    const cases = c.cases ?? [];
    return {
      ...base,
      run: ["bun", "nv", "suite", ...args.slice(skip), ...cases.flatMap((k) => ["--case", k])],
      tests: cases.length > 0 ? cases : undefined,
    };
  }
  if (c.kind === "nvs-suite") {
    const cases = (c.cases ?? []).map((k) => `\`${k}\``);
    let name = c.name;
    if (cases.length > 0) name = name === undefined ? `its cases: ${cases.join(", ")}` : `${name}; its cases: ${cases.join(", ")}`;
    notes.push(
      `${owner}: check ${c.id} lost its cases' conditions: without \`tools/nv/cmd/suite.ts\` it runs its suite ` +
        "through `cargo run --bin nvs` and judges its summary line alone",
    );
    return {
      ...base,
      name,
      run: ["cargo", "run", "--quiet", "--bin", "nvs", "--", ...(c.args ?? [])],
      stdout_contains: ["passed, 0 failed"],
    };
  }
  const sand: SandCheck = {
    id: c.id,
    kind: c.kind,
    stage: c.stage,
    name: c.name,
    argv: c.argv,
    args: c.args,
    cwd: c.cwd,
    tests: c.tests,
    stream: c.stream,
    want: c.want,
    exit: c.exit,
    stdoutContains: c.stdoutContains,
    stderrContains: c.stderrContains,
    minBytes: c.minBytes,
  };
  return checkOf(sand, owner, notes);
}

/**
 * The prose with its front matter and `#` heading dropped, and the text before its first section under
 * `## The target`, which is where the `sand` format keeps it.
 */
function proseInSandShape(text: string): string {
  let body = text;
  if (text.startsWith("---\n")) {
    const rest = text.slice(4);
    const end = rest.indexOf("\n---\n");
    if (end >= 0) body = rest.slice(end + 5);
  }
  let lead = "";
  let rest = "";
  let inSections = false;
  for (const line of body.match(/[^\n]*\n|[^\n]+$/g) ?? []) {
    if (line.startsWith("## ")) inSections = true;
    if (inSections) rest += line;
    else if (!line.startsWith("# ")) lead += line;
  }
  const target = lead.trim();
  return target === "" ? rest : `## The target\n\n${target}\n\n${rest}`;
}

/**
 * The playbook in looper's shape, or `undefined` when mwl kept none. Without `[project] playbook_dir`,
 * that is its sections and bullets. With it, it is the sections' headings, and each bullet is one of the
 * bullet files.
 */
function playbook(root: string, cfg: Config, notes: string[]): [string | undefined, [string, string][]] {
  const dir = join(root, "data/playbook");
  if (!existsSync(dir)) return [undefined, []];
  const ownDir = cfg.project.playbook_dir?.replaceAll("\\", "/").replace(/\/+$/, "");
  const sections: [number, string, string][] = [];
  for (const name of jsonNames(dir)) {
    const stem = name.slice(0, -".json".length);
    const s = readJson(root, `data/playbook/${name}`, sectionJson) ?? { title: stem, order: Number.MAX_SAFE_INTEGER };
    sections.push([s.order, stem, s.title]);
  }
  const byText = (a: string, b: string) => (a < b ? -1 : a > b ? 1 : 0);
  sections.sort((a, b) => a[0] - b[0] || byText(a[1], b[1]) || byText(a[2], b[2]));

  let text = "";
  const filesOut: [string, string][] = [];
  let bullets = 0;
  let untrailed = 0;
  for (const [, stem, title] of sections) {
    text += `## ${title}\n\n`;
    for (const f of jsonNames(join(dir, stem))) {
      const b = readJson(root, `data/playbook/${stem}/${f}`, bulletJson);
      if (b === undefined) throw new Refused(`data/playbook/${stem}/${f} vanished during the import`);
      const [line, trailed] = bullet(b);
      if (ownDir !== undefined) filesOut.push([`${ownDir}/${kebab(title)}/${f.slice(0, -".json".length)}.md`, `${line}\n`]);
      else text += `${line}\n`;
      bullets++;
      if (!trailed) untrailed++;
    }
    if (ownDir === undefined) text += "\n";
  }
  if (bullets === 0) return [undefined, []];
  const file = cfg.project.playbook;
  if (ownDir !== undefined) {
    notes.push(`the playbook: ${bullets} bullet(s) each get a file under ${ownDir}, and ${file} gets each section's heading it lacks`);
  } else {
    notes.push(
      `the playbook: ${bullets} bullet(s) go after what ${file} already holds; set \`[project] playbook_dir\` ` +
        "before the import to give each its own file, as mwl did",
    );
  }
  if (untrailed > 0) notes.push(`the playbook: ${untrailed} bullet(s) lost their \`until\`, which no looper trailer can say`);
  const out = ownDir !== undefined ? sections.map(([, , title]) => `## ${title}\n`).join("\n") : `${text.trimEnd()}\n`;
  return [out, filesOut];
}

/** One bullet on one line, and whether it kept its `until`. */
function bullet(b: BulletJson): [string, boolean] {
  let line = `- **${b.lead.trim()}** ${b.body.trim()}`;
  const named = b.files.map((f) => `\`${f}\``);
  if (named.length > 0 && !named.some((n) => line.includes(n))) line += ` Files: ${named.join(", ")}.`;
  const { kind, arg } = b.until;
  let trailer: string | undefined;
  if (kind === "gone" || kind === "exists" || kind === "test") trailer = `${kind} ${arg}`;
  else if (kind === "rule") trailer = `test ${arg}`;
  if (trailer === undefined || trailer.includes("`") || "err" in parseUntil(trailer)) return [line, false];
  return [`${line} \`until: ${trailer}\``, true];
}
