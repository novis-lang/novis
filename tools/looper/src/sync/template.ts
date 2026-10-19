// The templates of the meta layer every project shares, as `crates/looper-core/src/template.rs` holds
// them, and the marker lines and blocks of the managed text they write. Lint reads them to tell a
// project's own text from looper's, and `plan` in `sync.ts` compares a project with them.
//
// The kit reads its own `templates/`, or the bundle of them a project's `tools/looper/` carries, one
// file per file `looper init` writes, at the path it is written to. `core/` is written into every
// project, and `stack/<profile>/` into a project of that stack. Each profile's `profile.toml` and
// `layout.md` are never written: they hold the verify steps `{{verify}}` takes and the README rows
// `{{layout}}` takes. Sync gives every project the copy, so a template runs the copy where it names a
// command: the settings' guard hook and shell prefix, and the core's verify steps.
//
// A template's markers say how much of the written file is looper's (`Kind`):
//
// - **Whole**: one begin marker, on the first line or on the second after a `#!` line, and no end.
// - **Block**: each block from a begin marker to the end marker of the same name.
// - **Merged**: `.claude/settings.json`, whose entries are added to the project's.
// - **Seed**: no marker. Sync never touches it.
//
// A marker is a whole line, `<!-- looper:begin <name> v<N> -->` or `# looper:end <name>`. Managed text
// may hold the placeholders of `Lines`, each alone on its line, which `varsFromConfig` builds from the
// project's `looper.toml`. So two projects with different configs each match their own.
//
// The templates are a fixed set, read once and kept for the life of the process.

import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";
import { type Config, interactiveBudget, PROSE_SCOPE } from "../goal/config.ts";
import { lines } from "../goal/model.ts";

/** One managed block: its name, its version, its begin and end lines, and its text with both markers. */
export interface Block {
  name: string;
  version: number;
  begin: number;
  /** The end marker's line, or `undefined` for a block that runs to the end of a whole file. */
  end: number | undefined;
  text: string;
}

/** One marker line: whether it begins a block, the block's name, and the version a begin carries. */
export interface Marker {
  line: number;
  begin: boolean;
  name: string;
  version: number | undefined;
}

const MARKER = /^\s*(?:<!--|#)\s*looper:(begin|end)\s+([a-z0-9-]+)(?:\s+v(\d+))?\s*(?:-->)?\s*$/;

/** Every marker line in `text`, in order. */
export function markers(text: string): Marker[] {
  const out: Marker[] = [];
  lines(text).forEach((line, i) => {
    const c = MARKER.exec(line);
    if (!c) return;
    const version = c[3] === undefined ? undefined : Number(c[3]);
    out.push({
      line: i + 1,
      begin: c[1] === "begin",
      name: c[2] as string,
      version: version !== undefined && version <= 0xffff_ffff ? version : undefined,
    });
  });
  return out;
}

/**
 * The managed blocks of `text`. It throws on a begin without a version, a block opened inside another,
 * an end that closes no block or another block, a name used twice, and a block never closed unless it
 * is a whole file's only marker.
 */
export function blocks(text: string): Block[] {
  const all = lines(text);
  const wholeLine = all[0]?.startsWith("#!") ? 2 : 1;
  const found = markers(text);
  const out: Block[] = [];
  let open: { line: number; name: string; version: number } | undefined;
  const joined = (from: number, to: number) => all
    .slice(from, to)
    .map((l) => `${l}\n`)
    .join("");
  for (const m of found) {
    if (m.begin) {
      if (open !== undefined) {
        throw new Error(`line ${m.line}: \`${m.name}\` begins inside \`${open.name}\`, which began on line ${open.line}`);
      }
      if (m.version === undefined) throw new Error(`line ${m.line}: \`${m.name}\` begins without a version \`v<N>\``);
      if (out.some((b) => b.name === m.name)) throw new Error(`line ${m.line}: the block \`${m.name}\` is there twice`);
      open = { line: m.line, name: m.name, version: m.version };
    } else {
      if (open === undefined) throw new Error(`line ${m.line}: \`${m.name}\` ends a block that never began`);
      if (open.name !== m.name) throw new Error(`line ${m.line}: \`${m.name}\` ends, but the open block is \`${open.name}\``);
      out.push({ name: open.name, version: open.version, begin: open.line, end: m.line, text: joined(open.line - 1, m.line) });
      open = undefined;
    }
  }
  if (open !== undefined) {
    if (open.line !== wholeLine || found.length !== 1) throw new Error(`line ${open.line}: \`${open.name}\` never ends`);
    out.push({ name: open.name, version: open.version, begin: open.line, end: undefined, text: joined(open.line - 1, all.length) });
  }
  return out;
}

// ---------------------------------------------------------------------------------------------------
// The templates

/** The directory of the kit that runs: `kit/` in looper's checkout, or a project's `tools/looper/`. */
export const KIT_DIR = join(import.meta.dir, "..", "..");

/**
 * The file in a project's copy that bundles the templates, as a JSON array of `[path, text]`. One file
 * keeps a template such as a stack's `package.json` or test from looking like the project's own.
 */
export const BUNDLE = "templates.json";
/** The profile written into every project. */
export const CORE = "core";
/** The stack profiles, each a directory `templates/stack/<name>/`. */
export const PROFILES = ["rust", "bun"];
/** The file in each profile that declares its verify steps and is never written. */
export const PROFILE_FILE = "profile.toml";
/** The file in each stack that holds its rows of the README's layout table and is never written. */
export const LAYOUT_FILE = "layout.md";
/** The templates merged into the project's file rather than marked. */
export const MERGED = [".claude/settings.json"];

/** How much of a written file is looper's. */
export type Kind = "whole" | "block" | "merged" | "seed";

/** One template: the profile it belongs to, the path it is written to in a project, and its text. */
export interface Template {
  profile: string;
  path: string;
  text: string;
}

let files: [string, string][] | undefined;

/**
 * Every template file of the kit at `kit`, as its `/`-separated path below `templates/` and its text,
 * sorted by path. A copy reads its `BUNDLE`, and looper's checkout the `templates/` of `kit/`.
 */
export function templateEntries(kit: string): [string, string][] {
  const bundle = join(kit, BUNDLE);
  if (existsSync(bundle)) return JSON.parse(readFileSync(bundle, "utf8")) as [string, string][];
  const out: [string, string][] = [];
  const walk = (dir: string, prefix: string) => {
    for (const name of readdirSync(dir)) {
      const path = join(dir, name);
      if (statSync(path).isDirectory()) walk(path, `${prefix}${name}/`);
      else out.push([`${prefix}${name}`, readFileSync(path, "utf8")]);
    }
  };
  walk(join(kit, "templates"), "");
  return out.sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0));
}

