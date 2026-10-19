// The guard: Claude Code's `PreToolUse` hook. It ports `crates/looper/src/cmd/guard.rs`. `judge` reads
// the hook's JSON and gives the verdict: a call it refuses exits 2 with the reason on stderr, which the
// harness hands back to the agent. A call it lets through exits 0 and prints nothing.
//
// The project is the nearest directory at or above the hook's `cwd` that holds a `looper.toml`. Its
// `[[guard.deny]]` rules apply to every `Bash` and `PowerShell` command after the built-in ones of
// `rules.ts`:
//
// | Tool | Refused |
// |---|---|
// | `Bash`, `PowerShell` | a shell file write: a heredoc, a `>` or `>>` redirect to anything but a null device, another stream or the scratch directory, `tee` or `Tee-Object` to anything but the scratch directory, `sed -i`, `Set-Content`, `Add-Content`, `Out-File`, `[IO.File]::Write…` |
// | `Bash`, `PowerShell` | a sleep or a poll loop: `sleep`, `Start-Sleep`, `timeout /t`, `while true`, `while :`, `while ($true)`, `until`, `Wait-Process`, and a `wait`, `while`, `until` or `.WaitForExit` in a pipeline that finds a process with `pgrep`, `pidof`, `ps`, `Get-Process` or `tasklist`. The reason points at `run_in_background` and Monitor |
// | `Bash`, `PowerShell` | a kill by process name: `pkill`, `killall`, `taskkill /im`, `Stop-Process -Name`, and a `kill`, `Stop-Process` or `taskkill` in a pipeline that finds a process as above. The reason points at `TaskStop` and `looper exec kill` |
// | `Bash`, `PowerShell` | a build or test piped into `head`, `tail`, or `Select-Object` with `-First` or `-Last`, since the harness keeps the whole output. A pipe into `grep` passes |
// | `Bash`, `PowerShell` | an install or update of looper: `looper self-update`, `looper service install` and `looper service update`, and their `cargo run … --` forms |
// | `Bash`, `PowerShell` | a test run wider than `looper verify` (decision 0011): a `cargo test` without `--test`, `--lib`, `--bin`, `--example`, `--bench`, `--no-run` or a filter, and a `bun test` without a file or `-t` |
// | `Read` | a read without `limit` of a text file with more than `WHOLE_READ_LINES` lines from its `offset` on |
// | `Edit`, `Write`, `NotebookEdit` | an edit of a file in a git tree that another live session claims (`claims.ts`) |
// | `Bash`, `PowerShell` | a `git add` or `git commit` that would stage a file another live session claims. `git add -A`, `git add .`, `git add -u` and `git commit -a` stage every change, so any such claim in the tree refuses them |
//
// - The shell rules are judged on the command with the text inside its quotes removed. Every other tool
//   passes.
// - A project whose `[gates] guard` names a command has its own rules in place of the built-in ones,
//   while its `[[guard.deny]]` rules and the claims still apply. That command runs from the project
//   root after the claims, with the hook's JSON on stdin, and within `RULES_TIMEOUT_MS`. Its exit 2
//   refuses the call with its stderr as the reason. Any other exit but 0 lets the call through, and the
//   harness shows why.
// - An edit that passes every rule records its session's claim in the project's `.looper/claims/`.
//   The holder is the hook's `session_id`, the Claude Code process `CLAUDE_PID` names with its start
//   time, `LOOPER_LOOP` and `LOOPER_GOAL`. An interactive session's goal is the tree's live goal. A claim
//   that cannot be written lets the edit through, and the harness shows why.
// - The scratch directory is the project's `[project] scratch`, or `.agent-tmp` when its `looper.toml`
//   does not load. A write target is under it when the target, joined to the hook's `cwd`, is a path
//   inside it.
// - A hook input that does not parse, or a `looper.toml` that does not load, exits 1 with the reason:
//   the harness shows it and lets the call through. A built-in rule still refuses before that.
// - A `Bash` call it does not refuse, in a project, is written to its session's pending entries in the
//   project's `.looper/pending/` (`../jobs/pending.ts`), naming its subagent and tool call. The shell
//   wrapper takes the entry for its job's record. An entry that cannot be written changes no verdict.
//
// # The bounds it keeps
//
// - The hook's input is read up to `MAX_INPUT` bytes; a longer one does not parse.
// - The project command's stderr is kept up to `MAX_SAID` bytes.
//
// # Known gaps
//
// - A rule's command inside quotes is not seen, as in `bash -c "…"` or `powershell -Command '…'`.
// - A `git add` path built by a shell expansion, such as `$f`, is judged as the text it is written as.
// - A `git add` path is read from the hook's `cwd` or `git -C`, so a `cd` earlier in the command is not
//   followed.

import { existsSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { claim, conflict, dir as claimsDir, type Holder, machine } from "../claims/claims.ts";
import { checkEnv, type Config, defaultConfig, loadConfig, ruleRegex } from "../goal/config.ts";
import { liveGoal, loadChain } from "../goal/model.ts";
import { pendingDir, push } from "../jobs/pending.ts";
import { killTree, started } from "../os/index.ts";
import { SEGMENT, inScratch, shell, stem, wholeRead } from "./rules.ts";

/** The most bytes of hook input read from stdin. */
export const MAX_INPUT = 4 << 20;

/** How long the project's `[gates] guard` command may run before its tree is killed: 30 seconds. */
export const RULES_TIMEOUT_MS = 30_000;

/** The most bytes of the project command's stderr kept for its reason. */
export const MAX_SAID = 64 << 10;

/** What the hook makes of a call: a pass, a refusal for `deny`, or a pass the harness shows `broken` for. */
export type Verdict = { pass: true } | { deny: string } | { broken: string };

const PASS: Verdict = { pass: true };

/** The exit code and the stderr line of `v`. */
export function outcome(v: Verdict): { code: number; said: string } {
  if ("deny" in v) return { code: 2, said: `looper guard: ${v.deny}` };
  if ("broken" in v) return { code: 1, said: `looper guard: ${v.broken}` };
  return { code: 0, said: "" };
}

/** The hook's input, as far as the guard reads it. */
interface Hook {
  session_id?: unknown;
  agent_id?: unknown;
  tool_use_id?: unknown;
  cwd?: unknown;
  tool_name?: unknown;
  tool_input?: Record<string, unknown>;
}

/** The variables of the session's environment that the guard reads. */
export type Env = Record<string, string | undefined>;

function str(v: unknown): string | undefined {
  return typeof v === "string" ? v : undefined;
}

/** Judges the hook input `input`, with the session's environment `env`. */
export async function judge(input: string, env: Env = process.env): Promise<Verdict> {
  let hook: Hook;
  try {
    const parsed: unknown = JSON.parse(input);
    if (typeof parsed !== "object" || parsed === null || Array.isArray(parsed)) throw new Error("it is not an object");
    hook = parsed as Hook;
  } catch (e) {
    return { broken: `the hook's input is not JSON: ${(e as Error).message}` };
  }
  let verdict = builtIn(hook);
  if ("pass" in verdict) verdict = claims(hook, env);
  if ("pass" in verdict) verdict = await rules(hook, input);
  if (!("deny" in verdict)) pend(hook);
  return verdict;
}

/** Writes a Bash call to its session's pending entries, which the wrapper takes for its record. A failure is silent. */
function pend(hook: Hook): void {
  const session = str(hook.session_id);
  const cmd = str(hook.tool_input?.command);
  if (str(hook.tool_name) !== "Bash" || session === undefined || cmd === undefined) return;
  const root = projectRoot(hookCwd(hook));
  if (!existsSync(join(root, "looper.toml"))) return;
  const at = Date.now();
  const entry = { session_id: session, agent_id: str(hook.agent_id) ?? null, tool_use_id: str(hook.tool_use_id) ?? null, command: cmd, at };
  try {
    push(pendingDir(root), entry, at);
  } catch {}
}

/** The hook's `cwd`, or this process's. */
function hookCwd(hook: Hook): string {
  return str(hook.cwd) ?? process.cwd();
}

/** The nearest directory at or above `cwd` holding a `looper.toml`, or `cwd` when none does. */
function projectRoot(cwd: string): string {
  for (let d = resolve(cwd); ; d = dirname(d)) {
    if (existsSync(join(d, "looper.toml"))) return d;
    if (dirname(d) === d) return cwd;
  }
}

/** The project's config, or the error that keeps it from loading. */
function load(root: string): Config | Error {
  try {
    return loadConfig(root);
  } catch (e) {
    return e as Error;
  }
}

/** The command of a shell tool's call. */
function command(hook: Hook): string | undefined {
  const tool = str(hook.tool_name);
  return tool === "Bash" || tool === "PowerShell" ? str(hook.tool_input?.command) : undefined;
}

/** The built-in rules, unless the project has its own, then the project's `[[guard.deny]]`. */
function builtIn(hook: Hook): Verdict {
  const cwd = hookCwd(hook);
  const root = projectRoot(cwd);
  const cfg = load(root);
  const scratch = join(root, cfg instanceof Error ? defaultConfig().project.scratch : cfg.project.scratch);
  const ownRules = !(cfg instanceof Error) && cfg.gates.guard.length > 0;
  if (!ownRules) {
    const reason = rule(hook, cwd, scratch);
    if (reason !== undefined) return { deny: reason };
  }
  if (cfg instanceof Error) return { broken: cfg.message };
  const cmd = command(hook);
  const reason = cmd === undefined ? undefined : projectRule(cfg, cmd);
  return reason === undefined ? PASS : { deny: reason };
}

/** The reason a built-in rule refuses the call made in `cwd`, if one does. A shell write into `scratch` passes. */
function rule(hook: Hook, cwd: string, scratch: string): string | undefined {
  const cmd = command(hook);
  if (cmd !== undefined) return shell(cmd, (target) => inScratch(target, cwd, scratch));
  const input = hook.tool_input ?? {};
  const file = str(input.file_path);
  if (hook.tool_name !== "Read" || file === undefined || (input.limit !== undefined && input.limit !== null)) return undefined;
  const offset = typeof input.offset === "number" && Number.isInteger(input.offset) && input.offset >= 0 ? input.offset : 1;
  return wholeRead(file, cwd, offset);
}

/** The reason of the first `[[guard.deny]]` rule that matches `cmd` or one of its parts. */
function projectRule(cfg: Config, cmd: string): string | undefined {
  const parts = [cmd.trim(), ...cmd.split(SEGMENT).map((p) => p.trim())];
  for (const r of cfg.guard.deny) {
    const re = ruleRegex(r.pattern);
    if (parts.some((p) => re.test(p))) return r.reason;
  }
  return undefined;
}

/** The tools whose call edits a file, with the input key that names it. */
const EDITS: Record<string, string> = { Edit: "file_path", Write: "file_path", NotebookEdit: "notebook_path" };

/** `path` made absolute, with `/` between its parts and none at its end. */
function slashed(path: string): string {
  return resolve(path).replaceAll("\\", "/").replace(/\/+$/, "");
}

/** `path` from `tree`, when it lies inside it. Windows paths ignore case. */
function inside(tree: string, path: string): string | undefined {
  const head = path.slice(0, tree.length);
  const same = process.platform === "win32" ? head.toLowerCase() === tree.toLowerCase() : head === tree;
  const rest = path.slice(tree.length);
  return same && rest.startsWith("/") && rest.length > 1 ? rest.slice(1) : undefined;
}

/** Judges an edit by the claims of the project's sessions, and records its own. A shell command is judged by `stage`. Any other call, and an edit of a file outside a git tree, passes. */
function claims(hook: Hook, env: Env): Verdict {
  const cmd = command(hook);
  if (cmd !== undefined) return stage(hook, cmd);
  const key = EDITS[str(hook.tool_name) ?? ""];
  const session = str(hook.session_id);
  const file = key === undefined ? undefined : str(hook.tool_input?.[key]);
  if (session === undefined || file === undefined) return PASS;
  const cwd = hookCwd(hook);
  const root = projectRoot(cwd);
  if (!existsSync(join(root, ".git"))) return PASS;
  const tree = slashed(root);
  const path = inside(tree, slashed(resolve(cwd, file)));
  if (path === undefined) return PASS;
  const cfg = load(root);
  const v = (name: string) => (env[name] ? env[name] : null);
  const pidText = v("CLAUDE_PID");
  const pid = pidText !== null && /^\d+$/.test(pidText) ? Number(pidText) : null;
  const me: Holder = {
    session_id: session,
    pid,
    started: pid === null ? null : started(pid),
    loop: v("LOOPER_LOOP"),
    goal: v("LOOPER_GOAL"),
  };
  const goal = () => {
    if (cfg instanceof Error) return null;
    try {
      const goals = join(root, cfg.project.goals_dir);
      return liveGoal(root, goals, loadChain(goals)) ?? null;
    } catch {
      return null;
    }
  };
  const edit = { tree, path, agent_id: str(hook.agent_id) ?? null, now: Date.now() };
  try {
    const refused = claim(claimsDir(root), me, edit, machine, goal);
    return refused === null ? PASS : { deny: refused };
  } catch (e) {
    return { broken: `cannot record the claim on ${path}: ${(e as Error).message}` };
  }
}

/** Refuses a `git add` or `git commit` in `cmd` that would stage a file another live session claims. Any other command, and one outside a git tree, passes. */
function stage(hook: Hook, cmd: string): Verdict {
  const session = str(hook.session_id);
  if (session === undefined) return PASS;
  const cwd = hookCwd(hook);
  const root = projectRoot(cwd);
  if (!existsSync(join(root, ".git"))) return PASS;
  const tree = slashed(root);
  const specs = commands(cmd).flatMap((w) => staged(w, cwd, tree));
  if (specs.length === 0) return PASS;
  const dir = claimsDir(root);
  try {
    const held = conflict(dir, session, tree, (path) => specs.some((s) => covers(s, path)), machine, Date.now());
    return held === null ? PASS : { deny: `${held}, and this command would stage it. Stage only your own files, by name.` };
  } catch (e) {
    return { broken: `cannot read the claims in ${dir}: ${(e as Error).message}` };
  }
}

/** What a git command stages, from the tree's root: every changed file, or the file or folder at a path. */
type Spec = "all" | { under: string };

/** Whether `spec` stages the file at `path`. Windows paths ignore case. */
function covers(spec: Spec, path: string): boolean {
  if (spec === "all") return true;
  const fold = (p: string) => (process.platform === "win32" ? p.toLowerCase() : p);
  const [s, p] = [fold(spec.under), fold(path)];
  return p === s || p.startsWith(`${s}/`);
}

/** The flags that stage every change, and those that take the next word as their value, of each staging subcommand. */
const STAGING: Record<string, { allShort: string; allLong: string[]; valuedShort: string; valuedLong: string[] }> = {
  add: { allShort: "Au", allLong: ["--all", "--update", "--pathspec-from-file"], valuedShort: "", valuedLong: ["--chmod"] },
  commit: {
    allShort: "a",
    allLong: ["--all", "--pathspec-from-file"],
    valuedShort: "mFCct",
    valuedLong: [
      "--message",
      "--file",
      "--author",
      "--date",
      "--reuse-message",
      "--reedit-message",
      "--fixup",
      "--squash",
      "--template",
      "--cleanup",
      "--trailer",
    ],
  },
};

/**
 * What the one command `words`, run in `cwd`, stages in `tree`: a `git add` its paths, or every change
 * with `-A`, `-u` or no paths, and a `git commit` its paths, or every change with `-a`. Any other command
 * stages nothing.
 */
function staged(words: string[], cwd: string, tree: string): Spec[] {
  let i = 0;
  while (i < words.length && words[i]?.includes("=") && !words[i]?.startsWith("-")) i++;
  if (i >= words.length || stem(words[i] ?? "") !== "git") return [];
  i++;
  let dir = cwd;
  let sub: string | undefined;
  while (i < words.length && sub === undefined) {
    const w = words[i++] ?? "";
    if (w === "-C") dir = resolve(dir, words[i++] ?? "");
    else if (w === "-c") i++;
    else if (!w.startsWith("-")) sub = w;
  }
  const flags = sub === undefined ? undefined : STAGING[sub];
  if (flags === undefined) return [];
  const specs: Spec[] = [];
  let named = false;
  let pathsOnly = false;
  while (i < words.length) {
    const a = words[i++] ?? "";
    if (!pathsOnly && a === "--") {
      pathsOnly = true;
    } else if (!pathsOnly && a.startsWith("--")) {
      const name = a.split("=")[0] ?? "";
      named ||= flags.allLong.includes(name);
      if (!a.includes("=") && flags.valuedLong.includes(name)) i++;
    } else if (!pathsOnly && a.length > 1 && a.startsWith("-")) {
      const chars = [...a.slice(1)];
      for (const [at, ch] of chars.entries()) {
        named ||= flags.allShort.includes(ch);
        if (flags.valuedShort.includes(ch)) {
          if (at === chars.length - 1) i++;
          break;
        }
      }
    } else {
      const s = spec(tree, dir, a);
      if (s !== undefined) specs.push(s);
    }
  }
  // A flag that stages every change is limited by the paths named beside it.
  if (specs.length === 0 && named) specs.push("all");
  return specs;
}

/** The path `word` names from `cwd`, as a spec in `tree`. A pathspec with magic or a glob stages what cannot be told here, so it is every change. A path outside the tree is undefined. */
function spec(tree: string, cwd: string, word: string): Spec | undefined {
  if (word.startsWith(":") || /[*?[]/.test(word)) return "all";
  const path = slashed(resolve(cwd, word));
  if (path.length === tree.length && covers({ under: tree }, path)) return "all";
  const rest = inside(tree, path);
  return rest === undefined ? undefined : { under: rest };
}

/** The commands of `cmd`, each as its words with their quotes taken out. A command ends at an unquoted `;`, `&`, `|`, `(`, `)` or line break. A backslash outside single quotes escapes the next character. */
export function commands(cmd: string): string[][] {
  const out: string[][] = [[]];
  let word: string | undefined;
  const end = () => {
    if (word !== undefined) out[out.length - 1]?.push(word);
    word = undefined;
  };
  for (let i = 0; i < cmd.length; i++) {
    const c = cmd[i] ?? "";
    if (c === "'") {
      const close = cmd.indexOf("'", i + 1);
      const stop = close < 0 ? cmd.length : close;
      word = (word ?? "") + cmd.slice(i + 1, stop);
      i = stop;
    } else if (c === '"') {
      word ??= "";
      for (i++; i < cmd.length && cmd[i] !== '"'; i++) {
        if (cmd[i] === "\\") i++;
        word += cmd[i] ?? "";
      }
    } else if (c === "\\") {
      word = (word ?? "") + (cmd[++i] ?? "");
    } else if (";&|()\n".includes(c)) {
      end();
      out.push([]);
    } else if (/\s/.test(c)) {
      end();
    } else {
      word = (word ?? "") + c;
    }
  }
  end();
  return out.filter((w) => w.length > 0);
}

/** Reads `stream` to its end, keeping its first `cap` bytes. */
async function bounded(stream: ReadableStream<Uint8Array>, cap: number): Promise<string> {
  const parts: Uint8Array[] = [];
  let size = 0;
  for await (const chunk of stream) {
    if (size >= cap) continue;
    const take = chunk.subarray(0, cap - size);
    parts.push(take);
    size += take.length;
  }
  return Buffer.concat(parts).toString();
}

/**
 * Runs the project's `[gates] guard` command from its root with the hook's `input` on stdin. Exit 2
 * refuses with its stderr as the reason, and any exit but 0 is broken. A project that names no command,
 * or whose `looper.toml` does not load, passes, since `builtIn` has reported that.
 */
async function rules(hook: Hook, input: string): Promise<Verdict> {
  const root = projectRoot(hookCwd(hook));
  const cfg = load(root);
  if (cfg instanceof Error || cfg.gates.guard.length === 0) return PASS;
  const argv = cfg.gates.guard;
  const named = argv.join(" ");
  let proc: Bun.Subprocess<Blob, "ignore", "pipe">;
  try {
    proc = Bun.spawn(argv, { cwd: root, env: checkEnv(cfg.checks), stdin: new Blob([input]), stdout: "ignore", stderr: "pipe" });
  } catch (e) {
    return { broken: `[gates] guard \`${named}\`: ${(e as Error).message}` };
  }
  let timedOut = false;
  const timer = setTimeout(() => {
    timedOut = true;
    try {
      killTree(proc.pid);
    } catch {
      proc.kill();
    }
  }, RULES_TIMEOUT_MS);
  const [text, code] = await Promise.all([bounded(proc.stderr, MAX_SAID), proc.exited]);
  clearTimeout(timer);
  const said = text.trim();
  if (code === 0 && !timedOut) return PASS;
  if (code === 2 && !timedOut) return { deny: said === "" ? `[gates] guard \`${named}\` refused the call` : said };
  return { broken: `[gates] guard \`${named}\` exited ${code}${timedOut ? " (timed out)" : ""}: ${said}` };
}
