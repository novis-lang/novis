// The public roadmap: `website/src/data/roadmap.json`, its copy in `README.md`, and the check that keeps
// both in step with the goal chain.
//
// The file is a list of themes, each a title, one or two plain sentences and the goal slugs it
// summarizes, and `internal`, the goals that are not public work. The website's roadmap page renders the
// themes with `Roadmap.astro`, the first one labelled as the work in progress. `bun nv site --roadmap`
// writes the same list into `README.md` between `ROADMAP_START` and `ROADMAP_END`.
//
// `bun nv site --check roadmap` fails when a goal on the chain is in no theme and not internal, when a
// theme or `internal` names a goal that is not on the chain, when a goal is named twice, when a theme
// has no goals left, when the themes are not in the chain order of their first goal, and when the
// README's copy differs. So a new goal joins a theme when it joins the chain. Nobody keeps the theme
// order by hand: `bun nv site --roadmap` and `--roadmap --retired` both sort the themes by their first
// goal's place on the chain, so a retirement that empties the head of a theme, or a goal moved on the
// chain, never leaves the order red at a goal end's push. A retired goal leaves the file through
// `bun nv site --roadmap --retired`, looper's `after_retire` hook, and a theme leaves the page with its
// last goal.

import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { chainGoals } from "./chain.ts";

export const ROADMAP = "website/src/data/roadmap.json";
export const README = "README.md";
export const ROADMAP_START = "<!-- roadmap:start (bun nv site --roadmap writes this list from website/src/data/roadmap.json) -->";
export const ROADMAP_END = "<!-- roadmap:end -->";
/** The label of the first theme, the work in progress. The page's component says the same. */
export const NOW = "Working on it now";

export interface Theme {
  title: string;
  text: string;
  goals: string[];
}

export interface Roadmap {
  internal: string[];
  themes: Theme[];
}

const read = (root: string, path: string): string | null => {
  try {
    return readFileSync(join(root, path), "utf8").replace(/\r\n?/g, "\n");
  } catch {
    return null;
  }
};

/** The roadmap file, or the reason it cannot be read. */
export function readRoadmap(root: string): Roadmap | string {
  const text = read(root, ROADMAP);
  if (text === null) return `${ROADMAP}: missing`;
  let raw: unknown;
  try {
    raw = JSON.parse(text);
  } catch (e) {
    return `${ROADMAP}: not JSON: ${(e as Error).message}`;
  }
  const r = raw as Partial<Roadmap>;
  const strings = (v: unknown) => Array.isArray(v) && v.every((s) => typeof s === "string");
  if (!strings(r.internal)) return `${ROADMAP}: \`internal\` is not a list of slugs`;
  if (!Array.isArray(r.themes)) return `${ROADMAP}: \`themes\` is not a list`;
  for (const [i, t] of r.themes.entries()) {
    if (typeof t?.title !== "string" || typeof t.text !== "string" || !strings(t.goals)) {
      return `${ROADMAP}: theme ${i + 1} needs a \`title\`, a \`text\` and a list of \`goals\``;
    }
  }
  return r as Roadmap;
}

/** The one sentence the page and the README print when no theme is left. The page's component says the same. */
export const EMPTY = "Nothing is planned after the current release yet.";
/** The sentence above a list of themes. The page's component says the same. */
export const ORDER = "The work is in the order it is planned. The order can change.";

/**
 * The README's text between the markers: `ORDER` and one numbered line per theme, or `EMPTY` alone when
 * there is no theme, so the README never says "is planned" above an empty list.
 */
export function renderRoadmap(map: Roadmap): string {
  if (map.themes.length === 0) return EMPTY;
  const list = map.themes.map((t, i) => `${i + 1}. **${t.title}.**${i === 0 ? ` *${NOW}.*` : ""} ${t.text}`);
  return [ORDER, "", ...list].join("\n");
}

/** `text` with the list between the markers replaced, or null when the markers are not both there. */
export function withRoadmap(text: string, list: string): string | null {
  const from = text.indexOf(ROADMAP_START);
  const to = text.indexOf(ROADMAP_END);
  if (from < 0 || to < from) return null;
  return `${text.slice(0, from + ROADMAP_START.length)}\n${list}\n${text.slice(to)}`;
}

/**
 * `themes` sorted by the chain place of each one's first goal on the chain, stably. A theme with no goal
 * on the chain goes last, for the check to name.
 */
export function inChainOrder(themes: Theme[], order: Map<string, number>): Theme[] {
  const first = (t: Theme) => Math.min(Infinity, ...t.goals.map((s) => order.get(s) ?? Infinity));
  return themes.map((t, i) => ({ t, i, at: first(t) })).sort((a, b) => a.at - b.at || a.i - b.i).map((x) => x.t);
}

const chainOrder = (root: string) => new Map(chainGoals(root).map((g) => [g.slug, g.num]));
const sameOrder = (a: Theme[], b: Theme[]) => a.length === b.length && a.every((t, i) => t === b[i]);

/**
 * `--roadmap`: put the roadmap file's themes in chain order, then write the README's list from it.
 * Returns the problem, or null once written.
 */
