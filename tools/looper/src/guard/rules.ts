// The guard's built-in rules, which every project's own guard shared. It ports the rules of
// `crates/looper/src/cmd/guard.rs`. `guard.ts` decides when they run, and its head comment owns the
// table of what each refuses.
//
// - `shell` judges a `Bash` or `PowerShell` command with the text inside its quotes removed, so a
//   commit message that says `>` is not a redirect. A write into the scratch directory passes.
// - `wholeRead` judges a `Read` without `limit`, by the lines of its file from `offset` on.
// - Each pattern is the Rust guard's, with its inline `(?i)` as the `i` flag.
//
// # The bounds it keeps
//
// - The Read rule reads its file in chunks of `CHUNK` bytes, and only until it has passed
//   `WHOLE_READ_LINES` lines.

import { closeSync, openSync, readSync } from "node:fs";
import { isAbsolute, join, resolve, sep } from "node:path";

/** The most lines a Read without `limit` may return. */
export const WHOLE_READ_LINES = 400;

/** The bytes the Read rule reads at a time. */
export const CHUNK = 64 << 10;

/** A line break, `&&`, `||`, `;`, `|` or `(`: where a new command can start. */
const START = String.raw`(?:^|[\n;&|(])\s*`;

const HEREDOC = /(?:^|[^<])<<-?[ \t]*(?:''|""|\\?[A-Za-z_])/;
const REDIRECT = /([-=])?[0-9*&]?>>?\|?[ \t]*(&[0-9-]*|[^ \t\r\n|;&<>()]*)/g;
const TEE = new RegExp(String.raw`${START}(?:tee|tee-object)(?:\s|$)`, "gi");
const SED_I = new RegExp(String.raw`${START}sed\s(?:[^\n;&|]*\s)?(?:-[A-Za-z]*i|--in-place)`);
const CMDLET = /\b(?:set-content|add-content|out-file)\b|\[(?:system\.)?io\.file\]::(?:write|append)/i;
const SLEEP = new RegExp(String.raw`${START}(?:sleep\s|start-sleep\b|timeout\s+/t\b)|\b(?:do|then)\s+sleep\s`, "i");
const POLL = new RegExp(String.raw`\bwhile\s+(?:true|:)(?:\s|;|$)|\bwhile\s*\(\s*\$true\s*\)|${START}until\s|\}\s*until\s*\(`, "i");
const INSTALL = new RegExp(
  String.raw`${START}(?:&\s*)?(?:\S*[\\/])?(?:looper(?:\.exe)?|cargo\s+run\b[^\n;&|]*\s--)\s+(?:self-update|service\s+install)\b`,
  "i",
);
const SERVICE = new RegExp(
  String.raw`${START}(?:&\s*)?(?:\S*[\\/])?(?:looper(?:\.exe)?|cargo\s+run\b[^\n;&|]*\s--)\s+service\s+(?:stop|start|uninstall)\b`,
  "i",
);
/** Where a command ends: `&&`, `||`, `;`, `|` or a line break. */
export const SEGMENT = /&&|\|\||[;|\n]/;
/** Where a pipeline ends: `&&`, `||`, `;` or a line break. */
const PIPELINE = /&&|\|\||[;\n]/;
const KILL_NAME = new RegExp(
  String.raw`${START}(?:pkill|killall)(?:\.exe)?\b|\btaskkill(?:\.exe)?\b[^\n;&|]*\s(?:/{1,2}|-)im\b|\b(?:stop-process|spps)\b[^\n;&|]*\s-(?:n|na|nam|name|processname)\b`,
  "i",
);
/** A command that finds a process, by name or by listing them all. */
const FINDER = /\b(?:pgrep|pidof|ps|get-process|gps|tasklist)\b/i;
const KILLER = /\b(?:kill|stop-process|spps|taskkill)\b/i;
const WAITER = /\b(?:wait|while|until)\b|\.waitforexit\b/i;
const WAIT_PROCESS = /\bwait-process\b/i;

