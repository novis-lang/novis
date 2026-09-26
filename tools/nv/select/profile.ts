// Code only an optimized build compiles, which no footprint can hold: every footprint is recorded on
// the covws debug build, and a debug build sets `debug_assertions`.
//
// An item is profile-only when one of its `#[cfg(...)]` attributes is false with `debug_assertions` set
// and none is false without it, or when its `impl` is, or when its file is a module whose `mod`
// declaration is. `test` is unset for both, since the proof binary is no test build. A predicate that
// names anything else (a feature, a platform) counts as possibly true, so such an item counts as
// profile-only whenever a debug build leaves it out. The closure (`items.ts`) takes a changed
// profile-only item as a change to its twin, the item of the same file whose id differs only by the `#2`
// ordinal nv-scan gives a repeated id and that a debug build compiles; one with no twin moves
// `PROFILE_ONLY`, which selects every proof program.

import { existsSync, readFileSync } from "node:fs";
import { join, posix } from "node:path";
import type { FileItems, Item } from "../keys/scan.ts";
import { ROOT } from "../lib/paths.ts";

/** The side of a change a text is read from: the tree as it is, or the recorded base. */
export type Side = "head" | "base";

/** Whether an item of a file, on one side, is compiled only by an optimized build. */
export type ProfileOnly = (file: string, item: Item, side: Side) => boolean;

/** Nothing is profile-only: the closure's default, for a caller with no file text. */
export const NO_PROFILE: ProfileOnly = () => false;

interface CfgEnv {
  debug_assertions: boolean;
  test: boolean;
}

/** A `cfg` predicate's value under `env`: true, false, or null when it depends on anything else. */
export function cfgValue(pred: string, env: CfgEnv): boolean | null {
  const tokens = pred.match(/"(?:[^"\\]|\\.)*"|[A-Za-z_][A-Za-z0-9_]*|[(),=]/g) ?? [];
  let at = 0;
  const one = (): boolean | null => {
    const name = tokens[at++] ?? "";
    if (tokens[at] === "=") {
      at += 2;
      return null;
    }
    if (tokens[at] !== "(") return name === "debug_assertions" ? env.debug_assertions : name === "test" ? env.test : null;
    at++;
    const args: (boolean | null)[] = [];
    while (at < tokens.length && tokens[at] !== ")") {
      args.push(one());
      if (tokens[at] === ",") at++;
    }
    at++;
    if (name === "not") return args[0] === null || args[0] === undefined ? null : !args[0];
    if (name === "all") return args.includes(false) ? false : args.every((a) => a === true) ? true : null;
    if (name === "any") return args.includes(true) ? true : args.every((a) => a === false) ? false : null;
    return null;
  };
  return one();
}

/** The attributes at the head of `text`, each as the text between `#[` and its `]`, past doc comments,
 * plain comments and blank space; the first token of anything else ends them. */
export function leadingAttrs(text: string): string[] {
  const out: string[] = [];
  let i = 0;
  while (i < text.length) {
    const c = text[i]!;
    if (/\s/.test(c)) i++;
    else if (text.startsWith("//", i)) i = text.indexOf("\n", i) < 0 ? text.length : text.indexOf("\n", i);
    else if (text.startsWith("/*", i)) i = text.indexOf("*/", i) < 0 ? text.length : text.indexOf("*/", i) + 2;
    else if (text.startsWith("#[", i) || text.startsWith("#![", i)) {
      const open = text.indexOf("[", i);
      let depth = 0;
      let j = open;
      for (; j < text.length; j++) {
        const d = text[j]!;
        if (d === '"') {
          j++;
          while (j < text.length && text[j] !== '"') j += text[j] === "\\" ? 2 : 1;
        } else if (d === "[") depth++;
        else if (d === "]" && --depth === 0) break;
      }
      out.push(text.slice(open + 1, j).trim());
      i = j + 1;
    } else break;
  }
  return out;
}

