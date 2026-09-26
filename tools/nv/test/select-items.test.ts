import { afterAll, describe, expect, test } from "bun:test";
import { writeFileSync } from "node:fs";
import { join } from "node:path";
import { type FileItems, scanItems } from "../keys/scan.ts";
import { buildScripts, generatedIncludes, isInput, parseOutput } from "../select/build.ts";
import { closure, diffFile, rowClasses, type Scope, Universe } from "../select/items.ts";
import { pathKeys } from "../select/keys.ts";
import { scratch } from "./scratch.ts";

const tree = scratch();
afterAll(() => tree.cleanup());

const REGISTRY = `
pub struct ClassDoc { pub short: &'static str }
pub struct CoreClass { pub name: &'static str, pub doc: Option<&'static ClassDoc> }

/// The card.
pub const CARD: ClassDoc = ClassDoc { short: "Numbers." };
pub const MATH: CoreClass = CoreClass { name: "Core\\\\Math", doc: Some(&CARD) };
pub const CLASSES: &[CoreClass] = &[
    MATH,
    CoreClass { name: "Core\\\\Str", doc: None },
    CoreClass { name: "Core\\\\Arr", doc: None },
];

pub fn class(name: &str) -> Option<&'static CoreClass> {
    CLASSES.iter().find(|c| c.name == name)
}
`;

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
  return [...closure(d.changes, new Universe(head)).keys()].sort();
}

describe("the item diff and the reference-graph closure", () => {
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
    expect([...closure(d.changes, new Universe(head)).keys()].sort()).toEqual(["fn:crates/demo/src/lib.rs#BANNER", "fn:crates/demo/src/lib.rs#banner"]);
  });

  test("a card edit moves the card's class under card: and no class or program key", () => {
    const moved = movedBy("crates/reg/src/registry.rs", REGISTRY, REGISTRY.replace('"Numbers."', '"Whole numbers."'));
    expect(moved).toEqual(["card:core\\math", "fn:crates/reg/src/registry.rs#CARD"]);
  });

  test("a registry row edit moves that row's class alone", () => {
    const moved = movedBy("crates/reg/src/registry.rs", REGISTRY, REGISTRY.replace('name: "Core\\\\Arr", doc: None', 'name: "Core\\\\Arr", doc: Some(&CARD)'));
    expect(moved).toEqual(["class:core\\arr", "fn:crates/reg/src/registry.rs#CLASSES"]);
  });

  test("a class item moves its class and the fns of its own file that name it", () => {
    const moved = movedBy("crates/reg/src/registry.rs", REGISTRY, REGISTRY.replace("doc: Some(&CARD) };", "doc: None };"));
    expect(moved).toContain("class:core\\math");
    expect(moved).not.toContain("class:core\\str");
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
    const apart: Scope = { pkgOf: (f) => f.split("/")[1]!, sees: (user, owner) => user === owner };
    expect([...closure(d.changes, new Universe(head, apart)).keys()]).toEqual(["fn:crates/a/src/lib.rs#SHARED"]);
    const depends: Scope = { pkgOf: (f) => f.split("/")[1]!, sees: () => true };
    expect([...closure(d.changes, new Universe(head, depends)).keys()]).toContain("fn:crates/b/src/lib.rs#reads");
  });

  test("a test-only class table moves no class, and a file that does not parse is taken whole", () => {
    const file = "crates/demo/src/t.rs";
    const table = (rows: string) => `pub fn f() {}\n#[cfg(test)]\nmod tests {\n    const OWING: &[&str] = &[${rows}];\n    #[test]\n    fn owing() { assert!(!OWING.is_empty()); }\n}\n`;
    tree.put(file, table('"Core\\\\Arr", "Core\\\\Str"'));
    const base = scanned([file]);
    tree.put(file, table('"Core\\\\Str"'));
    const head = scanned([file]);
    const d = diffFile(base.get(file)!, head.get(file)!);
    const moved = [...closure(d.changes, new Universe(head)).keys()];
    expect(moved.filter((k) => k.startsWith("class:"))).toEqual([]);
    expect(moved).toContain("fn:crates/demo/src/t.rs#tests::owing");
    tree.put(file, "pub fn f( {\n");
    const broken = scanned([file]);
    expect(diffFile(base.get(file)!, broken.get(file)!).wide).toBe(true);
    const keys = [...closure([], new Universe(broken), [file]).keys()];
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