const WRITE =
  "A shell never carries file content into the tree: create and edit files with Write and Edit, and pass a multi-line argument as a file under the scratch directory.";

/** The words of `text`, split at whitespace. */
function words(text: string): string[] {
  return text.split(/\s+/).filter((w) => w !== "");
}

/**
 * The reason a built-in shell rule refuses `cmd`, if one does. A redirect or a `tee` whose every target
 * `scratch` accepts passes. A service stop, start or uninstall is refused only when `loop` is set.
 */
export function shell(cmd: string, scratch: (target: string) => boolean, loop: boolean): string | undefined {
  const bare = unquoted(cmd);
  if (HEREDOC.test(bare)) return `a heredoc. ${WRITE}`;
  for (const m of bare.matchAll(REDIRECT)) {
    const target = m[2] ?? "";
    if (m[1] === undefined && !isNullSink(target) && !scratch(target)) return `a redirect to ${JSON.stringify(target)}. ${WRITE}`;
  }
  const teeOutside = [...bare.matchAll(TEE)].some((m) => {
    let rest = bare.slice(m.index + m[0].length);
    const end = rest.search(SEGMENT);
    if (end >= 0) rest = rest.slice(0, end);
    // A redirect is judged above, and a `-Name:value` flag hides its value.
    const args = words(rest);
    const until = args.findIndex((w) => w.includes("<") || w.includes(">"));
    return args.slice(0, until < 0 ? args.length : until).some((w) => (w.startsWith("-") ? w.includes(":") : !scratch(w)));
  });
  if (teeOutside || SED_I.test(bare) || CMDLET.test(bare)) return `a shell file write. ${WRITE}`;
  const pipelines = bare.split(PIPELINE);
  const waitsByName = WAIT_PROCESS.test(bare) || pipelines.some((p) => FINDER.test(p) && WAITER.test(p));
  if (SLEEP.test(bare) || POLL.test(bare) || waitsByName) {
    return "a sleep, a poll loop or a wait on a process. Start a long command with `run_in_background` and end your turn: its report starts your next one. Watch its output with Monitor while you work, and wait for another session's job with `looper exec wait <id>` in the background.";
  }
  if (KILL_NAME.test(bare) || pipelines.some((p) => FINDER.test(p) && KILLER.test(p))) {
    return "a kill by process name, which can end another session's process. Stop your own background command with TaskStop, and another job with `looper exec kill <id>`, which `looper exec list` names.";
  }
  if (INSTALL.test(bare)) {
    return "a session never installs looper or builds it in its own job (docs/findings.md § loop-controls). Run `looper service update --wait`, which the server runs outside the session.";
  }
  if (loop && SERVICE.test(bare)) {
    return "a loop session never stops, starts or uninstalls the service, which outlives every session. When the server needs a person, file it with `looper decision ask --parked` and write `BLOCKED`.";
  }
  // A test run too wide to start at all is named before how its output is cut.
  for (const c of bare.split(SEGMENT)) {
    const wide = wideTest(c);
    if (wide !== undefined) return wide;
  }
  for (const p of pipelines) {
    const trim = trimmedBuild(p);
    if (trim !== undefined) {
      return `a build or test piped into \`${trim}\`. The harness keeps the whole output and previews it past 30,000 characters. Run it untrimmed, or pipe it into \`grep\` for the lines you need.`;
    }
  }
  return undefined;
}

/** The flags of `cargo test` that take the next word as their value. */
const CARGO_VALUED = [
  "-p",
  "--package",
  "--exclude",
  "-F",
  "--features",
  "--target",
  "--target-dir",
  "--profile",
  "--manifest-path",
  "-j",
  "--jobs",
  "--color",
  "--message-format",
  "--config",
  "-Z",
];
/** The flags of a libtest binary that take the next word as their value. */
const LIBTEST_VALUED = ["--test-threads", "--skip", "--format", "--color", "--logfile", "-Z", "--shuffle-seed"];
/** The flags of `bun test` that take the next word as their value. */
const BUN_VALUED = ["--timeout", "--preload", "--cwd", "--rerun-each", "--coverage-dir", "--coverage-reporter", "--reporter", "--reporter-outfile"];

