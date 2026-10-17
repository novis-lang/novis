// Where the chain stands: its order, the live goal, and the plan a command checks a goal against.
//
// All of it is looper's goal files under `docs/goals/`. The chain is the `chain:` list in the front matter
// of `docs/goals/README.md`, and its first slug is the live goal. A goal is `docs/goals/<slug>.md`, and
// its front matter is everything a tool reads about it. A side goal is a goal file whose slug is not on
// the chain. A process belongs to a side run when `SIDE_ENV` names a side goal whose file is on disk.
//
// **A goal's plan is its own front matter and nothing else** (`rule:tooling/the-chain-names-its-live-goal`).
// No check of another goal is carried into it: the permanent suites and `bun nv verify` protect finished
// work. `planOf` turns looper's fields back into the shape the commands here read: Novis's own fields
// (`files`, `env`, the `[context]` lists) sit under `extra`, and a check's kind follows from
// its fields.

import { existsSync, readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import type { Check } from "../accept/accept.ts";
import { ROOT } from "./paths.ts";

const GOALS = "docs/goals";

/** Set to a side goal's slug in every process of a side run: its hooks, its checks and its sessions. */
export const SIDE_ENV = "NOVIS_SIDE_GOAL";

/** The goal a looper hook names for the command it runs, which a command's `--goal` overrides. */
export const LOOPER_GOAL_ENV = "LOOPER_GOAL";

/** The stage a looper hook names for the command it runs, which a command's `--stage` overrides. */
export const LOOPER_STAGE_ENV = "LOOPER_STAGE";

/** A slug as a goal file names it. */
const SLUG = /^[a-z0-9]+(?:-[a-z0-9]+)*$/;

/** What `orient` and `pack` print for a goal, each list a set of selectors of its own kind. */
export interface GoalContext {
  modules?: string[];
  rules?: string[];
  shapes?: string[];
  playbook?: string[];
}

/** A goal's plan: its front matter, in the shape the commands here read. */
export interface Goal {
  title: string;
  /** `last` pins the goal behind every goal not pinned, whatever the chain's order. */
  position?: "last";
  /** `done` sweeps the acceptance list only after a session that reports DONE; unset sweeps after every session. */
  sweep?: "done";
  /** The fixtures its checks run. */
  files: string[];
  context: GoalContext;
  stages: { number: number; title: string; summary?: string; status?: string; context?: GoalContext }[];
  checks: Check[];
  /** What a goal's checks need from the machine beyond a build. */
  env: {
    docker?: { compose: string; services: string[]; memoizeOn?: string[] };
    valgrind?: { skip: string[] };
    wsl?: { targetDir: string };
  };
}

/** A goal file's front matter, which sits above its H1. */
const FRONT = /^---\n([\s\S]*?)\n---\n/;

export interface ChainGoal {
  slug: string;
  /** 1-based, its place on the chain. */
  num: number;
  /** The goal's file, repo-relative, or null when none is on disk. */
  md: string | null;
}

/** A goal file's H1: its first line once the front matter and the blank lines after it are gone. */
export function h1Of(text: string): string {
  return text.replace(/\r\n/g, "\n").replace(FRONT, "").replace(/^\n+/, "").split("\n", 1)[0]!.trim();
}

/** The front matter of the file at `rel`, parsed, or null when the file or its front matter is absent. */
function frontMatter(rel: string, root: string): Record<string, unknown> | null {
  const path = join(root, rel);
  if (!existsSync(path)) return null;
  const m = FRONT.exec(readFileSync(path, "utf8").replace(/\r\n/g, "\n"));
  if (m === null) return null;
  const got = Bun.YAML.parse(m[1]!);
  return got !== null && typeof got === "object" ? (got as Record<string, unknown>) : null;
}

/** The chain's slugs in order: the `chain:` list of `docs/goals/README.md`, or none when it is absent. */
function chainSlugs(root: string): string[] {
  const list = frontMatter(`${GOALS}/README.md`, root)?.chain;
  return Array.isArray(list) ? list.filter((s): s is string => typeof s === "string") : [];
}

/** slug -> its goal file, `<slug>.md` in the goals directory, for every goal on the chain or off it. */
export function goalPaths(root: string = ROOT): Map<string, string> {
  const out = new Map<string, string>();
  const dir = join(root, GOALS);
  if (!existsSync(dir)) return out;
  for (const name of readdirSync(dir)) {
    const m = /^([a-z0-9]+(?:-[a-z0-9]+)*)\.md$/.exec(name);
    if (m) out.set(m[1]!, `${GOALS}/${name}`);
  }
  return out;
}

type Raw = Record<string, unknown>;
const obj = (v: unknown): Raw => (v !== null && typeof v === "object" && !Array.isArray(v) ? (v as Raw) : {});
const strs = (v: unknown): string[] | undefined => (Array.isArray(v) ? v.map(String) : undefined);

/** A `[context]` block from `extra.context`, with looper's own `context` file list as its `modules`. */
function contextOf(files: unknown, extra: unknown): GoalContext {
  const ctx: GoalContext = { ...(obj(extra) as GoalContext) };
  const modules = strs(files);
  if (modules !== undefined && modules.length > 0) ctx.modules = [...modules, ...(ctx.modules ?? [])];
  return ctx;
}

/**
 * A looper check as a `Check`. A check over a `file` is judged on that file's text; one that names
 * `tests` is a cargo run that must print each name, or a `bun nv suite` run whose named cases must pass;
 * the rest are commands, judged on their exit and on `output_ordered` or `output_contains` in order.
 */
function checkOf(c: Raw): Check {
  const run = strs(c.run) ?? [];
  const tests = strs(c.tests);
  const base: Check = { id: String(c.id), kind: "command", stage: Number(c.stage) };
  if (typeof c.name === "string") base.name = c.name;
  if (typeof c.cwd === "string") base.cwd = c.cwd;
  if (c.exit === "nonzero") base.exit = "nonzero";
  const want = strs(c.output_ordered) ?? strs(c.output_contains);
  if (typeof c.file === "string") {
    base.file = c.file;
    if (typeof c.min_bytes === "number") return { ...base, kind: "min-bytes", minBytes: c.min_bytes };
    return { ...base, kind: "contains", stdoutContains: want ?? [] };
  }
  if (tests !== undefined && run[0] === "cargo") return { ...base, kind: "cargo-named", args: run.slice(1), tests };
  if (tests !== undefined && run.slice(0, 3).join(" ") === "bun nv suite") {
    return { ...base, kind: "nvs-suite", args: ["test", run[3] ?? "tests/"], cases: tests };
  }
  base.argv = run;
  if (want !== undefined) base.want = want;
  const stdout = strs(c.stdout_contains);
  if (stdout !== undefined) base.stdoutContains = stdout;
  return base;
}

/** Goal `slug`'s plan, from its front matter. Null when the goal has no file. */
function planOf(slug: string, root: string): Goal | null {
  const fm = frontMatter(`${GOALS}/${slug}.md`, root);
  if (fm === null) return null;
  const extra = obj(fm.extra);
  const stages = (Array.isArray(fm.stages) ? fm.stages : []).map((raw) => {
    const st = obj(raw);
    const se = obj(st.extra);
    const ctx = contextOf(st.context, se.context);
    return {
      number: Number(st.id),
      title: String(st.title ?? ""),
      ...(typeof se.summary === "string" ? { summary: se.summary } : {}),
      ...(typeof st.status === "string" ? { status: st.status } : {}),
      ...(Object.keys(ctx).length > 0 ? { context: ctx } : {}),
    };
  });
  return {
    title: String(fm.title ?? ""),
    ...(extra.position === "last" ? { position: "last" as const } : {}),
    ...(extra.sweep === "done" ? { sweep: "done" as const } : {}),
    files: strs(extra.files) ?? [],
    context: contextOf(fm.context, extra.context),
    stages,
    checks: (Array.isArray(fm.checks) ? fm.checks : []).map((c) => checkOf(obj(c))),
    env: obj(extra.env) as Goal["env"],
  };
}

/** Every goal on the chain, in chain order. */
export function chainGoals(root: string = ROOT): ChainGoal[] {
  const files = goalPaths(root);
  return chainSlugs(root).map((slug, i) => ({
    slug,
    num: i + 1,
    md: files.get(slug) ?? null,
  }));
}

/** The live goal, the chain's first, or null when the chain is empty. */
export function liveGoal(goals?: ChainGoal[], root: string = ROOT): ChainGoal | null {
  goals ??= chainGoals(root);
  return goals[0] ?? null;
}

/** The plan for goal `slug`, a chain goal's or a side goal's. Null when the goal has no file. */
export function goalPlan(slug: string, root: string = ROOT): Goal | null {
  return SLUG.test(slug) ? planOf(slug, root) : null;
}

/** Goal `slug`'s live stage: the first stage its front matter does not mark `done`. Null when none is open. */
export function liveStage(slug: string, root: string = ROOT): number | null {
  return goalPlan(slug, root)?.stages.find((s) => s.status !== "done")?.number ?? null;
}

/** The flags that narrow a `bun nv proofs` run to part of the roster. */
const ROSTER_SCOPES = ["--group", "--only", "--id"];

/** What a `bun nv proofs` check passes after the command's name, or null for any other check. */
function proofsArgs(c: Check): string[] | null {
  const argv = c.argv ?? [];
  return argv[0] === "bun" && argv[1] === "nv" && argv[2] === "proofs" ? argv.slice(3) : null;
}

/**
 * Every goal of `ahead`, the goals the run has still to reach in chain order, whose whole-roster gate
 * cannot pass where it stands, each with the goals behind it that are why. `bun nv proofs --gate` with no
 * scope is red while any roster group owes a proof, and a goal whose check names a group with `--group` is
 * that group's proofs still to write, so the gate is reached only behind it.
 */
export function gatesInFrontOfGroups(ahead: { slug: string; goal: Goal }[]): { slug: string; behind: string[] }[] {
  const has = (goal: Goal, test: (args: string[]) => boolean) =>
    goal.checks.some((c) => {
      const args = proofsArgs(c);
      return args !== null && test(args);
    });
  const out: { slug: string; behind: string[] }[] = [];
  ahead.forEach(({ slug, goal }, i) => {
    if (!has(goal, (a) => a.includes("--gate") && !ROSTER_SCOPES.some((s) => a.includes(s)))) return;
    const behind = ahead.slice(i + 1).filter((h) => has(h.goal, (a) => a.includes("--group"))).map((h) => h.slug);
    if (behind.length > 0) out.push({ slug, behind });
  });
  return out;
}

/** The side goal this process runs, or null in a chain run: `SIDE_ENV`'s slug, when its goal file is off the chain. */
export function sideGoal(root: string = ROOT): string | null {
  const slug = (process.env[SIDE_ENV] ?? "").trim();
  if (!SLUG.test(slug) || !existsSync(join(root, GOALS, `${slug}.md`))) return null;
  return chainSlugs(root).includes(slug) ? null : slug;
}

/**
 * The plan a side run is checked against: side goal `slug`'s. Its WSL leg builds into a target of its
 * own, `<targetDir>-side-<slug>`, because two runs sharing one would wait on each other's lock and
 * rebuild each other's crates. Null when the goal has no file.
 */
export function sidePlan(slug: string, root: string = ROOT): Goal | null {
  const plan = goalPlan(slug, root);
  if (plan === null) return null;
  const wsl = plan.env.wsl;
  return wsl?.targetDir ? { ...plan, env: { ...plan.env, wsl: { ...wsl, targetDir: `${wsl.targetDir}-side-${slug}` } } } : plan;
}

/**
 * The plan a command run by hand or by a hook works against: goal `slug`, its `--goal`, else the goal
 * `LOOPER_GOAL_ENV` names, else `currentPlan`'s. A side goal's plan is `sidePlan`'s. Null when the goal
 * named has no file, or none is named and no goal is current.
 */
export function namedPlan(slug: string | undefined, root: string = ROOT): { slug: string; goal: Goal } | null {
  const named = slug ?? ((process.env[LOOPER_GOAL_ENV] ?? "").trim() || undefined);
  if (named === undefined) return currentPlan(root);
  const goal = chainSlugs(root).includes(named) ? goalPlan(named, root) : sidePlan(named, root);
  return goal === null ? null : { slug: named, goal };
}

/**
 * The plan this process works against: the side goal `SIDE_ENV` names, with `sidePlan`'s plan, or else the
 * live goal, with `goalPlan`'s. Null when neither has a file, or no goal is live.
 */
export function currentPlan(root: string = ROOT): { slug: string; goal: Goal } | null {
  const side = sideGoal(root);
  if (side !== null) {
    const goal = sidePlan(side, root);
    return goal === null ? null : { slug: side, goal };
  }
  const live = chainSlugs(root)[0];
  const goal = live === undefined ? null : goalPlan(live, root);
  return goal === null || live === undefined ? null : { slug: live, goal };
}
