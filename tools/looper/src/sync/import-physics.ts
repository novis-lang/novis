// The `physics` format, physics-engine's, read as `crates/looper-core/src/import/physics.rs` reads it:
// one `docs/goals/goals.json` holding every goal in the order they run, with a page generated from each
// into `docs/goals/NNNN-<slug>.md`. The mapping is `docs/upgrade-guide.md` § *Mapping notes per project*.
//
// - **The chain** is the goals in their order, each by its own `slug`. A goal's number is dropped.
// - **A goal's front matter:** the title, the summary as the first sentence of `purpose`, `context`, and
//   the stages 1:1, a `done` stage done and a `planned` or `active` one open, each with its `context`.
// - **Acceptance** was prose only: the goal's `done_when`, `proves` and `fails_if` go under
//   `## Standing decisions`, a bullet each, and the goal gets one `TODO` check, `acceptance`, which
//   `lint` refuses until a real command replaces it.
// - **The prose:** `purpose` and `build` go under `## Why`, and so do `found` and the glance
//   (`accomplished`, `added`, `open`, `summary`), which the harvest reads. Each stage becomes
//   `### <id>. <title>` under `## Stages`: its tasks as items, ticked when the stage is done, then its
//   `done_when` and the handoff of a stage that is not live.
// - **The handoff** is the live stage's, the `active` one, or else the first open stage of an `active`
//   goal. Its own `handoff`, or a line naming it live, goes under `### State`, and its tasks are the
//   open items under `### Next`. Each item without a `path:line` anchor gets the stage's first context
//   file, or else the goal file, as one. Every other goal's handoff is empty.
// - **The generated pages** are the import's `generated`: each `docs/goals/NNNN-<slug>.md`.
//
// Known gap: a stage's `done_when` stays prose in its stage section, because the goal's one `TODO` check
// stands for all of the goal's acceptance.

import { readdirSync } from "node:fs";
import { join } from "node:path";
import { type Check, emptyCheck, FORMAT, type Goal, Refused, type Stage } from "../goal/model.ts";
import { list, str, strings, table, u32 } from "../goal/reader.ts";
import type { Import } from "./import.ts";
import { bullets, handoffText } from "./import-ob.ts";
import { firstSentence, opt, readJson } from "./import-sand.ts";

const GOALS = "docs/goals/goals.json";

interface StageJson {
  id: number;
  title: string;
  status: string;
  tasks: string[];
  done_when: string[];
  context?: string[];
  handoff?: string;
}

const stageJson = table<StageJson>(
  "StageJson",
  { id: u32, title: str, status: str, tasks: strings, done_when: strings, context: opt(strings), handoff: opt(str) },
  () => ({ id: 0, title: "", status: "", tasks: [], done_when: [] }),
  ["id", "title", "status"],
);

interface GoalJson {
  id: string;
  slug: string;
  title: string;
  status: string;
  stages: StageJson[];
  purpose?: string;
  build: string[];
  proves: string[];
  fails_if?: string;
  done_when: string[];
  accomplished?: string;
  added?: string;
  open?: string;
  summary?: string;
  found: string[];
  context: string[];
}

const goalJson = table<GoalJson>(
  "GoalJson",
  {
    id: str,
    slug: str,
    title: str,
    status: str,
    stages: list(stageJson),
    purpose: opt(str),
    build: strings,
    proves: strings,
    fails_if: opt(str),
    done_when: strings,
    accomplished: opt(str),
    added: opt(str),
    open: opt(str),
    summary: opt(str),
    found: strings,
    context: strings,
  },
  () => ({ id: "", slug: "", title: "", status: "", stages: [], build: [], proves: [], done_when: [], found: [], context: [] }),
  ["id", "slug", "title", "status", "stages"],
);

const fileJson = table<{ finished: number; goals: GoalJson[] }>(
  "FileJson",
  { finished: u32, goals: list(goalJson) },
  () => ({ finished: 0, goals: [] }),
  ["finished", "goals"],
);

/** Reads the physics project at `root`. */
export function readPhysics(root: string): Import {
  const file = readJson(root, GOALS, fileJson);
  if (file === undefined) throw new Refused(`${GOALS} is missing, so this is not a physics project`);
  const notes: string[] = [];
  const goals = file.goals.map((g) => goal(g, notes));
  return {
    chain: { looper: FORMAT, chain: goals.map((g) => g.slug), floor: [] },
    goals,
    playbook: undefined,
    bullet_files: [],
    notes,
    generated: generatedPages(root),
  };
}

