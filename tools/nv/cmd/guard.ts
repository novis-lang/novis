// `bun nv guard`: the one `PreToolUse` hook, wired in `.claude/settings.json` for `Read` and `Bash`.
// It reads the harness's event from standard input and denies a call that does by hand
// what an `nv` command or an AGENTS.md rule already covers. A denial's reason starts `guard: <rule>:`
// and names the call to make instead. A hook's denial holds in every permission mode, including
// the `auto` mode the loop runs its sessions in and the allowlist they match, so this is where a habit is
// stopped rather than a prompt.
//
// Each rule in `RULES` denies one raw pattern, and each is narrow: a rule that denies a legitimate
// command costs every session a round trip. The guard fails open. A payload it cannot read, a tool it
// does not know and a command it cannot parse are all allowed. `tools/nv/test/guard.test.ts` holds one
// command each rule denies and one near miss it allows, and a new rule lands with both. A tool that
// replaces a raw habit adds its rule here in the same commit.
//
// A shell command is split into segments at `;`, `&&`, `||`, `|` and newlines, and each segment into
// words with its quotes removed. This is not a shell: it is enough to find a command's name, its flags
// and its file arguments, and a heredoc's body is skipped so its text is never read as commands.
//
// `bun nv guard --check` says whether the hook is wired for both tools, and whether every `nv`
// command a rule names is registered. It exits 0 when both hold and 1 when one does not. The hook
// itself always exits 0.

import { existsSync, readFileSync, statSync } from "node:fs";
import { isAbsolute, join, relative, resolve } from "node:path";
import { rel as relOf, ROOT } from "../lib/paths.ts";
import { run as runProc } from "../lib/proc.ts";
import { DEFAULT_MAX_LINES } from "./peek.ts";

export const summary = "the PreToolUse hook that denies a raw command an nv command replaces: nv guard [--check]";

export type Tool = "Read" | "Bash";
export const TOOLS: Tool[] = ["Read", "Bash"];
/** The hook command `.claude/settings.json` runs. */
export const HOOK = "bun nv guard";

export interface Call {
  tool: Tool;
  input: Record<string, unknown>;
  cwd: string;
}

export interface Word {
  text: string;
  /** A redirect or heredoc operator, such as `>`, `2>&` or `<<`. */
  op?: boolean;
}

export interface Segment {
  words: Word[];
  /** Its output goes into the next segment through a `|`, so it does not reach the context whole. */
  piped: boolean;
}

export interface Parsed {
  text: string;
  segments: Segment[];
}

export interface Rule {
  name: string;
  tools: Tool[];
  /** The `nv` commands the reason names, which `--check` holds to the registry. */
  commands: string[];
  /** The reason after `guard: <name>: `, or null when the call is allowed. */
  deny(call: Call, parsed: Parsed | null): string | null;
}

