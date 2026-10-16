// `bun nv splice`: replaces exact blocks of text, across as many files as one edit touches, in one call.
// The blocks come from a file written with the Write tool, never from the command line, because a shell
// parses a block's quotes and backslashes before it runs anything (AGENTS.md rule 1).
//
// Every block in every file is matched exactly once before a single byte is written, so a stale anchor
// in the last file leaves the first one untouched. A failed match says where the anchor stopped
// matching, which is almost always a line of whitespace or a character that has since changed. A target
// that uses CRLF is matched as LF and written back as CRLF, so its line endings survive the edit.
//
// Exits 0 when the patch applies or `--dry-run` finds every anchor, 1 on an anchor that matches zero
// or several times, and 2 on a bad argument, an unreadable file or a malformed patch.

import { readFileSync, writeFileSync } from "node:fs";
import { pyRepr } from "../lib/py.ts";
import { record } from "../lib/written.ts";

export const summary = "replace exact blocks across files, all or nothing: nv splice --patch <file>";

const FORMS = `    bun nv splice --patch <patch-file>              # any number of files, preferred
    bun nv splice <target> --patch <patch-file>     # one file, targets named on argv
    bun nv splice <target> <old-file> <new-file>    # two files
    bun nv splice --patch <f> --dry-run             # do the anchors match?
    bun nv splice --help                            # this, down to the patch format`;

const HELP = `Replace exact blocks of text, across as many files as one edit touches, in one call.

${FORMS}

A **patch file** holds the blocks, in conflict-marker form, each under the file it edits:

    --- crates/nvs-ir/src/lower/expr.rs
    <<<<<<< OLD
    the exact text to find
    =======
    the text to put there instead
    >>>>>>> NEW
    <<<<<<< OLD
    a second block in the same file
    =======
    its replacement
    >>>>>>> NEW
    --- crates/nvs-types/src/expr/mod.rs
    <<<<<<< OLD
    a block in a different file
    =======
    its replacement
    >>>>>>> NEW

A \`--- <path>\` line applies to every block under it until the next one. With a \`<target>\` on the
command line the headers may be left out entirely, which is the one-file form. **Either way the whole
patch applies or none of it does** -- every block in every file is matched before a single byte is
written, so a stale anchor in the last file cannot leave the first one half-edited.`;

const OPEN = "<<<<<<< OLD";
const MID = "=======";
const CLOSE = ">>>>>>> NEW";
const FILE_MARK = "--- ";

/** A refusal: its message is printed and the command exits with its code. */
class Refusal extends Error {
  constructor(message: string, readonly code = 1) {
    super(message);
  }
}

const refuse = (message: string, code = 1): never => {
  throw new Refusal(message, code);
};

interface Block {
  target: string;
  old: string;
  new: string;
}

/** The file's text with CRLF read as LF, the way Python's text mode reads it. */
function read(path: string): string {
  try {
    return readFileSync(path, "utf8").replace(/\r\n/g, "\n");
  } catch (e) {
    return refuse(`nv splice: cannot read ${path}: ${(e as Error).message}`, 2);
  }
}

/**
 * The patch's blocks, in file order then block order. A `--- <path>` line switches which file the
 * blocks below it edit. Without one, every block goes to `defaultTarget`; a target on the command line
 * together with headers in the file is refused, since the two say different things about where a
 * block lands.
 */
export function parsePatch(text: string, path: string, defaultTarget: string | null = null): Block[] {
  const blocks: Block[] = [];
  const lines = text.split("\n");
  let target = defaultTarget;
  let sawHeader = false;
  let i = 0;
  while (i < lines.length) {
    const line = lines[i]!;
    if (line.startsWith(FILE_MARK) && line.trimEnd() !== MID) {
      const named = line.slice(FILE_MARK.length).trim();
      if (!named) refuse(`nv splice: ${path} line ${i + 1} is a bare \`${FILE_MARK.trim()}\` with no path after it`, 2);
      if (defaultTarget !== null) {
        refuse(`nv splice: ${path} names files with \`${FILE_MARK}${named}\` and a target was also given on the command line. Use one or the other.`, 2);
      }
      target = named;
      sawHeader = true;
      i++;
      continue;
    }
    if (line.trimEnd() !== OPEN) {
      if (line.trim() && blocks.length === 0 && !sawHeader) {
        refuse(`nv splice: ${path} does not start with \`${OPEN}\` or \`${FILE_MARK.trim()} <path>\` -- \`bun nv splice --help\` prints the patch format`, 2);
      }
      i++;
      continue;
    }
    if (target === null) {
      refuse(`nv splice: ${path} line ${i + 1} opens a block before any \`${FILE_MARK}<path>\` line, and no target was given on the command line`, 2);
    }
    const mid = lines.findIndex((l, j) => j > i && l.trimEnd() === MID);
    const end = mid < 0 ? -1 : lines.findIndex((l, j) => j > mid && l.trimEnd() === CLOSE);
    if (end < 0) refuse(`nv splice: a \`${OPEN}\` block in ${path} is missing its \`${MID}\` or \`${CLOSE}\``, 2);
    blocks.push({ target: target!, old: lines.slice(i + 1, mid).join("\n"), new: lines.slice(mid + 1, end).join("\n") });
    i = end + 1;
  }
  if (blocks.length === 0) refuse(`nv splice: no \`${OPEN}\` block in ${path}`, 2);
  return blocks;
}

