// Lint: every rule `docs/goal-format.md` states, reported as data, as `crates/looper-core/src/lint.rs`
// reports it. Lint reads files and never rewrites one, and it reports every problem it finds rather
// than the first. Each message is looper's text, so the two lints compare line by line.
//
// `lintGoal` checks one parsed goal: unique stage and check ids, checks that name existing stages and
// carry a real command, the sections in their order, a heading for every stage that matches its title,
// the Handoff's subsections, and a repo-rooted `path:line` anchor on every open `### Next` item.
// `lintAll` adds what only the tree can show:
//
// - a goal file or chain whose front matter does not parse, and a slug on the chain twice or without a
//   file;
// - a README body that differs from what `looper render` writes;
// - a goal named by number in any tracked `*.md`, `*.rs`, `*.ts` or `*.tsx` file outside backticks;
// - the playbook's problems that `playbookLint` reports;
// - the meta layer: `managed`, the kit's copy as `copyIssues` in `kit/src/sync/copy.ts` checks it,
//   `decisionRecords`, `agentsPriorities` and `launchers`. `launchWarnings` reports, without failing, a
//   service that holds its slot until it ends.
//
// `managed` reads the templates of the kit that runs, through `plan` in `kit/src/sync/sync.ts`.

import { existsSync, readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { headings } from "../gates/gates.ts";
import { copyIssues } from "../sync/copy.ts";
import { type Change, plan, templates } from "../sync/sync.ts";
import { type Block, blocks, KIT_DIR, kind, markers, MERGED, render, type Template, varsFromConfig } from "../sync/template.ts";
import { ABOUT_MAX, type Config, MAX_LAUNCHERS, PLACEHOLDERS, READY_WINDOW, ruleRegex } from "./config.ts";
import {
  type Chain,
  EXTRA_MAX,
  extraJson,
  type Goal,
  GoalError,
  goalPath,
  goalSlugs,
  loadChain,
  loadGoal,
  normalize,
  README,
} from "./model.ts";
import { loadPlaybook, playbookLint } from "./playbook.ts";
import { debug } from "./reader.ts";
import { jsonQuote, renderReadme } from "./render.ts";
import { anchors, HANDOFF_SECTIONS, items, names, SECTIONS, type Section, section, sectionsOf, stageHeadings } from "./sections.ts";

/** One problem lint found: the file from the project root with `/`, its line when it has one, and why. */
export interface Issue {
  file: string;
  line: number | undefined;
  message: string;
}

/** The issue as looper prints it: `file:line: message`, or `file: message` without a line. */
export function issueText(i: Issue): string {
  return i.line === undefined ? `${i.file}: ${i.message}` : `${i.file}:${i.line}: ${i.message}`;
}

const SLUG = /^[a-z0-9]+(-[a-z0-9]+)*$/;

/** Whether the text is a kebab-case slug. */
export function isSlug(s: string): boolean {
  return SLUG.test(s);
}

/** The first `n` characters of `s`, counted as Rust counts `chars`. */
function first(s: string, n: number): string {
  return Array.from(s).slice(0, n).join("");
}

/** A refusal to read a goal file or the chain, as an issue against `file`. */
function refusalIssue(e: unknown, file: string): Issue {
  if (e instanceof GoalError) return { file, line: e.line, message: e.detail };
  return { file, line: undefined, message: e instanceof Error ? e.message : String(e) };
}

// ---------------------------------------------------------------------------------------------------
// One goal

type At = (line: number | undefined, message: string) => void;

/** Every problem in one goal. `file` names the goal file in each issue. */
export function lintGoal(g: Goal, file: string): Issue[] {
  const out: Issue[] = [];
  const at: At = (line, message) => out.push({ file, line, message });
  lintFront(g, at);
  lintProse(g, at);
  return out;
}

/** The rules on the front matter alone. */
function lintFront(g: Goal, at: At): void {
  const f = g.front;
  if (!isSlug(g.slug)) at(undefined, "the slug is not kebab-case");
  if (f.title === "") at(undefined, "front matter has no `title`");
  if (f.summary === "") at(undefined, "front matter has no `summary`");
  if (f.stages.length === 0) at(undefined, "front matter has no `stages`");
  const ids = new Set<number>();
  for (const s of f.stages) {
    if (ids.has(s.id)) at(undefined, `stage id ${s.id} appears twice`);
    ids.add(s.id);
    if (s.title === "") at(undefined, `stage ${s.id} has no title`);
  }
  const laid: [string, number | undefined][] = [["", undefined]];
  for (const s of f.stages) if (s.extra !== undefined) laid.push([`stage ${s.id}: `, s.id]);
  for (const [whose, stage] of laid) {
    const json = extraJson(g, stage);
    const bytes = json === undefined ? 0 : Buffer.byteLength(json);
    if (bytes > EXTRA_MAX) at(undefined, `${whose}\`extra\` is ${bytes} bytes of JSON; it takes at most ${EXTRA_MAX}`);
  }
  const checkIds = new Set<string>();
  for (const c of f.checks) {
    if (!isSlug(c.id)) at(undefined, `a check has no kebab-case \`id\`: ${jsonQuote(c.id)}`);
    if (checkIds.has(c.id)) at(undefined, `check id ${c.id} appears twice`);
    checkIds.add(c.id);
    if (c.run.length === 0 && c.file === undefined) at(undefined, `check ${c.id} has no \`run\` argv`);
    const streams =
      c.stdout_contains !== undefined || c.stderr_contains !== undefined || c.stdout_exact !== undefined || c.stdout_ordered !== undefined;
    if (c.run.length === 0 && c.file !== undefined && streams) {
      at(undefined, `check ${c.id} judges a file, which has no streams: use an \`output_*\` matcher`);
    }
    if (c.every === 0) at(undefined, `check ${c.id} has \`every: 0\`; it takes 1 or more`);
    if (c.run[0] === "TODO") at(undefined, `check ${c.id} is a TODO: replace it with a real command`);
    if (c.stage !== undefined && !ids.has(c.stage)) at(undefined, `check ${c.id} names stage ${c.stage}, which the goal lacks`);
  }
}

/** The rules on the prose, and on how it agrees with the front matter. */
function lintProse(g: Goal, at: At): void {
  const f = g.front;
  const ids = new Set(f.stages.map((s) => s.id));
  const all = sectionsOf(g.body, 2);
  const found = names(all);
  const missing = SECTIONS.filter((s) => !found.includes(s)).map((s) => `## ${s}`);
  const ordered = found.filter((n) => (SECTIONS as readonly string[]).includes(n));
  if (missing.length > 0) {
    at(undefined, `missing section(s): ${missing.join(", ")}`);
  } else if (ordered.join("\n") !== SECTIONS.join("\n")) {
    at(undefined, `sections are out of order; want ${SECTIONS.map((s) => `## ${s}`).join(", ")}`);
  }
  // A line within a section's text, as a line of the file.
  const fileLine = (s: Section, line: number) => g.body_line + s.text_line + line - 2;

  const stages = section(all, "Stages");
  const headed = stages ? stageHeadings(stages.text) : [];
  for (const s of f.stages) {
    const h = headed.find((h) => h.id === s.id);
    if (h === undefined) {
      at(undefined, `stage ${s.id} has no \`### ${s.id}. ${s.title}\` heading under ## Stages`);
    } else if (h.title !== s.title) {
      at(stages && fileLine(stages, h.line), `stage ${s.id}'s heading says "${h.title}", the front matter "${s.title}"`);
    }
  }
  for (const h of headed) {
    if (!ids.has(h.id)) at(stages && fileLine(stages, h.line), `## Stages has a heading for stage ${h.id}, which the front matter lacks`);
  }

  const handoff = section(all, "Handoff");
  const subs = handoff ? sectionsOf(handoff.text, 3) : [];
  for (const name of HANDOFF_SECTIONS) {
    if (section(subs, name) === undefined) at(handoff && g.body_line + handoff.line - 1, `## Handoff has no ### ${name}`);
  }
  const next = section(subs, "Next");
  if (handoff && next) {
    for (const item of items(next.text)) {
      if (!item.done && anchors(item.text).length === 0) {
        const line = fileLine(handoff, next.text_line + item.line - 1);
        at(line, `an open ### Next item has no repo-rooted path:line anchor: ${first(item.text.trim(), 80)}`);
      }
    }
  }
}

// ---------------------------------------------------------------------------------------------------
// Goal numbers

/** A goal named by its number, such as `goal 3` or `goals #12`, outside backticks. */
const NUMBERED = /\bgoals?\s+#?[0-9]+\b/i;

/** Every line of one file's text that names a goal by its number. `file` names it in each issue. */
export function numberedMentionsIn(file: string, text: string): Issue[] {
  const out: Issue[] = [];
  text.split("\n").forEach((line, i) => {
    const plain = line.replace(/`[^`]*`/g, "``");
    if (NUMBERED.test(plain) && !line.includes("NUMBERED") && !line.includes("numberedMentions")) {
      out.push({ file, line: i + 1, message: `names a goal by number: ${first(line.trim(), 100)}` });
    }
  });
  return out;
}

/**
 * Every tracked or untracked-but-not-ignored markdown, Rust and TypeScript file in the repository at
 * `root` that names a goal by its number.
 */
export function numberedMentions(root: string): Issue[] {
  let listed: string;
  try {
    const run = Bun.spawnSync(["git", "ls-files", "-z", "-co", "--exclude-standard", "--", "*.md", "*.ts", "*.rs", "*.tsx"], {
      cwd: root,
      stdout: "pipe",
      stderr: "ignore",
    });
    listed = new TextDecoder().decode(run.stdout);
  } catch {
    return [{ file: ".", line: undefined, message: "`git ls-files` did not run, so goal-number mentions were not checked" }];
  }
  return listed
    .split("\0")
    .filter((f) => f !== "")
    .flatMap((f) => {
      let text: string;
      try {
        text = new TextDecoder("utf-8", { fatal: true }).decode(readFileSync(join(root, f)));
      } catch {
        return [];
      }
      return numberedMentionsIn(f, text);
    });
}

// ---------------------------------------------------------------------------------------------------
// The tree

/**
 * Every problem in the project at `root`: the chain, each goal file, the README's freshness, goal
 * numbers anywhere in the tree, the playbook and the meta layer.
 */
export function lintAll(root: string, cfg: Config): Issue[] {
  const out: Issue[] = [];
  const dir = join(root, cfg.project.goals_dir);
  const rel = (name: string) => `${cfg.project.goals_dir.replace(/\/+$/, "")}/${name}`;
  const readme = join(dir, README);
  const issue = (file: string, message: string): Issue => ({ file, line: undefined, message });
  if (!existsSync(readme)) out.push(issue(rel(README), "the chain's file is missing"));
  let chain: Chain | undefined;
  try {
    chain = loadChain(dir);
  } catch (e) {
    out.push(refusalIssue(e, rel(README)));
  }
  const seen = new Set<string>();
  for (const slug of chain?.chain ?? []) {
    if (seen.has(slug)) out.push(issue(rel(README), `chain: ${slug} appears twice`));
    seen.add(slug);
    if (!existsSync(goalPath(dir, slug))) out.push(issue(rel(README), `chain: ${slug} has no file ${rel(`${slug}.md`)}`));
  }
  for (const slug of goalSlugs(dir)) {
    const file = rel(`${slug}.md`);
    try {
      out.push(...lintGoal(loadGoal(dir, slug), file));
    } catch (e) {
      out.push(refusalIssue(e, file));
    }
  }
  if (chain !== undefined && readmeStale(dir, chain)) out.push(issue(rel(README), "the README is stale: run `looper render`"));
  try {
    const book = loadPlaybook(root, cfg);
    if (book !== undefined) out.push(...playbookLint(book));
  } catch {
    // A playbook that cannot be read has no problems lint can name, as looper's `unwrap_or_default`.
  }
  out.push(...numberedMentions(root));
  out.push(...managed(root, cfg));
  out.push(...copyIssues(root, KIT_DIR));
  out.push(...decisionRecords(root));
  out.push(...agentsPriorities(root));
  out.push(...launchers(cfg));
  return out;
}

/** Whether the README in `dir` exists and differs from what `looper render` writes for `chain`. */
export function readmeStale(dir: string, chain: Chain): boolean {
  let text: string;
  try {
    text = readFileSync(join(dir, README), "utf8");
  } catch {
    return false;
  }
  return normalize(text) !== renderReadme(chain, dir);
}

// ---------------------------------------------------------------------------------------------------
// The meta layer

/** The sections every decision record has, in order. */
export const RECORD_SECTIONS = ["Decision", "Why"];
/** The sections a decision record may add after `RECORD_SECTIONS`, in order. */
export const RECORD_OPTIONAL = ["Consequences", "Rejected"];
/** The directory of the decision records. */
export const DECISIONS = "docs/decisions";

function readOrUndefined(path: string): string | undefined {
  try {
    return readFileSync(path, "utf8");
  } catch {
    return undefined;
  }
}

/**
 * Every managed file that differs from the templates, as `looper sync --check` sees it. A block older
 * than the template's is behind this looper, and any other difference is a hand edit. A project none of
 * whose managed files carries a marker has never been synced, and is not checked.
 */
export function managed(root: string, cfg: Config): Issue[] {
  const issue = (file: string, message: string): Issue => ({ file, line: undefined, message });
  const why = (e: unknown) => (e instanceof Error ? e.message : String(e));
  let all: Template[];
  try {
    all = templates(cfg.project.stacks);
  } catch (e) {
    return [issue("looper.toml", why(e))];
  }
  const vars = varsFromConfig(cfg);
  const marked = all
    .filter((t) => kind(t) === "whole" || kind(t) === "block")
    .map((t) => render(t.path, vars))
    .filter((path) => !cfg.sync.skip.includes(path))
    .some((path) => {
      const text = readUtf8(join(root, path));
      return text !== undefined && markers(text).length > 0;
    });
  if (!marked) return [];
  let changes: Change[];
  try {
    changes = plan(root, cfg);
  } catch (e) {
    return [issue(".", why(e))];
  }
  return changes.map((c) => {
    if (c.before === undefined) return issue(c.path, "a managed file is missing: run `looper sync`");
    let want: Block[] = [];
    try {
      const t = all.find((t) => render(t.path, vars) === c.path);
      if (t !== undefined) want = blocks(t.text);
    } catch {
      // A template whose markers do not parse has no versions to compare, as looper's `.ok()`.
    }
    let have: Block[] = [];
    try {
      have = blocks(c.before);
    } catch {
      // A file whose markers do not parse has no versions to compare, as looper's `unwrap_or_default`.
    }
    for (const w of want) {
      const h = have.find((h) => h.name === w.name);
      if (h !== undefined && h.version < w.version) {
        return issue(
          c.path,
          `a managed block is behind this looper: \`${w.name}\` is v${h.version}, and this looper's is v${w.version}. Run \`looper sync\``,
        );
      }
    }
    if (MERGED.includes(c.path)) return issue(c.path, "lacks looper's allow entries or its guard hook: run `looper sync`");
    return issue(c.path, "managed text was edited by hand: run `looper sync`, and move the project's text outside the markers");
  });
}