/** The template files of the kit that runs, read once. */
function templateFiles(): [string, string][] {
  files ??= templateEntries(KIT_DIR);
  return files;
}

/** The profile and the written path of a file below `templates/`, or `undefined` outside a profile. */
function split(rel: string): [string, string] | undefined {
  if (rel.startsWith("core/")) return [CORE, rel.slice("core/".length)];
  if (!rel.startsWith("stack/")) return undefined;
  const rest = rel.slice("stack/".length);
  const at = rest.indexOf("/");
  return at < 0 ? undefined : [rest.slice(0, at), rest.slice(at + 1)];
}

/** Every template that is written into a project, in path order within each profile. */
export function allTemplates(): Template[] {
  const out: Template[] = [];
  for (const [rel, text] of templateFiles()) {
    const s = split(rel);
    if (s !== undefined && s[1] !== PROFILE_FILE && s[1] !== LAYOUT_FILE) out.push({ profile: s[0], path: s[1], text });
  }
  return out;
}

/** The templates of one profile, `CORE` or a stack, or `undefined` for a profile looper does not have. */
export function profile(name: string): Template[] | undefined {
  if (name !== CORE && !PROFILES.includes(name)) return undefined;
  return allTemplates().filter((t) => t.profile === name);
}

/** The text of a profile's never-written `file`, or `""` for a profile without one. */
function profileFile(name: string, file: string): string {
  return templateFiles().find(([rel]) => {
    const s = split(rel);
    return s !== undefined && s[0] === name && s[1] === file;
  })?.[1] ?? "";
}

/** The rows a profile adds to the README's layout table, or `""` for a profile without a `LAYOUT_FILE`. */
export function layout(name: string): string {
  return profileFile(name, LAYOUT_FILE);
}

/** The `[[verify.step]]` tables a profile adds to `looper.toml`, as TOML text. */
export function steps(name: string): string {
  return profileFile(name, PROFILE_FILE);
}

/** How much of the file `t` writes is looper's. A template whose markers do not parse is a seed. */
export function kind(t: Template): Kind {
  if (MERGED.includes(t.path)) return "merged";
  let found: Block[];
  try {
    found = blocks(t.text);
  } catch {
    return "seed";
  }
  if (found.length === 0) return "seed";
  if (found.length === 1 && found[0]?.end === undefined) return "whole";
  return "block";
}

// ---------------------------------------------------------------------------------------------------
// Rendering

