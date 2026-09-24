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
// Read off `scan`, so a comment is never matched and a literal never splits. A literal rustc opens
// itself (`include_str!`, `#[path]`) is in the dep-info already and is not a way out.

import { existsSync, readFileSync } from "node:fs";
import { isAbsolute, relative, resolve } from "node:path";
import { ROOT } from "../lib/paths.ts";
import { scan } from "./scan.ts";

export const WIDE_LIST = "tools/data/impact-wide.txt";

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

/** Does this relative path climb above the directory it starts in? An absolute one names nothing in
 * the tree, so it does not. */
function leaves(path: string): boolean {
  if (!path || path[0] === "/" || path[0] === "\\" || path.slice(1, 2) === ":") return false;
  let depth = 0;
  for (const part of path.split("\\\\").join("/").split(SEGMENT)) {
    depth += part === ".." ? -1 : part === "" || part === "." ? 0 : 1;
    if (depth < 0) return true;
  }
  return false;
}

/** How this source can reach a file outside its package without `nvs_repo`, or "". */
export function wayOut(text: string): string {
  const { code, literals } = scan(text);
  const count = (s: string) => s.split("\x02").length - 1;
  for (const m of code.matchAll(SPAWN)) {
    const index = count(code.slice(0, m.index));
    OWN_BIN.lastIndex = m.index;
    const own = OWN_BIN.test(code) && index < literals.length;
    if (!(own && literals[index]!.includes("CARGO_BIN_EXE_"))) return "starts a process with `Command::new`";
  }
  // `Command::current_dir(dir)` takes an argument and moves a child, which was judged above.
  if (code.includes("set_current_dir") || code.includes("current_dir()")) return "reads or sets the working directory";
  const rustcOpens = new Set<number>();
  const starts: number[] = [];
  for (let at = code.indexOf("\x02"); at >= 0; at = code.indexOf("\x02", at + 1)) {
    if (RUSTC_OPENS.test(code.slice(Math.max(0, at - 20), at))) rustcOpens.add(starts.length);
    starts.push(at);
  }
  if (starts.length !== literals.length) return "its literals could not be placed";
  const manifest = literals.some((l) => l.includes("CARGO_MANIFEST_DIR"));
  for (const [index, lit] of literals.entries()) {
    const b = body(lit);
    if (rustcOpens.has(index) || !b) continue;
    if (b.includes("target/debug") || b.includes("target\\\\debug")) return `names the target directory in ${lit.slice(0, 60)}`;
    const at = starts[index]!;
    const bare = b === ".." && !manifest && !JOINS.test(code.slice(Math.max(0, at - 12), at));
    if (leaves(b) && !bare) return `the literal ${lit.slice(0, 60)} climbs out of its directory`;
    if (manifest && b.split("\\\\").join("/").split(SEGMENT).includes("..")) return `\`CARGO_MANIFEST_DIR\` beside the literal ${lit.slice(0, 60)}`;
  }
  if (manifest && CLIMBS.test(code)) return "`CARGO_MANIFEST_DIR` beside `.parent()`, `.ancestors()` or `.pop()`";
  return "";
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

/** The first of these sources with a way out, as `file: how`, or "". One that cannot be read counts. */
export function escapes(sources: string[]): string {
  for (const rel of sources) {
    if (!rel.endsWith(".rs")) continue;
    let text: string;
    try {
      text = readFileSync(resolve(ROOT, rel), "utf8");
    } catch {
      return `${rel}: could not be read`;
    }
    const how = wayOut(text);
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
          `\`nvs_repo::spawn\`, so the binary is run when what it reads changes and not on every change. If it ` +
          `cannot be narrow, give it a line in ${WIDE_LIST}.`,
      );
    }
  }
  const names = new Set(binaries.map((b) => b.name));
  for (const name of [...allowed].sort()) {
    if (wide.has(name) || pending.has(name)) continue;
    const state = names.has(name) ? "has a narrow key now" : "is not a test binary of this build";
    out.push(`\`${name}\` is listed in ${WIDE_LIST} and ${state}: delete its line.`);
  }
  return out;
}
