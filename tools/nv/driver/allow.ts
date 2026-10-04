// The allowlist in `.claude/settings.json`, grown by the loop itself. A session runs under
// `--permission-mode auto`: a shell call that matches an allow rule runs at once, and any other call waits
// on the harness's classifier, which approves it or blocks it. This module watches each session's calls,
// and the driver turns what it saw into rules between sessions.
//
// Two kinds of rule are added without a person:
//
// - every `bun nv` command `main.ts` dispatches, except `NV_NEVER`, as soon as it exists;
// - a command prefix the classifier approved in `PROMOTE_AFTER` sessions and never blocked.
//
// A refused call is never added: the classifier blocked it, or the guard hook denied it. It goes to the
// ledger, and a person adds it by hand or leaves it out. A
// prefix is two plain words (three for `bun nv` and `gh`), or a path to a binary under `target/`; a call
// whose first word is in `NEVER` (deletes, interpreters, shells, network, process control, history
// rewrites) never becomes a rule, because a prefix rule would approve every later call that starts the
// same way, whatever follows it. A read-only command is not counted either: auto mode runs it unasked.
//
// `.loop/allow-seen.json` keeps the count from one session to the next. A rule is committed by the driver
// alone, by path, and never while `.claude/settings.json` has an uncommitted edit of somebody else's.

import { existsSync, mkdirSync, readFileSync, writeFileSync, rmSync } from "node:fs";
import { dirname, join } from "node:path";
import { ROOT } from "../lib/paths.ts";
import { run } from "../lib/proc.ts";

export const SETTINGS = ".claude/settings.json";
export const SEEN = ".loop/allow-seen.json";
/** Sessions in which the classifier must have approved a prefix before it becomes a rule. */
export const PROMOTE_AFTER = 3;
/** `bun nv` commands that are never allowlisted: `bg` runs any command, so the classifier reviews each call. */
export const NV_NEVER = new Set(["bg"]);

const SHELLS = new Set(["Bash", "PowerShell"]);

