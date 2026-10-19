// The typed goal model: a goal file and the chain in `docs/goals/README.md`, read as `Goal` and `Chain`
// in `crates/looper-core/src/model.rs` read them. `docs/goal-format.md` specifies every field.
//
// - `Bun.YAML` parses the front matter, and a reader per struct checks each key in the file's order.
// - A key no struct knows, a value of the wrong type and a missing required key are refused with
//   looper's message and line. `Bun.YAML` keeps no positions, so `lineOf` finds the line in the text.
// - A field that lint reports as missing, such as a title or a check's `run`, reads as empty instead.
// - A string field takes any scalar, as serde_yaml does, so `title: 5` is the title "5".
// - `render.ts` writes the model back in its canonical form.

import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { join, resolve } from "node:path";
import {
  isMap,
  list,
  mapOf,
  type Point,
  type Reader,
  Refusal,
  type Seg,
  table,
  variants,
  wrongType,
} from "./reader.ts";

/** The format version every file declares as `looper: 1`. */
export const FORMAT = 1;

/** The name of the chain's file inside the goals directory. */
export const README = "README.md";

/** The prefix of a side branch: on `side/<slug>`, the side goal `<slug>` is live. */
export const SIDE_BRANCH = "side/";

/** The most bytes of JSON one `extra` may hold, so `LOOPER_EXTRA` stays far under Windows' 32,767. */
export const EXTRA_MAX = 16 * 1024;

/**
 * A goal file or the chain that the kit cannot read, as `Error::Parse` and `Error::Io` print it. `line`
 * is the file's own 1-based line.
 */
export class GoalError extends Error {
  constructor(
    readonly path: string,
    readonly line: number | undefined,
    readonly detail: string,
  ) {
    super(`${path}${line === undefined ? "" : `:${line}`}: ${detail}`);
  }
}

/** A change the kit will not make, as `Error::Refused` prints it: the message alone. */
export class Refused extends Error {}

// ---------------------------------------------------------------------------------------------------
// The types

/** A stage is `open` until its items are finished and its checks pass. */
export type Status = "open" | "done";

/** A YAML mapping looper never reads inside, in the file's key order. */
export type Mapping = Record<string, unknown>;

export interface Stage {
  id: number;
  title: string;
  status: Status;
  /** Files added to the goal's context while this stage is live. */
  context: string[] | undefined;
  /** Project data laid over the goal's `extra` while this stage is live. */
  extra: Mapping | undefined;
}

/** The exit status a check wants: `0`, or any status but zero. */
export type Exit = "zero" | "nonzero";

/** One acceptance check, as `docs/goal-format.md` § *Checks* specifies. A floor check carries `from`. */
export interface Check {
  id: string;
  stage: number | undefined;
  name: string | undefined;
  run: string[];
  cwd: string | undefined;
  exit: Exit | undefined;
  stdout_contains: string[] | undefined;
  stderr_contains: string[] | undefined;
  stdout_exact: string | undefined;
  stdout_ordered: string[] | undefined;
  /** The `output_*` matchers judge stdout followed by stderr, or a file check's text. */
  output_contains: string[] | undefined;
  output_exact: string | undefined;
  output_ordered: string[] | undefined;
  tests: string[] | undefined;
  file: string | undefined;
  min_bytes: number | undefined;
  timeout: number | undefined;
  /** Variables laid over looper's environment and `[checks] env` for this check's process. */
  env: Record<string, string> | undefined;
  /** The cadence of an every-session sweep, at least 1: the check runs on every Nth sweep of a run. */
  every: number | undefined;
  inputs: string[] | undefined;
  reach: boolean | undefined;
  floor: boolean | undefined;
  from: string | undefined;
}

