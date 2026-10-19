// Edits: every change the kit's `goal` commands and `render` make to a project's goal files and chain,
// as `crates/looper-core/src/edit.rs` makes them.
//
// An edit loads everything it changes first and refuses, writing nothing, when a file does not parse,
// when the change names something that is not there, or when the edited goal would carry a lint issue
// it did not carry before. Then it writes the goal file canonically (`goalText`) and re-renders the
// README, so the generated overview never lags an edit.
//
// Every file is written to a synced sibling `<name>.tmp` and renamed over the old one, so a crash leaves
// the old file or the new one and never half of either. An edit that writes two files writes them in
// the order that leaves a lintable tree after each step.
//
// Each function returns one line saying what it changed, for the caller to print.

import { closeSync, existsSync, fsyncSync, mkdirSync, openSync, readFileSync, renameSync, rmSync, writeSync } from "node:fs";
import { join } from "node:path";
import { isSlug, issueText, lintGoal } from "./lint.ts";
import {
  type Chain,
  type Check,
  FORMAT,
  type Goal,
  GoalError,
  goalPath,
  loadChain,
  loadGoal,
  normalize,
  parseGoal,
  README,
  Refused,
  type Status,
} from "./model.ts";
import { goalText, renderReadme } from "./render.ts";
import { sectionsOf } from "./sections.ts";

/** Writes `text` to `path` through a synced sibling temp file and a rename. */
export function writeFile(path: string, text: string): void {
  const tmp = `${path}.tmp`;
  try {
    const fd = openSync(tmp, "w");
    try {
      writeSync(fd, text);
      fsyncSync(fd);
    } finally {
      closeSync(fd);
    }
    renameSync(tmp, path);
  } catch (e) {
    rmSync(tmp, { force: true });
    throw new GoalError(path, undefined, (e as Error).message);
  }
}

/** Writes the README for `chain` when its text differs from the file's. Returns whether it wrote. */
function writeReadme(goalsDir: string, chain: Chain): boolean {
  const text = renderReadme(chain, goalsDir);
  const path = join(goalsDir, README);
  try {
    if (normalize(readFileSync(path, "utf8")) === text) return false;
  } catch {
    // A missing README is written.
  }
  writeFile(path, text);
  return true;
}

/** Re-renders the README from the chain and the goal files. Returns whether the file changed. */
export function render(goalsDir: string): boolean {
  return writeReadme(goalsDir, loadChain(goalsDir));
}

function load(goalsDir: string, slug: string): Goal {
  if (!existsSync(goalPath(goalsDir, slug))) throw new Refused(`there is no goal ${slug}`);
  return loadGoal(goalsDir, slug);
}

/** The messages of the goal's lint issues. */
function issueMessages(g: Goal): Set<string> {
  return new Set(lintGoal(g, "").map((i) => i.message));
}

/**
 * Loads a goal, applies `change`, and writes the goal and the README, refusing when the written text
 * would not parse or would carry a lint issue the goal did not carry before.
 */
function editGoal(goalsDir: string, slug: string, change: (g: Goal) => string): string {
  const chain = loadChain(goalsDir);
  const goal = load(goalsDir, slug);
  const before = issueMessages(goal);
  const said = change(goal);
  const path = goalPath(goalsDir, slug);
  const text = goalText(goal);
  const edited = parseGoal(slug, path, text);
  const added = [...issueMessages(edited)].filter((m) => !before.has(m));
  if (added.length > 0) throw new Refused(`${slug}: the edit would add lint issue(s): ${added.join("; ")}`);
  writeFile(path, text);
  writeReadme(goalsDir, chain);
  return said;
}

function stageOf(g: Goal, slug: string, id: number) {
  const stage = g.front.stages.find((s) => s.id === id);
  if (stage === undefined) throw new Refused(`${slug} has no stage ${id}`);
  return stage;
}

/** Marks a stage `done` or `open`. */
export function setStage(goalsDir: string, slug: string, id: number, status: Status): string {
  return editGoal(goalsDir, slug, (g) => {
    stageOf(g, slug, id).status = status;
    return `${slug}: stage ${id} is ${status}`;
  });
}

/** Appends a check. Lint refuses a duplicate id, an empty argv or a stage the goal lacks. */
export function addCheck(goalsDir: string, slug: string, check: Check): string {
  return editGoal(goalsDir, slug, (g) => {
    g.front.checks.push(check);
    return `${slug}: added check ${check.id}`;
  });
}

export function removeCheck(goalsDir: string, slug: string, id: string): string {
  return editGoal(goalsDir, slug, (g) => {
    const at = g.front.checks.findIndex((c) => c.id === id);
    if (at < 0) throw new Refused(`${slug} has no check ${id}`);
    g.front.checks.splice(at, 1);
    return `${slug}: removed check ${id}`;
  });
}

