import { afterAll, describe, expect, test } from "bun:test";
import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { type FileItems, scanItems } from "../keys/scan.ts";
import { closure, diffFile, Universe } from "../select/items.ts";
import { PROFILE_ONLY } from "../select/keys.ts";
import { cfgValue, leadingAttrs, optimizedOnly, profileReader, type Side } from "../select/profile.ts";
import { scratch } from "./scratch.ts";

const tree = scratch();
afterAll(() => tree.cleanup());

const LIB = `
mod plain;
// The pool is compiled only where it is used.
#[cfg(any(test, not(debug_assertions)))]
mod pool;
`;

const BACKING = `
/// What the counter forwards to.
#[cfg(all(not(test), not(debug_assertions), not(feature = "sanitizer")))]
use crate::pool::Pooled as Backing;
#[cfg(all(not(test), any(debug_assertions, feature = "sanitizer")))]
use std::alloc::System as Backing;

pub fn allocate() -> usize {
    Backing::size()
}

#[cfg(not(debug_assertions))]
pub fn fast() -> u8 {
    1
}

#[cfg(debug_assertions)]
pub fn checked() -> u8 {
    2
}
`;

/** Every file of the tree scanned together, by path. */
function scanned(files: string[]): Map<string, FileItems> {
  return new Map(scanItems(files, tree.root).map((f) => [f.file, f]));
}

/** The keys an edit of `file` from `before` to `after` moves, with the profile-only test on. */
function movedBy(file: string, before: string, after: string): string[] {
  tree.put("crates/demo/src/lib.rs", LIB);
  tree.put(file, before);
  const base = scanned([file]);
  tree.put(file, after);
  const head = scanned([file]);
  const read = (f: string, side: Side) => {
    if (side === "base" && f === file) return before;
    const full = join(tree.root, f);
    return existsSync(full) ? readFileSync(full, "utf8") : null;
  };
  const d = diffFile(base.get(file)!, head.get(file)!);
  return [...closure(d.changes, new Universe(head), [], new Map(), profileReader(read, head)).keys()].sort();
}

describe("code only an optimized build compiles", () => {
  test("a cfg predicate is true, false, or unknown when it names anything else", () => {
    const debug = { debug_assertions: true, test: false };
    expect(cfgValue("not(debug_assertions)", debug)).toBe(false);
    expect(cfgValue("any(test, not(debug_assertions))", debug)).toBe(false);
    expect(cfgValue('all(not(test), not(debug_assertions), not(feature = "sanitizer"))', debug)).toBe(false);
    expect(cfgValue('all(not(test), not(debug_assertions), not(feature = "sanitizer"))', { debug_assertions: false, test: false })).toBe(null);
    expect(cfgValue("unix", debug)).toBe(null);
    expect(optimizedOnly(["not(debug_assertions)"])).toBe(true);
    expect(optimizedOnly(["debug_assertions"])).toBe(false);
    expect(optimizedOnly(["test"])).toBe(false);
    expect(optimizedOnly(["unix"])).toBe(false);
  });

  test("the attributes of an item are read past its doc comments, and end at its first token", () => {
    expect(leadingAttrs('/// Doc.\n#[cfg(not(debug_assertions))]\n#[expect(x, reason = "a ] b")]\nfn f() {}')).toEqual(["cfg(not(debug_assertions))", 'expect(x, reason = "a ] b")']);
    expect(leadingAttrs("fn f() { #[cfg(test)] let x = 1; }")).toEqual([]);
  });

  test("a changed profile-only item moves its debug twin and what names the twin", () => {
    const file = "crates/demo/src/backing.rs";
    const keys = movedBy(file, BACKING, BACKING.replace("crate::pool::Pooled", "crate::pool::Other"));
    expect(keys).toContain(`fn:${file}#use:Backing#2`);
    expect(keys).toContain(`fn:${file}#allocate`);
    expect(keys).not.toContain(PROFILE_ONLY);
  });

  test("a changed profile-only item with no twin selects every proof program", () => {
    const file = "crates/demo/src/backing.rs";
    expect(movedBy(file, BACKING, BACKING.replace("    1\n", "    3\n"))).toContain(PROFILE_ONLY);
  });

  test("a changed item a debug build compiles moves no more than its own keys", () => {
    const file = "crates/demo/src/backing.rs";
    expect(movedBy(file, BACKING, BACKING.replace("    2\n", "    4\n"))).toEqual([`fn:${file}#checked`]);
  });

  test("every item of a module only an optimized build compiles is profile-only", () => {
    const pool = "pub fn take() -> u8 {\n    1\n}\n";
    expect(movedBy("crates/demo/src/pool.rs", pool, pool.replace("1", "2"))).toContain(PROFILE_ONLY);
    expect(movedBy("crates/demo/src/plain.rs", pool, pool.replace("1", "2"))).not.toContain(PROFILE_ONLY);
  });

  test("code under a package's tests is never part of the proof binary", () => {
    const file = "crates/demo/tests/latency.rs";
    const text = "#[cfg(not(debug_assertions))]\nconst ONLY: f64 = 25.0;\n";
    expect(movedBy(file, text, text.replace("25.0", "30.0"))).not.toContain(PROFILE_ONLY);
  });
});
