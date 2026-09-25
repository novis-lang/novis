// Whether a test binary can read a file outside its package without saying so through `nvs_repo`. A
// binary that can is **wide**: its key is every file in the tree, and `tools/data/impact-wide.txt`
// lists it with the reason. `verify`'s `test` step fails on a wide binary the list does not name, and
// on a listed one that is narrow now, so the list and the code cannot drift apart.
//
// The sources judged are the ones rustc's dep-info names for the binary. The ways out:
//
// - it starts a process that is not its package's own binary, which reads what it likes;
// - it reads or sets its own working directory, which a test binary starts in its package;
// - a string literal climbs out of the directory it starts in, or names `target/debug`. A bare `".."`
//   counts only where it is joined, pushed or sits beside the manifest directory;
// - it reads `CARGO_MANIFEST_DIR` and also holds a `..` segment in any literal, or climbs with
//   `.parent()`, `.ancestors()` or `.pop()`.
//
// A climbing literal that is data -- a path a pure function takes apart, an entry name in an archive
// built in memory, an escape the code under test is expected to refuse, a detour that stays inside
// the package -- opens nothing outside the package, and `impact-data-literals.txt` lists each one
// with the reason. No other way out can be listed there.
// A wide binary may not name a file a wrap writes, because its key leaves those out.
//
// Read off `scan` below, a lexer that separates comments and literals from code, so a comment is never
// matched and a literal never splits. A literal rustc opens itself (`include_str!`, `#[path]`) is in the
// dep-info already and is not a way out.

import { existsSync, readFileSync } from "node:fs";
import { isAbsolute, relative, resolve } from "node:path";
import { ROOT } from "../lib/paths.ts";
import { WRAP_WRITES } from "./partition.ts";

export const WIDE_LIST = "tools/data/impact-wide.txt";
export const DATA_LIST = "tools/data/impact-data-literals.txt";