/**
 * The managed lines `varsFromConfig` builds, by placeholder, each one whole list item, table row or
 * paragraph wrapped as the templates are. A line that is empty is left out of the file with its
 * placeholder.
 */
export interface Lines {
  ceiling: string;
  slice: string;
  work: string;
  prose_link: string;
  prose_rule: string;
  prose_scope: string;
  goals: string;
  harvest: string;
  design: string;
  design_home: string;
  design_shrink: string;
  decisions: string;
  commit_types: string;
  subject: string;
}

/** What `render` puts in place of each placeholder. */
export interface Vars {
  name: string;
  /** `{{stacks}}` as a TOML array, and the profiles whose steps `{{verify}}` takes, in order. */
  stacks: string[];
  rust_version: string;
  bun_version: string;
  lines: Lines;
}

/** The decision records' README. A project that keeps it out of sync has no records. */
export const DECISIONS_README = "docs/decisions/README.md";

/** The column the managed text wraps at. */
const WIDTH = 105;

/** The project's name and stacks, and the managed lines, from its config. The tool versions stay empty. */
export function varsFromConfig(cfg: Config): Vars {
  const b = cfg.budget;
  const interactive = interactiveBudget(b);
  const [hard, inter, slice] = [tokens(b.hard), tokens(interactive), tokens(b.slice)];
  const ceiling = interactive === b.hard
    ? `- Every session and every subagent works under **${hard}** tokens of context.`
    : `- A loop session works under **${hard}** tokens of context, and an interactive session or a subagent under **${inter}**.`;
  const handOff = interactive === b.hard ? `At ${hard}` : "At the ceiling";
  const scope = cfg.prose.scope.trim();
  const lower = lowerFirst(scope);
  const goals = cfg.project.goals_dir.replace(/\/+$/, "");
  const floor = cfg.project.carry_to_floor ? "its checks move into the floor" : "only a check that sets `floor: true` moves into the floor";
  const h = cfg.project.harvest;
  const harvest = h === null
    ? `- **A reached goal is not harvested: its file is deleted.** ${upperFirst(floor)}.`
    : `- **A reached goal is harvested into [${h.file}](${h.file})** § \`${
      h.section.replace(/^#+/, "").trim().replaceAll("{slug}", "<slug>")
    }\`: its target, its findings and the decisions that still bind. Then ${floor}${
      cfg.project.carry_to_floor ? "" : ","
    } and its file is deleted.`;
  const design = cfg.project.design;
  const types = cfg.commits.types.map((t) => `\`${t}\``);
  const last = types[types.length - 1];
  const typeText = last === undefined ? "" : types.length === 1 ? `is ${last}` : `is one of ${types.slice(0, -1).join(", ")} or ${last}`;
  return {
    name: cfg.project.name,
    stacks: [...cfg.project.stacks],
    rust_version: "",
    bun_version: "",
    lines: {
      ceiling: wrap(ceiling),
      slice: wrap(`- Take no new slice past **${slice}**. ${handOff}, hand off.`),
      work: wrap(
        "2. **Do the work: as much of the group as fits.** The handoff names a group of related slices and the files "
          + "they share. Keep taking slices while the next one touches files already loaded and you are under "
          + `${slice} with the previous one committed. Before you take an item, write two to six checkpoints under `
          + "it in the goal file, each a step you can tick alone. Tick each as you reach it, and the item once all "
          + "are ticked. Tick on from checkpoints an earlier session left.",
      ),
      prose_link: wrap(`- **[docs/agent/prose.md](docs/agent/prose.md)**: how ${lower} is written.`),
      prose_rule: wrap(
        `8. **${scope} follows the prose rules:** at most 25 words, one idea, and no dash joining two sentences (prose.md).`,
      ),
      prose_scope: scope === PROSE_SCOPE ? "" : wrap(`The rules below bind ${lower} only.`),
      goals: wrap(
        `- **The goals are [${goals}/](${goals}/README.md),** in looper's goal format. The chain, which is the order `
          + `the loop walks, is the front matter of \`${goals}/README.md\`, and its first slug is the live goal. `
          + "`looper render` writes the README's body, so never edit it by hand.",
      ),
      harvest: wrap(harvest),
      design: design === "" ? "" : `| Looking for how something works | [${design}](${design}) until a module's head comment owns it |`,
      design_home: designHome(design),
      design_shrink: designShrink(design),
      decisions: cfg.sync.skip.includes(DECISIONS_README)
        ? ""
        : "- **[docs/decisions/](docs/decisions/README.md)**: numbered decision records. Cite one as\n  `decision 0003`.",
      commit_types: wrap(`- **\`type\`** ${typeText}.`),
      subject: wrap(
        "- **The subject** is lower case after the colon, has no trailing period, and is at most "
          + `${cfg.commits.subject_max} characters. It states what is now true, so \`git log --oneline\` alone says `
          + "what each commit did.",
      ),
    },
  };
}

