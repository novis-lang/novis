import { describe, expect, test } from "bun:test";
import { OTHER, STATE, partitionOf, wrapWritten } from "../keys/partition.ts";
import { readFileSync } from "node:fs";
import { abs } from "../lib/paths.ts";
import { analyse } from "../keys/scan.ts";
import { readsCard } from "../keys/escape.ts";
import { metadata } from "../keys/graph.ts";
import { CARD_READERS, builtFrom, keyOf, testBuild } from "../keys/key.ts";
import { units } from "../keys/checks.ts";
import { Tree } from "../keys/tree.ts";

describe("partitionOf", () => {
  test("a goal's handoff record is state, so a wrap stales no key, and the other records are not", () => {
    expect(partitionOf("data/goals/core-math-1-3.handoff.json")).toBe(STATE);
    expect(partitionOf("data/goals/side/restart-free.handoff.json")).toBe(STATE);
    expect(partitionOf("data/goals/core-math-1-3.json")).toBe(OTHER);
    expect(partitionOf("data/chain.json")).toBe(OTHER);
    expect(partitionOf("docs/agent/goals/core-math-1-3.md")).toBe("goals");
  });

  test("the plan, a milestone, both playbook homes and the perf ledger are written after verify, and their neighbours are not", () => {
    for (const rel of ["docs/implementation-plan.md", "docs/plan/m8.md", "docs/agent/playbook/running-things/x.md", "data/playbook/running-things/x.json", "docs/perf/members.ndjson"]) {
      expect(wrapWritten(rel)).toBe(true);
    }
    for (const rel of ["docs/implementation-plan.mdx", "docs/planning.md", "docs/agent/goals/x.md", "data/goals/x.json", "docs/perf/userland-gap.md"]) {
      expect(wrapWritten(rel)).toBe(false);
    }
  });
});

const BASE = `//! A module.
use std::fmt;

/// Says one.
pub fn one() -> i32 { 1 }

const CARD: MethodDoc = MethodDoc { short: "Returns one.", params: &[ParamDoc { name: "x" }] };
const ROWS: &[CaseDoc] = &[CaseDoc { name: "A" }];
const LIMIT: usize = 4;

#[cfg(test)]
mod tests {
    #[test]
    fn it() { assert_eq!(super::one(), 1); let _ = include_str!("fixture.txt"); }
}

static DATA: &str = include_str!("data.txt");
`;

function tiersOf(text: string) {
  const a = analyse(text);
  return { docs: a.docs, code: a.code, shipped: a.shipped, card: a.card };
}

/** Which tiers moved between two texts. */
function moved(a: string, b: string): string[] {
  const x = tiersOf(a);
  const y = tiersOf(b);
  return (Object.keys(x) as (keyof typeof x)[]).filter((k) => x[k] !== y[k]);
}

describe("a test binary's dependency tier", () => {
  test("a card edit moves a card reader's test binary and no other", async () => {
    const graph = await metadata();
    expect(graph).not.toBeNull();
    const tree = await Tree.read();
    const math = "crates/nvs-stdlib/src/math.rs";
    const text = readFileSync(abs(math), "utf8");
    const edited = tree.edited({ [math]: text.replace('short: "Returns `$n` without its sign', 'short: "Gives `$n` without its sign') });
    expect(edited.raw(math)).not.toBe(tree.raw(math));
    const key = (t: Tree, pkg: string, kind: string) => keyOf(pkg, builtFrom(t, graph!, testBuild(pkg, kind)));
    expect(key(edited, "nvs-types", "test")).toBe(key(tree, "nvs-types", "test"));
    expect(key(edited, "nvs-cli", "bin")).not.toBe(key(tree, "nvs-cli", "bin"));
    expect(key(edited, "nvs-lsp", "lib")).not.toBe(key(tree, "nvs-lsp", "lib"));
    expect(key(edited, "nvs-stdlib", "lib")).not.toBe(key(tree, "nvs-stdlib", "lib"));
  });

  test("an `nvs-cli` integration test that starts no card command, and an `nvs` command that prints no card, skip a card edit", async () => {
    const graph = (await metadata())!;
    const tree = await Tree.read();
    const math = "crates/nvs-stdlib/src/math.rs";
    const text = readFileSync(abs(math), "utf8");
    const carded = tree.edited({ [math]: text.replace('short: "Returns `$n` without its sign', 'short: "Gives `$n` without its sign') });
    const example = tree.edited({ "docs/examples/core/Math/abs/zz-probe.nvs": "<?nvs\necho 1;\n", "tests/hostile/core/Math/abs/zz-probe.nvs": "<?nvs\necho 1;\n" });
    const config = tree.edited({ "nvs.toml": `${tree.text("nvs.toml")}\n# probe\n` });
    const checks = [
      { kind: "command", name: "migrate", argv: ["{nvs}", "queue", "migrate", "--connection", "main"] },
      { kind: "command", name: "meta", argv: ["{nvs}", "meta", "--json"] },
      { kind: "command", name: "fmt anywhere", argv: ["{nvs}", "fmt", "--check"] },
    ];
    const us = units({ graph, checks, reads: new Map(), wide: new Set<string>(), proofReads: new Map(), nvReads: new Map() });
    const key = (t: Tree, name: string) => keyOf(name, us.find((u) => u.name === name)!.parts(t));
    expect(key(carded, "nvs-cli test live_config")).toBe(key(tree, "nvs-cli test live_config"));
    expect(key(carded, "nvs-cli test meta")).not.toBe(key(tree, "nvs-cli test meta"));
    expect(key(carded, "migrate")).toBe(key(tree, "migrate"));
    expect(key(example, "migrate")).toBe(key(tree, "migrate"));
    expect(key(config, "migrate")).not.toBe(key(tree, "migrate"));
    expect(key(carded, "meta")).not.toBe(key(tree, "meta"));
    expect(key(example, "fmt anywhere")).not.toBe(key(tree, "fmt anywhere"));
  });

  test("only the registered card readers read a card outside the registry's package", () => {
    const types = ["MethodDoc", "ClassDoc"];
    expect(readsCard("fn f(d: &MethodDoc) {}", types)).toBe(true);
    expect(readsCard("/// a MethodDoc\nfn f() {}", types)).toBe(false);
    expect(readsCard("use nvs_stdlib::registry; fn f(m: &Member) { m.doc.unwrap(); }", types)).toBe(true);
    expect(readsCard("fn f(decl: &Decl) { decl.doc.as_ref(); }", types)).toBe(false);
    for (const reader of CARD_READERS) expect(reader).toMatch(/^nvs-/);
  });
});