/** Adds a path to the goal's `context`, or to one stage's when `stage` names it. */
export function addContext(goalsDir: string, slug: string, path: string, stage: number | undefined): string {
  return editGoal(goalsDir, slug, (g) => {
    let list = g.front.context;
    let place = "the goal's context";
    if (stage !== undefined) {
      const s = stageOf(g, slug, stage);
      s.context ??= [];
      list = s.context;
      place = `stage ${stage}'s context`;
    }
    if (list.includes(path)) throw new Refused(`${slug}: ${path} is already in ${place}`);
    list.push(path);
    return `${slug}: added ${path} to ${place}`;
  });
}

/** The offset at which the 1-based `line` of `text` starts. */
function lineStart(text: string, line: number): number {
  let at = 0;
  for (let i = 1; i < line; i++) {
    const nl = text.indexOf("\n", at);
    if (nl < 0) return text.length;
    at = nl + 1;
  }
  return at;
}

/** Replaces the text of `## Handoff` with `text`, keeping the heading and every other byte of the body. */
export function setHandoff(goalsDir: string, slug: string, text: string): string {
  const handoff = normalize(text);
  return editGoal(goalsDir, slug, (g) => {
    const all = sectionsOf(g.body, 2);
    const at = all.findLastIndex((s) => s.name === "Handoff");
    if (at < 0) throw new Refused(`${slug} has no ## Handoff`);
    const head = lineStart(g.body, all[at]!.line);
    const nl = g.body.indexOf("\n", head);
    const headEnd = nl < 0 ? g.body.length : nl;
    let body = `${g.body.slice(0, headEnd)}\n\n${handoff.trim()}\n`;
    const next = all[at + 1];
    if (next !== undefined) body += `\n${g.body.slice(lineStart(g.body, next.line))}`;
    g.body = body;
    return `${slug}: wrote ## Handoff`;
  });
}

/** A goal file's version: a stable hash of its text, which changes whenever any byte of it does. */
export function goalVersion(goalsDir: string, slug: string): string {
  const path = goalPath(goalsDir, slug);
  let text: string;
  try {
    text = readFileSync(path, "utf8");
  } catch (e) {
    throw new GoalError(path, undefined, (e as Error).message);
  }
  // FNV-1a over the UTF-8 bytes, so a version means what looper's does.
  let hash = 0xcbf29ce484222325n;
  for (const b of new TextEncoder().encode(normalize(text))) hash = BigInt.asUintN(64, (hash ^ BigInt(b)) * 0x100000001b3n);
  return hash.toString(16).padStart(16, "0");
}

/** What the goal editor changes: the title, the summary, stage titles by id, and the prose. */
export interface GoalEdit {
  title: string;
  summary: string;
  /** `[id, title]` for each stage whose title changes. A stage not named keeps its title. */
  stages: [number, string][];
  body: string;
}

/**
 * Saves the editor's change to a goal, but only over the file `version` names (`goalVersion`), so a save
 * never overwrites what someone else wrote since the editor read it. It refuses, naming each issue with
 * its line, when the saved goal would carry a lint issue it did not carry before.
 */
export function saveGoal(goalsDir: string, slug: string, version: string, change: GoalEdit): string {
  if (existsSync(goalPath(goalsDir, slug)) && goalVersion(goalsDir, slug) !== version) {
    throw new Refused(`${slug} changed since the editor read it; reload it and edit again`);
  }
  const chain = loadChain(goalsDir);
  const goal = load(goalsDir, slug);
  const before = issueMessages(goal);
  goal.front.title = change.title.trim();
  goal.front.summary = change.summary.trim();
  for (const [id, title] of change.stages) stageOf(goal, slug, id).title = title.trim();
  // The body keeps the blank line after the front matter and ends in a newline, whatever the editor
  // did with either.
  goal.body = `\n${normalize(change.body).replace(/^\n+/, "").trimEnd()}\n`;
  const path = goalPath(goalsDir, slug);
  const file = `${slug}.md`;
  const text = goalText(goal);
  const saved = parseGoal(slug, file, text);
  const added = lintGoal(saved, file)
    .filter((i) => !before.has(i.message))
    .map(issueText);
  if (added.length > 0) throw new Refused(`${slug} was not saved, because lint found:\n${added.join("\n")}`);
  writeFile(path, text);
  writeReadme(goalsDir, chain);
  return `${slug}: saved`;
}