/** Every `docs/goals/NNNN-<slug>.md` page, sorted. */
function generatedPages(root: string): string[] {
  let names: string[];
  try {
    names = readdirSync(join(root, "docs/goals"));
  } catch {
    return [];
  }
  return names
    .filter((name) => /^[0-9]{4}-/.test(name) && name.endsWith(".md"))
    .map((name) => `docs/goals/${name}`)
    .sort();
}

/** The `## Stages` text and the front matter's stages. The stage at `live` keeps its handoff out of it. */
function stagesOf(stages: StageJson[], live: number | undefined): [string, Stage[]] {
  let text = "";
  const out: Stage[] = [];
  stages.forEach((st, i) => {
    const done = st.status === "done";
    text += `### ${st.id}. ${st.title}\n\n`;
    for (const task of st.tasks) text += `- [${done ? "x" : " "}] ${task.trim().replaceAll("\n", "\n  ")}\n`;
    if (st.tasks.length > 0) text += "\n";
    if (st.done_when.length > 0) text += `**Done when:**\n\n${bullets(st.done_when)}\n\n`;
    if (st.handoff !== undefined && i !== live) text += `**Handoff:** ${st.handoff.trim()}\n\n`;
    out.push({ id: st.id, title: st.title, status: done ? "done" : "open", context: st.context, extra: undefined });
  });
  return [text, out];
}

/** The `## Why` text: the purpose, the build list, the findings and the glance. */
function why(g: GoalJson): string {
  let out = g.purpose === undefined ? "" : `### Purpose\n\n${g.purpose.trim()}\n\n`;
  for (const [name, items] of [
    ["Build", g.build],
    ["Found", g.found],
  ] as const) {
    if (items.length > 0) out += `### ${name}\n\n${bullets(items)}\n\n`;
  }
  for (const [name, text] of [
    ["Accomplished", g.accomplished],
    ["Added", g.added],
    ["Open", g.open],
    ["Summary", g.summary],
  ] as const) {
    if (text !== undefined) out += `### ${name}\n\n${text.trim()}\n\n`;
  }
  return out;
}

/** The handoff of the stage at `live`, or an empty one. */
function handoff(g: GoalJson, live: number | undefined, owner: string, notes: string[]): string {
  const goalFile = `docs/goals/${g.slug}.md`;
  const st = live === undefined ? undefined : g.stages[live];
  if (st === undefined) return handoffText("", undefined, [], [], goalFile, owner, notes);
  const fallback = st.context?.[0] ?? goalFile;
  const state = st.handoff ?? `Stage ${st.id} is live. A done stage's handoff is under its heading in \`## Stages\`.`;
  return handoffText(state, `**Stage ${st.id}: ${st.title}.**`, st.tasks, [], fallback, owner, notes);
}

function goal(g: GoalJson, notes: string[]): Goal {
  const owner = `goal ${g.slug}`;
  const active = g.stages.findIndex((s) => s.status === "active");
  const open = g.status === "active" ? g.stages.findIndex((s) => s.status !== "done") : -1;
  const live = active >= 0 ? active : open >= 0 ? open : undefined;
  const [stagesText, stages] = stagesOf(g.stages, live);

  const acceptance = [
    ...g.done_when.map((t) => `**Done when:** ${t.trim()}`),
    ...g.proves.map((t) => `**Proves:** ${t.trim()}`),
    ...(g.fails_if === undefined ? [] : [`**Fails if:** ${g.fails_if.trim()}`]),
  ];
  const checks: Check[] = [];
  if (acceptance.length > 0) {
    checks.push({
      ...emptyCheck(),
      id: "acceptance",
      name: "the done-when, proves and fails-if under ## Standing decisions",
      run: ["TODO"],
    });
    notes.push(`${owner}: its acceptance was prose, now under ## Standing decisions, and check acceptance is a TODO`);
  }

  const handoffBody = handoff(g, live, owner, notes);
  const body =
    `\n# ${g.title}\n\n## Why\n\n${why(g)}## Standing decisions\n\n${bullets(acceptance)}\n\n` +
    `## Stages\n\n${stagesText}## Handoff\n\n${handoffBody}`;
  const sentence = g.purpose === undefined ? "" : firstSentence(g.purpose);
  return {
    slug: g.slug,
    front: {
      looper: FORMAT,
      title: g.title,
      summary: sentence === "" ? g.title : sentence,
      context: g.context,
      stages,
      checks,
      settings: undefined,
      extra: undefined,
    },
    body: `${body.trimEnd()}\n`,
    body_line: 1,
  };
}
