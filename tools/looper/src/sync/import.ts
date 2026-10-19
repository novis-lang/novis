// Import: a project's goals, read from the format its own loop tooling kept, written as goal files and
// the chain in looper's format, as `crates/looper-core/src/import/mod.rs` writes them
// (`docs/upgrade-guide.md` Part 1 § *Import the goals*).
//
// One module per source format reads the project's files into an `Import`, held in memory. The shared
// core here validates it and writes it, so every format ends in the same files:
//
// - one `<slug>.md` per goal, in the goals directory `looper.toml` names;
// - the chain and the floor in that directory's `README.md`, its body rendered from the goal files;
// - the playbook, when the format kept one in another shape, and its bullet files in the directory form.
//
// The import never deletes a file and never overwrites one that holds anything else. A target that
// exists with the same text is left as it is, so an import runs twice to the same tree. A target that
// exists with other text refuses the whole import before anything is written: the README when its chain
// differs from the import's, a goal file when its text does. Two exceptions:
//
// - **The playbook:** the import's bullets go after what it holds, and a playbook that holds them
//   already is left as it is. When the format writes `bullet_files`, the playbook gets only each
//   section's `##` heading it lacks, and each bullet file is written as a goal file is.
// - **A page the source tool generated** from data that stays on disk (`generated`) is overwritten. The
//   import names every such page, because one in the goals directory that it does not overwrite fails
//   lint until the project agent deletes it and runs render.
//
// Every goal is linted in memory before a file is written. A lint issue refuses the import, except a
// `TODO` check, which stands for acceptance the source wrote only as prose: the import writes it, says
// so, and lint refuses it until a person or agent writes the real command.

import { mkdirSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import type { Config } from "../goal/config.ts";
import { writeFile } from "../goal/edit.ts";
import { issueText, lintGoal } from "../goal/lint.ts";
import { type Chain, type Goal, goalPath, normalize, parseChain, parseGoal, README, Refused } from "../goal/model.ts";
import { parsePlaybook, playbookSection } from "../goal/playbook.ts";
import { chainFrontNode, frontMatter, goalText, renderReadme } from "../goal/render.ts";
import { readNv } from "./import-nv.ts";
import { readOb } from "./import-ob.ts";
import { readPhysics } from "./import-physics.ts";
import { readSand } from "./import-sand.ts";

/** The source formats, each named as `import --from` takes it, in the order the usage lists them. */
export const KINDS = ["sand", "nv", "ob", "physics"] as const;

export type Kind = (typeof KINDS)[number];

/** A project's goals as one format read them, before anything is written. */
export interface Import {
  chain: Chain;
  /** Every goal, on the chain or a side goal. */
  goals: Goal[];
  /**
   * The playbook's new text, when the format kept it in another shape. `undefined` leaves it alone. With
   * `bullet_files`, it is their sections' `##` headings alone.
   */
  playbook: string | undefined;
  /** The bullets in the directory form, each as its file's path from the root and its text. */
  bullet_files: [string, string][];
  /** What the import changed or could not carry over, one line each, for the caller to print. */
  notes: string[];
  /** The pages the source tool generated from its data, as paths from the root. */
  generated: string[];
}

/** Reads the project at `root` in the `kind` format. */
export function readImport(kind: Kind, root: string, cfg: Config): Import {
  const readers: Record<Kind, () => Import> = {
    sand: () => readSand(root),
    nv: () => readNv(root, cfg),
    ob: () => readOb(root),
    physics: () => readPhysics(root),
  };
  const read = readers[kind]();
  if (read.generated.length > 0) {
    read.notes.push(
      `the old tool generated ${read.generated.join(", ")}: the import overwrites the one it writes the same path as; ` +
        "delete the rest and run `looper render` before `looper lint`, which reads every page in the goals directory as a goal",
    );
  }
  return read;
}

/** One file an import writes, as a path from the project root. */
export interface Target {
  path: string;
  /** The file already holds what the import would write. */
  unchanged: boolean;
}

/** One file to write. The README's text is `undefined`, because it is rendered once the goals are on disk. */
interface Planned {
  path: string;
  shown: string;
  text: string | undefined;
  unchanged: boolean;
}

function readOr(path: string): string | undefined {
  try {
    return normalize(readFileSync(path, "utf8"));
  } catch {
    return undefined;
  }
}

/** Validates the import against the tree and lists what it writes, refusing as `writeImport` would. */
function prepare(root: string, cfg: Config, im: Import): { planned: Planned[]; todos: string[] } {
  const dir = join(root, cfg.project.goals_dir);
  const rel = (name: string) => `${cfg.project.goals_dir.replace(/\/+$/, "")}/${name}`;
  const todos: string[] = [];
  const refusals: string[] = [];
  for (const slug of im.chain.chain) {
    if (!im.goals.some((g) => g.slug === slug)) refusals.push(`the chain names ${slug}, which the source has no goal for`);
  }
  const planned: Planned[] = [];
  for (const g of im.goals) {
    const shown = rel(`${g.slug}.md`);
    const text = goalText(g);
    // Lint the text as it will be read back, so the line numbers are the written file's.
    const written = parseGoal(g.slug, shown, text);
    for (const issue of lintGoal(written, shown)) {
      if (issue.message.endsWith("is a TODO: replace it with a real command")) todos.push(issueText(issue));
      else refusals.push(issueText(issue));
    }
    planned.push(planFile(goalPath(dir, g.slug), shown, text, im.generated.includes(shown), refusals));
  }
  if (im.playbook !== undefined) {
    const path = join(root, cfg.project.playbook);
    const shown = cfg.project.playbook;
    planned.push(im.bullet_files.length === 0 ? planPlaybook(path, shown, im.playbook) : planHeadings(path, shown, im.playbook));
  }
  for (const [shown, text] of im.bullet_files) planned.push(planFile(join(root, shown), shown, text, false, refusals));

  const readme = join(dir, README);
  const shown = rel(README);
  const goalsUnchanged = planned.every((p) => p.unchanged);
  const old = readOr(readme);
  let unchanged = false;
  if (old !== undefined && !im.generated.includes(shown)) {
    let chain: Chain | undefined;
    try {
      chain = parseChain(shown, old);
    } catch {
      chain = undefined;
    }
    if (chain !== undefined && sameChain(chain, im.chain)) {
      // With every goal file already on disk as written, the README renders from what is there.
      unchanged = goalsUnchanged && old === renderReadme(im.chain, dir);
    } else {
      refusals.push(`${shown} exists and holds another chain; the import never overwrites it`);
    }
  }
  planned.push({ path: readme, shown, text: undefined, unchanged });
  if (refusals.length > 0) throw new Refused(`nothing was imported:\n  ${refusals.join("\n  ")}`);
  return { planned, todos };
}

/** Whether two chains hold the same goals and floor, compared as their canonical front matter. */
function sameChain(a: Chain, b: Chain): boolean {
  return frontMatter(chainFrontNode(a)) === frontMatter(chainFrontNode(b));
}

/** One file to write. A `generated` page that holds other text is overwritten rather than refused. */
function planFile(path: string, shown: string, text: string, generated: boolean, refusals: string[]): Planned {
  const old = readOr(path);
  const unchanged = old === text;
  if (old !== undefined && !unchanged && !generated) {
    refusals.push(`${shown} exists and holds other text; the import never overwrites it`);
  }
  return { path, shown, text, unchanged };
}

/** The playbook with the import's bullets after what it holds, or unchanged when it holds them already. */
function planPlaybook(path: string, shown: string, bullets: string): Planned {
  const old = readOr(path);
  if (old === undefined) return { path, shown, text: bullets, unchanged: false };
  if (old.includes(bullets.trimEnd())) return { path, shown, text: old, unchanged: true };
  return { path, shown, text: `${old.trimEnd()}\n\n${bullets}`, unchanged: false };
}

/**
 * The playbook with each of the `##` headings in `headings` that it lacks after what it holds, or
 * unchanged when it has them all.
 */
function planHeadings(path: string, shown: string, headings: string): Planned {
  const old = readOr(path);
  const held = old ?? "";
  const have = parsePlaybook(shown, held);
  const missing = headings
    .split("\n")
    .filter((l) => l.startsWith("## ") && playbookSection(have, l.slice(3).trim()) === undefined);
  const unchanged = old !== undefined && missing.length === 0;
  const kept = held.trimEnd();
  let text: string;
  if (kept === "") text = `${missing.join("\n\n")}\n`;
  else if (missing.length === 0) text = held;
  else text = `${kept}\n\n${missing.join("\n\n")}\n`;
  return { path, shown, text, unchanged };
}

/** What an import wrote or would write, and every `TODO` check it carries as a lint line. */
export interface Written {
  targets: Target[];
  todos: string[];
}

/** What `writeImport` would write, refusing as it would. */
export function planImport(root: string, cfg: Config, im: Import): Written {
  const { planned, todos } = prepare(root, cfg, im);
  return { targets: planned.map((p) => ({ path: p.shown, unchanged: p.unchanged })), todos };
}

/**
 * Writes the import: every goal file, then the playbook, then the README rendered from the goal files.
 * Nothing is written when anything would be refused.
 */
export function writeImport(root: string, cfg: Config, im: Import): Written {
  const { planned, todos } = prepare(root, cfg, im);
  const dir = join(root, cfg.project.goals_dir);
  for (const p of planned) {
    if (p.unchanged) continue;
    mkdirSync(dirname(p.path), { recursive: true });
    writeFile(p.path, p.text ?? renderReadme(im.chain, dir));
  }
  return { targets: planned.map((p) => ({ path: p.shown, unchanged: p.unchanged })), todos };
}