/** Whether `a` is the flag `f`, alone or as `f=value`. */
function isFlag(a: string, f: string): boolean {
  return a === f || a.startsWith(`${f}=`);
}

/**
 * The reason a test run wider than `looper verify` refuses the one command `cmd`, if it is one: a
 * `cargo test` that names no target and no filter and builds to run, or a `bun test` that names no file
 * and no test.
 */
function wideTest(cmd: string): string | undefined {
  const ws = words(cmd).filter((w) => !w.includes("<") && !w.includes(">"));
  const verify = "Run `looper verify`, which runs only the tests a change reaches";
  const cargoAt = ws.findIndex((w) => stem(w) === "cargo");
  const cargo = cargoAt < 0 ? [] : ws.slice(cargoAt + 1);
  const toolchain = cargo.findIndex((w) => !w.startsWith("+"));
  const [sub, ...args] = toolchain < 0 ? [] : cargo.slice(toolchain);
  if (sub === "test" || sub === "t") {
    const dash = args.indexOf("--");
    const own = dash < 0 ? args : args.slice(0, dash);
    const toTests = dash < 0 ? [] : args.slice(dash + 1);
    const target = own.some((a) => ["--test", "--lib", "--bin", "--example", "--bench", "--no-run"].some((f) => isFlag(a, f)));
    const narrow = target || hasFilter(own, CARGO_VALUED) || hasFilter(toTests, LIBTEST_VALUED);
    return narrow ? undefined : `a bare \`cargo test\`. ${verify}, or name one target with \`--test <name>\` or \`--lib\`, or a filter.`;
  }
  const bunAt = ws.findIndex((w) => stem(w) === "bun");
  if (bunAt < 0 || ws[bunAt + 1] !== "test") return undefined;
  const bunArgs = ws.slice(bunAt + 2);
  const named = bunArgs.some((a) => isFlag(a, "-t") || isFlag(a, "--test-name-pattern"));
  return named || hasFilter(bunArgs, BUN_VALUED) ? undefined : `a bare \`bun test\`. ${verify}, or name a test file.`;
}

/** The trim that the one pipeline `pipeline` pipes a build or a test into, if it does: `head`, `tail`, or `Select-Object` with `-First` or `-Last`. */
function trimmedBuild(pipeline: string): string | undefined {
  const stages = pipeline.split("|").map((s) => words(s).filter((w) => !w.includes("=") && !w.includes("<") && !w.includes(">")));
  for (let at = 1; at < stages.length; at++) {
    const [program, ...args] = stages[at] ?? [];
    if (program === undefined) continue;
    let trim: string;
    switch (stem(program)) {
      case "head":
      case "tail":
        trim = stem(program);
        break;
      case "select-object":
      case "select":
        if (!args.some((a) => /^-[fl]/i.test(a))) continue;
        trim = "Select-Object";
        break;
      default:
        continue;
    }
    if (stages.slice(0, at).some(isBuild)) return trim;
  }
  return undefined;
}

/** Whether the words `stage` start a build or a test. */
function isBuild(stage: string[]): boolean {
  const [program, ...args] = stage;
  if (program === undefined) return false;
  const sub = (args.find((a) => !a.startsWith("-") && !a.startsWith("+")) ?? "").toLowerCase();
  switch (stem(program)) {
    case "cargo":
      return ["build", "b", "test", "t", "check", "c", "clippy", "run", "r", "nextest", "bench"].includes(sub);
    case "bun":
    case "npm":
    case "pnpm":
    case "yarn":
      return ["test", "run", "build"].includes(sub);
    case "looper":
      return ["verify", "check"].includes(sub);
    case "go":
    case "dotnet":
      return ["build", "test"].includes(sub);
    case "cmake":
      return args.includes("--build");
    case "pytest":
    case "vitest":
    case "tsc":
    case "make":
    case "msbuild":
      return true;
    default:
      return false;
  }
}