const TOKEN =
  /(?<doc>\/\/(?:\/(?!\/)|!)[^\n]*)|(?<line>\/\/[^\n]*)|(?<block>\/\*)|(?<raw>(?<![A-Za-z0-9_])(?:b|c)?r(?<hashes>#*)"[\s\S]*?"\k<hashes>)|(?<str>(?:(?<![A-Za-z0-9_])(?:b|c))?"(?:\\[\s\S]|[^"\\])*")|(?<chr>(?:(?<![A-Za-z0-9_])b)?'(?:\\(?:u\{[^}\n]*\}|x[0-9a-fA-F]{2}|[^\n])|[^\\'\n])')/g;
const NEST = /\/\*|\*\//g;
const WORD = "[\\p{L}\\p{N}_]";
const NOT_WORD = "[^\\p{L}\\p{N}_]";
const GLUE = new RegExp(
  `(?<=${WORD}) (?=${NOT_WORD})|(?<=${NOT_WORD}) (?=${WORD})|(?<=[()\\[\\]{},;\\x01\\x02]) | (?=[()\\[\\]{},;\\x01\\x02])`,
  "gu",
);
const DOC_RUN = /\x01(?: ?\x01)+/g;

/** A `.rs` file's code with layout and comments removed, `\x01` for a run of doc comments and `\x02`
 * for each literal, and the literals in order. */
function scan(text: string): { code: string; literals: string[] } {
  const code: string[] = [];
  const literals: string[] = [];
  let pos = 0;
  for (;;) {
    TOKEN.lastIndex = pos;
    const m = TOKEN.exec(text);
    if (m === null) {
      code.push(text.slice(pos));
      break;
    }
    code.push(text.slice(pos, m.index));
    let end = m.index + m[0].length;
    const g = m.groups!;
    if (g.block !== undefined) {
      // Block comments nest, which no regular expression follows. An unclosed one runs to the end
      // of the file, as it does for rustc.
      let depth = 1;
      let at = end;
      while (depth > 0) {
        NEST.lastIndex = at;
        const n = NEST.exec(text);
        if (n === null) {
          at = text.length;
          break;
        }
        depth += n[0] === "/*" ? 1 : -1;
        at = n.index + 2;
      }
      end = at;
      const body = text.slice(m.index, end);
      const isDoc = body.startsWith("/*!") || (body.startsWith("/**") && !body.startsWith("/***") && !body.startsWith("/**/"));
      code.push(isDoc ? " \x01 " : " ");
    } else if (g.doc !== undefined) {
      code.push(" \x01 ");
    } else if (g.line !== undefined) {
      code.push(" ");
    } else {
      literals.push(m[0]);
      code.push("\x02");
    }
    pos = end;
  }
  const flat = code.join("").replace(/\s+/g, " ").trim().replace(GLUE, "");
  return { code: flat.replace(DOC_RUN, "\x01"), literals };
}

const SPAWN = /Command::new\(/g;
/** `Command::new(env!("CARGO_BIN_EXE_..."))`: the package's own binary, compiled from what the key holds. */
const OWN_BIN = /Command::new\(env!\(\x02\)\)/y;
const CLIMBS = /\.parent\(\)|\.ancestors\(\)|\.pop\(\)/;
/** The code in front of a literal rustc opens itself. */
const RUSTC_OPENS = /(?:include_str!\(|include_bytes!\(|include!\(|path=)$/;
const SEGMENT = /[\\/]/;
const JOINS = /(?:join|push|from|new)\($/;

/** A string literal's text without its prefix, hashes and quotes. */
function body(literal: string): string {
  const start = literal.indexOf('"');
  const end = literal.lastIndexOf('"');
  return start >= 0 && start < end ? literal.slice(start + 1, end) : "";
}

/** A literal's path segments. In a raw literal a backslash is a separator. In any other literal `\\` is
 * one, and a single backslash starts an escape such as `\n` or `\u{…}`, which is not. */
function segments(literal: string): string[] {
  const b = body(literal);
  return /^(?:b|c)?r/.test(literal) ? b.split(SEGMENT) : b.split("\\\\").join("/").split("/");
}

/** Does this relative path climb above the directory it starts in? An absolute one names nothing in
 * the tree, so it does not. */
function leaves(literal: string): boolean {
  const path = body(literal);
  if (!path || path[0] === "/" || path[0] === "\\" || path.slice(1, 2) === ":") return false;
  let depth = 0;
  for (const part of segments(literal)) {
    depth += part === ".." ? -1 : part === "" || part === "." ? 0 : 1;
    if (depth < 0) return true;
  }
  return false;
}

export interface WayOut {
  how: string;
  /** The literal that climbs out of its directory, as the source writes it. Only this kind of way
   * out can be cleared, by a line in `DATA_LIST` that says the literal is data and opens nothing. */
  climbs?: string;
}

/** Every way this source can reach a file outside its package without `nvs_repo`. */
export function waysOut(text: string): WayOut[] {
  const { code, literals } = scan(text);
  const out: WayOut[] = [];
  const count = (s: string) => s.split("\x02").length - 1;
  for (const m of code.matchAll(SPAWN)) {
    const index = count(code.slice(0, m.index));
    OWN_BIN.lastIndex = m.index;
    const own = OWN_BIN.test(code) && index < literals.length;
    if (!(own && literals[index]!.includes("CARGO_BIN_EXE_"))) {
      out.push({ how: "starts a process with `Command::new`" });
      break;
    }
  }
  // `Command::current_dir(dir)` takes an argument and moves a child, which was judged above.
  if (code.includes("set_current_dir") || code.includes("current_dir()")) out.push({ how: "reads or sets the working directory" });
  const rustcOpens = new Set<number>();
  const starts: number[] = [];
  for (let at = code.indexOf("\x02"); at >= 0; at = code.indexOf("\x02", at + 1)) {
    if (RUSTC_OPENS.test(code.slice(Math.max(0, at - 20), at))) rustcOpens.add(starts.length);
    starts.push(at);
  }
  if (starts.length !== literals.length) return [...out, { how: "its literals could not be placed" }];
  const manifest = literals.some((l) => l.includes("CARGO_MANIFEST_DIR"));
  for (const [index, lit] of literals.entries()) {
    const b = body(lit);
    if (rustcOpens.has(index) || !b) continue;
    if (b.includes("target/debug") || b.includes("target\\\\debug")) out.push({ how: `names the target directory in ${lit.slice(0, 60)}` });
    const at = starts[index]!;
    const bare = b === ".." && !manifest && !JOINS.test(code.slice(Math.max(0, at - 12), at));
    if (leaves(lit) && !bare) out.push({ how: `the literal ${lit.slice(0, 60)} climbs out of its directory`, climbs: lit });
    if (manifest && segments(lit).includes("..")) out.push({ how: `\`CARGO_MANIFEST_DIR\` beside the literal ${lit.slice(0, 60)}` });
  }
  if (manifest && CLIMBS.test(code)) out.push({ how: "`CARGO_MANIFEST_DIR` beside `.parent()`, `.ancestors()` or `.pop()`" });
  return out;
}

/** How this source can reach a file outside its package without `nvs_repo`, or "". A climbing
 * literal in `data` is not a way out. */
export function wayOut(text: string, data: ReadonlySet<string> = new Set()): string {
  return waysOut(text).find((w) => w.climbs === undefined || !data.has(w.climbs))?.how ?? "";
}

/** The bodies of this source's string literals, with `\\` read as `/`. */
export function literalPaths(text: string): string[] {
  return scan(text).literals.map((l) => segments(l).join("/"));
}

/** The sources rustc compiled the executable at `exe` from, repo-relative, or `null` when its dep-info
 * is missing or unreadable. The file sits beside the executable under the same stem. A path outside
 * the repository is a registry crate, which `Cargo.lock` stands for, and is left out. */
export function depInfo(exe: string): string[] | null {
  const path = exe.replace(/\.[^./\\]*$/, "") + ".d";
  let text: string;
  try {
    text = readFileSync(path, "utf8");
  } catch {
    return null;
  }
  const head = text.split("\n\n")[0]!.split("\\\n").join(" ");
  const colon = head.indexOf(": ");
  const tail = colon < 0 ? "" : head.slice(colon + 2).trim();
  const out: string[] = [];
  for (const raw of tail.split(/(?<!\\) /)) {
    if (!raw) continue;
    const full = resolve(ROOT, raw.split("\\ ").join(" "));
    const rel = relative(ROOT, full);
    if (rel.startsWith("..") || isAbsolute(rel)) continue;
    out.push(rel.replace(/\\/g, "/"));
  }
  return out.length > 0 ? out : null;
}

/** The first of these sources with a way out, as `file: how`, or "". One that cannot be read counts.
 * A climbing literal `data` names for its file is not a way out. */
export function escapes(sources: string[], data: Map<string, Set<string>> = dataLiterals()): string {
  for (const rel of sources) {
    if (!rel.endsWith(".rs")) continue;
    let text: string;
    try {
      text = readFileSync(resolve(ROOT, rel), "utf8");
    } catch {
      return `${rel}: could not be read`;
    }
    const how = wayOut(text, data.get(rel));
    if (how) return `${rel}: ${how}`;
  }
  return "";
}

/** The binaries the wide list names. */
export function allowedWide(): Set<string> {
  const path = resolve(ROOT, WIDE_LIST);
  if (!existsSync(path)) return new Set();
  const out = new Set<string>();
  for (const line of readFileSync(path, "utf8").split(/\r?\n/)) {
    if (line.trim() && !line.startsWith("#")) out.add(line.split("  --  ")[0]!.trim());
  }
  return out;
}

/** The climbing literals `DATA_LIST` clears, by source file, each written as the source writes it. */
export function dataLiterals(): Map<string, Set<string>> {
  const path = resolve(ROOT, DATA_LIST);
  const out = new Map<string, Set<string>>();
  if (!existsSync(path)) return out;
  for (const line of readFileSync(path, "utf8").split(/\r?\n/)) {
    if (!line.trim() || line.startsWith("#")) continue;
    const m = /^(\S+)\s+(.+)$/.exec(line.split("  --  ")[0]!.trim());
    if (!m) continue;
    let set = out.get(m[1]!);
    if (!set) out.set(m[1]!, (set = new Set()));
    set.add(m[2]!);
  }
  return out;
}

/** A line of `DATA_LIST` whose literal no longer climbs out of its file, one finding each. */
function staleData(data: Map<string, Set<string>>): string[] {
  const out: string[] = [];
  for (const [rel, literals] of [...data].sort()) {
    let text = "";
    try {
      text = readFileSync(resolve(ROOT, rel), "utf8");
    } catch {
      // A file that is gone climbs nowhere: every line naming it is stale.
    }
    const climbs = new Set(waysOut(text).flatMap((w) => (w.climbs === undefined ? [] : [w.climbs])));
    for (const lit of [...literals].sort()) {
      if (!climbs.has(lit)) out.push(`${rel}  ${lit} is listed in ${DATA_LIST} and no longer climbs out of its directory there: delete its line.`);
    }
  }
  return out;
}

/** A file a wrap writes that this wide binary's sources name, as `file: literal`, or "". A directory
 * above one counts from two segments up: a bare `"data"` or `"docs"` is a word far more often than a
 * path, as the `data` field of a server-sent event is. */
function namesWrapWritten(sources: string[]): string {
  for (const rel of sources) {
    if (!rel.endsWith(".rs")) continue;
    let text: string;
    try {
      text = readFileSync(resolve(ROOT, rel), "utf8");
    } catch {
      continue;
    }
    const hit = literalPaths(text).find((p) => WRAP_WRITES.some((w) => p === w || p.startsWith(`${w}/`) || (p.includes("/") && w.startsWith(`${p}/`))));
    if (hit !== undefined) return `${rel}: "${hit}"`;
  }
  return "";
}

export interface Judged {
  name: string;
  owner: string;
  exe: string;
  /** Its recorded run-time reads, or `undefined` when no run has recorded any. */
  reads: string[] | undefined;
  /** Is `owner` a workspace package? */
  known: boolean;
}

/** Why a binary is wide, "" when it is narrow, or `null` when it cannot be judged yet: it reads
 * through `nvs_repo` and no run has recorded what, which the run asking is about to do. */
export function wideWhy(b: Judged): string | null {
  if (!b.known) return `no workspace package is named ${JSON.stringify(b.owner)}`;
  const sources = depInfo(b.exe);
  if (sources === null) return "its dep-info could not be read";
  const how = escapes(sources);
  if (how) return `leaves its package without \`nvs_repo\` -- ${how}`;
  if (b.reads?.includes(".")) return "it asked `nvs_repo::root` for the whole tree";
  if (b.reads === undefined) {
    for (const rel of sources) {
      try {
        if (rel.endsWith(".rs") && /\bnvs_repo::/.test(readFileSync(resolve(ROOT, rel), "utf8"))) return null;
      } catch {
        // Unreadable sources were judged by `escapes` above.
      }
    }
  }
  return "";
}

/** What the `test` step fails on: a wide binary the list does not name, and a listed one that is narrow
 * or is no test binary of this build. */
export function findings(binaries: Judged[]): string[] {
  const allowed = allowedWide();
  const out: string[] = [];
  const wide = new Set<string>();
  const pending = new Set<string>();
  for (const b of binaries) {
    const why = wideWhy(b);
    if (why === null) {
      pending.add(b.name);
      continue;
    }
    if (!why) continue;
    wide.add(b.name);
    if (!allowed.has(b.name)) {
      out.push(
        `\`${b.name}\` ${why}.\n    Reach the file through \`nvs_repo::path\`, or a child process through ` +
          `\`nvs_repo::spawn\`, so the binary is run when what it reads changes and not on every change. A ` +
          `climbing literal that is data and opens nothing gets a line in ${DATA_LIST}. If the binary ` +
          `cannot be narrow, give it a line in ${WIDE_LIST}.`,
      );
    }
    const wrap = namesWrapWritten(depInfo(b.exe) ?? []);
    if (wrap) {
      out.push(
        `\`${b.name}\` is wide and names a file a wrap writes -- ${wrap}.\n    A wide binary's key leaves ` +
          `out what a wrap writes (\`WRAP_WRITES\` in tools/nv/keys/partition.ts), so this read would go ` +
          `unkeyed. Make the binary narrow and reach the file through \`nvs_repo::path\`.`,
      );
    }
  }
  out.push(...staleData(dataLiterals()));
  const names = new Set(binaries.map((b) => b.name));
  for (const name of [...allowed].sort()) {
    if (wide.has(name) || pending.has(name)) continue;
    const state = names.has(name) ? "has a narrow key now" : "is not a test binary of this build";
    out.push(`\`${name}\` is listed in ${WIDE_LIST} and ${state}: delete its line.`);
  }
  return out;
}
