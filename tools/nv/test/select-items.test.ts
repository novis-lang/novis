import { afterAll, describe, expect, test } from "bun:test";
import { writeFileSync } from "node:fs";
import { join } from "node:path";
import type { Graph } from "../keys/graph.ts";
import { type FileItems, scanItems } from "../keys/scan.ts";
import { buildScripts, generatedIncludes, isInput, parseOutput } from "../select/build.ts";
import { diffFile, referenceWalk, rowClasses, type Scope, Universe } from "../select/items.ts";
import { pathKeys } from "../select/keys.ts";
import { graphScope } from "../select/select.ts";
import { scratch } from "./scratch.ts";

const tree = scratch();
afterAll(() => tree.cleanup());

const REGISTRY = `
pub struct ClassDoc { pub short: &'static str }
pub struct CoreClass { pub name: &'static str, pub flags: u8, pub doc: Option<&'static ClassDoc> }

/// The card.
pub const CARD: ClassDoc = ClassDoc { short: "Numbers." };
pub const MATH: CoreClass = CoreClass { name: "Core\\\\Math", flags: 0, doc: Some(&CARD) };
pub const CLASSES: &[CoreClass] = &[
    MATH,
    CoreClass { name: "Core\\\\Str", flags: 0, doc: None },
    CoreClass { name: "Core\\\\Arr", flags: 0, doc: None },
];

pub fn class(name: &str) -> Option<&'static CoreClass> {
    CLASSES.iter().find(|c| c.name == name)
}

pub fn math() -> &'static CoreClass {
    &MATH
}
`;

/** A class of its own file, which imports what it builds from the registry. */
const SCRIPT = `use crate::registry::CoreClass;

pub const CLASS: CoreClass = CoreClass { name: "Core\\\\Script", flags: 0, doc: None };

pub fn script() -> &'static CoreClass {
    &CLASS
}
`;

/** SCRIPT given a card: the card type imported, the card added, and the row linking it. */
const SCRIPT_CARDED = SCRIPT.replace("use crate::registry::CoreClass;", "use crate::registry::{ClassDoc, CoreClass};\n\nconst CARD: ClassDoc = ClassDoc { short: \"Scripts.\" };").replace("doc: None", "doc: Some(&CARD)");

const LIB = `
pub const LIMIT: usize = 4;
pub const TABLE: &[usize] = &[LIMIT, 8];
pub struct Point { pub x: i64 }

macro_rules! twice {
    ($e:expr) => { $e * 2 };
}

pub fn lookup(i: usize) -> usize {
    TABLE[i]
}

pub fn make() -> Point {
    Point { x: 1 }
}

pub fn doubled() -> i64 {
    twice!(3)
}

pub fn plain() -> i64 {
    7
}

pub static BANNER: &str = include_str!("banner.txt");

pub fn banner() -> &'static str {
    BANNER
}
`;

/** Every file of the tree scanned together, by path. */
function scanned(files: string[]): Map<string, FileItems> {
  return new Map(scanItems(files, tree.root).map((f) => [f.file, f]));
}

/** The keys an edit of `file` from `before` to `after` moves. */
function movedBy(file: string, before: string, after: string, others: string[] = []): string[] {
  tree.put(file, before);
  const base = scanned([file, ...others]);
  tree.put(file, after);
  const head = scanned([file, ...others]);
  const d = diffFile(base.get(file)!, head.get(file)!);
  expect(d.wide).toBe(false);
  return [...referenceWalk(d.changes, new Universe(head)).keys()].sort();
}

