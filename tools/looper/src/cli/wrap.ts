// `wrap <file> [--status <path>]`: applies a session's tail all or nothing, as
// `crates/looper/src/cmd/wrap.rs` does. `wrap --template` prints the file's shape for this project, and
// takes no `--json`. It runs on the project in the current directory.
//
// The file is Markdown in `##` sections, any of them left out, in any order:
//
// | Section | What wrap does with it |
// |---|---|
// | `## playbook: <section>` | appends its bullets to that `##` section of the playbook, or with `[project] playbook_dir` writes each as a new file, as `bulletPath` names it |
// | `## handoff` | overwrites the live goal's `## Handoff` with it, as `goal handoff set` does |
// | `## commit: <path> <path> …` | stages those paths and commits only them, with the section as the message |
// | `## status` | writes its one line to the status file, `--status` and `.loop/status.txt` by default, last |
// | `## <name>` or `## <name>: <arg>` | hands the section to the command a `[[wrap.section]]` of `looper.toml` names |
//
// Commits are made in the file's order. Text before the first section is ignored. Its `--json`
// document is `{"lines", "changed", "commits", "status"}`: `lines` is its text, `changed` each file wrap
// wrote whose bytes differ after it, each commit `{"sha", "subject", "paths"}`, and `status` the line.
//
// # A project's sections and checks
//
// A project section's command is called twice from the project root, with the section's text on stdin,
// the goal's variables, `LOOPER_WRAP_MODE` and `LOOPER_WRAP_ARG`, the text after the colon:
//
// - In `check` mode it changes nothing and prints each path it will write, one per line, and exits 0.
// - In `apply` mode it writes those paths and exits 0.
//
// The checks run before anything is written. The applies run in the file's order, after the handoff
// and the playbook and before the first commit. Wrap compares `git status` before and after each apply,
// and a path the apply changed that its check did not name fails the wrap and is put back.
//
// Then each `[wrap] checks` command runs from the project root. Any exit other than 0 fails the wrap,
// and wrap reports the tail of its output. Every project command runs as a `check` job within
// `[checks] timeout`.
//
// Wrap checks everything it can before it writes anything, and reports every problem it finds:
//
// - a `[[wrap.section]]` without a command, or named like a built-in section;
// - an unknown section, or a second `## handoff` or `## status`;
// - a project section whose check exits other than 0, or names a path outside the tree;
// - a playbook section the playbook lacks, or a bullet whose `shape` is broken;
// - a commit with no paths, a path that neither exists nor is tracked, or a subject the commit-msg hook
//   refuses, as `subjectProblems` judges it under `[commits]`;
// - a commit that holds a file another live session claims (`conflict` of `../claims/claims.ts`). The
//   session running wrap is `CLAUDE_CODE_SESSION_ID`. The goal file, the README, the playbook and each
//   path wrap writes are never claimed against it;
// - a goal file, playbook, bullet file or project section's path that wrap writes and no commit's
//   paths cover;
// - a status that is not one line of `CONTINUE`, `DONE` or `BLOCKED` and its text.
//
// A commit message loses every attribution trailer before it is used, the lines the commit-msg hook
// refuses, and wrap names each line it dropped. A commit whose paths hold no change is skipped, and
// wrap names it.
//
// Once every check passes and before it writes anything, wrap waits for the tree's verify run when one
// is going, and says `waiting for verify job <id>` (`awaitGoing`). It does not join the run. While it
// waits it tells the hang watch so (`../jobs/hang.ts`).
//
// When a step fails after writing began (a handoff that would not parse, a failed apply or wrap check,
// a commit a hook refuses), wrap moves `HEAD` back with `git reset --soft`. The commits it made are
// undone and their changes stay in the tree and the index. Wrap puts back the goal file, the README,
// the playbook, each new bullet file and every path a project section's check named, as they were. The
// status file is written only when everything else landed.
//
// # Known gaps
//
// - A commit path cannot hold a space.
// - An apply's stray change to a path that was already changed cannot be put back, and wrap says so.