/** A goal file's front matter, in the key order of `docs/goal-format.md`. */
export interface GoalFront {
  looper: number;
  title: string;
  summary: string;
  context: string[];
  stages: Stage[];
  checks: Check[];
  /** Per-goal overrides of `looper.toml`'s `[agent]`, `[budget]` and `[loop]`, kept in their order. */
  settings: Mapping | undefined;
  /** Project data looper never reads inside. It reaches every project command as `LOOPER_EXTRA`. */
  extra: Mapping | undefined;
}

/** One goal file: its typed front matter and its prose, which is kept verbatim. */
export interface Goal {
  slug: string;
  front: GoalFront;
  /** Everything after the closing `---`, byte for byte. */
  body: string;
  /** The 1-based file line on which `body` starts. */
  body_line: number;
}

/** The front matter of `docs/goals/README.md`: the order the loop works goals in, and the floor. */
export interface Chain {
  looper: number;
  chain: string[];
  floor: Check[];
}

export function emptyCheck(): Check {
  return {
    id: "",
    stage: undefined,
    name: undefined,
    run: [],
    cwd: undefined,
    exit: undefined,
    stdout_contains: undefined,
    stderr_contains: undefined,
    stdout_exact: undefined,
    stdout_ordered: undefined,
    output_contains: undefined,
    output_exact: undefined,
    output_ordered: undefined,
    tests: undefined,
    file: undefined,
    min_bytes: undefined,
    timeout: undefined,
    env: undefined,
    every: undefined,
    inputs: undefined,
    reach: undefined,
    floor: undefined,
    from: undefined,
  };
}

export function emptyChain(): Chain {
  return { looper: FORMAT, chain: [], floor: [] };
}

// ---------------------------------------------------------------------------------------------------
// The readers, with serde_yaml's leniency and messages

/** Any scalar as a string, as serde_yaml reads one into a `String`. */
const text: Reader<string> = {
  expects: "a string",
  read(v, at) {
    if (typeof v === "string") return v;
    if (typeof v === "number" || typeof v === "boolean") return String(v);
    if (v === null) return "";
    return wrongType(v, "a string", at);
  },
};

const texts = list(text);

const flag: Reader<boolean> = {
  expects: "a boolean",
  read: (v, at) => (typeof v === "boolean" ? v : wrongType(v, "a boolean", at)),
};

/** An unsigned integer. serde_yaml calls a negative one the wrong type, not the wrong value. */
function unsigned(name: string, max: number): Reader<number> {
  return {
    expects: name,
    read(v, at) {
      if (typeof v !== "number" || !Number.isInteger(v) || v < 0) return wrongType(v, name, at);
      if (v > max) throw new Refusal(at, `invalid value: integer \`${v}\`, expected ${name}`, "value");
      return v;
    },
  };
}

const u32 = unsigned("u32", 0xffff_ffff);
const u64 = unsigned("u64", Number.MAX_SAFE_INTEGER);

/** An `Option`: a null reads as absent. */
function opt<T>(reader: Reader<T>): Reader<T | undefined> {
  return { expects: reader.expects, read: (v, at) => (v === null ? undefined : reader.read(v, at)) };
}

const mapping: Reader<Mapping> = {
  expects: "a YAML mapping",
  read: (v, at) => (isMap(v) ? v : wrongType(v, "a YAML mapping", at)),
};

/** `exit`, read as looper's untagged `ExitRepr`: an integer or a word. Its refusal names the check. */
const exit: Reader<Exit> = {
  expects: "0 or nonzero",
  read(v, at) {
    const check = at.slice(0, -1);
    if (typeof v === "number" && Number.isInteger(v)) {
      if (v === 0) return "zero";
      throw new Refusal(check, `\`exit\` is ${v}; it takes \`0\` or \`nonzero\``, "table");
    }
    if (typeof v === "string") {
      if (v === "nonzero") return "nonzero";
      throw new Refusal(check, `\`exit\` is \`${v}\`; it takes \`0\` or \`nonzero\``, "table");
    }
    throw new Refusal(check, "data did not match any variant of untagged enum ExitRepr", "table");
  },
};

