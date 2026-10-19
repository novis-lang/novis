// The playbook, read in both its forms, as `crates/looper-core/src/playbook.rs` reads it. wrap, orient,
// lint and retire read it only through `loadPlaybook`.
//
// - The file `[project] playbook` names is the home of the head and of the sections. Its text before
//   the first `##` heading is the head, and its `##` headings name the sections in their order.
// - A bullet is a line that opens with `- ` and every indented line after it.
// - With `[project] playbook_dir` set, a bullet may also stand in a file of its own,
//   `<playbook_dir>/<section>/<name>.md`, where `<section>` is the `kebab` form of a heading. A
//   section's bullets are the file's, then the directory's in file name order. `bulletPath` names the
//   file a new bullet goes in, so two writers never share one.
// - Loading reports, as lint issues, what breaks the directory form. That is a bullet file that does
//   not hold exactly one bullet, and an entry that is not a section's directory or a `.md` file in one.
// - A bullet's optional `` `until: <condition>` `` trailer is an `Until`, which `untilOf` reads and
//   `holds` tests against the tree. The table of conditions is in `docs/goal-format.md`.
// - `playbookLint` adds what breaks each bullet's `shape` to the directory form's problems.
//   `retirable` names the bullets whose condition holds, and `retire` deletes them. `bulletPaths` names
//   the paths a bullet cites.
//
// A `Playbook` holds every bullet of one playbook, and lives for one call. It caches nothing.

import { existsSync, readdirSync, readFileSync, statSync, unlinkSync } from "node:fs";
import { join } from "node:path";
import type { Config } from "./config.ts";
import { writeFile } from "./edit.ts";
import type { Issue } from "./lint.ts";
import { GoalError, normalize, Refused } from "./model.ts";

/** The most characters of a bullet file's name, before its `.md`. */
export const NAME_CAP = 60;

/** The most bytes a bullet may hold. */
export const MAX_BULLET = 700;