const lineAt = (text: string, at: number) => text.slice(0, at).split("\n").length;

/** The longest prefix of `old` that is in `src`, and what follows it on both sides, in code points. */
export function whereItDiverges(src: string, old: string): string {
  const chars = Array.from(old);
  const prefix = (n: number) => chars.slice(0, n).join("");
  let lo = 0;
  let hi = chars.length;
  while (lo < hi) {
    const mid = Math.floor((lo + hi + 1) / 2);
    if (src.includes(prefix(mid))) lo = mid;
    else hi = mid - 1;
  }
  if (lo === 0) return "  not even the first line of the anchor appears in the target.";
  const found = prefix(lo);
  const at = src.indexOf(found);
  const line = lineAt(src, at) + found.split("\n").length - 1;
  const got = Array.from(src.slice(at + found.length).split("\n", 1)[0]!).slice(0, 70).join("");
  const want = Array.from(chars.slice(lo).join("").split("\n", 1)[0]!).slice(0, 70).join("");
  return (
    `  the anchor matches for its first ${lo} character(s), up to target line ${line}.\n` +
    `    the file has: ${pyRepr(got)}\n` +
    `    the anchor wants: ${pyRepr(want)}`
  );
}

/** Every block matched and replaced in memory, keyed by target in first-seen order; refuses on a miss. */
export function stage(blocks: Block[], load: (path: string) => string = read): Map<string, string> {
  const staged = new Map<string, string>();
  blocks.forEach((b, k) => {
    const n = k + 1;
    if (!staged.has(b.target)) staged.set(b.target, load(b.target));
    if (!b.old) refuse(`nv splice: block ${n} has an empty OLD section -- refusing`, 2);
    const probe = staged.get(b.target)!;
    const at: number[] = [];
    for (let m = probe.indexOf(b.old); m >= 0; m = probe.indexOf(b.old, m + 1)) at.push(m);
    if (at.length !== 1) {
      const label = blocks.length > 1 ? `block ${n} of ${blocks.length}` : "the anchor";
      if (at.length === 0) refuse(`nv splice: ${label} does not appear in ${b.target}.\n` + whereItDiverges(probe, b.old));
      refuse(`nv splice: ${label} appears ${at.length} times in ${b.target} (lines ${at.map((m) => lineAt(probe, m)).join(", ")}) -- make it unique`);
    }
    staged.set(b.target, probe.slice(0, at[0]) + b.new + probe.slice(at[0]! + b.old.length));
  });
  return staged;
}

function apply(args: string[]): number {
  const dry = args.includes("--dry-run");
  const rest = args.filter((a) => a !== "--dry-run");
  let blocks: Block[];
  if (rest[0] === "--patch") {
    if (rest.length !== 2) refuse("nv splice: --patch takes exactly one file", 2);
    blocks = parsePatch(read(rest[1]!), rest[1]!);
  } else if (rest.length >= 2 && rest[1] === "--patch") {
    if (rest.length !== 3) refuse("nv splice: --patch takes exactly one file", 2);
    blocks = parsePatch(read(rest[2]!), rest[2]!, rest[0]!);
  } else if (rest.length === 3) {
    blocks = [{ target: rest[0]!, old: read(rest[1]!), new: read(rest[2]!) }];
  } else {
    return refuse(`${FORMS}\n\n\`--help\` prints the patch format in full.`, 2);
  }

  const staged = stage(blocks);
  if (dry) {
    console.log(`nv splice: dry run -- all ${blocks.length} block(s) match exactly once across ${staged.size} file(s)`);
    return 0;
  }
  for (const [target, text] of staged) {
    const crlf = readFileSync(target, "utf8").includes("\r\n");
    writeFileSync(target, crlf ? text.replace(/\n/g, "\r\n") : text, "utf8");
  }
  // A patch leaves no trace of the files it reached on the session's event stream, so the ledger does.
  record(...staged.keys());
  const lines = [`nv splice: ${blocks.length} block(s) spliced into ${staged.size} file(s)`];
  for (const t of staged.keys()) {
    const n = blocks.filter((b) => b.target === t).length;
    lines.push(`  ${String(n).padStart(3)} block(s)  ${t}`);
  }
  console.log(lines.join("\n"));
  return 0;
}

export async function run(args: string[]): Promise<number> {
  if (args.some((a) => a === "-h" || a === "--help")) {
    console.log(HELP);
    return 0;
  }
  try {
    return apply(args);
  } catch (e) {
    if (!(e instanceof Refusal)) throw e;
    console.log(e.message);
    return e.code;
  }
}