const stage = table<Stage>(
  "Stage",
  {
    id: u32,
    title: text,
    status: variants<Status>(["open", "done"]),
    context: opt(texts),
    extra: opt(mapping),
  },
  () => ({ id: 0, title: "", status: "open", context: undefined, extra: undefined }),
  ["id", "status"],
);

const check = table<Check>(
  "Check",
  {
    id: text,
    stage: opt(u32),
    name: opt(text),
    run: texts,
    cwd: opt(text),
    exit: opt(exit),
    stdout_contains: opt(texts),
    stderr_contains: opt(texts),
    stdout_exact: opt(text),
    stdout_ordered: opt(texts),
    output_contains: opt(texts),
    output_exact: opt(text),
    output_ordered: opt(texts),
    tests: opt(texts),
    file: opt(text),
    min_bytes: opt(u64),
    timeout: opt(u64),
    env: opt(mapOf(text)),
    every: opt(u32),
    inputs: opt(texts),
    reach: opt(flag),
    floor: opt(flag),
    from: opt(text),
  },
  emptyCheck,
);

const goalFront = table<GoalFront>(
  "GoalFront",
  {
    looper: u32,
    title: text,
    summary: text,
    context: texts,
    stages: list(stage),
    checks: list(check),
    settings: opt(mapping),
    extra: opt(mapping),
  },
  () => ({
    looper: FORMAT,
    title: "",
    summary: "",
    context: [],
    stages: [],
    checks: [],
    settings: undefined,
    extra: undefined,
  }),
);

const chainFront = table<Chain>("Chain", { looper: u32, chain: texts, floor: list(check) }, emptyChain);

// ---------------------------------------------------------------------------------------------------
// Reading a file

/** A file split at its front matter: the YAML between the `---` fences and the text after them. */
export interface Split {
  yaml: string | undefined;
  body: string;
  /** The 1-based line of the file on which the body starts. */
  body_line: number;
}

/**
 * Splits `---\n<yaml>\n---\n<body>`. The text must already have LF line endings, as `normalize` makes
 * them. A file that does not start with `---` has no front matter, and all of it is the body.
 */
export function splitFrontMatter(text: string): Split {
  if (!text.startsWith("---\n")) return { yaml: undefined, body: text, body_line: 1 };
  const rest = text.slice(4);
  const end = rest.indexOf("\n---\n");
  if (end < 0) throw new Error("front matter has no closing `---`");
  const consumed = 4 + end + 5;
  return {
    yaml: rest.slice(0, end),
    body: text.slice(consumed),
    body_line: 1 + (text.slice(0, consumed).match(/\n/g)?.length ?? 0),
  };
}

/** The text with CRLF line endings turned into LF. Every reader calls it, and every writer writes LF. */
export function normalize(text: string): string {
  return text.replaceAll("\r\n", "\n");
}

/**
 * The lines of `text` as Rust's `str::lines` gives them: split at `\n`, one trailing `\r` dropped from
 * each, and no empty line after a final `\n`.
 */
export function lines(text: string): string[] {
  const out = text.split("\n");
  if (out[out.length - 1] === "") out.pop();
  return out.map((l) => (l.endsWith("\r") ? l.slice(0, -1) : l));
}

/** The split, or a refusal on the file's first line. */
function split(path: string, text: string): Split {
  try {
    return splitFrontMatter(text);
  } catch (e) {
    throw new GoalError(path, 1, (e as Error).message);
  }
}

/**
 * Reads the YAML of a front matter with `reader`, refusing with looper's message and the file's line.
 * The YAML starts on the file's second line, after the opening `---`.
 */
function readYaml<T>(path: string, yaml: string, reader: Reader<T>): T {
  let doc: unknown;
  try {
    doc = Bun.YAML.parse(yaml);
  } catch (e) {
    throw new GoalError(path, undefined, (e as Error).message);
  }
  try {
    return reader.read(doc ?? {}, []);
  } catch (e) {
    if (!(e instanceof Refusal)) throw e;
    const where = pathText(e.point === "key" ? e.at.slice(0, -1) : e.at);
    throw new GoalError(path, lineOf(yaml, e.at, e.point) + 2, where === "" ? e.message : `${where}: ${e.message}`);
  }
}

