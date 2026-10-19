// `doctor`, as `crates/looper/src/cmd/project.rs` prints it: each gap between the project in the current
// directory, this machine and what the kit needs, one line per gap, with its fix when one is known.
//
// - **The project.** `looper.toml` must load, set no deprecated key, and name prompt files that exist and
//   hold `{status}`. Each `[gates]` program must resolve: a path from the root, or a name on `PATH`.
//   `.claude/settings.json` must exist, be JSON, and hold a `PreToolUse` hook that runs the guard. Each
//   `Bash(...)` or `PowerShell(...)` allow entry must name a program that exists.
// - **The machine.** git, and `core.hooksPath` set to `tools/git-hooks` when the project has that
//   folder. Bun, at the version `package.json`'s `engines.bun` pins. With the `claude` adapter, Git Bash
//   where Claude Code looks for it, `[agent] tools` with Monitor and TaskStop and without PowerShell, and
//   the Claude CLI logged in. With the `rust` stack, rustup and the toolchain `rust-toolchain.toml` pins.
// - It never checks for a looper install. So it reads no shell prefix, and an allow entry that runs
//   `looper` is not judged.
//
// It exits 1 on any gap. A line reads `<problem>. Fix: <fix>`, and `--json` prints `{"problems"}`, each
// `{"problem", "fix"}` split at the line's last `. Fix: `, with `fix` null without one.

import { existsSync, readFileSync, statSync } from "node:fs";
import { dirname, extname, join, resolve } from "node:path";
import { ConfigError, type Config, defaultConfig, deprecatedKeys, gatesSet, loadConfig, promptsNamed } from "../goal/config.ts";
import { runsGuard } from "../sync/sync.ts";
import { type Command, Exit, usage } from "./cli.ts";

/** The bun subcommands a `bun <word>` entry may name besides a script or a file. */
export const BUN_COMMANDS = [
  "add", "build", "create", "exec", "i", "init", "install", "link", "outdated", "patch", "pm", "publish", "remove", "repl", "run",
  "test", "unlink", "update", "upgrade", "x",
];

/** The programs a Bash entry may name that Git Bash supplies whether or not they are on `PATH`. */
export const POSIX_UTILITIES = [
  "awk", "cat", "cd", "chmod", "cp", "cut", "diff", "echo", "env", "false", "find", "grep", "head", "ls", "mkdir", "mv", "printf", "pwd",
  "rm", "sed", "sleep", "sort", "tail", "test", "touch", "tr", "true", "uniq", "wc", "which", "xargs",
];

/** One gap doctor found, and the step that fixes it when doctor knows one. */
export interface Problem {
  problem: string;
  fix: string | null;
}

/** The problem a line of doctor's text states, split at its last `. Fix: `. */
function problemOf(line: string): Problem {
  const at = line.lastIndexOf(". Fix: ");
  if (at < 0) return { problem: line, fix: null };
  return { problem: line.slice(0, at), fix: line.slice(at + ". Fix: ".length) };
}

function isFile(path: string): boolean {
  try {
    return statSync(path).isFile();
  } catch {
    return false;
  }
}

function isDir(path: string): boolean {
  try {
    return statSync(path).isDirectory();
  } catch {
    return false;
  }
}

/** What `name` prints on stdout for `args`; `null` when it is not on `PATH` or exits non-zero. */
function output(name: string, args: string[]): string | null {
  const path = Bun.which(name);
  if (path === null) return null;
  try {
    const ran = Bun.spawnSync([path, ...args], { stdin: "ignore", stdout: "pipe", stderr: "ignore" });
    return ran.exitCode === 0 ? ran.stdout.toString() : null;
  } catch {
    return null;
  }
}

/** The script names in `<root>/package.json`; none when it is missing or not JSON. */
function packageScripts(root: string): string[] {
  try {
    const scripts = JSON.parse(readFileSync(join(root, "package.json"), "utf8")).scripts;
    return scripts !== null && typeof scripts === "object" ? Object.keys(scripts) : [];
  } catch {
    return [];
  }
}

/** Why the allow entry `entry` names a program that does not exist, or `null` when it exists or is not judged. */
function deadEntry(root: string, entry: string): string | null {
  const m = /^(Bash|PowerShell)\((.*)\)$/s.exec(entry);
  if (m === null) return null;
  const [, shell, inner] = m as unknown as [string, string, string];
  let command = inner;
  if (command.endsWith(":*")) command = command.slice(0, -2);
  else if (command.endsWith(" *")) command = command.slice(0, -2);
  const [program, second] = command.split(/\s+/).filter((w) => w !== "");
  if (program === undefined || program === "looper") return null;
  const isPath = (w: string) => w.includes("/") || w.includes("\\");
  if (isPath(program)) {
    const path = join(root, program);
    const found = isFile(path) || (extname(path) === "" && isFile(`${path}.exe`));
    return found ? null : `${program} is not in the project`;
  }
  if (program === "bun") {
    if (second === undefined || second.startsWith("-") || BUN_COMMANDS.includes(second) || packageScripts(root).includes(second)) return null;
    if (isPath(second) || second.includes(".")) return isFile(join(root, second)) ? null : `${second} is not in the project`;
    return `bun has no command ${second}, and package.json no script ${second}`;
  }
  const builtin = shell === "Bash" ? POSIX_UTILITIES.includes(program) : /^[A-Z][^-]*-[^-]*$/.test(program);
  return builtin || Bun.which(program) !== null ? null : `${program} is not on PATH`;
}

