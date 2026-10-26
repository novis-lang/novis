// A `.term`'s commands, run in a copy of its fixture, and the `.term.out` they record.
// `tools/nv/cmd/site.ts` § *A snippet* is the home of both formats; this module reads the first, runs
// each command in a fresh copy of the fixture, writes what `nvs` printed in the second, and compares two
// recordings with the values that differ from run to run masked.

import { cpSync, existsSync, mkdirSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { CACHE } from "./paths.ts";

/** The extensions of a terminal block's commands and of its recording. */
export const TERM = "term";
export const TERM_OUT = "term.out";

/** One command of a `.term`: the line as the page shows it, and its words. */
export interface Command {
  line: string;
  argv: string[];
}

/** The commands of a `.term`, one per non-blank line, or why the text is not one. A word is a run of
 * characters other than white space, or a run inside `"` or `'`, which has no escapes. Every command
 * starts with `nvs`. */
export function parseTerm(text: string): Command[] | string {
  const commands: Command[] = [];
  const lines = text.replace(/\r\n?/g, "\n").split("\n");
  for (const [i, raw] of lines.entries()) {
    const line = raw.trim();
    if (line === "") continue;
    const word = /"([^"]*)"|'([^']*)'|([^\s"']+)/g;
    if (line.replace(word, "").trim() !== "") return `line ${i + 1}: a quote is not closed`;
    const argv = [...line.matchAll(word)].map((m) => m[1] ?? m[2] ?? m[3]!);
    if (argv[0] !== "nvs") return `line ${i + 1}: \`${line}\` does not start with \`nvs\``;
    commands.push({ line, argv: argv.slice(1) });
  }
  return commands.length ? commands : "it has no command";
}

/** Where a copy's own path is written in a recording. Its real path names the machine that ran it. */
export const PROJECT = "~/project";
/** Where the machine's temporary directory is written in a recording, for the same reason. */
export const TMP = "/tmp";

/** `text` with each path that starts at `from` written from `to`, and the separators after it as `/`. */
function rewrite(text: string, from: string, to: string): string {
  const forms = [...new Set([from, from.replaceAll("\\", "/")])].map((f) => f.replace(/[\\/]+$/, ""));
  let out = text;
  for (const f of forms) {
    const at = new RegExp(`${f.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}([^\\s'"]*)`, "gi");
    out = out.replace(at, (_, tail: string) => `${to}${tail.replaceAll("\\", "/")}`);
  }
  return out;
}

/** The values that differ from run to run, each with the fixed text it is compared as. A date comes
 * before a time and a time before a duration, so each value is masked once, whole. */
const MASKS: [RegExp, string][] = [
  [/(?:\/tmp|\/var\/folders)\/[^\s'"]*/g, "<tmp>"],
  [/\b\d{4}-\d{2}-\d{2}(?:[T ]\d{2}:\d{2}(?::\d{2}(?:\.\d+)?)?(?:Z|[+-]\d{2}:?\d{2})?)?/g, "<date>"],
  [/\b\d{2}:\d{2}:\d{2}(?:\.\d+)?\b/g, "<time>"],
  [/\b[0-9a-f]{32}\b|\b[0-9a-f]{16}\b/g, "<id>"],
  [/\b(localhost|127\.0\.0\.1|0\.0\.0\.0|\[::1?\]):\d{1,5}\b/g, "$1:<port>"],
  [/\bport \d{1,5}\b/g, "port <port>"],
  [/\b\d+(?:\.\d+)?(?:\s?(?:ns|µs|μs|us|ms|min)|s|m|h)\b/g, "<duration>"],
];

/** `text` with every value in `MASKS` replaced, line endings as LF and trailing white space dropped. */
export function mask(text: string): string {
  let out = text.replace(/\r\n?/g, "\n").replace(/[ \t]+$/gm, "").trimEnd();
  for (const [re, to] of MASKS) out = out.replace(re, to);
  return out;
}

/** Whether two recordings are the same once masked. */
export const sameTerm = (a: string, b: string) => mask(a) === mask(b);

/** What one command printed, and its exit code. */
export interface Printed {
  code: number;
  stdout: string;
  stderr: string;
}

/** One block of a `.term.out`: `$ ` and the command's line, what it printed to standard output and then
 * to standard error, and `exit N`. */
function block(command: Command, printed: Printed): string {
  const text = (s: string) => s.replace(/\r\n?/g, "\n").replace(/[ \t]+$/gm, "").replace(/^\n+/, "").trimEnd();
  const output = [text(printed.stdout), text(printed.stderr)].filter((s) => s !== "");
  return [`$ ${command.line}`, ...output, `exit ${printed.code}`].join("\n");
}

/**
 * The `.term.out` the commands of `term` record now. `fixture` is the directory copied, and is empty when
 * it does not exist. `play` runs one command's words in the copy. The copy lives under `.cache/` for the
 * run and is deleted after it. Each command runs in the same copy, in order, so one sees what the one
 * before it wrote.
 */
export async function recordTerm(term: string, fixture: string, play: (argv: string[], cwd: string) => Promise<Printed>): Promise<{ recording: string } | { failure: string }> {
  const commands = parseTerm(term);
  if (typeof commands === "string") return { failure: commands };
  const copy = join(CACHE, "nv-term", crypto.randomUUID());
  mkdirSync(copy, { recursive: true });
  try {
    if (existsSync(fixture)) cpSync(fixture, copy, { recursive: true });
    const blocks: string[] = [];
    for (const command of commands) {
      const printed = await play(command.argv, copy);
      const out = rewrite(rewrite(block(command, printed), copy, PROJECT), tmpdir(), TMP);
      blocks.push(out);
    }
    return { recording: `${blocks.join("\n\n")}\n` };
  } finally {
    rmSync(copy, { recursive: true, force: true });
  }
}