describe("the item diff and the reference-graph walk", () => {
  tree.put("crates/demo/src/banner.txt", "hello\n");

  test("a fn body edit moves that fn and nothing else", () => {
    const moved = movedBy("crates/demo/src/lib.rs", LIB, LIB.replace("    7\n", "    8\n"));
    expect(moved).toEqual(["fn:crates/demo/src/lib.rs#plain"]);
  }, 180_000);

  test("a doc comment edit is no change", () => {
    const moved = movedBy("crates/demo/src/lib.rs", LIB, LIB.replace("pub fn plain", "/// Seven.\npub fn plain"));
    expect(moved).toEqual([]);
  });

  test("a const in a table moves the table and every fn that reads it", () => {
    const moved = movedBy("crates/demo/src/lib.rs", LIB, LIB.replace("LIMIT: usize = 4", "LIMIT: usize = 5"));
    expect(moved).toEqual(["fn:crates/demo/src/lib.rs#LIMIT", "fn:crates/demo/src/lib.rs#TABLE", "fn:crates/demo/src/lib.rs#lookup"]);
  });

  test("a struct field moves every item that names the struct", () => {
    const moved = movedBy("crates/demo/src/lib.rs", LIB, LIB.replace("pub x: i64 }", "pub x: i64, pub y: i64 }"));
    expect(moved).toEqual(["fn:crates/demo/src/lib.rs#Point", "fn:crates/demo/src/lib.rs#make"]);
  });

  test("a macro_rules! edit moves every item that invokes it", () => {
    const moved = movedBy("crates/demo/src/lib.rs", LIB, LIB.replace("$e * 2", "$e + $e"));
    expect(moved).toContain("fn:crates/demo/src/lib.rs#doubled");
    expect(moved).not.toContain("fn:crates/demo/src/lib.rs#plain");
  });

  test("an include_str! target moves the item that embeds it and what reads that", () => {
    tree.put("crates/demo/src/lib.rs", LIB);
    const base = scanned(["crates/demo/src/lib.rs"]);
    tree.put("crates/demo/src/banner.txt", "goodbye\n");
    const head = scanned(["crates/demo/src/lib.rs"]);
    const d = diffFile(base.get("crates/demo/src/lib.rs")!, head.get("crates/demo/src/lib.rs")!);
    expect([...referenceWalk(d.changes, new Universe(head)).keys()].sort()).toEqual(["fn:crates/demo/src/lib.rs#BANNER", "fn:crates/demo/src/lib.rs#banner"]);
  });

  test("a card edit moves the card's class under card: and no class or program key", () => {
    const moved = movedBy("crates/reg/src/registry.rs", REGISTRY, REGISTRY.replace('"Numbers."', '"Whole numbers."'));
    expect(moved).toEqual(["card:core\\math", "fn:crates/reg/src/registry.rs#CARD"]);
  });

  test("a registry row edit moves that row's class alone", () => {
    const moved = movedBy("crates/reg/src/registry.rs", REGISTRY, REGISTRY.replace('"Core\\\\Arr", flags: 0', '"Core\\\\Arr", flags: 1'));
    expect(moved).toEqual(["class:core\\arr", "fn:crates/reg/src/registry.rs#CLASSES"]);
  });

  test("a class item moves its class and the fns of its own file that name it", () => {
    const moved = movedBy("crates/reg/src/registry.rs", REGISTRY, REGISTRY.replace('"Core\\\\Math", flags: 0', '"Core\\\\Math", flags: 1'));
    expect(moved).toContain("class:core\\math");
    expect(moved).toContain("fn:crates/reg/src/registry.rs#math");
    expect(moved).not.toContain("class:core\\str");
  });

  test("a row or a class item that only links or unlinks a card moves that class's card and no class or program key", () => {
    const registry = "crates/reg/src/registry.rs";
    const row = movedBy(registry, REGISTRY, REGISTRY.replace('"Core\\\\Arr", flags: 0, doc: None', '"Core\\\\Arr", flags: 0, doc: Some(&CARD)'));
    expect(row).toEqual(["card:core\\arr", "fn:crates/reg/src/registry.rs#CLASSES"]);
    const item = movedBy(registry, REGISTRY, REGISTRY.replace("flags: 0, doc: Some(&CARD) };", "flags: 0, doc: None };"));
    expect(item).toEqual(["card:core\\math", "fn:crates/reg/src/registry.rs#MATH"]);
  });

  test("a card link that comes with any other edit of the row moves the class as before", () => {
    const moved = movedBy("crates/reg/src/registry.rs", REGISTRY, REGISTRY.replace("flags: 0, doc: Some(&CARD) };", "flags: 1, doc: None };"));
    expect(moved).toContain("class:core\\math");
    expect(moved).toContain("fn:crates/reg/src/registry.rs#math");
    const renamed = movedBy("crates/reg/src/registry.rs", REGISTRY, REGISTRY.replace('"Core\\\\Arr", flags: 0, doc: None', '"Core\\\\Set", flags: 0, doc: Some(&CARD)'));
    expect(renamed).toEqual(expect.arrayContaining(["class:core\\arr", "class:core\\set"]));
  });

  test("a new card, its import and its link reach the card's readers alone", () => {
    const moved = movedBy("crates/reg/src/script.rs", SCRIPT, SCRIPT_CARDED);
    expect(moved).toEqual(["card:core\\script", "fn:crates/reg/src/script.rs#CARD", "fn:crates/reg/src/script.rs#CLASS", "fn:crates/reg/src/script.rs#use:ClassDoc,CoreClass"]);
    const back = movedBy("crates/reg/src/script.rs", SCRIPT_CARDED, SCRIPT);
    expect(back).toEqual(["card:core\\script", "fn:crates/reg/src/script.rs#CARD", "fn:crates/reg/src/script.rs#CLASS", "fn:crates/reg/src/script.rs#use:CoreClass"]);
  });

  test("an import that changes more than its card types reaches what names it", () => {
    const moved = movedBy("crates/reg/src/script.rs", SCRIPT, SCRIPT_CARDED.replace("{ClassDoc, CoreClass}", "{ClassDoc, CoreClass, CoreTy}"));
    expect(moved).toContain("class:core\\script");
    expect(moved).toContain("fn:crates/reg/src/script.rs#script");
  });

  test("a class added or taken out with its card moves its class", () => {
    const registry = "crates/reg/src/registry.rs";
    const withSet = REGISTRY.replace("];", '    CoreClass { name: "Core\\\\Set", flags: 0, doc: Some(&CARD) },\n];');
    expect(movedBy(registry, REGISTRY, withSet)).toContain("class:core\\set");
    expect(movedBy(registry, withSet, REGISTRY)).toContain("class:core\\set");
    const added = movedBy("crates/reg/src/script.rs", "pub fn other() {}\n", SCRIPT_CARDED);
    expect(added).toContain("class:core\\script");
    expect(added).toContain("card:core\\script");
    const removed = movedBy("crates/reg/src/script.rs", SCRIPT_CARDED, "pub fn other() {}\n");
    expect(removed).toContain("class:core\\script");
  });

  test("an item of a package another does not depend on is not reached through a shared name", () => {
    const a = "crates/a/src/lib.rs";
    const b = "crates/b/src/lib.rs";
    tree.put(a, "pub const SHARED: u8 = 1;\n");
    tree.put(b, "pub fn reads() -> u8 { SHARED }\n");
    const base = scanned([a, b]);
    tree.put(a, "pub const SHARED: u8 = 2;\n");
    const head = scanned([a, b]);
    const d = diffFile(base.get(a)!, head.get(a)!);
    const apart: Scope = { pkgOf: (f) => f.split("/")[1]!, sees: (user, owner) => user === owner, modDir: () => null };
    expect([...referenceWalk(d.changes, new Universe(head, apart)).keys()]).toEqual(["fn:crates/a/src/lib.rs#SHARED"]);
    const depends: Scope = { pkgOf: (f) => f.split("/")[1]!, sees: () => true, modDir: () => null };
    expect([...referenceWalk(d.changes, new Universe(head, depends)).keys()]).toContain("fn:crates/b/src/lib.rs#reads");
  });

  test("a private item is reached from its own module's files, a pub(crate) one from its package, and vendored code from neither", () => {
    const regex = "crates/std/src/regex.rs";
    const child = "crates/std/src/regex/tier.rs";
    const sibling = "crates/std/src/http.rs";
    const other = "crates/app/src/lib.rs";
    const vendored = "vendor/lib/src/lib.rs";
    tree.put(regex, "enum Piece { A }\npub(crate) struct Tier;\n");
    tree.put(child, "fn piece() -> super::Piece { super::Piece::A }\n");
    tree.put(sibling, "struct Piece;\nfn tier() -> Tier { Tier }\nfn own() -> Piece { Piece }\n");
    tree.put(other, "pub fn tier() -> Tier { Tier }\n");
    tree.put(vendored, "pub fn piece(p: Piece) {}\n");
    const files = [regex, child, sibling, other, vendored];
    const base = scanned(files);
    tree.put(regex, "enum Piece { A, B }\npub(crate) struct Tier(u8);\n");
    const head = scanned(files);
    expect(head.get(regex)!.items.map((i) => i.scope)).toEqual(["private", "crate"]);
    const d = diffFile(base.get(regex)!, head.get(regex)!);
    const pkg = (f: string) => (f.startsWith("vendor/") ? null : f.split("/")[1]!);
    const scope: Scope = { pkgOf: pkg, sees: () => true, modDir: (f) => `${f.replace(/\.rs$/, "")}/` };
    const moved = [...referenceWalk(d.changes, new Universe(head, scope)).keys()];
    expect(moved).toContain(`fn:${child}#piece`);
    expect(moved).toContain(`fn:${sibling}#tier`);
    expect(moved).not.toContain(`fn:${sibling}#own`);
    expect(moved).not.toContain(`fn:${other}#tier`);
    expect(moved).not.toContain(`fn:${vendored}#piece`);
  });

  /** The keys the edit of `owner` from `before` to `after` moves, over `files` scanned in a workspace of
   * `pkgs`, each `[name, deps]` at `crates/<name>`. */
  function reachedIn(pkgs: [string, [string, "normal" | "dev"][]][], files: Record<string, string>, owner: string, before: string, after: string): string[] {
    const graph: Graph = new Map(
      pkgs.map(([name, deps]) => [name, { name, dir: `crates/${name}`, deps: new Map(deps), targets: [{ kind: "lib", name, src: `crates/${name}/src/lib.rs`, test: true }] }]),
    );
    for (const [f, text] of Object.entries(files)) tree.put(f, text);
    const all = [owner, ...Object.keys(files)];
    tree.put(owner, before);
    const base = scanned(all);
    tree.put(owner, after);
    const head = scanned(all);
    const d = diffFile(base.get(owner)!, head.get(owner)!);
    return [...referenceWalk(d.changes, new Universe(head, graphScope(graph))).keys()];
  }

  test("shipped code does not see a dev-dependency, and its tests and integration tests do", () => {
    const moved = reachedIn(
      [["t", []], ["s", [["t", "dev"]]]],
      {
        "crates/s/src/mail.rs": "use t::Wire;\npub fn send(w: &Wire) {}\n#[cfg(test)]\nmod tests {\n    use t::Wire;\n    #[test]\n    fn sends() { let _: Option<Wire> = None; }\n}\n",
        "crates/s/tests/mail.rs": "use t::Wire;\n#[test]\nfn it() { let _: Option<Wire> = None; }\n",
      },
      "crates/t/src/lib.rs",
      "pub struct Wire { pub a: u8 }\n",
      "pub struct Wire { pub a: u16 }\n",
    );
    expect(moved).not.toContain("fn:crates/s/src/mail.rs#send");
    expect(moved).toContain("fn:crates/s/src/mail.rs#tests::sends");
    expect(moved).toContain("fn:crates/s/tests/mail.rs#it");
  });

  test("another package reaches an item only where it imports or qualifies it, directly or through a re-export", () => {
    const moved = reachedIn(
      [["a", []], ["r", [["a", "normal"]]], ["u", [["a", "normal"], ["r", "normal"]]]],
      {
        "crates/r/src/lib.rs": "pub use a::Call;\n",
        "crates/u/src/lib.rs": "pub use a::Call as Again;\n",
        "crates/u/src/used.rs": "use a::Call;\npub fn used() -> Call { todo!() }\n",
        "crates/u/src/qualified.rs": "pub fn qualified() -> a::Call { todo!() }\n",
        "crates/u/src/through.rs": "use r::Call;\npub fn through() -> Call { todo!() }\n",
        "crates/u/src/own.rs": "pub enum Inst { Call(u8) }\npub fn own(i: Inst) -> u8 { match i { Inst::Call(n) => n } }\n",
        "crates/u/src/local.rs": "use crate::Again as Call;\npub fn local() -> Call { todo!() }\n",
      },
      "crates/a/src/lib.rs",
      "pub struct Call(pub u8);\n",
      "pub struct Call(pub u16);\n",
    );
    expect(moved).toContain("fn:crates/u/src/used.rs#used");
    expect(moved).toContain("fn:crates/u/src/qualified.rs#qualified");
    expect(moved).toContain("fn:crates/u/src/through.rs#through");
    expect(moved).toContain("fn:crates/u/src/local.rs#local");
    expect(moved).not.toContain("fn:crates/u/src/own.rs#own");
    expect(moved).not.toContain("fn:crates/u/src/own.rs#Inst");
  });

  test("a glob binds an item only when it is from a crate that leads to it", () => {
    const owner = "crates/a/src/lib.rs";
    const before = "pub struct Call(pub u8);\n";
    const after = "pub struct Call(pub u16);\n";
    const moved = reachedIn(
      [["a", []], ["u", [["a", "normal"]]]],
      {
        "crates/u/src/glob.rs": "use a::*;\npub fn globbed() -> Call { todo!() }\n",
        "crates/u/src/parent.rs": "use a::Call;\npub fn outer() {}\n#[cfg(test)]\nmod tests {\n    use super::*;\n    #[test]\n    fn inner() { let _: Option<Call> = None; }\n}\n",
      },
      owner,
      before,
      after,
    );
    expect(moved).toContain("fn:crates/u/src/glob.rs#globbed");
    expect(moved).toContain("fn:crates/u/src/parent.rs#tests::inner");
    const ext = { "crates/v/src/ext.rs": "use cranelift::prelude::*;\npub fn external(i: Inst) -> u8 { match i { Inst::Call(n) => n } }\n" };
    const pkgs: [string, [string, "normal" | "dev"][]][] = [["a", []], ["v", [["a", "normal"]]]];
    expect(reachedIn(pkgs, ext, owner, before, after)).not.toContain("fn:crates/v/src/ext.rs#external");
    // Once a module of `v` brings the name in from `a`, a glob of its own modules or of an external
    // crate may be what binds it, so it is reached.
    const imported = reachedIn(pkgs, { ...ext, "crates/v/src/lib.rs": "pub use a::*;\n" }, owner, before, after);
    expect(imported).toContain("fn:crates/v/src/ext.rs#external");
  });

  test("a test-only class table moves no class, and a file that does not parse is taken whole", () => {
    const file = "crates/demo/src/t.rs";
    const table = (rows: string) => `pub fn f() {}\n#[cfg(test)]\nmod tests {\n    const OWING: &[&str] = &[${rows}];\n    #[test]\n    fn owing() { assert!(!OWING.is_empty()); }\n}\n`;
    tree.put(file, table('"Core\\\\Arr", "Core\\\\Str"'));
    const base = scanned([file]);
    tree.put(file, table('"Core\\\\Str"'));
    const head = scanned([file]);
    const d = diffFile(base.get(file)!, head.get(file)!);
    const moved = [...referenceWalk(d.changes, new Universe(head)).keys()];
    expect(moved.filter((k) => k.startsWith("class:"))).toEqual([]);
    expect(moved).toContain("fn:crates/demo/src/t.rs#tests::owing");
    tree.put(file, "pub fn f( {\n");
    const broken = scanned([file]);
    expect(diffFile(base.get(file)!, broken.get(file)!).wide).toBe(true);
    const keys = [...referenceWalk([], new Universe(broken), [file]).keys()];
    expect(keys).toEqual(["fn:crates/demo/src/t.rs#", "class:*", "card:*"]);
  });

  test("a row added, taken out or edited moves that row alone, and a reordered table every moved position", () => {
    const row = (d: string, c: string) => ({ digest: d, classes: [c] });
    expect([...rowClasses([row("1", "A"), row("2", "B")], [row("1", "A"), row("3", "B")])]).toEqual(["B"]);
    expect([...rowClasses([row("1", "A")], [row("1", "A"), row("2", "C")])]).toEqual(["C"]);
    expect([...rowClasses([row("1", "A"), row("2", "B"), row("3", "C")], [row("1", "A"), row("3", "C")])]).toEqual(["B"]);
    expect([...rowClasses([row("1", "A"), row("2", "B")], [row("2", "B"), row("1", "A")])].sort()).toEqual(["A", "B"]);
  });
});