/** The predicate of each `cfg(...)` attribute among `attrs`. */
function cfgsOf(attrs: string[]): string[] {
  return attrs.filter((a) => /^cfg\s*\(/.test(a)).map((a) => a.slice(a.indexOf("(") + 1, a.lastIndexOf(")")));
}

/** Whether attributes with these `cfg` predicates leave the code out of a debug build and keep it in an
 * optimized one. */
export function optimizedOnly(cfgs: string[]): boolean {
  const compiled = (debug: boolean) => {
    const values = cfgs.map((p) => cfgValue(p, { debug_assertions: debug, test: false }));
    return !values.includes(false);
  };
  return !compiled(true) && compiled(false);
}

/** The text of the header above line `line` (1-based): the attribute and comment lines that lead into
 * it, from the line after the last one that ends an item, a block or a statement. */
function headerAbove(lines: string[], line: number): string {
  let top = line - 1;
  while (top > 0) {
    const prev = lines[top - 1]!.trim();
    if (prev === "" || /[;{}]$/.test(prev)) break;
    top--;
  }
  return lines.slice(top, line).join("\n");
}

/** The name the `mod` declaration of `file` spells, and the files that can hold that declaration. Null
 * for a crate root or a build script. */
function declarers(file: string): { name: string; parents: string[] } | null {
  const { basename, dirname, join: at } = posix;
  const stem = basename(file, ".rs");
  if (stem === "lib" || stem === "main" || stem === "build") return null;
  const dir = stem === "mod" ? dirname(dirname(file)) : dirname(file);
  const name = stem === "mod" ? basename(dirname(file)) : stem;
  const own = ["mod.rs", "lib.rs", "main.rs"].map((f) => at(dir, f));
  return { name, parents: dir === "." ? own : [...own, `${at(dirname(dir), basename(dir))}.rs`] };
}

/**
 * The profile-only test over the files of `view`, reading each file's text on either side with `read`.
 * Every answer is kept, so a file is read and an item judged once.
 */
export function profileReader(read: (file: string, side: Side) => string | null, view: Map<string, FileItems> = new Map()): ProfileOnly {
  const texts = new Map<string, string[] | null>();
  const lines = (file: string, side: Side) => {
    const k = `${side}\0${file}`;
    if (!texts.has(k)) texts.set(k, read(file, side)?.replace(/\r\n/g, "\n").split("\n") ?? null);
    return texts.get(k)!;
  };
  const files = new Map<string, boolean>();
  const fileOnly = (file: string, side: Side): boolean => {
    const k = `${side}\0${file}`;
    if (files.has(k)) return files.get(k)!;
    files.set(k, false);
    let only = false;
    const d = declarers(file);
    for (const parent of d?.parents ?? []) {
      const text = lines(parent, side);
      if (!text) continue;
      const decl = new RegExp(`^\\s*(?:pub(?:\\s*\\([^)]*\\))?\\s+)?mod\\s+${d!.name}\\s*;`);
      const at = text.findIndex((l) => decl.test(l));
      if (at < 0) continue;
      only = optimizedOnly(cfgsOf(leadingAttrs(headerAbove(text, at + 1)))) || fileOnly(parent, side);
      break;
    }
    files.set(k, only);
    return only;
  };
  const items = new Map<string, boolean>();
  const itemOnly = (file: string, item: Item, side: Side): boolean => {
    const k = `${side}\0${file}#${item.id}`;
    if (items.has(k)) return items.get(k)!;
    items.set(k, false);
    const text = lines(file, side);
    let only = text !== null && optimizedOnly(cfgsOf(leadingAttrs(text.slice(item.start - 1, item.end).join("\n"))));
    if (!only && item.parent && side === "head") {
      const parent = view.get(file)?.items.find((i) => i.id === item.parent);
      if (parent) only = itemOnly(file, parent, side);
    }
    only ||= fileOnly(file, side);
    items.set(k, only);
    return only;
  };
  return itemOnly;
}

/** The text of `file` on each side: the working tree's for `head`, and commit `since`'s for `base`; or
 * commit `until`'s for `head` in a change replayed from history. */
export function gitTexts(since: string, until: string | null, root: string = ROOT): (file: string, side: Side) => string | null {
  const at = (rev: string, file: string) => {
    const r = Bun.spawnSync(["git", "show", `${rev}:${file}`], { cwd: root, stdout: "pipe", stderr: "ignore" });
    return r.exitCode === 0 ? r.stdout.toString() : null;
  };
  return (file, side) => {
    if (side === "base") return at(since, file);
    if (until) return at(until, file);
    const full = join(root, file);
    return existsSync(full) ? readFileSync(full, "utf8") : null;
  };
}

/** The id an item has without nv-scan's `#N` ordinal. */
export const baseId = (id: string) => id.replace(/#\d+$/, "");

/** Whether `file` can be part of the proof binary at all: code under a package's `tests`, `benches` or
 * `examples` never is. */
export const shipsIn = (file: string) => !/(^|\/)(tests|benches|examples)\//.test(file);