/** Splits a Bash command into segments of words, or returns null when a quote never closes. */
export function parse(text: string): Parsed | null {
  const segments: Segment[] = [];
  let words: Word[] = [];
  let cur = "";
  let open = false;
  let delimNext: boolean | null = null;
  const heredocs: { delim: string; strip: boolean }[] = [];
  const flush = () => {
    if (!open) return;
    words.push({ text: cur });
    if (delimNext !== null) heredocs.push({ delim: cur, strip: delimNext });
    delimNext = null;
    cur = "";
    open = false;
  };
  const end = (piped: boolean) => {
    flush();
    if (words.length) segments.push({ words, piped });
    words = [];
  };
  let i = 0;
  while (i < text.length) {
    const c = text[i]!;
    if (c === "'") {
      const close = text.indexOf("'", i + 1);
      if (close < 0) return null;
      cur += text.slice(i + 1, close);
      open = true;
      i = close + 1;
      continue;
    }
    if (c === '"') {
      let j = i + 1;
      let s = "";
      while (j < text.length && text[j] !== '"') {
        const esc = text[j] === "\\" && '"\\$`'.includes(text[j + 1] ?? "");
        if (esc && j + 1 < text.length) {
          s += text[j + 1];
          j += 2;
        } else {
          s += text[j];
          j++;
        }
      }
      if (j >= text.length) return null;
      cur += s;
      open = true;
      i = j + 1;
      continue;
    }
    if (c === "\\" && i + 1 < text.length) {
      if (text[i + 1] === "\n" || text.startsWith("\r\n", i + 1)) {
        i += text[i + 1] === "\n" ? 2 : 3;
      } else {
        cur += text[i + 1];
        open = true;
        i += 2;
      }
      continue;
    }
    if (c === "\n") {
      end(false);
      i++;
      for (const h of heredocs.splice(0)) {
        while (i < text.length) {
          const nl = text.indexOf("\n", i);
          const line = text.slice(i, nl < 0 ? text.length : nl).replace(/\r$/, "");
          i = nl < 0 ? text.length : nl + 1;
          if ((h.strip ? line.replace(/^\t+/, "") : line) === h.delim) break;
        }
      }
      continue;
    }
    if (c === " " || c === "\t" || c === "\r") {
      flush();
      i++;
      continue;
    }
    if (c === ";") {
      end(false);
      i++;
      continue;
    }
    if (c === "|") {
      const or = text[i + 1] === "|";
      end(!or);
      i += or ? 2 : 1;
      continue;
    }
    if (c === "&") {
      end(false);
      i += text[i + 1] === "&" ? 2 : 1;
      continue;
    }
    if (c === ">" || c === "<") {
      let op = "";
      if (open && /^\d+$/.test(cur)) {
        op = cur;
        cur = "";
        open = false;
      } else {
        flush();
      }
      while (i < text.length && "<>&".includes(text[i]!)) op += text[i++];
      if (/^\d*<<$/.test(op) && text[i] === "-") {
        op += "-";
        i++;
      }
      words.push({ text: op, op: true });
      if (/^\d*<<-?$/.test(op)) delimNext = op.endsWith("-");
      continue;
    }
    cur += c;
    open = true;
    i++;
  }
  end(false);
  return { text, segments };
}

/** A command word's name: no directory, no `.exe`, lower case. */
export function base(word: string | undefined): string {
  return (word ?? "").replace(/^.*[\\/]/, "").replace(/\.exe$/i, "").toLowerCase();
}

/** The segment's words with every redirect operator and its target taken out. */
function plain(seg: Segment): string[] {
  const out: string[] = [];
  for (let j = 0; j < seg.words.length; j++) {
    if (seg.words[j]!.op) {
      j++;
      continue;
    }
    out.push(seg.words[j]!.text);
  }
  return out;
}

/** The targets of a segment's standard-output redirects, `>` and `>>`. */
function redirectTargets(seg: Segment): string[] {
  const out: string[] = [];
  for (let j = 0; j + 1 < seg.words.length; j++) {
    const w = seg.words[j]!;
    if (w.op && /^1?>>?$/.test(w.text) && !seg.words[j + 1]!.op) out.push(seg.words[j + 1]!.text);
  }
  return out;
}

/** A path as the tool would open it. A Git Bash `/d/…` path is read as `D:/…` on Windows. */
export function resolvePath(raw: string, cwd: string): string {
  let p = raw;
  const drive = /^\/([a-zA-Z])(\/|$)/.exec(p);
  if (process.platform === "win32" && drive) p = `${drive[1]!.toUpperCase()}:/${p.slice(3)}`;
  return resolve(cwd, p);
}

function inTree(path: string): string | null {
  const r = relative(ROOT, path);
  if (r === "" || r.startsWith("..") || isAbsolute(r)) return null;
  return r.replace(/\\/g, "/");
}

function git(args: string[]): number {
  return Bun.spawnSync(["git", ...args], { cwd: ROOT, stdout: "ignore", stderr: "ignore" }).exitCode;
}

function tracked(path: string): string | null {
  const r = inTree(path);
  return r !== null && git(["ls-files", "--error-unmatch", "--", r]) === 0 ? r : null;
}

