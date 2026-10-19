// `project add [<dir>] [--name N]`, as `crates/looper/src/cmd/project.rs` does it: it writes
// `<dir>/looper.toml` when the directory has none, and registers the directory with looper's service.
//
// - The file is built from the layout `detect` finds, and each guessed value is printed with what the
//   guess rests on. An existing `looper.toml` is never rewritten, and it must load.
// - The name is `--name`, else `[project] name`, else the directory's name.
// - Registering is `POST /api/projects` through `askService`. A directory the service knows already is
//   reported, not refused. With no service it exits 3 with `looper.toml` already written, so add can be
//   run again later.
// - `init` calls `configure` and `register` after its commit.
//
// Its `--json` document is `{"root", "written", "guesses", "changed", "known", "project"}`: each guess is
// `{"key", "value", "why"}`, and `project` is the service's answer. With no service it is the error alone.

import { existsSync, statSync, readFileSync, writeFileSync } from "node:fs";
import { basename, join, resolve } from "node:path";
import { defaultConfig, defaultHarvest, loadConfig, parseConfig } from "../goal/config.ts";
import { Absent, askService, looperHome } from "../jobs/slots.ts";
import { type Command, type Context, Exit, Refusal, usage } from "./cli.ts";

/** One value `detect` chose, and what it rests on. */
export interface Guess {
  key: string;
  value: string;
  why: string;
}

/** What `project add` did, which `--json` prints. */
export interface Added {
  root: string;
  /** Whether add wrote `looper.toml`, which it does only where there is none. */
  written: boolean;
  guesses: Guess[];
  /** `looper.toml` when add wrote it. */
  changed: string[];
  /** Whether the service knew the project already. */
  known: boolean;
  /** The project as the service answers it, or null before it is registered. */
  project: Record<string, unknown> | null;
  /** The name add registers the project by. */
  name: string;
}

/** Whether `name` can name a project in the service: letters, digits, `.`, `_` and `-`, up to 64, no leading `.`. */
export function validProjectName(name: string): boolean {
  return /^[A-Za-z0-9._-]{1,64}$/.test(name) && !name.startsWith(".");
}

function isDir(path: string): boolean {
  return statSync(path, { throwIfNoEntry: false })?.isDirectory() ?? false;
}

function isFile(path: string): boolean {
  return statSync(path, { throwIfNoEntry: false })?.isFile() ?? false;
}

/** The script names in `<root>/package.json`, in its order; none when it is missing or not JSON. */
function packageScripts(root: string): string[] {
  try {
    const scripts = (JSON.parse(readFileSync(join(root, "package.json"), "utf8")) as { scripts?: unknown }).scripts;
    return scripts !== null && typeof scripts === "object" && !Array.isArray(scripts) ? Object.keys(scripts) : [];
  } catch {
    return [];
  }
}

/** The project's verify command and what it rests on, as `detect` describes. */
function verifyCommand(root: string): [string[], string] | null {
  if (isFile(join(root, "scripts/check.cmd"))) return [["cmd", "/c", "scripts\\check.cmd"], "found scripts/check.cmd"];
  const scripts = packageScripts(root);
  const tool = scripts.find((s) => isFile(join(root, `tools/${s}/cmd/verify.ts`)));
  if (tool !== undefined) return [["bun", tool, "verify"], `found tools/${tool}/cmd/verify.ts and the script ${tool}`];
  return scripts.includes("verify") ? [["bun", "run", "verify"], "found the script verify in package.json"] : null;
}

/**
 * The `looper.toml` for the project at `root`, and each value in it that was guessed.
 *
 * Each path is the first candidate that exists, or looper's default. The verify command is
 * `scripts\check.cmd` when there is one, else `bun <tool> verify` for a `package.json` script `<tool>`
 * with a `tools/<tool>/cmd/verify.ts`, else `bun run verify` for a script `verify`, else none.
 */
export function detect(root: string, name: string): [string, Guess[]] {
  const defaults = defaultConfig().project;
  const harvestDefault = defaultHarvest();
  const pick = (key: string, candidates: string[], dir: boolean, fallback: string): Guess => {
    const found = candidates.find((c) => (dir ? isDir : isFile)(join(root, c)));
    return found !== undefined
      ? { key, value: found, why: `found ${found}` }
      : { key, value: fallback, why: "looper's default: none of the usual places exists" };
  };
  const goals = pick("project.goals_dir", ["docs/goals"], true, defaults.goals_dir);
  const playbook = pick("project.playbook", ["docs/agent/playbook.md"], false, defaults.playbook);
  const harvest = pick("project.harvest.file", ["docs/findings.md", "docs/agent/decisions.md", "docs/decisions.md"], false, harvestDefault.file);
  const scratch = pick("project.scratch", [".agent-tmp", "tmp"], true, defaults.scratch);
  const verify = verifyCommand(root);

  const q = (s: string) => JSON.stringify(s);
  const argv = verify === null ? "" : ` [${verify[0].map(q).join(", ")}] `;
  const config = [
    "# Written by `looper project add` from the layout it found. Check every value against",
    "# docs/upgrade-guide.md in the looper repository, Part 1 step 3.",
    "",
    "[project]",
    `name = ${q(name)}`,
    `goals_dir = ${q(goals.value)}`,
    `playbook = ${q(playbook.value)}`,
    `harvest = { file = ${q(harvest.value)}, section = ${q(harvestDefault.section)} }`,
    `scratch = ${q(scratch.value)}`,
    "",
    "[verify]",
    `run = [${argv}]`,
    "",
  ].join("\n");
  const verifyGuess =
    verify === null
      ? { key: "verify.run", value: "nothing", why: "no verify command found: set the one sessions run" }
      : { key: "verify.run", value: verify[0].join(" "), why: verify[1] };
  return [config, [goals, playbook, harvest, scratch, verifyGuess]];
}