/** The file at `path` as UTF-8, or `undefined` when it cannot be read or is not UTF-8. */
function readUtf8(path: string): string | undefined {
  try {
    return new TextDecoder("utf-8", { fatal: true }).decode(readFileSync(path));
  } catch {
    return undefined;
  }
}

/** Every decision record out of shape, once the project's `docs/decisions/README.md` is looper's. */
export function decisionRecords(root: string): Issue[] {
  const dir = join(root, DECISIONS);
  const index = readOrUndefined(join(dir, README));
  const managed = index !== undefined && markers(index).length > 0;
  let entries: string[];
  try {
    entries = readdirSync(dir);
  } catch {
    return [];
  }
  if (!managed) return [];
  const out: Issue[] = [];
  for (const name of entries.sort()) {
    const number = /^([0-9]{4})\.md$/.exec(name)?.[1];
    if (number === undefined) continue;
    const raw = readOrUndefined(join(dir, name));
    if (raw === undefined) continue;
    const text = normalize(raw);
    const file = `${DECISIONS}/${name}`;
    const heads = headings(text);
    const top = heads[0];
    const prefix = `${number}: `;
    const titleOk = top !== undefined && top.level === 1 && top.text.startsWith(prefix) && top.text.slice(prefix.length).trim() !== "";
    if (!titleOk) {
      out.push({ file, line: 1, message: `a decision record starts with the title \`# ${number}: <what is now true>\`` });
    }
    const sections = heads.filter((h) => h.level === 2).map((h) => h.text);
    if (!recordShaped(sections)) {
      const shown = sections.map((s) => `## ${s}`).join(", ");
      out.push({
        file,
        line: undefined,
        message: `the sections are [${shown}]; a decision record has ## Decision, ## Why, then optionally ## Consequences and ## Rejected, in that order`,
      });
    }
  }
  return out;
}