/** The tree-relative path when writing `path` puts content into the tree, outside every git-ignored directory. */
function intoTree(path: string): string | null {
  const r = inTree(path);
  if (r === null || r === ".agent-tmp" || r.startsWith(".agent-tmp/")) return null;
  return git(["check-ignore", "-q", "--", r]) === 0 ? null : r;
}

/** Why reading all of `path` is denied, or null when it is small, untracked, binary or missing. */
function wholeFile(path: string, advice = ""): string | null {
  try {
    if (!existsSync(path) || !statSync(path).isFile()) return null;
    const bytes = readFileSync(path);
    if (bytes.includes(0)) return null;
    let lines = 1;
    for (const b of bytes) if (b === 10) lines++;
    if (lines <= DEFAULT_MAX_LINES) return null;
    const r = tracked(path);
    if (r === null) return null;
    return (
      `${r} is ${lines.toLocaleString("en-US")} lines (${bytes.length.toLocaleString("en-US")} bytes), and this reads all of it ` +
      `into the context. Read the region you need (AGENTS.md rule 3): \`bun nv peek ${r}:120-160\`, \`${r}:@symbol\` or ` +
      `\`${r}:re:pattern\`, as many targets as you have questions in one call, or \`bun nv peek --outline ${r}\` for its seams.` +
      advice
    );
  } catch {
    return null;
  }
}

/** The files a segment prints whole, or null when it is not a whole read. */
function wholeReads(words: string[]): string[] | null {
  const cmd = base(words[0]);
  const args = words.slice(1);
  if (cmd === "cat") return args.filter((w) => !w.startsWith("-"));
  if (cmd === "sed") {
    if (!args.some((w) => /^-[a-zA-Z]*n[a-zA-Z]*$/.test(w))) return null;
    const e = args.findIndex((w) => w === "-e");
    const rest = args.filter((w, j) => !w.startsWith("-") && (e < 0 || j !== e + 1));
    const script = e >= 0 ? args[e + 1] : rest.shift();
    const m = /^(\d+)(?:,(\d+|\$))?p$/.exec(script ?? "");
    if (script !== "p" && !m) return null;
    const span = script === "p" || m![2] === "$" ? Infinity : m![2] ? Number(m![2]) - Number(m![1]) + 1 : 1;
    return span > DEFAULT_MAX_LINES ? rest : null;
  }
  return null;
}