/** First words a prefix rule is never made from. Compared lower case, without `.exe`. */
const NEVER = new Set([
  "rm", "rmdir", "del", "erase", "rd", "remove-item", "ri", "clear-content",
  "mv", "cp", "ln", "dd", "tee", "truncate", "chmod", "chown", "icacls", "takeown",
  "move-item", "copy-item", "rename-item", "new-item", "set-content", "add-content", "out-file", "set-itemproperty",
  "python", "python3", "py", "node", "deno", "npx", "npm", "pnpm", "yarn", "rustc", "perl", "ruby",
  "bash", "sh", "zsh", "pwsh", "powershell", "cmd", "wsl", "env", "sudo", "eval", "exec", "source",
  "invoke-expression", "iex", "start-process", "stop-process", "kill", "taskkill", "wait-process",
  "curl", "wget", "invoke-webrequest", "iwr", "invoke-restmethod", "irm", "ssh", "scp", "gh", "docker",
  "reg", "sc", "net", "netsh", "shutdown", "format", "timeout", "time", "nohup", "xargs", "start-sleep",
]);
/** `git` subcommands that rewrite or discard work, or reach a remote. */
const GIT_NEVER = new Set(["push", "reset", "checkout", "clean", "rebase", "filter-branch", "gc", "reflog", "branch", "switch", "worktree", "config", "remote", "stash", "restore", "fetch", "pull", "update-ref", "tag"]);
const CARGO_NEVER = new Set(["install", "uninstall", "publish", "login", "logout", "owner", "yank"]);
/** Read-only commands: auto mode runs them without the classifier, so they are not counted. */
const READ_ONLY = new Set([
  "cd", "set-location", "echo", "printf", "true", "false", "test", "pwd", "which", "type", "sleep", "export",
  "grep", "egrep", "rg", "sed", "awk", "cat", "head", "tail", "ls", "dir", "wc", "find", "cut", "sort", "uniq",
  "diff", "file", "date", "df", "du", "stat", "basename", "dirname", "tr", "nl", "column", "jq",
  "get-content", "gc", "get-childitem", "gci", "select-string", "sls", "test-path", "measure-object",
  "select-object", "where-object", "foreach-object", "sort-object", "format-table", "format-list",
  "out-string", "write-output", "write-host", "get-date", "get-item", "get-process", "get-ciminstance", "get-command", "resolve-path", "join-path", "split-path",
]);
/** Shell keywords: a segment that starts with one is a statement, not a command. */
const KEYWORDS = new Set(["for", "foreach", "do", "done", "while", "until", "if", "then", "else", "elif", "fi", "case", "esac", "function", "select", "try", "catch", "finally", "return", "break", "continue", "exit"]);
/** Shell syntax that makes a word something other than a plain argument. */
const UNPLAIN = /[$'"`<>(){}*?;&|=\\[\]!#~%@]/;

/**
 * The rule prefix one segment of a shell command would be allowed by, or null when it is read-only, shell
 * syntax, or never made a rule.
 */
export function prefixOf(segment: string): string | null {
  const words = segment.trim().split(/\s+/).filter((w) => w !== "");
  const first = words[0];
  if (first === undefined) return null;
  const name = first.toLowerCase().replace(/\.exe$/, "");
  if (READ_ONLY.has(name) || NEVER.has(name) || KEYWORDS.has(name)) return null;
  // A binary the build made: the path alone is the prefix, as the hand-written rules have it.
  if (/^(\.[\\/])?target[\\/]/.test(first)) return /[$'"`*?]/.test(first) || first.includes("..") ? null : first;
  if (UNPLAIN.test(first) || first.startsWith("-") || first.includes("/")) return null;
  const depth = name === "gh" || (name === "bun" && words[1] === "nv") ? 3 : 2;
  const taken = words.slice(0, depth);
  if (taken.length < depth || taken.some((w) => w.startsWith("-") || UNPLAIN.test(w))) return null;
  const sub = taken[1]!;
  if (name === "git" && GIT_NEVER.has(sub)) return null;
  if (name === "cargo" && CARGO_NEVER.has(sub)) return null;
  if (name === "bun" && sub !== "nv" && sub !== "test") return null;
  if (name === "bun" && sub === "nv" && NV_NEVER.has(taken[2]!)) return null;
  return taken.join(" ");
}

/**
 * A command line cut at its unquoted `&&`, `||`, `;`, `|` and line breaks. A heredoc's body is not
 * commands, so a line that opens one ends the scan.
 */
export function segments(command: string): string[] {
  const out: string[] = [];
  let cur = "";
  let quote = "";
  for (let i = 0; i < command.length; i++) {
    const ch = command[i]!;
    if (quote) {
      if (ch === "\\" && quote === '"') {
        cur += ch + (command[++i] ?? "");
        continue;
      }
      if (ch === quote) quote = "";
      cur += ch;
      continue;
    }
    if (ch === "'" || ch === '"') {
      quote = ch;
      cur += ch;
      continue;
    }
    if (ch === "\n" && cur.includes("<<")) {
      out.push(cur);
      return out;
    }
    if (ch === ";" || ch === "\n" || ch === "|" || (ch === "&" && command[i + 1] === "&")) {
      if ((ch === "|" || ch === "&") && command[i + 1] === ch) i++;
      out.push(cur);
      cur = "";
      continue;
    }
    cur += ch;
  }
  out.push(cur);
  return out;
}

/** Every rule prefix a whole command line is made of; a segment that yields none is left out. */
export function prefixesOf(command: string): string[] {
  const out = new Set<string>();
  for (const segment of segments(command)) {
    const p = prefixOf(segment);
    if (p !== null) out.add(p);
  }
  return [...out];
}

/** The rule text for a prefix: `Bash(git commit:*)`. */
export const ruleText = (tool: string, prefix: string) => `${tool}(${prefix}:*)`;

/** Whether an allow rule already approves a segment that starts with `prefix`. */
export function covered(rules: string[], tool: string, prefix: string): boolean {
  for (const r of rules) {
    const m = /^(\w+)\((.*):\*\)$/.exec(r);
    if (!m || m[1] !== tool) continue;
    const p = m[2]!;
    if (prefix === p || prefix.startsWith(`${p} `)) return true;
  }
  return false;
}

/** What one session did with its shell calls: the prefixes that ran, and the ones that were blocked. */
export interface Outcome {
  approved: string[];
  blocked: { key: string; command: string }[];
}

const keyOf = (tool: string, prefix: string) => `${tool}\t${prefix}`;

/**
 * Watches a session's stream-json events. A shell call counts as approved once its result comes back
 * without a permission error, and as blocked when the harness lists it in `permission_denials` or its
 * result reads as a refusal. A call whose result never arrives counts as neither.
 */
export class AllowWatch {
  private calls = new Map<string, { tool: string; command: string }>();
  private denied = new Set<string>();
  private ran = new Set<string>();

  note(e: Record<string, unknown>): void {
    const content = (e.message as { content?: unknown } | undefined)?.content;
    if (Array.isArray(content)) {
      for (const c of content as Record<string, unknown>[]) {
        if (c.type === "tool_use" && SHELLS.has(String(c.name))) {
          const command = String((c.input as { command?: unknown } | undefined)?.command ?? "");
          this.calls.set(String(c.id), { tool: String(c.name), command });
        } else if (c.type === "tool_result") {
          const id = String(c.tool_use_id);
          if (!this.calls.has(id)) continue;
          if (c.is_error === true && refusal(c.content)) this.denied.add(id);
          else this.ran.add(id);
        }
      }
    }
    if (e.type === "result" && Array.isArray(e.permission_denials)) {
      for (const d of e.permission_denials as Record<string, unknown>[]) this.denied.add(String(d.tool_use_id));
    }
  }

  outcome(): Outcome {
    const approved = new Set<string>();
    const blocked = new Map<string, string>();
    for (const [id, call] of this.calls) {
      const keys = prefixesOf(call.command).map((p) => keyOf(call.tool, p));
      if (this.denied.has(id)) for (const k of keys) blocked.set(k, call.command);
      else if (this.ran.has(id)) for (const k of keys) approved.add(k);
    }
    for (const k of blocked.keys()) approved.delete(k);
    return { approved: [...approved], blocked: [...blocked].map(([key, command]) => ({ key, command })) };
  }
}

/**
 * Whether a tool result is the harness refusing the call, not the command failing. Narrow on purpose: a
 * command's own "permission denied" is a failure that ran, and `permission_denials` is the authority.
 */
function refusal(content: unknown): boolean {
  const text = typeof content === "string" ? content : JSON.stringify(content ?? "");
  return /auto mode|classifier|hook error/i.test(text);
}

/** Per prefix key: the sessions that ran it, and whether one was blocked. */
export type Seen = Record<string, { sessions: string[]; blocked: boolean }>;

/** Folds one session's outcome into the counts, and returns the keys newly blocked. */
export function record(seen: Seen, session: string, o: Outcome): string[] {
  const fresh: string[] = [];
  for (const k of o.approved) {
    const s = (seen[k] ??= { sessions: [], blocked: false });
    if (!s.sessions.includes(session)) s.sessions.push(session);
  }
  for (const { key } of o.blocked) {
    const s = (seen[key] ??= { sessions: [], blocked: false });
    if (!s.blocked) fresh.push(key);
    s.blocked = true;
  }
  return fresh;
}

/** The rules the counts and the dispatch table owe that `rules` does not hold yet. */
export function owed(seen: Seen, rules: string[], nvCommands: string[]): string[] {
  const out: string[] = [];
  const want = (tool: string, prefix: string) => {
    if (!covered(rules, tool, prefix) && !covered(out, tool, prefix)) out.push(ruleText(tool, prefix));
  };
  for (const name of nvCommands) {
    if (NV_NEVER.has(name)) continue;
    for (const tool of SHELLS) want(tool, `bun nv ${name}`);
  }
  for (const [k, s] of Object.entries(seen)) {
    if (s.blocked || s.sessions.length < PROMOTE_AFTER) continue;
    const [tool, prefix] = k.split("\t") as [string, string];
    want(tool, prefix);
  }
  return out;
}

/** The settings text with `rules` appended to `permissions.allow`, keeping the file's layout. */
export function withRules(text: string, rules: string[]): string {
  if (rules.length === 0) return text;
  const m = /("allow"\s*:\s*\[)([\s\S]*?)(\n(\s*)\])/.exec(text);
  if (!m) throw new Error(`${SETTINGS} has no permissions.allow array`);
  const indent = `${m[4]}  `;
  const body = m[2]!.trimEnd();
  const lines = rules.map((r) => `${indent}${JSON.stringify(r)}`).join(",\n");
  const joined = body.trim() === "" ? `\n${lines}` : `${body},\n${lines}`;
  const out = text.slice(0, m.index) + m[1] + joined + m[3] + text.slice(m.index + m[0].length);
  JSON.parse(out);
  return out;
}

/** The commands `main.ts` dispatches, read from its table without loading it. */
export function nvCommands(root = ROOT): string[] {
  const text = readFileSync(join(root, "tools/nv/main.ts"), "utf8");
  return [...text.matchAll(/^\s+"?([a-z][a-z0-9-]*)"?: \(\) => import\("\.\/cmd\//gm)].map((m) => m[1]!);
}

export function loadSeen(root = ROOT): Seen {
  const path = join(root, SEEN);
  if (!existsSync(path)) return {};
  try {
    return JSON.parse(readFileSync(path, "utf8")) as Seen;
  } catch {
    return {};
  }
}

export function saveSeen(seen: Seen, root = ROOT): void {
  const path = join(root, SEEN);
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, `${JSON.stringify(seen, null, 1)}\n`);
}

/**
 * Adds and commits the rules the tree owes. Returns the rules added, or a reason nothing was: an
 * uncommitted edit to the settings file is somebody's work, so the rules wait for the next session.
 */
export async function settle(seen: Seen, root = ROOT): Promise<{ added: string[] } | { skipped: string }> {
  const path = join(root, SETTINGS);
  const text = readFileSync(path, "utf8");
  const rules = ((JSON.parse(text) as { permissions?: { allow?: string[] } }).permissions?.allow ?? []).slice();
  const add = owed(seen, rules, nvCommands(root));
  if (add.length === 0) return { added: [] };
  const git = (args: string[]) => run(["git", ...args], { cwd: root, timeoutMs: 120_000 });
  if ((await git(["diff", "--quiet", "HEAD", "--", SETTINGS])).code !== 0) return { skipped: `${SETTINGS} has an uncommitted edit` };
  writeFileSync(path, withRules(text, add));
  const msg = join(root, ".agent-tmp", "allow-msg.txt");
  mkdirSync(dirname(msg), { recursive: true });
  const subject = add.length === 1 ? `the allowlist takes \`${add[0]}\`` : `the allowlist takes ${add.length} rules, \`${add[0]}\` first`;
  writeFileSync(msg, `build(loop): ${subject}\n\nAdded by the loop's driver: ${add.map((r) => `\`${r}\``).join(", ")}.\n`);
  const done = await git(["commit", "-q", "-F", msg, "--only", "--", SETTINGS]);
  rmSync(msg, { force: true });
  if (done.code !== 0) {
    writeFileSync(path, text);
    return { skipped: `the commit failed -- ${(done.stderr || done.stdout).trim().split("\n")[0]}` };
  }
  return { added: add };
}