/** Writes `<dir>/looper.toml` when it has none, and gives what it did. */
export function configure(dir: string, name: string | undefined): Added {
  const root = resolve(dir);
  if (!isDir(root)) throw new Refusal(`${root} is not a directory.`);
  const file = join(root, "looper.toml");
  const written = !existsSync(file);
  let named = name;
  if (!written) {
    let cfg;
    try {
      cfg = loadConfig(root);
    } catch (e) {
      throw new Refusal((e as Error).message);
    }
    named ??= cfg.project.name !== "" ? cfg.project.name : undefined;
  }
  named ??= basename(root);
  if (!validProjectName(named)) {
    throw new Refusal(`"${named}" cannot name a project: use letters, digits, '.', '_' and '-', up to 64. Pass --name.`);
  }
  let guesses: Guess[] = [];
  if (written) {
    const [config, guessed] = detect(root, named);
    parseConfig("looper.toml", config);
    try {
      writeFileSync(file, config, { flag: "wx" });
    } catch (e) {
      throw new Refusal(`${file}: ${(e as Error).message}`);
    }
    guesses = guessed;
  }
  return { root, written, guesses, changed: written ? ["looper.toml"] : [], known: false, project: null, name: named };
}

/** The path `path` as two spellings of one Windows path compare equal. */
function comparable(path: string): string {
  return path.replace(/^\\\\\?\\/, "").replaceAll("/", "\\").replace(/\\+$/, "").toLowerCase();
}

/** Registers the project `added` names with the service of `home`, unless it knows the project already. Exit 3 when none answers. */
export async function register(home: string | null, added: Added): Promise<void> {
  try {
    const all = await askService(home, "/api/projects");
    const known = Array.isArray(all)
      ? (all as Record<string, unknown>[]).find((p) => comparable(String(p.root)) === comparable(added.root))
      : undefined;
    if (known !== undefined) {
      added.known = true;
      added.project = known;
      return;
    }
    added.project = (await askService(home, "/api/projects", { root: added.root, name: added.name })) as Record<string, unknown>;
  } catch (e) {
    if (e instanceof Absent) throw new Refusal(`looper's server is not running: ${e.message}. Start it with \`looper serve\`.`, Exit.serviceAbsent);
    throw new Refusal((e as Error).message);
  }
}

/** The lines about `looper.toml`. */
export function configText(added: Added): string {
  const file = join(added.root, "looper.toml");
  if (!added.written) return `${file}: kept as it is`;
  return [`${file}: written. Check each guess:`, ...added.guesses.map((g) => `  ${g.key} = ${g.value}  (${g.why})`)].join("\n");
}

/** The line about the registration, or nothing before it. */
export function projectText(added: Added): string {
  const p = added.project;
  if (p === null) return "";
  return `${added.known ? "already" : "added"} project ${JSON.stringify(p.id)} ${String(p.name)} at ${String(p.root)}`;
}

/** The document `--json` prints for `added`. */
export function addedDoc(added: Added): Omit<Added, "name"> {
  const { name: _, ...doc } = added;
  return doc;
}

/** `project add`: writes the config, prints its lines through `ctx`, then registers. */
async function add(dir: string, name: string | undefined, ctx: Context) {
  const added = configure(dir, name);
  if (!ctx.json) ctx.line(configText(added));
  await register(looperHome(), added);
  return { doc: addedDoc(added), text: projectText(added) };
}

export const projectCommand: Command = {
  name: "project",
  about: "`project add [<dir>] [--name N]`: writes looper.toml from the layout found, and registers the project with looper.",
  json: true,
  run: (args, ctx) => {
    const [action, ...rest] = args;
    if (action !== "add") throw usage("`project` takes `add`; the service's own CLI lists and removes projects");
    let dir: string | undefined;
    let name: string | undefined;
    for (let i = 0; i < rest.length; i++) {
      const a = rest[i] ?? "";
      if (a === "--name" || a.startsWith("--name=")) {
        name = a.includes("=") ? a.slice(a.indexOf("=") + 1) : rest[++i];
        if (name === undefined) throw usage("`--name` needs a value");
      } else if (a.startsWith("-") || dir !== undefined) {
        throw usage(`\`project add\` takes a directory and \`--name\`, and got \`${a}\``);
      } else dir = a;
    }
    return add(dir ?? ".", name, ctx);
  },
};