/** The Git Bash Claude Code runs, found where it looks, or `null`. */
export function gitBash(): string | null {
  const named = process.env.CLAUDE_CODE_GIT_BASH_PATH;
  const candidates: string[] = [];
  if (named !== undefined && /^(bash|sh)(\.exe)?$/i.test(named.split(/[\\/]/).pop() ?? "")) candidates.push(named);
  for (const key of ["ProgramFiles", "ProgramFiles(x86)"]) {
    const dir = process.env[key];
    if (dir !== undefined) candidates.push(join(dir, "Git", "bin", "bash.exe"));
  }
  const git = Bun.which("git.exe");
  if (git !== null) candidates.push(join(dirname(dirname(git)), "bin", "bash.exe"));
  return candidates.find(isFile) ?? null;
}

/** The gaps in what Claude Code runs a Bash call with: Git Bash, and an `[agent] tools` list of the project's own. */
function shellGaps(cfg: Config, gaps: string[]): void {
  if (gitBash() === null) gaps.push("Git Bash is not installed, so Claude Code has no Bash tool. Fix: winget install Git.Git");
  const tools = cfg.agent.tools;
  const lacks = ["Monitor", "TaskStop"].filter((want) => !tools.includes(want));
  const wrong: string[] = [];
  const fix: string[] = [];
  if (lacks.length > 0) {
    wrong.push(`lacks ${lacks.join(" and ")}`);
    fix.push(`add ${lacks.join(" and ")} to it`);
  }
  if (tools.includes("PowerShell")) {
    wrong.push("holds PowerShell");
    fix.push("take PowerShell out");
  }
  if (wrong.length > 0) gaps.push(`looper.toml: [agent] tools ${wrong.join(" and ")}. Fix: ${fix.join(" and ")}`);
}

/** The `rust` stack's gaps: rustup, and the toolchain `rust-toolchain.toml` pins. */
function rustGaps(root: string, gaps: string[]): void {
  const installed = output("rustup", ["toolchain", "list"]);
  if (installed === null) {
    gaps.push("rustup is not on PATH. Fix: winget install Rustlang.Rustup");
    return;
  }
  let toolchain = "";
  try {
    toolchain = readFileSync(join(root, "rust-toolchain.toml"), "utf8");
  } catch {}
  const line = toolchain.split(/\r?\n/).find((l) => l.split("=")[0]?.trim() === "channel" && l.includes("="));
  if (line === undefined) return;
  const channel = line.slice(line.indexOf("=") + 1).trim().replace(/^"+|"+$/g, "");
  const have = installed.split(/\r?\n/).map((l) => l.trim().split(/\s+/)[0] ?? "");
  if (!have.some((t) => t === channel || t.startsWith(`${channel}-`))) {
    gaps.push(`the rust toolchain ${channel} that rust-toolchain.toml pins is not installed. Fix: rustup toolchain install ${channel}`);
  }
}

/** Bun's gaps: bun on `PATH`, at the version `package.json`'s `engines.bun` pins when it names one. */
function bunGaps(root: string, gaps: string[]): void {
  const have = output("bun", ["--version"])?.trim();
  if (have === undefined) {
    gaps.push(`bun is not on PATH. Fix: powershell -c "irm bun.sh/install.ps1 | iex"`);
    return;
  }
  let pin: unknown;
  try {
    pin = JSON.parse(readFileSync(join(root, "package.json"), "utf8"))?.engines?.bun;
  } catch {}
  if (typeof pin !== "string" || !/^[0-9.]+$/.test(pin)) return;
  const parts = (v: string) => v.split(".").map((n) => Number.parseInt(n, 10) || 0);
  const order = compare(parts(have), parts(pin));
  if (order < 0) gaps.push(`bun is ${have}, and package.json pins ${pin}. Fix: bun upgrade`);
  else if (order > 0) gaps.push(`bun is ${have}, and package.json pins ${pin}. Fix: set engines.bun to ${have} in a commit of its own`);
}

/** `a` against `b` as Rust compares two lists of numbers. */
function compare(a: number[], b: number[]): number {
  for (let i = 0; i < Math.min(a.length, b.length); i++) {
    if (a[i] !== b[i]) return (a[i] as number) - (b[i] as number);
  }
  return a.length - b.length;
}