export function writeRoadmap(root: string): string | null {
  let map = readRoadmap(root);
  if (typeof map === "string") return map;
  const themes = inChainOrder(map.themes, chainOrder(root));
  if (!sameOrder(themes, map.themes)) {
    map = { ...map, themes };
    const raw = JSON.parse(read(root, ROADMAP) ?? "{}") as Roadmap;
    writeFileSync(join(root, ROADMAP), `${JSON.stringify({ ...raw, themes }, null, 2)}\n`);
  }
  const readme = read(root, README);
  const next = readme === null ? null : withRoadmap(readme, renderRoadmap(map));
  if (next === null) return `${README}: no roadmap markers; put ${ROADMAP_START} and ${ROADMAP_END} where the list goes`;
  if (next !== readme) writeFileSync(join(root, README), next);
  return null;
}

/**
 * `--roadmap --retired`, looper's `after_retire` hook: drop each goal that is off the chain and has no
 * goal file, which is what a retirement leaves, and each theme left with no goal, then put the themes
 * in chain order. Then write the README's list and commit both files when either changed. The
 * retirement commits only the goals directory, and it can move another theme's first goal ahead, so
 * without this the check is red after a goal end and the push it runs is refused. A slug off the chain
 * whose file is still there is not retired, and stays for the check to name. Gives what it did, or the
 * problem.
 */
export function pruneRetired(root: string): { said: string; ok: boolean } {
  const map = readRoadmap(root);
  if (typeof map === "string") return { said: map, ok: false };
  const order = chainOrder(root);
  const retired = (slug: string) => !order.has(slug) && !existsSync(join(root, "docs/goals", `${slug}.md`));
  const gone = [...map.internal, ...map.themes.flatMap((t) => t.goals)].filter(retired);
  const kept = map.themes.map((t) => ({ ...t, goals: t.goals.filter((s) => !retired(s)) })).filter((t) => t.goals.length > 0);
  const themes = inChainOrder(kept, order);
  const moved = !sameOrder(themes, kept);
  if (gone.length === 0 && !moved) return { said: "the roadmap names no retired goal, and its themes are in chain order", ok: true };
  const raw = JSON.parse(read(root, ROADMAP) ?? "{}") as Roadmap;
  raw.internal = raw.internal.filter((s) => !retired(s));
  raw.themes = themes;
  const dropped = map.themes.length - kept.length;
  writeFileSync(join(root, ROADMAP), `${JSON.stringify(raw, null, 2)}\n`);
  const problem = writeRoadmap(root);
  if (problem !== null) return { said: problem, ok: false };
  const git = (args: string[]) => Bun.spawnSync(["git", ...args], { cwd: root, stdout: "pipe", stderr: "pipe" });
  const subject =
    gone.length === 0
      ? "chore(site): the roadmap's themes follow the chain's order"
      : `chore(site): the roadmap drops ${gone.join(", ")}, retired from the chain${moved ? ", and its themes follow the chain's order" : ""}`;
  const commit = git(["commit", "-q", "-m", subject, "--only", "--", ROADMAP, README]);
  if (commit.exitCode !== 0) return { said: `the commit failed: ${commit.stderr.toString().trim()}`, ok: false };
  const did = gone.length === 0 ? "put the themes in chain order" : `dropped ${gone.join(", ")} and ${dropped} theme(s)${moved ? ", put the themes in chain order" : ""}`;
  return { said: `${did}, and committed`, ok: true };
}

/** Every way the roadmap is out of step with the chain or the README, one line each. */
export function roadmapProblems(root: string): string[] {
  const map = readRoadmap(root);
  if (typeof map === "string") return [map];
  const problems: string[] = [];
  const order = chainOrder(root);
  const seen = new Map<string, string>();
  const named = (slug: string, where: string) => {
    const before = seen.get(slug);
    if (before !== undefined) problems.push(`${ROADMAP}: ${slug} is in ${before} and in ${where}; a goal is in one place`);
    else seen.set(slug, where);
    if (!order.has(slug)) problems.push(`${ROADMAP}: ${where} names ${slug}, which is not on the chain`);
  };
  for (const slug of map.internal) named(slug, "`internal`");
  let last = 0;
  let lastTitle = "";
  for (const t of map.themes) {
    for (const slug of t.goals) named(slug, `"${t.title}"`);
    const firsts = t.goals.map((s) => order.get(s)).filter((n): n is number => n !== undefined);
    if (firsts.length === 0) {
      problems.push(`${ROADMAP}: "${t.title}" has no goal left on the chain; remove the theme`);
      continue;
    }
    const first = Math.min(...firsts);
    if (first < last) problems.push(`${ROADMAP}: "${t.title}" comes after "${lastTitle}", but its first goal is earlier on the chain; run \`bun nv site --roadmap\``);
    else {
      last = first;
      lastTitle = t.title;
    }
  }
  for (const slug of order.keys()) {
    if (!seen.has(slug)) problems.push(`${ROADMAP}: ${slug} is on the chain and in no theme; add it to a theme, or to \`internal\``);
  }
  const readme = read(root, README);
  const want = readme === null ? null : withRoadmap(readme, renderRoadmap(map));
  if (want === null) problems.push(`${README}: no roadmap markers; put ${ROADMAP_START} and ${ROADMAP_END} where the list goes`);
  else if (want !== readme) problems.push(`${README}: the roadmap differs from ${ROADMAP}; run \`bun nv site --roadmap\``);
  return problems;
}