/** Whether the `##` sections are `RECORD_SECTIONS`, then some of `RECORD_OPTIONAL` in their order. */
function recordShaped(sections: string[]): boolean {
  if (!RECORD_SECTIONS.every((s, i) => sections[i] === s)) return false;
  let at = 0;
  return sections.slice(RECORD_SECTIONS.length).every((s) => {
    const p = RECORD_OPTIONAL.slice(at).indexOf(s);
    if (p < 0) return false;
    at += p + 1;
    return true;
  });
}

/** The agents file whose project part names its priority ordering. */
export const AGENTS = "AGENTS.md";

/**
 * An issue when `AGENTS.md` exists and its text outside looper's blocks has no heading that names a
 * priority ordering.
 */
export function agentsPriorities(root: string): Issue[] {
  const raw = readOrUndefined(join(root, AGENTS));
  if (raw === undefined) return [];
  const text = normalize(raw);
  let found: ReturnType<typeof blocks>;
  try {
    found = blocks(text);
  } catch {
    return [];
  }
  const inside = (line: number) => found.some((b) => b.begin <= line && (b.end === undefined || line <= b.end));
  if (headings(text).some((h) => !inside(h.line) && h.text.toLowerCase().includes("priorit"))) return [];
  return [
    {
      file: AGENTS,
      line: undefined,
      message: "the project part has no priority ordering: add a `## The priority ordering` heading outside looper's block",
    },
  ];
}