import { existsSync, mkdirSync, readFileSync, renameSync, rmSync, writeFileSync } from "node:fs";
import { dirname, isAbsolute, join, relative, resolve } from "node:path";
import { conflict, dir as claimsDir, machine } from "../claims/claims.ts";
import { type Commits, type Config, loadConfig, type WrapSection } from "../goal/config.ts";
import { setHandoff, writeFile } from "../goal/edit.ts";
import { emptyCheck, goalPath, liveGoal, loadChain, normalize, README, Refused } from "../goal/model.ts";
import { bulletPath, loadPlaybook, parsePlaybook, playbookSection, shape } from "../goal/playbook.ts";
import { sectionsOf } from "../goal/sections.ts";
import { subjectProblems, TRAILER_KEYS } from "../gates/commit-msg.ts";
import { envOf, note, runCommand } from "../verify/accept.ts";
import { waitOnPurpose } from "../jobs/hang.ts";
import { awaitGoing } from "../verify/shared.ts";
import { type Command, type Context, type Report, usage } from "./cli.ts";
import { goalVars } from "./orient.ts";

/** The status file `--status` names by default, from the project root. */
export const STATUS_FILE = ".loop/status.txt";

const STATUS = /^(CONTINUE|DONE|BLOCKED) \S/;
/** A trailer the commit-msg hook refuses, by its key. */
const TRAILER = new RegExp(`^[ \\t]*(${TRAILER_KEYS.join("|")})[ \\t]*:`, "i");
/** The prose form of a trailer, `Generated with [Tool](url)`, refused when no ASCII letter or backtick comes before it on its line. */
const PROSE_TRAILER = /(generated|created|made)[ \t]+(with|by)[ \t]+\[/i;

/** The sections wrap itself knows, which a project's section may not be named like. */
const BUILT_IN = ["playbook", "handoff", "commit", "status"];

/** The lines of a failed project command's output a problem quotes. */
const TAIL_LINES = 20;

/** One project section of a wrap file. */
interface Section {
  name: string;
  arg: string;
  text: string;
  run: string[];
  /** The paths its check said it will write, cleaned. */
  paths: string[];
}

interface WrapCommit {
  paths: string[];
  message: string;
  /** The trailer lines taken out of the message. */
  stripped: string[];
}

/** A wrap file, parsed. */
interface Wrap {
  playbook: [string, string[]][];
  handoff: string | undefined;
  /** The project's sections, in the file's order. */
  sections: Section[];
  commits: WrapCommit[];
  status: string | undefined;
}

/** A commit wrap made: its short hash, its subject and the paths it holds. */
interface Made {
  sha: string;
  subject: string;
  paths: string[];
}

/** What a wrap that landed prints with `--json`. */
interface Wrapped {
  lines: string[];
  changed: string[];
  commits: Made[];
  status: string | null;
}

/** Where a wrap writes, and what its project commands run with. */
interface Target {
  root: string;
  cfg: Config;
  goalsDir: string;
  playbookPath: string;
  status: string;
  /** The goal's variables, which every project command gets. */
  vars: Record<string, string | undefined>;
}

/** A file wrap may write, and its bytes before, or null when it did not exist. */
type Kept = [string, Buffer | null][];

/** The shape of a wrap file for the project at `root`. */
export function template(root: string, cfg: Config): string {
  const dir = join(root, cfg.project.goals_dir);
  let live: string | undefined;
  try {
    live = liveGoal(root, dir, loadChain(dir));
  } catch {
    live = undefined;
  }
  const goal = `${cfg.project.goals_dir}/${live ?? "<slug>"}.md`;
  const playbook = cfg.project.playbook_dir ?? cfg.project.playbook;
  const sections = (loadPlaybook(root, cfg)?.sections ?? []).map((s) => s.name);
  const own = cfg.wrap.section.map((s) => `\n\n## ${s.name}: <arg>\n\n<the text \`${s.run.join(" ")}\` checks, then applies>`).join("");
  return `<!-- \`looper wrap <this file>\` applies it all or nothing. Delete every section you do not need. -->

## playbook: <one of: ${sections.join(", ")}>

- **The trap, as one claim a reader can match against their symptom.** Why it happens, in one or two
  sentences. What to do instead, naming \`path/to/the/file\`.

## handoff

### State

### Next

### Backlog

## commit: <path> <path>

type(scope): what is now true

## commit: ${goal} ${playbook}

docs(goals): hand off <the next group>

## status

CONTINUE <what landed>${own}`;
}

/** Applies the wrap file at `file` to the project at `root`, writing its status line to `status`, both from `root`. */
export async function wrap(root: string, file: string, status: string, ctx: Context): Promise<Report> {
  const cfg = loadConfig(root);
  let raw: string;
  try {
    raw = readFileSync(resolve(root, file), "utf8");
  } catch (e) {
    throw new Refused(`${file}: ${(e as Error).message}`);
  }
  const text = normalize(raw);
  const problems = configProblems(cfg);
  const w = parse(text, cfg.wrap.section, cfg.commits, problems);
  const goalsDir = join(root, cfg.project.goals_dir);
  const t: Target = {
    root,
    cfg,
    goalsDir,
    playbookPath: join(root, cfg.project.playbook),
    status: resolve(root, status),
    vars: goalVars(root, goalsDir),
  };
  await checkSections(t, w.sections, problems);
  const live = w.handoff === undefined ? undefined : liveGoal(root, goalsDir, loadChain(goalsDir));
  const [playbookNew, bulletFiles] = playbookWrites(root, cfg, w.playbook, problems);
  const written: string[] = [];
  if (w.handoff !== undefined) {
    if (live !== undefined) written.push(`${cfg.project.goals_dir}/${live}.md`);
    else problems.push("the chain has no live goal to write the handoff into");
  }
  if (w.playbook.length > 0 && cfg.project.playbook_dir === undefined) written.push(cfg.project.playbook);
  written.push(...bulletFiles.map((f) => f[0]));
  written.push(...w.sections.flatMap((s) => s.paths));
  for (const p of written.map(clean)) {
    if (!w.commits.some((c) => c.paths.some((q) => covers(q, p)))) problems.push(`wrap writes ${p}, and no ## commit names it`);
  }
  claimed(root, cfg, w.commits, written, problems);
  const made = [...w.sections.flatMap((s) => s.paths), ...bulletFiles.map((f) => f[0])];
  for (const c of w.commits) {
    for (const p of c.paths) {
      if (made.some((m) => covers(p, m)) || existsSync(join(root, p)) || tracked(root, p)) continue;
      problems.push(`commit ${JSON.stringify(subject(c.message))}: ${p} neither exists nor is tracked`);
    }
  }
  let head = "";
  try {
    head = git(root, ["rev-parse", "--verify", "HEAD"]).trim();
  } catch (e) {
    problems.push(`the repository has no commit to wrap onto: ${(e as Error).message}`);
  }
  if (problems.length > 0) throw new Refused(`wrap refused, nothing written:\n  ${problems.join("\n  ")}`);

  const waiting = waitOnPurpose("wrap waits for the tree's verify run");
  await awaitGoing(root, ctx.line).finally(waiting);
  const bullets: [string, string][] = bulletFiles.map(([p, b]) => [join(root, p), b]);
  const files = [join(goalsDir, README), ...(live === undefined ? [] : [goalPath(goalsDir, live)]), t.playbookPath, ...bullets.map((b) => b[0])];
  const kept = snapshot(root, files, w.sections);
  let wrapped: Wrapped;
  try {
    wrapped = await apply(t, w, live, playbookNew, bullets);
  } catch (e) {
    const why = (e as Error).message;
    const failed = undo(root, head, kept);
    if (failed.length === 0) throw new Refused(`wrap failed and was undone, so nothing landed: ${why}`);
    throw new Refused(`wrap failed: ${why}\nand undoing it failed too:\n  ${failed.join("\n  ")}`);
  }
  wrapped.changed = kept.filter(([path, was]) => !same(read(path), was)).map(([path]) => relative(root, path).replaceAll("\\", "/"));
  return { doc: wrapped, text: wrapped.lines.join("\n") };
}

/** Adds a problem for each of `commits` that holds a file another live session claims. The files wrap writes itself, `own` and the goals README, are never claimed against it. */
function claimed(root: string, cfg: Config, commits: WrapCommit[], own: string[], problems: string[]): void {
  const dir = claimsDir(root);
  if (!existsSync(dir)) return;
  const tree = resolve(root).replaceAll("\\", "/").replace(/\/+$/, "");
  const session = process.env.CLAUDE_CODE_SESSION_ID ?? "";
  const key = (p: string) => (process.platform === "win32" ? p.toLowerCase() : p);
  const mine = [...own, `${cfg.project.goals_dir}/${README}`].map((p) => key(clean(p)));
  for (const c of commits) {
    const picks = (p: string) => !mine.includes(key(p)) && c.paths.some((q) => covers(key(q), key(p)));
    try {
      const held = conflict(dir, session, tree, picks, machine, Date.now());
      if (held !== null) problems.push(`commit ${JSON.stringify(subject(c.message))}: ${held}. Commit only this session's files.`);
    } catch (e) {
      problems.push(`cannot read the claims in ${dir}: ${(e as Error).message}`);
    }
  }
}

/**
 * What the playbook sections write, adding a problem for a missing playbook or section. In the file
 * form, it is the playbook's new text. With `[project] playbook_dir`, it is each new bullet's file from
 * the root and its text, and the playbook file is left alone.
 */
function playbookWrites(root: string, cfg: Config, sections: [string, string[]][], problems: string[]): [string | undefined, [string, string][]] {
  const playbook = cfg.project.playbook;
  let text: string | undefined;
  try {
    text = normalize(readFileSync(join(root, playbook), "utf8"));
  } catch {
    text = undefined;
  }
  const files: [string, string][] = [];
  for (const [section, bullets] of sections) {
    if (text === undefined) {
      problems.push(`there is no playbook at ${playbook}`);
      continue;
    }
    const dir = cfg.project.playbook_dir;
    if (dir === undefined) {
      const added = addBullets(text, section, bullets);
      if (added === undefined) problems.push(`${playbook} has no section ## ${section}`);
      else text = added;
      continue;
    }
    if (playbookSection(parsePlaybook(playbook, text), section) === undefined) problems.push(`${playbook} has no section ## ${section}`);
    for (const b of bullets) {
      files.push([bulletPath(root, dir, section, b, files.map((f) => f[0])), `${b}\n`]);
    }
  }
  return [cfg.project.playbook_dir === undefined ? text : undefined, files];
}

/** What is wrong with the project's `[wrap]` in `looper.toml`. */
function configProblems(cfg: Config): string[] {
  const problems: string[] = [];
  for (const s of cfg.wrap.section) {
    if (s.run.length === 0 || s.name === "" || /[: ]/.test(s.name) || BUILT_IN.includes(s.name)) {
      problems.push(`looper.toml [[wrap.section]] ${JSON.stringify(s.name)}: a section has a run and a one-word name that is not ${BUILT_IN.join(", ")}`);
    }
  }
  if (cfg.wrap.checks.some((c) => c.length === 0)) problems.push("looper.toml [wrap] checks: each check is an argv, and one is empty");
  return problems;
}

/** Calls each project section's command in check mode and keeps the paths it names, adding a problem for each check that fails and each path outside the tree. */
async function checkSections(t: Target, sections: Section[], problems: string[]): Promise<void> {
  for (const s of sections) {
    try {
      const out = await call(t, s.run, mode("check", s.arg), s.text);
      s.paths = out.split("\n").map((l) => l.trim()).filter((l) => l !== "").map(clean);
    } catch (e) {
      problems.push(`## ${s.name}: ${(e as Error).message}`);
    }
    for (const p of s.paths) {
      if (isAbsolute(p) || p.split("/").includes("..")) problems.push(`## ${s.name}: its check names ${p}, which is outside the tree`);
    }
  }
}

/** The bytes of each of `files` and of each path a section's check named, or null for one that does not exist, as an undo puts them back. */
function snapshot(root: string, files: string[], sections: Section[]): Kept {
  for (const p of sections.flatMap((s) => s.paths)) {
    const path = join(root, p);
    if (!files.includes(path)) files.push(path);
  }
  return files.map((p) => [p, read(p)]);
}

function read(path: string): Buffer | null {
  try {
    return readFileSync(path);
  } catch {
    return null;
  }
}

function same(a: Buffer | null, b: Buffer | null): boolean {
  return a === null || b === null ? a === b : a.equals(b);
}

/** Moves `HEAD` back to `head` and puts each kept file back as it was. Gives what could not be undone. */
function undo(root: string, head: string, kept: Kept): string[] {
  const failed: string[] = [];
  try {
    if (git(root, ["rev-parse", "HEAD"]).trim() !== head) git(root, ["reset", "-q", "--soft", head]);
  } catch (e) {
    failed.push(`git reset --soft ${head} failed: ${(e as Error).message}`);
  }
  for (const [path, bytes] of kept) {
    try {
      if (bytes === null) rmSync(path, { force: true });
      else writeBytes(path, bytes);
    } catch (e) {
      failed.push(`${path} could not be put back: ${(e as Error).message}`);
    }
  }
  return failed;
}

/** Writes `bytes` to `path` through a sibling temp file and a rename. */
function writeBytes(path: string, bytes: Buffer): void {
  const tmp = `${path}.tmp`;
  writeFileSync(tmp, bytes);
  renameSync(tmp, path);
}

/**
 * Writes the handoff, the playbook file or each new bullet file, applies the project's sections, runs
 * the wrap checks, makes the commits and writes the status. It gives one line per step, each commit and
 * the status, and leaves `changed` to the caller. The first step that fails throws.
 */
async function apply(t: Target, w: Wrap, live: string | undefined, playbookNew: string | undefined, bullets: [string, string][]): Promise<Wrapped> {
  const root = t.root;
  const said: string[] = [];
  const commits: Made[] = [];
  if (w.handoff !== undefined && live !== undefined) said.push(setHandoff(t.goalsDir, live, w.handoff));
  if (playbookNew !== undefined && w.playbook.length > 0) {
    writeFile(t.playbookPath, playbookNew);
    for (const [section, b] of w.playbook) said.push(`added ${b.length} bullet(s) to the playbook's ## ${section}`);
  }
  for (const [path, text] of bullets) {
    mkdirSync(dirname(path), { recursive: true });
    writeFile(path, text);
    said.push(`wrote the playbook bullet ${relative(root, path).replaceAll("\\", "/")}`);
  }
  for (const s of w.sections) {
    const before = changes(root);
    let failed: Error | undefined;
    try {
      await call(t, s.run, mode("apply", s.arg), s.text);
    } catch (e) {
      failed = e as Error;
    }
    const stray = [...changes(root)].filter(([p, st]) => before.get(p) !== st && !s.paths.includes(p)).map(([p]) => p);
    if (stray.length > 0) {
      let why = `## ${s.name}: its apply wrote ${stray.join(", ")}, which its check did not name`;
      for (const p of stray) {
        const left = putBack(root, p, before.has(p));
        if (left !== null) why += `\n  ${p} could not be put back: ${left}`;
      }
      throw new Error(why);
    }
    if (failed !== undefined) throw new Error(`## ${s.name}: ${failed.message}`);
    said.push(`applied ## ${s.name}, writing ${s.paths.join(", ")}`);
  }
  for (const argv of t.cfg.wrap.checks) {
    try {
      await call(t, argv, {}, undefined);
    } catch (e) {
      throw new Error(`the wrap check ${(e as Error).message}`);
    }
    said.push(`the wrap check \`${argv.join(" ")}\` passed`);
  }
  for (const c of w.commits) {
    const subj = subject(c.message);
    git(root, ["add", "-A", "--", ...c.paths]);
    if (quiet(root, c.paths)) {
      said.push(`skipped commit ${JSON.stringify(subj)}: nothing changed in ${c.paths.join(" ")}`);
      continue;
    }
    try {
      git(root, ["commit", "-q", "-F", "-", "--", ...c.paths], c.message);
    } catch (e) {
      throw new Error(`commit ${JSON.stringify(subj)}: ${(e as Error).message}`);
    }
    const sha = git(root, ["rev-parse", "--short", "HEAD"]).trim();
    said.push(`committed ${sha} ${subj}`);
    for (const line of c.stripped) said.push(`  dropped the trailer ${JSON.stringify(line)}`);
    commits.push({ sha, subject: subj, paths: c.paths });
  }
  if (w.status !== undefined) {
    mkdirSync(dirname(t.status), { recursive: true });
    writeFile(t.status, `${w.status}\n`);
    said.push(`wrote ${JSON.stringify(w.status)} to ${t.status}`);
  }
  return { lines: said, changed: [], commits, status: w.status ?? null };
}

/** Whether the index holds no change to `paths` against `HEAD`. */
function quiet(root: string, paths: string[]): boolean {
  try {
    git(root, ["diff", "--cached", "--quiet", "--", ...paths]);
    return true;
  } catch {
    return false;
  }
}

/** The variables a project section's command gets on top of the goal's. */
function mode(mode: string, arg: string): Record<string, string> {
  return { LOOPER_WRAP_MODE: mode, LOOPER_WRAP_ARG: arg };
}

/** Runs `argv` from the root as a `check` job with the goal's variables and `own` over them, feeding it `stdin`. Gives its stdout, or throws why it failed with the tail of its output. */
async function call(t: Target, argv: string[], own: Record<string, string>, stdin: string | undefined): Promise<string> {
  const named = argv.join(" ");
  const checks = t.cfg.checks;
  const env = envOf({ checks, vars: { ...t.vars, ...own } }, emptyCheck());
  const ran = await runCommand({ root: t.root, argv, cwd: t.root, env, secs: checks.timeout, kind: "check", hide: checks.hide_windows, onWait: null, stdin });
  if (ran.ended === "timed out") throw new Error(`\`${named}\` timed out after ${checks.timeout}s`);
  if (ran.code === 0) return ran.stdout;
  const code = ran.code === null ? "without a code" : String(ran.code);
  const lines = `${ran.stdout.trimEnd()}\n${ran.stderr.trimEnd()}`.split("\n").filter((l) => l.trim() !== "");
  const tail = lines.slice(-TAIL_LINES).join("\n    ");
  throw new Error(`\`${named}\` exited ${code}${note(ran)}, want 0:\n    ${tail}`);
}

/** The paths `git status` shows as changed or untracked, each with its two status letters. */
function changes(root: string): Map<string, string> {
  const out = git(root, ["status", "--porcelain=v1", "-z", "--untracked-files=all"]);
  const map = new Map<string, string>();
  const entries = out.split("\0");
  for (let i = 0; i < entries.length; i++) {
    const e = entries[i]!;
    if (e.length < 4) continue;
    const code = e.slice(0, 2);
    if (/^[RC]/.test(code)) i++;
    map.set(e.slice(3), code);
  }
  return map;
}

/** Puts back a path an apply changed and its check did not name, or gives why it cannot. Only a path that showed no change before the apply can be put back. */
function putBack(root: string, path: string, wasChanged: boolean): string | null {
  if (wasChanged) return "it was already changed before the apply";
  try {
    if (tracked(root, path)) git(root, ["checkout", "-q", "--", path]);
    else rmSync(join(root, path), { force: true });
    return null;
  } catch (e) {
    return (e as Error).message;
  }
}

/** Parses the sections of a wrap file, adding a problem for each malformed one. A section named by one of `project` is the project's, and a commit's subject is judged by `rules`. */
function parse(text: string, project: WrapSection[], rules: Commits, problems: string[]): Wrap {
  const w: Wrap = { playbook: [], handoff: undefined, sections: [], commits: [], status: undefined };
  for (const s of sectionsOf(text, 2)) {
    const colon = s.name.indexOf(":");
    const [kind, arg] = colon < 0 ? [s.name, ""] : [s.name.slice(0, colon).trim(), s.name.slice(colon + 1).trim()];
    if (kind === "playbook" && arg !== "") {
      w.playbook.push([arg, bulletsOf(s.text, problems)]);
    } else if (kind === "handoff" && arg === "" && w.handoff === undefined) {
      w.handoff = s.text;
    } else if (kind === "commit") {
      w.commits.push(commitOf(arg, s.text, rules, problems));
    } else if (kind === "status" && arg === "" && w.status === undefined) {
      if (!STATUS.test(s.text) || s.text.includes("\n")) {
        problems.push(`## status must be one line, CONTINUE, DONE or BLOCKED and its text, not ${JSON.stringify(s.text)}`);
      }
      w.status = s.text;
    } else {
      const p = BUILT_IN.includes(kind) ? undefined : project.find((p) => p.name === kind);
      if (p !== undefined) {
        w.sections.push({ name: kind, arg, text: s.text, run: p.run, paths: [] });
      } else {
        const own = project.map((p) => `, and ## ${p.name}`).join("");
        problems.push(`## ${s.name}: the sections are ## playbook: <section>, ## handoff, ## commit: <paths> and ## status, the handoff and the status once each${own}`);
      }
    }
  }
  return w;
}

/** The bullets of a playbook section, each its lines joined, adding a problem for each malformed one. */
function bulletsOf(text: string, problems: string[]): string[] {
  const out: string[] = [];
  for (const line of text.split("\n").filter((l) => l.trim() !== "")) {
    if (out.length > 0 && !line.startsWith("- ")) out[out.length - 1] += `\n${line}`;
    else out.push(line);
  }
  for (const b of out) {
    const head = Array.from(b).slice(0, 60).join("");
    problems.push(...shape(b).map((p) => `playbook bullet ${JSON.stringify(head)}: ${p}`));
  }
  if (out.length === 0) problems.push("a ## playbook section holds no bullet");
  return out;
}

/** A commit section, its trailers stripped, adding a problem for each thing wrong with it. */
function commitOf(paths: string, text: string, rules: Commits, problems: string[]): WrapCommit {
  const kept: string[] = [];
  const stripped: string[] = [];
  for (const line of text.split("\n")) (isTrailer(line) ? stripped : kept).push(line);
  const message = `${kept.join("\n").trimEnd()}\n`;
  const named = paths.split(/\s+/).filter((p) => p !== "").map(clean);
  const subj = subject(message);
  const quoted = JSON.stringify(subj);
  if (named.length === 0) problems.push(`commit ${quoted}: ## commit names no path`);
  problems.push(...subjectProblems(subj, rules).map((p) => `commit ${quoted}: ${p}`));
  const second = message.split("\n")[1];
  if (second !== undefined && second.trim() !== "") {
    problems.push(`commit ${quoted}: a blank line goes between the subject and the body`);
  }
  return { paths: named, message, stripped };
}

/** Whether the commit-msg hook would refuse `line` as an attribution trailer. */
function isTrailer(line: string): boolean {
  if (TRAILER.test(line)) return true;
  const m = PROSE_TRAILER.exec(line);
  return m !== null && !/[A-Za-z`]/.test(line.slice(0, m.index));
}

function subject(message: string): string {
  return message.split("\n")[0] ?? "";
}

/** Whether git tracks `path` in the tree at `root`. */
function tracked(root: string, path: string): boolean {
  try {
    git(root, ["ls-files", "--error-unmatch", "--", path]);
    return true;
  } catch {
    return false;
  }
}

/** A path as git and the coverage test compare it: forward slashes, no leading `./`, no trailing `/`. */
function clean(path: string): string {
  let p = path.replaceAll("\\", "/");
  if (p.startsWith("./")) p = p.slice(2);
  p = p.replace(/\/+$/, "");
  return p === "" ? "." : p;
}

/** Whether a commit of `path` takes in `file`. */
function covers(path: string, file: string): boolean {
  return path === "." || path === file || (file.startsWith(path) && file[path.length] === "/");
}

/** `text` with `bullets` appended to its `## section`, or undefined when it has no such section. */
function addBullets(text: string, section: string, bullets: string[]): string | undefined {
  const lines = text.split("\n");
  if (lines.at(-1) === "") lines.pop();
  const head = lines.findIndex((l) => l.startsWith("## ") && l.slice(3).trim() === section);
  if (head < 0) return undefined;
  const after = lines.slice(head + 1).findIndex((l) => l.startsWith("## "));
  let end = after < 0 ? lines.length : head + 1 + after;
  while (end > head + 1 && lines[end - 1]!.trim() === "") end--;
  const out = [...lines.slice(0, end), ...bullets.flatMap((b) => b.split("\n")), ...lines.slice(end)];
  return `${out.join("\n")}\n`;
}

/** Runs git in `root`, feeding it `stdin` when given. Gives its stdout, or throws what it said. */
function git(root: string, args: string[], stdin?: string): string {
  let out: { exitCode: number; stdout: Buffer; stderr: Buffer };
  try {
    out = Bun.spawnSync(["git", ...args], { cwd: root, stdin: stdin === undefined ? "ignore" : Buffer.from(stdin), stdout: "pipe", stderr: "pipe" });
  } catch (e) {
    throw new Error(`git cannot start: ${(e as Error).message}`);
  }
  if (out.exitCode !== 0) {
    const said = out.stderr.toString().trim();
    throw new Error(said === "" ? `git ${args[0]} exited ${out.exitCode}` : said);
  }
  return out.stdout.toString();
}

export const wrapCommand: Command = {
  name: "wrap",
  about: "Applies a session's tail all or nothing: wrap <file> [--status <path>], or wrap --template.",
  json: true,
  run: async (args, ctx) => {
    let file: string | undefined;
    let status = STATUS_FILE;
    let shape = false;
    for (let i = 0; i < args.length; i++) {
      const a = args[i]!;
      if (a === "--template") shape = true;
      else if (a === "--status") {
        const v = args[++i];
        if (v === undefined) throw usage("`--status` needs a path");
        status = v;
      } else if (a.startsWith("--status=")) status = a.slice("--status=".length);
      else if (a.startsWith("-")) throw usage(`\`wrap\` takes no \`${a}\``);
      else if (file === undefined) file = a;
      else throw usage(`\`wrap\` takes one file, and got \`${a}\` too`);
    }
    const root = process.cwd();
    if (shape) {
      if (file !== undefined) throw usage("`--template` cannot be used with a file");
      if (ctx.json) throw usage("`wrap --template` takes no --json");
      return { text: template(root, loadConfig(root)) };
    }
    if (file === undefined) throw usage("`wrap` needs a file, or `--template`");
    return wrap(root, file, status, ctx);
  },
};