describe("the tiers", () => {
  test("a plain comment and a layout change move no tier", () => {
    expect(moved(BASE, BASE.replace("use std::fmt;", "use std::fmt; // why"))).toEqual([]);
    expect(moved(BASE, BASE.replace("pub fn one() -> i32 { 1 }", "pub fn one()\n    -> i32 {\n    1\n}"))).toEqual([]);
  });

  test("a doc comment moves the docs tier alone", () => {
    expect(moved(BASE, BASE.replace("Says one.", "Says the number one."))).toEqual(["docs"]);
  });

  test("an edit inside the test module moves docs and code, and neither shipped nor card", () => {
    expect(moved(BASE, BASE.replace("fn it() {", "fn it() { let _two = 2;"))).toEqual(["docs", "code"]);
  });

  test("a card's text moves every tier but card, a slice of cards too", () => {
    expect(moved(BASE, BASE.replace("Returns one.", "Gives one."))).toEqual(["docs", "code", "shipped"]);
    expect(moved(BASE, BASE.replace('name: "A"', 'name: "B"'))).toEqual(["docs", "code", "shipped"]);
  });

  test("a const that is not a card moves the card tier", () => {
    expect(moved(BASE, BASE.replace("LIMIT: usize = 4", "LIMIT: usize = 5"))).toEqual(["docs", "code", "shipped", "card"]);
  });

  test("linking a class to its new card moves every tier but card", () => {
    const bare = `use crate::registry::{CoreClass, MethodDoc};\npub const CLASS: CoreClass = CoreClass { name: "A", doc: None };\n`;
    const carded = `use crate::registry::{ClassDoc, CoreClass, MethodDoc,};\n/// The card.\nconst CARD: ClassDoc = ClassDoc { short: "A class." };\npub const CLASS: CoreClass = CoreClass { name: "A", doc: Some(&CARD) };\n`;
    expect(moved(bare, carded)).toEqual(["docs", "code", "shipped"]);
  });

  test("a link to something that is not one of the file's cards moves the card tier", () => {
    const bare = `pub const CLASS: CoreClass = CoreClass { doc: None };\n`;
    expect(moved(bare, bare.replace("doc: None", "doc: Some(&OTHER)"))).toContain("card");
  });

  test("code any build without `cfg(test)` shuts off is not shipped, and code one might keep is", () => {
    const f = (cfg: string, n: number) => `#[cfg(${cfg})]\nfn helper() -> i32 { ${n} }\npub fn one() -> i32 { 1 }\n`;
    expect(moved(f("all(test, unix)", 1), f("all(test, unix)", 2))).toEqual(["docs", "code"]);
    expect(moved(f("not(not(test))", 1), f("not(not(test))", 2))).toEqual(["docs", "code"]);
    expect(moved(f("any(test, unix)", 1), f("any(test, unix)", 2))).toEqual(["docs", "code", "shipped", "card"]);
  });

  test("a file the parser cannot read is its text in every tier", () => {
    expect(moved("fn f( {", "fn f(  {")).toEqual(["docs", "code", "shipped", "card"]);
  });

  test("the scanner's card types are the registry's", () => {
    const registry = readFileSync(abs("crates/nvs-stdlib/src/registry.rs"), "utf8");
    const declared = [...registry.matchAll(/^pub struct (\w+Doc)\b/gm)].map((m) => m[1]!).sort();
    const listed = readFileSync(abs("tools/nv-scan/src/main.rs"), "utf8").match(/const CARD_TYPES: &\[&str\] = &\[([^\]]*)\]/)![1]!;
    expect([...listed.matchAll(/"(\w+)"/g)].map((m) => m[1]!).sort()).toEqual(declared);
  });

  test("tokens that layout would join never share a key", () => {
    expect(moved("fn f() { a & &b }", "fn f() { a &&b }")).toContain("code");
  });

  test("an include site knows whether it is in a test module", () => {
    expect(analyse(BASE).includes).toEqual([
      { path: "fixture.txt", inTest: true },
      { path: "data.txt", inTest: false },
    ]);
  });
});
