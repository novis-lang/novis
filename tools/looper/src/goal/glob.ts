// The globs a goal file and `looper.toml` name, read as `Bun.Glob` reads them, as
// `crates/looper-core/src/glob.rs` reads them. `*` and `?` stay within one path segment, `**` spans any
// number of segments, `[…]` and `[!…]` are classes and `{a,b}` is an alternation. Every path is
// relative to the project root and uses `/`.
//
// `expand` matches a pattern's regex against the files under its literal prefix, the directories before
// its first glob character, so `crates/**/*.rs` walks `crates/` only. An entry whose name starts with
// `.` is skipped, as `Bun.Glob` skips it without its `dot` option. Symbolic links are not followed.

import { type Dirent, readdirSync } from "node:fs";
import { join } from "node:path";

/** Whether the pattern holds a glob character and is expanded rather than taken as a path. */
export function isGlob(pattern: string): boolean {
  return /[*?[{]/.test(pattern);
}

/** The files under `root` that the pattern matches, as sorted root-relative paths. */
export function expand(root: string, pattern: string): string[] {
  const re = toRegex(pattern);
  if (re === undefined) return [];
  const segments = pattern.split("/");
  let literal = 0;
  while (literal < segments.length - 1 && !isGlob(segments[literal] as string)) literal++;
  const found: string[] = [];
  const dirs = [segments.slice(0, literal).join("/")];
  for (let dir = dirs.pop(); dir !== undefined; dir = dirs.pop()) {
    let entries: Dirent[];
    try {
      entries = readdirSync(join(root, dir), { withFileTypes: true });
    } catch {
      continue;
    }
    for (const entry of entries) {
      if (entry.name.startsWith(".")) continue;
      const rel = dir === "" ? entry.name : `${dir}/${entry.name}`;
      if (entry.isDirectory()) dirs.push(rel);
      else if (entry.isFile() && re.test(rel)) found.push(rel);
    }
  }
  return found.sort();
}

/** The pattern as an anchored regex, or `undefined` when it does not form one, such as an unclosed class. */
export function toRegex(pattern: string): RegExp | undefined {
  const chars = [...pattern];
  let re = "^";
  let braces = 0;
  for (let i = 0; i < chars.length; i++) {
    const c = chars[i] as string;
    if (c === "*" && chars[i + 1] === "*") {
      const atStart = i === 0 || chars[i - 1] === "/";
      if (atStart && chars[i + 2] === "/") {
        re += "(?:[^/]*/)*";
        i += 2;
      } else if (atStart && i + 2 === chars.length) {
        re += ".*";
        i += 1;
      } else {
        re += "[^/]*";
        i += 1;
      }
    } else if (c === "*") {
      re += "[^/]*";
    } else if (c === "?") {
      re += "[^/]";
    } else if (c === "[") {
      const close = chars.indexOf("]", i + 1);
      if (close < 0) return undefined;
      let cls = chars.slice(i + 1, close).join("");
      if (cls.startsWith("!")) cls = `^${cls.slice(1)}`;
      re += `[${cls.replaceAll("\\", "\\\\").replaceAll("[", "\\[")}]`;
      i = close;
    } else if (c === "{") {
      braces++;
      re += "(?:";
    } else if (c === "," && braces > 0) {
      re += "|";
    } else if (c === "}" && braces > 0) {
      braces--;
      re += ")";
    } else {
      re += c.replace(/[\\^$.*+?()[\]{}|/]/g, "\\$&");
    }
  }
  if (braces > 0) return undefined;
  try {
    return new RegExp(`${re}$`);
  } catch {
    return undefined;
  }
}