/** The program `word` names, lowercased, without its directory or extension. */
export function stem(word: string): string {
  const name = word.replace(/[\\/]+$/, "").split(/[\\/]/).pop() ?? "";
  const dot = name.lastIndexOf(".");
  return (dot > 0 ? name.slice(0, dot) : name).toLowerCase();
}

/** Whether `args` holds a positional word, a filter or a path, that is not the value of a flag in `valued`. */
function hasFilter(args: string[], valued: string[]): boolean {
  let value = false;
  for (const a of args) {
    const positional = !value && !a.startsWith("-");
    value = valued.includes(a);
    if (positional) return true;
  }
  return false;
}

/** A redirect target that writes no file: a null device or another stream. */
function isNullSink(target: string): boolean {
  return target.startsWith("&") || ["/dev/null", "$null", "nul", "nul:", "/dev/stderr", "/dev/stdout"].includes(target.toLowerCase());
}

/**
 * Whether the shell write target `target`, named in `cwd`, is a file under `scratch`. A target with a
 * `..`, a quote or a shell expansion is never one, because what it names is not known here.
 */
export function inScratch(target: string, cwd: string, scratch: string): boolean {
  if (target === "" || target.includes("..") || /["'$%`~*?{]/.test(target)) return false;
  const path = resolve(cwd, target);
  const dir = resolve(scratch);
  return path.startsWith(dir + sep);
}

/** `cmd` with the text inside each pair of quotes removed, the quotes kept. In double quotes a backslash or a backtick escapes the next character. */
export function unquoted(cmd: string): string {
  let out = "";
  for (let i = 0; i < cmd.length; i++) {
    const c = cmd[i];
    out += c;
    if (c === "'") {
      const end = cmd.indexOf("'", i + 1);
      if (end < 0) break;
      out += "'";
      i = end;
    } else if (c === '"') {
      for (i++; i < cmd.length; i++) {
        const q = cmd[i];
        if (q === "\\" || q === "`") i++;
        else if (q === '"') {
          out += q;
          break;
        }
      }
    }
  }
  return out;
}

/** The lines of a text file, counted only past `stop`. Undefined for a file that cannot be read or holds a NUL byte in what was read, which marks it binary. */
function countLines(path: string, stop: number): number | undefined {
  let fd: number;
  try {
    fd = openSync(path, "r");
  } catch {
    return undefined;
  }
  try {
    const buf = Buffer.alloc(CHUNK);
    let lines = 0;
    let last = 0x0a;
    for (;;) {
      const n = readSync(fd, buf, 0, CHUNK, null);
      if (n === 0) break;
      const chunk = buf.subarray(0, n);
      if (chunk.includes(0)) return undefined;
      for (const b of chunk) if (b === 0x0a) lines++;
      last = chunk[n - 1] ?? 0x0a;
      if (lines > stop) return lines;
    }
    return lines + (last === 0x0a ? 0 : 1);
  } catch {
    return undefined;
  } finally {
    closeSync(fd);
  }
}

/** The reason the Read rule refuses a read of `file`, named from `cwd`, without a limit from `offset` on, if it does. */
export function wholeRead(file: string, cwd: string, offset: number): string | undefined {
  const path = isAbsolute(file) ? file : join(cwd, file);
  const from = Math.max(1, offset);
  const lines = countLines(path, WHOLE_READ_LINES + from);
  if (lines === undefined || lines - (from - 1) <= WHOLE_READ_LINES) return undefined;
  return `${path} has more than ${WHOLE_READ_LINES} lines. Grep for the anchor you need, then Read with an offset and a limit.`;
}