/** A count of tokens as the rules write it: `250k` for 250000. */
function tokens(n: number): string {
  return n % 1000 === 0 ? `${n / 1000}k` : String(n);
}

/** `{{design_home}}`: doc-style's item on where each fact lives, linking `design` when it is set. */
function designHome(design: string): string {
  const doc = design === "" ? "" : `, or in [${design.split("/").pop() ?? design}](${fromAgentDocs(design)}) until one exists`;
  return wrap(
    `- **State a fact once.** How a part works lives in its module's head comment${doc}. A rule that binds every `
      + "agent lives in `AGENTS.md`. A shape lives in [conventions.md](conventions.md). A trap lives in "
      + "[playbook.md](playbook.md). Link to the home from everywhere else, because a summary in a second place goes "
      + "stale without a sound.",
  );
}

/** `{{design_shrink}}`: the paragraph on shrinking `design`, after a blank line, or empty when it is unset. */
function designShrink(design: string): string {
  if (design === "") return "";
  return `\n${wrap(`When a head comment takes over a topic from \`${design}\`, shrink that section to a link in the same commit.`)}`;
}

/** The link to `path`, from the project root, as a file in `docs/agent/` writes it. */
function fromAgentDocs(path: string): string {
  if (path.startsWith("docs/agent/")) return path.slice("docs/agent/".length);
  if (path.startsWith("docs/")) return `../${path.slice("docs/".length)}`;
  return `../../${path}`;
}

function lowerFirst(s: string): string {
  const [f, ...rest] = Array.from(s);
  return f === undefined ? "" : f.toLowerCase() + rest.join("");
}

function upperFirst(s: string): string {
  const [f, ...rest] = Array.from(s);
  return f === undefined ? "" : f.toUpperCase() + rest.join("");
}

const MARK = /^(?:[-*]|[0-9]+\.) /;

/** One line wrapped at `WIDTH` between words. A list item's later lines are indented under its text. */
function wrap(line: string): string {
  const indent = " ".repeat(MARK.exec(line)?.[0].length ?? 0);
  let out = "";
  let width = 0;
  for (const word of line.split(" ")) {
    const n = Array.from(word).length;
    if (width > 0 && width + 1 + n > WIDTH) {
      out += `\n${indent}`;
      width = indent.length;
    } else if (width > 0) {
      out += " ";
      width += 1;
    }
    out += word;
    width += n;
  }
  return out;
}

const PLACEHOLDER = /\{\{([a-z_]+)\}\}/g;
const ONLY_PLACEHOLDER = /^\{\{([a-z_]+)\}\}$/;

/** The line of the placeholder `name`, or `undefined` for a name that is not one of `Lines`. */
function lineOf(vars: Vars, name: string): string | undefined {
  return Object.hasOwn(vars.lines, name) ? vars.lines[name as keyof Lines] : undefined;
}

/** The lines of `text`, each with its `\n`, as Rust's `split_inclusive('\n')` gives them. */
export function inclusiveLines(text: string): string[] {
  return text.match(/[^\n]*\n|[^\n]+$/g) ?? [];
}

/**
 * `text` with each placeholder filled from `vars`. A placeholder `Vars` does not know stays as it is. A
 * line that is only a placeholder of `Lines`, and whose line is empty, is left out.
 */
export function render(text: string, vars: Vars): string {
  return inclusiveLines(text)
    .filter((line) => {
      const c = ONLY_PLACEHOLDER.exec(line.trim());
      return c === null || lineOf(vars, c[1] as string) !== "";
    })
    .map((line) =>
      line.replace(PLACEHOLDER, (whole, name: string) => {
        switch (name) {
          case "name":
            return vars.name;
          case "stacks":
            return `[${vars.stacks.map((s) => JSON.stringify(s)).join(", ")}]`;
          case "verify":
            return verifyText(vars.stacks);
          case "layout":
            return [CORE, ...vars.stacks].map((name) => layout(name)).join("");
          case "rust_version":
            return vars.rust_version;
          case "bun_version":
            return vars.bun_version;
          default:
            return lineOf(vars, name) ?? whole;
        }
      })
    )
    .join("");
}

/** The verify steps of `stacks`, in order, then the core's, as TOML without a trailing newline. */
function verifyText(stacks: string[]): string {
  return [...stacks, CORE]
    .map((name) => steps(name))
    .filter((s) => s !== "")
    .join("\n")
    .trimEnd();
}