const PROOF_DIRS = ["docs/examples/", "tests/hostile/", "benches/members/"];
const LOOP_WORD = /^(?:for|while|until|xargs)(?![\w-])/;
/** A write in inline code, with the path it writes as the second group. */
const CODE_WRITES = [
  /open\(\s*[rbfu]*(['"])(.+?)\1\s*,\s*(?:mode\s*=\s*)?[rbfu]*(['"])[^'"]*[wax][^'"]*\3/g,
  /Path\(\s*[rbfu]*(['"])(.+?)\1\s*\)\s*\.write_(?:text|bytes)/g,
  /(?:writeFileSync|appendFileSync|Bun\.write)\(\s*(['"`])(.+?)\1/g,
];

export const RULES: Rule[] = [
  {
    name: "whole-read",
    tools: ["Read", "Bash"],
    commands: ["peek"],
    deny(call, parsed) {
      if (call.tool === "Read") {
        const a = call.input;
        if (a.offset || a.limit || a.pages) return null;
        const raw = typeof a.file_path === "string" ? a.file_path : "";
        return raw ? wholeFile(resolvePath(raw, call.cwd), " `Read` with an `offset` and a `limit` is allowed too.") : null;
      }
      for (const seg of parsed!.segments) {
        if (seg.piped) continue;
        const files = wholeReads(plain(seg));
        for (const f of files ?? []) {
          const why = f ? wholeFile(resolvePath(f, call.cwd)) : null;
          if (why) return why;
        }
      }
      return null;
    },
  },
  {
    name: "proof-loop",
    tools: ["Bash"],
    commands: ["proofs"],
    deny(_call, parsed) {
      const segs = parsed!.segments;
      if (!segs.some((s) => LOOP_WORD.test(plain(s)[0] ?? ""))) return null;
      const all = segs.flatMap(plain);
      if (!all.some((w, j) => base(w) === "nvs" && all[j + 1] === "run")) return null;
      const dir = PROOF_DIRS.find((d) => all.some((w) => `${w.replace(/\\/g, "/")}/`.includes(d)));
      if (!dir) return null;
      return (
        `this runs \`nvs run\` in a loop over ${dir}. \`bun nv proofs --run --id <feature> --show\` runs each of a ` +
        `feature's examples and attacks in one call and reports what each printed.`
      );
    },
  },
  {
    name: "inline-write",
    tools: ["Bash"],
    commands: ["splice"],
    deny(call, parsed) {
      for (const seg of parsed!.segments) {
        const words = plain(seg);
        const cmd = base(words[0]);
        const targets: string[] = [];
        if (seg.words.some((w) => w.op && /^\d*<<-?$/.test(w.text))) {
          targets.push(...redirectTargets(seg));
          if (cmd === "tee") targets.push(...words.slice(1).filter((w) => !w.startsWith("-")));
        }
        const flag = cmd.startsWith("python") || cmd === "py" ? ["-c"] : cmd === "bun" || cmd === "node" ? ["-e", "--eval"] : [];
        const at = words.findIndex((w, j) => j > 0 && flag.includes(w));
        if (at >= 0) {
          const code = words[at + 1] ?? "";
          for (const re of CODE_WRITES) for (const m of code.matchAll(re)) targets.push(m[2]!);
        }
        for (const t of targets) {
          const r = t ? intoTree(resolvePath(t, call.cwd)) : null;
          if (r === null) continue;
          return (
            `this writes ${r} from text inside the command, and the shell parses that text first (AGENTS.md rule 1). ` +
            `Write it with the Write or Edit tool, with \`bun nv splice --patch <file>\`, or with a script ` +
            `under \`.agent-tmp/\`.`
          );
        }
      }
      return null;
    },
  },
  {
    name: "cargo-p",
    tools: ["Bash"],
    commands: ["verify"],
    deny(_call, parsed) {
      for (const seg of parsed!.segments) {
        const words = plain(seg);
        if (base(words[0]) !== "cargo") continue;
        const sub = words.slice(1).find((w) => !w.startsWith("-") && !w.startsWith("+"));
        if (sub !== "build" && sub !== "test" && sub !== "clippy") continue;
        if (!words.some((w) => /^(?:-p|--package)(?:=|$)|^-p\S/.test(w))) continue;
        if (words.some((w) => w === "--release" || w === "-r" || /^--profile(?:=|$)/.test(w))) continue;
        return (
          `\`cargo ${sub} -p\` resolves features over one package and writes a second copy of every workspace crate ` +
          `(AGENTS.md rule 5). Leave out \`-p\` and narrow what runs with \`--test <name>\` or \`--lib\`, or run ` +
          `\`bun nv verify -p <crate>\`.`
        );
      }
      return null;
    },
  },
  {
    name: "heavy-run",
    tools: ["Bash"],
    commands: ["affected"],
    deny(call, parsed) {
      for (const seg of parsed!.segments) {
        const why = heavyRun(plain(seg), call.cwd);
        if (why === null) continue;
        return (
          `${why}. \`bun nv affected\` names what your change reaches, and \`bun nv affected --run\` runs exactly ` +
          `that; \`--since <rev>\` names a committed change. To look into one test, name it: \`cargo test --test <name>\` ` +
          `or \`cargo test --lib <filter>\`. The owed floor is paid by the loop, and at a push by the person pushing.`
        );
      }
      return null;
    },
  },
];

/** The words after `bun nv` when a segment runs an `nv` command, else null. */
function nvArgs(words: string[]): string[] | null {
  if (base(words[0]) !== "bun") return null;
  if (words[1] === "nv") return words.slice(2);
  if (words[1] === "run" && words[2] === "nv") return words.slice(3);
  if ((words[1] ?? "").replace(/\\/g, "/").endsWith("tools/nv/main.ts")) return words.slice(2);
  return null;
}

/** `cargo test`'s flags that take a value, whose value is not a test-name filter. */
const CARGO_VALUED = /^(?:--features|-F|--jobs|-j|--manifest-path|--target|--target-dir|--message-format|--color|--profile|--config|-Z|--exclude|--package|-p)$/;
/** `cargo test`'s flags that pick which test targets build and run. */
const CARGO_TARGETS = /^(?:--lib|--bin|--bins|--test|--tests|--doc|--no-run|--example|--examples|--bench|--benches)(?:=|$)/;

/** Whether a `cargo test` segment names what it runs: a target flag, or a test-name filter before or
 * after `--`. */
function narrowCargoTest(args: string[]): boolean {
  for (let j = 0; j < args.length; j++) {
    const w = args[j]!;
    if (w === "--") return args.slice(j + 1).some((a) => !a.startsWith("-"));
    if (CARGO_TARGETS.test(w)) return true;
    if (CARGO_VALUED.test(w)) j++;
    else if (!w.startsWith("-")) return true;
  }
  return false;
}

/** A segment's words without the commands that only wrap the next one: `timeout 100`, `time`, `nice`,
 * `nohup`, `env` and a leading `NAME=value`. */
function unwrapped(words: string[]): string[] {
  let j = 0;
  while (j < words.length) {
    const w = base(words[j]);
    if (/^[A-Za-z_][A-Za-z0-9_]*=/.test(words[j]!)) j++;
    else if (w === "time" || w === "nice" || w === "nohup" || w === "env") j++;
    else if (w === "timeout") {
      j++;
      while (j < words.length && words[j]!.startsWith("-")) j++;
      j++;
    } else break;
  }
  return words.slice(j);
}

/** Why a segment runs verification its change does not decide, or null. */
function heavyRun(all: string[], cwd: string): string | null {
  const words = unwrapped(all);
  const nv = nvArgs(words);
  if (nv !== null) {
    const [cmd, ...rest] = nv;
    if (cmd === "sweep" && !rest.some((w) => w === "--help" || w === "-h")) return "`bun nv sweep` is the hook looper runs after each session: it pays owed and red atoms for a fixed time, whatever your change reaches";
    if (cmd === "benches") return "`bun nv benches` judges every bench the selection picks on the release build, whatever your change reaches; the loop's hook after each session pays owed and red benches";
    if (cmd === "proofs" && rest.some((w) => w === "--verify" || w === "--run") && !rest.some((w) => /^--(?:id|only|group)(?:=|$)/.test(w))) {
      return "`bun nv proofs` over every feature runs each one's examples and attacks, whatever your change reaches";
    }
    if (cmd === "verify" && rest.includes("--no-cache")) return "`bun nv verify --no-cache` runs every step whatever its inputs, and the green cache is keyed on exactly what each step reads";
    if (cmd === "select" && rest.includes("--full")) return "`bun nv select --full` runs every atom the store knows, whatever your change reaches; a person runs it, and nightly CI catches what the selection misses";
    return null;
  }
  if (base(words[0]) === "looper" && words[1] === "check" && !words.some((w) => /^--name(?:=|$)/.test(w))) {
    return "`looper check` with no `--name` runs the live goal's whole acceptance, whatever your change reaches";
  }
  if (base(words[0]) !== "cargo") return null;
  const args = words.slice(1).filter((w) => !w.startsWith("+"));
  if (args[0] !== "test" || narrowCargoTest(args.slice(1))) return null;
  // A scratch crate is not the workspace.
  if (args.includes("--manifest-path") || inTree(cwd)?.startsWith(".agent-tmp")) return null;
  return "`cargo test` with no target and no filter runs every test binary in the workspace, whatever your change reaches";
}

/** The reason a hook event is denied, or null when it is allowed. Anything unexpected allows it. */
export function decide(event: unknown): string | null {
  try {
    if (!event || typeof event !== "object") return null;
    const e = event as Record<string, unknown>;
    const tool = e.tool_name;
    if (tool !== "Read" && tool !== "Bash") return null;
    const input = (e.tool_input && typeof e.tool_input === "object" ? e.tool_input : {}) as Record<string, unknown>;
    const cwd = typeof e.cwd === "string" && e.cwd ? e.cwd : process.cwd();
    let parsed: Parsed | null = null;
    if (tool !== "Read") {
      if (typeof input.command !== "string") return null;
      parsed = parse(input.command);
      if (!parsed) return null;
    }
    const call: Call = { tool, input, cwd };
    for (const rule of RULES) {
      if (!rule.tools.includes(tool)) continue;
      const why = rule.deny(call, parsed);
      if (why) return `guard: ${rule.name}: ${why}`;
    }
    return null;
  } catch {
    return null;
  }
}

/** The tools `.claude/settings.json` runs the guard for. */
export function wiredTools(settings: unknown): Tool[] {
  const entries = (settings as { hooks?: { PreToolUse?: unknown } })?.hooks?.PreToolUse;
  if (!Array.isArray(entries)) return [];
  return TOOLS.filter((tool) =>
    entries.some((entry) => {
      const matcher = typeof entry?.matcher === "string" ? entry.matcher : "";
      let matches: boolean;
      try {
        matches = matcher === "" || matcher === "*" || new RegExp(`^(?:${matcher})$`).test(tool);
      } catch {
        matches = false;
      }
      const hooks = Array.isArray(entry?.hooks) ? entry.hooks : [];
      return matches && hooks.some((h: { command?: unknown }) => typeof h?.command === "string" && h.command.trim() === HOOK);
    }),
  );
}

async function check(): Promise<number> {
  let ok = true;
  const settingsPath = join(ROOT, ".claude", "settings.json");
  let settings: unknown = null;
  try {
    settings = JSON.parse(readFileSync(settingsPath, "utf8"));
  } catch (err) {
    console.log(`guard: cannot read ${relOf(settingsPath)}: ${(err as Error).message}`);
  }
  const wired = wiredTools(settings);
  const missing = TOOLS.filter((t) => !wired.includes(t));
  if (missing.length === 0) console.log("guard: wired for Read and Bash");
  else {
    ok = false;
    console.log(`guard: \`${HOOK}\` is not wired in ${relOf(settingsPath)} for ${missing.join(", ")}`);
  }
  const help = await runProc([process.execPath, join(ROOT, "tools", "nv", "main.ts"), "help"], { timeoutMs: 60_000 });
  const registered = new Set([...help.stdout.matchAll(/^ {2}(\S+) {2}/gm)].map((m) => m[1]!));
  let named = true;
  for (const rule of RULES) {
    const absent = rule.commands.filter((c) => !registered.has(c));
    if (absent.length) named = false;
    const shown = rule.commands.length ? rule.commands.map((c) => `nv ${c}${registered.has(c) ? "" : " (not registered)"}`).join(", ") : "no nv command";
    console.log(`  ${rule.name}  ${shown}`);
  }
  if (named) console.log("guard: every rule names a registered command");
  else {
    ok = false;
    console.log("guard: a rule names a command `bun nv help` does not list");
  }
  return ok ? 0 : 1;
}

export async function run(args: string[]): Promise<number> {
  if (args[0] === "--check" && args.length === 1) return check();
  if (args.length) {
    console.error("usage: bun nv guard [--check]   (the hook reads its event from standard input)");
    return 2;
  }
  let event: unknown;
  try {
    event = JSON.parse(await Bun.stdin.text());
  } catch {
    return 0;
  }
  const reason = decide(event);
  if (reason) {
    const out = { hookSpecificOutput: { hookEventName: "PreToolUse", permissionDecision: "deny", permissionDecisionReason: reason } };
    console.log(JSON.stringify(out));
  }
  return 0;
}