/** A new goal file with every section and one open stage, on the chain's end unless `onChain` is false. */
export function newGoal(goalsDir: string, slug: string, title: string, summary: string, stage: string, onChain: boolean): string {
  if (!isSlug(slug)) throw new Refused(`${slug} is not a kebab-case slug`);
  const path = goalPath(goalsDir, slug);
  if (existsSync(path)) throw new Refused(`${path} exists already`);
  const chain = loadChain(goalsDir);
  const goal: Goal = {
    slug,
    front: {
      looper: FORMAT,
      title,
      summary,
      context: [],
      stages: [{ id: 1, title: stage, status: "open", context: undefined, extra: undefined }],
      checks: [],
      settings: undefined,
      extra: undefined,
    },
    body: `\n# ${title}\n\n## Why\n\n## Standing decisions\n\n## Stages\n\n### 1. ${stage}\n\n## Handoff\n\n### State\n\n### Next\n\n### Backlog\n`,
    body_line: 0,
  };
  const text = goalText(goal);
  const issues = lintGoal(parseGoal(slug, path, text), "").map((i) => i.message);
  if (issues.length > 0) throw new Refused(`${slug}: the new goal would not lint: ${issues.join("; ")}`);
  try {
    mkdirSync(goalsDir, { recursive: true });
  } catch (e) {
    throw new GoalError(goalsDir, undefined, (e as Error).message);
  }
  writeFile(path, text);
  if (onChain) chain.chain.push(slug);
  writeReadme(goalsDir, chain);
  return onChain ? `created ${slug}, position ${chain.chain.length} on the chain` : `created ${slug}, a side goal`;
}

/**
 * Moves a goal to the 1-based `position` on the chain, putting a side goal on it. A position past the
 * end means the end.
 */
export function moveGoal(goalsDir: string, slug: string, position: number): string {
  if (position === 0) throw new Refused("chain positions start at 1");
  load(goalsDir, slug);
  const chain = loadChain(goalsDir);
  chain.chain = chain.chain.filter((s) => s !== slug);
  chain.chain.splice(Math.min(position - 1, chain.chain.length), 0, slug);
  writeReadme(goalsDir, chain);
  return `chain: ${chain.chain.join(" → ")}`;
}

/** Whether git holds the file committed and unchanged. Anything git cannot vouch for reads as false. */
function committed(goalsDir: string, file: string): boolean {
  try {
    const run = Bun.spawnSync(["git", "status", "--porcelain", "--ignored", "--", file], {
      cwd: goalsDir,
      stdin: "ignore",
      stderr: "ignore",
    });
    return run.exitCode === 0 && run.stdout.length === 0;
  } catch {
    return false;
  }
}

function removeFile(path: string): void {
  try {
    rmSync(path);
  } catch (e) {
    throw new GoalError(path, undefined, (e as Error).message);
  }
}

function refuseUncommitted(goalsDir: string, slug: string): void {
  const file = `${slug}.md`;
  if (!committed(goalsDir, file)) throw new Refused(`${file} is not committed and unchanged in git; commit it first`);
}

/**
 * Takes a goal off the chain and deletes its file. It refuses unless git holds the file committed and
 * unchanged, so the goal can always be brought back.
 */
export function removeGoal(goalsDir: string, slug: string): string {
  load(goalsDir, slug);
  refuseUncommitted(goalsDir, slug);
  const chain = loadChain(goalsDir);
  chain.chain = chain.chain.filter((s) => s !== slug);
  // The chain lets go of the goal before its file goes, so no step leaves the chain naming a missing file.
  writeReadme(goalsDir, chain);
  removeFile(goalPath(goalsDir, slug));
  writeReadme(goalsDir, chain);
  return `removed ${slug}; chain: ${chain.chain.join(" → ")}`;
}

/**
 * Retires a reached goal: its checks move into the floor, it leaves the chain, and its file is deleted.
 * A moved check drops its `stage`, gains `from: <slug>`, and is renamed `<slug>-<id>` when the floor
 * already holds its id. With `carry`, every check moves except one marked `floor: false`. Without it,
 * only a check marked `floor: true` moves. It refuses as `removeGoal` does.
 */
export function retireGoal(goalsDir: string, slug: string, carry: boolean): string {
  const goal = load(goalsDir, slug);
  refuseUncommitted(goalsDir, slug);
  const chain = loadChain(goalsDir);
  const ids = new Set(chain.floor.map((c) => c.id));
  for (const check of goal.front.checks.filter((c) => c.floor ?? carry)) {
    const id = ids.has(check.id) ? `${slug}-${check.id}` : check.id;
    ids.add(id);
    chain.floor.push({ ...check, id, stage: undefined, from: slug });
  }
  chain.chain = chain.chain.filter((s) => s !== slug);
  // The floor gains the checks before the file goes, so no step loses them.
  writeReadme(goalsDir, chain);
  removeFile(goalPath(goalsDir, slug));
  writeReadme(goalsDir, chain);
  return `retired ${slug}; the chain moves to ${chain.chain[0] ?? "nothing (chain complete)"}`;
}