/** A path in backticks: a run without a space that holds a `/` or a `.`. */
const PATH_IN_TICKS = /`[^`\s]*[/.][^`\s]*`/g;

/** One bullet: its lines joined by `\n`, and where it stands. */
export interface Bullet {
  text: string;
  /** The file it stands in, from the project root with forward slashes. */
  file: string;
  /** The 1-based line it opens on. */
  line: number;
}

/** The condition of a bullet's `until:` trailer, after which `looper playbook retire` deletes it. */
export type Until =
  /** A test of this name guards the trap. */
  | { kind: "test"; name: string }
  /** The path exists, or with a text, the file holds it. */
  | { kind: "exists"; path: string; text: string | undefined }
  /** The path does not exist, or with a text, the file is missing or does not hold it. */
  | { kind: "gone"; path: string; text: string | undefined };

/** A parsed condition, or why it does not parse. */
export type Parsed = { ok: Until } | { err: string };

/** A `##` section of the playbook file and its bullets in order. */
export interface PlaybookSection {
  name: string;
  bullets: Bullet[];
}

/** A project's playbook, read whole. */
export interface Playbook {
  /** The playbook file's text before its first `##` heading. */
  head: string;
  sections: PlaybookSection[];
  /** What breaks the directory form. */
  problems: Issue[];
}

/** How many lines a bullet spans. */
export function bulletLines(b: Bullet): number {
  return b.text.split("\n").length;
}

/**
 * The paths a bullet names: each run in backticks that `shape` takes for a path, without its ticks,
 * a `:line` suffix or a trailing `/`.
 */
export function bulletPaths(b: Bullet): string[] {
  return [...b.text.matchAll(PATH_IN_TICKS)].map((m) => {
    const run = m[0].replace(/^`+|`+$/g, "");
    const colon = run.indexOf(":");
    return (colon < 0 ? run : run.slice(0, colon)).replace(/\/+$/, "");
  });
}

/**
 * The first `` `until: …` `` trailer of a bullet's text, parsed, or `undefined` when it has none. A
 * trailer across lines is joined with one space.
 */
export function untilOf(text: string): Parsed | undefined {
  const at = text.indexOf("`until:");
  if (at < 0) return undefined;
  const rest = text.slice(at + "`until:".length);
  const end = rest.indexOf("`");
  if (end < 0) return { err: "an until trailer has no closing backtick" };
  return parseUntil(
    rest
      .slice(0, end)
      .split("\n")
      .map((l) => l.trim())
      .join(" "),
  );
}

/**
 * What breaks the shape of a bullet's text, one message each: it opens with `- **`, holds at most
 * `MAX_BULLET` bytes, names a path in backticks, and its `until` trailer parses.
 */
export function shape(text: string): string[] {
  const out: string[] = [];
  if (!text.startsWith("- **")) out.push("it opens with `- **` and the trap in bold");
  const bytes = Buffer.byteLength(text);
  if (bytes > MAX_BULLET) out.push(`${bytes} bytes, more than ${MAX_BULLET}`);
  if (!text.match(PATH_IN_TICKS)) out.push("it names no path in backticks");
  const until = untilOf(text);
  if (until && "err" in until) out.push(until.err);
  return out;
}

/** Parses a condition, the trailer's text after `until:`. */
export function parseUntil(raw: string): Parsed {
  const condition = raw.trim();
  const space = condition.indexOf(" ");
  const kind = space < 0 ? condition : condition.slice(0, space);
  const arg = space < 0 ? "" : condition.slice(space + 1).trim();
  if (arg === "") return { err: `until \`${condition}\` names no argument` };
  const located = (kind: "exists" | "gone"): Parsed => {
    const colon = arg.indexOf(":");
    if (colon < 0) return { ok: { kind, path: arg, text: undefined } };
    const path = arg.slice(0, colon).trim();
    const text = arg.slice(colon + 1);
    if (path === "") return { err: `until \`${condition}\` names no path` };
    if (text === "") return { err: `until \`${condition}\` names no text after its \`:\`` };
    return { ok: { kind, path, text } };
  };
  if (kind === "test") return { ok: { kind, name: arg } };
  if (kind === "exists" || kind === "gone") return located(kind);
  return { err: `until \`${condition}\` is not test, exists or gone` };
}

/** Whether the condition holds in the project at `root`, whose playbook `cfg` names. */
export function holds(u: Until, root: string, cfg: Config): boolean {
  if (u.kind === "test") return guarded(root, cfg, u.name);
  const at = join(root, u.path);
  const found = u.text === undefined ? existsSync(at) : holdsText(at, u.text);
  return u.kind === "exists" ? found : !found;
}

/** Whether the file at `path` holds `text`, its line endings normalized. A missing file holds nothing. */
function holdsText(path: string, text: string): boolean {
  try {
    return normalize(readFileSync(path).toString("utf8")).includes(text);
  } catch {
    return false;
  }
}

/**
 * Whether `git grep -w -F` finds `name` in a tracked file of `root` outside the playbook file and
 * directory. A git that does not run finds nothing.
 */
function guarded(root: string, cfg: Config, name: string): boolean {
  const args = ["git", "grep", "-q", "-w", "-F", "-e", name, "--", "."];
  args.push(`:(exclude)${slashed(cfg.project.playbook)}`);
  if (cfg.project.playbook_dir !== undefined) args.push(`:(exclude)${trimSlash(slashed(cfg.project.playbook_dir))}`);
  try {
    return Bun.spawnSync(args, { cwd: root, stdin: "ignore", stdout: "ignore", stderr: "ignore" }).exitCode === 0;
  } catch {
    return false;
  }
}

/** Reads the playbook of the project at `root`, or `undefined` when its file is missing. */
export function loadPlaybook(root: string, cfg: Config): Playbook | undefined {
  const rel = slashed(cfg.project.playbook);
  let text: string;
  try {
    text = normalize(readFileSync(join(root, rel), "utf8"));
  } catch {
    return undefined;
  }
  const book = parsePlaybook(rel, text);
  if (cfg.project.playbook_dir !== undefined) readDir(book, root, trimSlash(slashed(cfg.project.playbook_dir)), rel);
  return book;
}

/** The playbook file's text, as it stands in `file`, without the directory's bullets. */
export function parsePlaybook(file: string, text: string): Playbook {
  const book: Playbook = { head: "", sections: [], problems: [] };
  const head: string[] = [];
  const body: [number, number, string][] = [];
  text.split("\n").forEach((line, i) => {
    const name = line.startsWith("## ") ? line.slice(3).trim() : "";
    if (name !== "") book.sections.push({ name, bullets: [] });
    else if (book.sections.length === 0) head.push(line);
    else body.push([book.sections.length - 1, i + 1, line]);
  });
  book.head = head.join("\n");
  book.sections.forEach((s, at) => {
    s.bullets = bulletsOf(
      file,
      body.filter((b) => b[0] === at).map((b) => [b[1], b[2]]),
    );
  });
  return book;
}

/** The section named `name`. */
export function playbookSection(book: Playbook, name: string): PlaybookSection | undefined {
  return book.sections.find((s) => s.name === name);
}

/** Every bullet, section by section. */
export function bullets(book: Playbook): Bullet[] {
  return book.sections.flatMap((s) => s.bullets);
}

/**
 * What lint reports: the directory form's problems, then what breaks each bullet's `shape`, at the line
 * it opens on.
 */
export function playbookLint(book: Playbook): Issue[] {
  const out = [...book.problems];
  for (const b of bullets(book)) {
    for (const message of shape(b.text)) out.push({ file: b.file, line: b.line, message });
  }
  return out;
}

/** The bullets whose `until` condition holds in the project at `root`, in the playbook's order. */
export function retirable(book: Playbook, root: string, cfg: Config): Bullet[] {
  return bullets(book).filter((b) => {
    const u = untilOf(b.text);
    return u !== undefined && "ok" in u && holds(u.ok, root, cfg);
  });
}

/**
 * Deletes `gone` of the project at `root`, and gives each path it changed. A bullet of the playbook file
 * loses its lines, and one blank line where two would meet. A bullet in a file of its own deletes the
 * file. It refuses, and writes nothing, when the playbook file no longer holds a bullet's text at its
 * line.
 */
export function retire(root: string, cfg: Config, gone: Bullet[]): string[] {
  const file = slashed(cfg.project.playbook);
  const inline = gone.filter((b) => b.file === file);
  const own = gone.filter((b) => b.file !== file);
  const changed: string[] = [];
  if (inline.length > 0) {
    const path = join(root, file);
    let text: string;
    try {
      text = normalize(readFileSync(path, "utf8"));
    } catch (e) {
      throw new GoalError(path, undefined, (e as Error).message);
    }
    const lines = text.split("\n");
    const spans = inline.map((b): [number, Bullet] => [b.line - 1, b]).sort((x, y) => x[0] - y[0]);
    for (const [start, b] of spans.reverse()) {
      const end = start + bulletLines(b);
      if (end > lines.length || lines.slice(start, end).join("\n") !== b.text) {
        throw new Refused(`${file}:${b.line} no longer holds the bullet it held`);
      }
      lines.splice(start, end - start);
      if (start > 0 && lines[start] === "" && lines[start - 1] === "") lines.splice(start, 1);
    }
    writeFile(path, lines.join("\n"));
    changed.push(file);
  }
  for (const b of own) {
    const path = join(root, b.file);
    try {
      unlinkSync(path);
    } catch (e) {
      throw new GoalError(path, undefined, (e as Error).message);
    }
    changed.push(b.file);
  }
  return changed;
}

/** Appends each section's bullet files under `<root>/<dir>`, and reports what breaks its shape. */
function readDir(book: Playbook, root: string, dir: string, file: string): void {
  let entries: string[];
  try {
    entries = readdirSync(join(root, dir));
  } catch {
    return;
  }
  const problem = (path: string, message: string) => book.problems.push({ file: path, line: undefined, message });
  const found = new Set<string>();
  for (const name of entries) {
    const path = `${dir}/${name}`;
    if (!isDir(join(root, path))) problem(path, "a bullet file stands in a section's directory");
    else if (!book.sections.some((s) => kebab(s.name) === name)) {
      problem(path, `${file} has no ## section whose kebab-case form is ${name}`);
    } else found.add(name);
  }
  for (const s of book.sections) {
    const name = kebab(s.name);
    if (!found.has(name)) continue;
    let files: string[];
    try {
      files = readdirSync(join(root, dir, name)).sort();
    } catch {
      continue;
    }
    for (const f of files) {
      const rel = `${dir}/${name}/${f}`;
      let text: string;
      try {
        text = new TextDecoder("utf-8", { fatal: true }).decode(readFileSync(join(root, rel)));
      } catch (e) {
        problem(rel, e instanceof TypeError ? "stream did not contain valid UTF-8" : (e as Error).message);
        continue;
      }
      if (extension(f) !== "md") {
        problem(rel, "a bullet file's name ends in .md");
        continue;
      }
      const lines = normalize(text)
        .split("\n")
        .map((l, i): [number, string] => [i + 1, l]);
      const own = bulletsOf(rel, lines);
      if (own.length !== 1) problem(rel, `holds ${own.length} bullets, and a bullet file holds one`);
      s.bullets.push(...own);
    }
  }
}

/** The bullets among `lines`, each a numbered line of `file`. */
function bulletsOf(file: string, lines: [number, string][]): Bullet[] {
  const out: Bullet[] = [];
  let open = false;
  for (const [n, line] of lines) {
    if (line.startsWith("- ")) {
      out.push({ text: line, file, line: n });
      open = true;
    } else if (open && (line.startsWith(" ") || line.startsWith("\t")) && line.trim() !== "") {
      (out[out.length - 1] as Bullet).text += `\n${line}`;
    } else {
      open = false;
    }
  }
  return out;
}

/**
 * The kebab-case form of `text`: its ASCII letters and digits lowercased, every other run of
 * characters one `-`, and no `-` at either end.
 */
export function kebab(text: string): string {
  let out = "";
  for (const c of text) {
    if (/^[A-Za-z0-9]$/.test(c)) out += c.toLowerCase();
    else if (out !== "" && !out.endsWith("-")) out += "-";
  }
  return out.replace(/-+$/, "");
}

/**
 * The name of a bullet's file without `.md`: the kebab-case form of its bold lead, cut to `NAME_CAP`
 * characters, or `bullet` when that is empty.
 */
export function bulletName(bullet: string): string {
  let lead = bullet;
  if (bullet.startsWith("- **")) {
    const rest = bullet.slice(4);
    const end = rest.indexOf("**");
    lead = end < 0 ? rest : rest.slice(0, end);
  }
  const name = kebab(lead).slice(0, NAME_CAP).replace(/-+$/, "");
  return name === "" ? "bullet" : name;
}

/**
 * The path from the project root of a new file for `bullet` in `section` of the directory `dir`: its
 * `bulletName`, then `-2`, `-3` and so on past each name that exists under `root` or is in `taken`.
 */
export function bulletPath(root: string, dir: string, section: string, bullet: string, taken: string[]): string {
  const base = `${trimSlash(slashed(dir))}/${kebab(section)}/${bulletName(bullet)}`;
  let path = `${base}.md`;
  for (let n = 2; taken.includes(path) || existsSync(join(root, path)); n++) path = `${base}-${n}.md`;
  return path;
}

/** The extension of a file name as Rust's `Path::extension` reads it: none for `.md` or `md`. */
function extension(name: string): string | undefined {
  const dot = name.lastIndexOf(".");
  return dot > 0 ? name.slice(dot + 1) : undefined;
}

function isDir(path: string): boolean {
  try {
    return statSync(path).isDirectory();
  } catch {
    return false;
  }
}

function slashed(path: string): string {
  return path.replaceAll("\\", "/");
}

function trimSlash(path: string): string {
  return path.replace(/\/+$/, "");
}