/** The path as serde_yaml names it, such as `stages[0].id`. An unknown key is named by its struct's path. */
function pathText(at: Seg[]): string {
  let out = "";
  for (const seg of at) out += typeof seg === "number" ? `[${seg}]` : out === "" ? seg : `.${seg}`;
  return out;
}

function readText(path: string): string {
  try {
    return normalize(readFileSync(path, "utf8"));
  } catch (e) {
    throw new GoalError(path, undefined, (e as Error).message);
  }
}

/** Parses a goal file's text. `path` names the file in a refusal. */
export function parseGoal(slug: string, path: string, source: string): Goal {
  const s = split(path, normalize(source));
  const yaml = s.yaml !== undefined && s.yaml.trim() !== "" ? s.yaml : "{}";
  return { slug, front: readYaml(path, yaml, goalFront), body: s.body, body_line: s.body_line };
}

/** Reads `<goalsDir>/<slug>.md`. */
export function loadGoal(goalsDir: string, slug: string): Goal {
  const path = goalPath(goalsDir, slug);
  return parseGoal(slug, path, readText(path));
}

export function goalPath(goalsDir: string, slug: string): string {
  return join(goalsDir, `${slug}.md`);
}

/** The slug of every goal file in the goals directory, on the chain or not, sorted. */
export function goalSlugs(goalsDir: string): string[] {
  let names: string[];
  try {
    names = readdirSync(goalsDir);
  } catch {
    return [];
  }
  return names
    .filter((f) => f !== README && f.endsWith(".md"))
    .map((f) => f.slice(0, -3))
    .sort();
}

/** Parses the README's text. Its generated body is ignored. */
export function parseChain(path: string, source: string): Chain {
  const s = split(path, normalize(source));
  if (s.yaml === undefined || s.yaml.trim() === "") return emptyChain();
  return readYaml(path, s.yaml, chainFront);
}

/** Reads `<goalsDir>/README.md`. A goals directory without one has an empty chain. */
export function loadChain(goalsDir: string): Chain {
  const path = join(goalsDir, README);
  if (!existsSync(path)) return emptyChain();
  return parseChain(path, readText(path));
}

// ---------------------------------------------------------------------------------------------------
// What a goal and the chain answer

/** The live stage: the first one that is not done. */
export function liveStage(g: Goal): Stage | undefined {
  return g.front.stages.find((s) => s.status !== "done");
}

/**
 * The goal's `extra` with stage `stageId`'s laid over it key by key, as compact JSON: the value of
 * `LOOPER_EXTRA`. `undefined` when neither has an `extra`.
 */
export function extraJson(g: Goal, stageId: number | undefined): string | undefined {
  const over = stageId === undefined ? undefined : g.front.stages.find((s) => s.id === stageId)?.extra;
  if (g.front.extra === undefined && over === undefined) return undefined;
  return JSON.stringify({ ...g.front.extra, ...over });
}

/**
 * The live goal of the checkout at `root`, whose goals are in `goalsDir`. On a branch `side/<slug>`
 * whose goal file exists, it is `<slug>`. Anywhere else, it is the chain's first slug, or `undefined`.
 */
export function liveGoal(root: string, goalsDir: string, chain: Chain): string | undefined {
  const side = headBranch(root)?.match(/^side\/([^/\\]+)$/)?.[1];
  if (side !== undefined && isFile(goalPath(goalsDir, side))) return side;
  return chain.chain[0];
}

/**
 * The branch checked out at `root`, read from its `HEAD` file so no process starts. A worktree's `.git`
 * is a file naming its git directory. A detached `HEAD`, or a `root` outside a repository, is `undefined`.
 */