/** The file the launchers are declared in. */
export const LOOPER_TOML = "looper.toml";

/** Whether `cwd` is relative and has no `..`, so it stays inside the root. */
function staysInside(cwd: string): boolean {
  if (/^[\\/]/.test(cwd) || /^[A-Za-z]:/.test(cwd)) return false;
  return cwd.split(/[\\/]/).every((part) => part !== "..");
}

/**
 * Every problem in the `[[launch]]` tables: more than `MAX_LAUNCHERS`, a missing `name`, `title`,
 * `about` or `run`, a name that is not kebab-case or is taken twice, an `about` over `ABOUT_MAX`, a
 * placeholder other than `PLACEHOLDERS`, `{port}` without a `port`, `{args}` without `args = true`, a
 * `cwd` that leaves the root, and a `ready` regex that does not compile.
 */
export function launchers(cfg: Config): Issue[] {
  const out: Issue[] = [];
  const issue = (message: string) => out.push({ file: LOOPER_TOML, line: undefined, message });
  if (cfg.launch.length > MAX_LAUNCHERS) issue(`${cfg.launch.length} launchers, over the ${MAX_LAUNCHERS} a project may declare`);
  const seen = new Set<string>();
  cfg.launch.forEach((l, i) => {
    const who = l.name === "" ? `launch #${i + 1}` : `launch ${l.name}`;
    const required: [string, boolean][] = [
      ["name", l.name === ""],
      ["title", l.title.trim() === ""],
      ["about", l.about.trim() === ""],
      ["run", l.run.length === 0],
    ];
    for (const [field, missing] of required) if (missing) issue(`${who}: needs \`${field}\``);
    if (l.name !== "" && !isSlug(l.name)) issue(`${who}: the name must be kebab-case`);
    if (l.name !== "") {
      if (seen.has(l.name)) issue(`${who}: the name is taken twice`);
      seen.add(l.name);
    }
    const about = Array.from(l.about).length;
    if (about > ABOUT_MAX) issue(`${who}: \`about\` has ${about} characters, over ${ABOUT_MAX}`);
    const fields: [string, string[]][] = [
      ["run", l.run],
      ["build", l.build],
      ["env", Object.values(l.env)],
      ["open", l.open === undefined ? [] : [l.open]],
    ];
    for (const [field, texts] of fields) {
      for (const found of texts.flatMap((t) => t.match(/\{[^{}\s]*\}/g) ?? [])) {
        if (!PLACEHOLDERS.includes(found)) {
          issue(`${who}: \`${found}\` in \`${field}\` is not a placeholder; only \`{port}\` and \`{args}\` are`);
        } else if (found === "{port}" && l.port === undefined) {
          issue(`${who}: \`{port}\` in \`${field}\` needs a \`port\``);
        } else if (found === "{args}" && !l.args) {
          issue(`${who}: \`{args}\` in \`${field}\` needs \`args = true\``);
        }
      }
    }
    if (l.cwd !== undefined && !staysInside(l.cwd)) issue(`${who}: \`cwd\` ${debug(l.cwd)} must be relative and stay inside the root`);
    if (l.ready !== undefined && l.ready !== READY_WINDOW) {
      try {
        ruleRegex(l.ready);
      } catch (e) {
        issue(`${who}: \`ready\` ${debug(l.ready)} does not compile: ${(e as Error).message}`);
      }
    }
  });
  return out;
}

/** What lint reports about the launchers without failing: a heavy service with no `ready`. */
export function launchWarnings(cfg: Config): Issue[] {
  return cfg.launch
    .filter((l) => l.kind === "service" && l.heavy && l.ready === undefined)
    .map((l) => ({
      file: LOOPER_TOML,
      line: undefined,
      message: `launch ${l.name}: a service with no \`ready\` holds its heavy slot until it ends`,
    }));
}