describe("paths and build scripts", () => {
  test("a new file in a listed directory moves that directory's listing", () => {
    const keys = pathKeys("tests/conformance/arr/new.nvst", true);
    expect(keys).toContain("dir:tests/conformance/arr");
    expect(keys).toContain("exists:tests/conformance/arr/new.nvst");
    expect(keys).toContain("tree:tests/conformance");
    expect(pathKeys("tests/conformance/arr/old.nvst", false)).not.toContain("dir:tests/conformance/arr");
  });

  test("a changed file moves its extension, which a `git grep -- *.<ext>` holds, and a file without one moves none", () => {
    expect(pathKeys("crates/nvs-ir/src/Lower.RS", false)).toContain("ext:rs");
    expect(pathKeys("data/playbook/a/b.json", false)).not.toContain("ext:rs");
    expect(pathKeys("Makefile", false).filter((k) => k.startsWith("ext:"))).toEqual([]);
  });

  test("a build script's inputs are what cargo recorded, and the item including its output is found", () => {
    const out = parseOutput(
      [`cargo:rerun-if-changed=build.rs`, `cargo:rerun-if-changed=${join(tree.root, "docs", "core")}`, "cargo:rustc-env=NVS_X=1", "cargo:rerun-if-changed=C:\\elsewhere\\x"].join("\r\n"),
      "crates/demo",
    );
    expect(out.envs).toEqual(["NVS_X"]);
    expect(out.inputs).toContain("crates/demo/build.rs");
    expect(isInput("crates/demo/build.rs", out.inputs)).toBe(true);
    tree.put("crates/demo/src/gen.rs", 'mod intros {\n    include!(concat!(env!("OUT_DIR"), "/intros.rs"));\n}\n');
    const found = generatedIncludes(scanItems(["crates/demo/src/gen.rs"], tree.root), () => "demo", tree.root);
    expect(found).toEqual([{ pkg: "demo", name: "intros.rs", file: "crates/demo/src/gen.rs", id: "intros::include!" }]);
  });

  test("the newest build directory of a package is its build script's", () => {
    tree.put("build/demo-0000000000000001/output", "cargo:rerun-if-changed=old.txt\n");
    tree.put("build/demo-0000000000000002/output", "cargo:rerun-if-changed=new.txt\n");
    const later = Date.now() / 1000 + 5;
    require("node:fs").utimesSync(join(tree.root, "build/demo-0000000000000002/output"), later, later);
    writeFileSync(join(tree.root, "build/not-a-build"), "");
    const scripts = buildScripts(join(tree.root, "build"), new Map([["demo", "crates/demo"]]));
    expect(scripts.get("demo")?.dir).toEndWith("demo-0000000000000002");
    expect(scripts.get("demo")?.inputs).toEqual(["crates/demo/build.rs", "crates/demo/new.txt"]);
  });
});