/** Each gap between this machine and what the project at `root` needs, with its fix. */
function machine(root: string, cfg: Config): string[] {
  const gaps: string[] = [];
  if (output("git", ["--version"]) === null) {
    gaps.push("git is not on PATH. Fix: winget install Git.Git");
  } else if (isDir(join(root, "tools", "git-hooks"))) {
    const set = output("git", ["-C", root, "config", "--get", "core.hooksPath"])?.trim().replaceAll("\\", "/");
    const folder = (path: string) => resolve(root, path).toLowerCase();
    if (set === undefined || folder(set) !== folder("tools/git-hooks")) {
      const now = set === undefined ? "unset" : `\`${set}\``;
      gaps.push(`core.hooksPath is ${now}, so the commit-msg hook never runs. Fix: git config core.hooksPath tools/git-hooks`);
    }
  }
  bunGaps(root, gaps);
  if (cfg.agent.adapter === "claude") {
    shellGaps(cfg, gaps);
    if (output("claude", ["--version"]) === null) {
      gaps.push("claude is not on PATH. Fix: irm https://claude.ai/install.ps1 | iex");
    } else {
      let loggedIn: unknown;
      try {
        loggedIn = JSON.parse(output("claude", ["auth", "status", "--json"]) ?? "").loggedIn;
      } catch {}
      if (loggedIn === false) gaps.push("claude is not logged in. Fix: claude auth login");
      else if (loggedIn !== true) gaps.push("claude cannot report its login through `claude auth status`. Fix: claude update");
    }
  }
  if (cfg.project.stacks.includes("rust")) rustGaps(root, gaps);
  return gaps;
}

/** Every gap of the project at `root` and this machine, as doctor's lines. */
export function doctor(root: string): string[] {
  const problems: string[] = [];
  let cfg: Config;
  try {
    cfg = loadConfig(root);
  } catch (e) {
    problems.push(`looper.toml: ${e instanceof ConfigError ? e.detail : (e as Error).message}`);
    cfg = defaultConfig();
  }
  let toml = "";
  try {
    toml = readFileSync(join(root, "looper.toml"), "utf8");
  } catch {}
  for (const [key, fix] of deprecatedKeys(toml)) problems.push(`looper.toml: ${key} is deprecated. Fix: ${fix}`);
  for (const [which, path] of promptsNamed(cfg.prompts)) {
    let text: string;
    try {
      text = readFileSync(join(root, path), "utf8");
    } catch (e) {
      problems.push(`looper.toml: the ${which} prompt ${path} cannot be read: ${(e as Error).message}`);
      continue;
    }
    if (!text.includes("{status}")) {
      problems.push(`looper.toml: the ${which} prompt ${path} has no {status}, so its session cannot know where its status line goes`);
    }
  }
  for (const [gate, argv] of gatesSet(cfg.gates)) {
    const program = argv[0] as string;
    const found = /[/\\:]/.test(program) ? isFile(join(root, program)) : Bun.which(program) !== null;
    if (!found) problems.push(`looper.toml: [gates] ${gate} runs ${program}, which does not resolve, so the kit's ${gate} fails`);
  }
  const settings = join(root, ".claude", "settings.json");
  if (!existsSync(settings)) {
    problems.push(".claude/settings.json: missing, so `looper guard` never runs. Fix: looper sync");
  } else {
    let json: unknown;
    try {
      json = JSON.parse(readFileSync(settings, "utf8"));
    } catch (e) {
      problems.push(`.claude/settings.json: not JSON: ${(e as Error).message}`);
    }
    if (json !== undefined) {
      if (!runsGuard(json as never)) problems.push(".claude/settings.json: no PreToolUse hook runs `looper guard`. Fix: looper sync");
      const allow = (json as { permissions?: { allow?: unknown } })?.permissions?.allow;
      for (const entry of Array.isArray(allow) ? allow : []) {
        if (typeof entry !== "string") continue;
        const why = deadEntry(root, entry);
        if (why !== null) problems.push(`.claude/settings.json: ${entry} names no command: ${why}`);
      }
    }
  }
  problems.push(...machine(root, cfg));
  return problems;
}

export const doctorCommand: Command = {
  name: "doctor",
  about: "Names each gap between the project, this machine and what the kit needs, with its fix.",
  json: true,
  run: (args) => {
    if (args.length > 0) throw usage(`\`doctor\` takes no argument, and got \`${args[0]}\``);
    const lines = doctor(process.cwd());
    return {
      code: lines.length === 0 ? Exit.ok : Exit.refused,
      doc: { problems: lines.map(problemOf) },
      text: lines.length === 0 ? "doctor: no problems" : lines.join("\n"),
    };
  },
};