export function headBranch(root: string): string | undefined {
  const dotGit = join(root, ".git");
  try {
    let gitDir = dotGit;
    if (!statSync(dotGit).isDirectory()) {
      const named = readFileSync(dotGit, "utf8").trim();
      if (!named.startsWith("gitdir:")) return undefined;
      // git writes the path absolute, or relative to `root` with `worktree.useRelativePaths`.
      gitDir = resolve(root, named.slice("gitdir:".length).trim());
    }
    const head = readFileSync(join(gitDir, "HEAD"), "utf8").trim();
    return head.startsWith("ref: refs/heads/") ? head.slice("ref: refs/heads/".length) : undefined;
  } catch {
    return undefined;
  }
}

function isFile(path: string): boolean {
  try {
    return statSync(path).isFile();
  } catch {
    return false;
  }
}

// ---------------------------------------------------------------------------------------------------
// Where in the YAML a refusal points

/** A place in the YAML: a 0-based line and the column its node starts at. */
interface Pos {
  line: number;
  col: number;
}

/**
 * The 0-based line of the YAML that serde_yaml reports for a refusal at `at`. A key points at its own
 * line, a value at the line it starts on, and a table at the line its map starts on. A path the block
 * form does not hold, such as one inside a flow map, points at the deepest node found.
 */
function lineOf(yaml: string, at: Seg[], point: Point): number {
  const lines = yaml.split("\n");
  let node = firstNode(lines, { line: 0, col: 0 });
  let keyLine = node.line;
  for (const seg of at) {
    const next = typeof seg === "number" ? item(lines, node, seg) : key(lines, node, seg);
    if (next === undefined) return node.line;
    keyLine = next.key;
    node = next.value;
  }
  return point === "key" ? keyLine : node.line;
}

/** The first line at or after `from` with content, at the column its content starts. */
function firstNode(lines: string[], from: Pos): Pos {
  for (let i = from.line; i < lines.length; i++) {
    const text = (lines[i] as string).slice(i === from.line ? from.col : 0);
    const content = text.search(/\S/);
    if (content >= 0 && !text.trimStart().startsWith("#")) {
      return { line: i, col: (i === from.line ? from.col : 0) + content };
    }
  }
  return from;
}

/** The lines of the block map or list at `node`: its first line, then each line indented to it or deeper. */
function* block(lines: string[], node: Pos): Generator<Pos> {
  yield node;
  for (let i = node.line + 1; i < lines.length; i++) {
    const line = lines[i] as string;
    const indent = line.search(/\S/);
    if (indent < 0 || line.trimStart().startsWith("#")) continue;
    if (indent < node.col) return;
    if (indent === node.col) yield { line: i, col: indent };
  }
}

/** The key `name` of the block map at `node`: the line it stands on, and where its value starts. */
function key(lines: string[], node: Pos, name: string): { key: number; value: Pos } | undefined {
  const quoted = JSON.stringify(name);
  for (const at of block(lines, node)) {
    const rest = (lines[at.line] as string).slice(at.col);
    const head = [name, quoted, `'${name}'`].find((k) => rest.startsWith(`${k}:`));
    if (head === undefined) continue;
    const after = at.col + head.length + 1;
    const value = (lines[at.line] as string).slice(after);
    if (value.trim() === "" || value.trimStart().startsWith("#")) {
      return { key: at.line, value: firstNode(lines, { line: at.line + 1, col: 0 }) };
    }
    return { key: at.line, value: { line: at.line, col: after + value.search(/\S/) } };
  }
  return undefined;
}

/** Item `n` of the block list at `node`, which starts after its `- `. */
function item(lines: string[], node: Pos, n: number): { key: number; value: Pos } | undefined {
  let seen = 0;
  for (const at of block(lines, node)) {
    const rest = (lines[at.line] as string).slice(at.col);
    if (rest !== "-" && !rest.startsWith("- ")) return undefined;
    if (seen++ < n) continue;
    const value = firstNode(lines, { line: at.line, col: at.col + 1 });
    return { key: at.line, value };
  }
  return undefined;
}
