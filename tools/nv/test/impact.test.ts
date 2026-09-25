import { describe, expect, test } from "bun:test";
import type { Unit } from "../keys/checks.ts";
import type { Part } from "../keys/key.ts";
import { Tree } from "../keys/tree.ts";
import { type Probe, applyEdit, judge } from "../cmd/impact.ts";
import { describe as whyLines } from "../cmd/why.ts";

function unit(role: Unit["role"], name: string): Unit {
  return { name, role, source: "package key", parts: () => [] };
}

const us = [unit("binary", "a lib a"), unit("binary", "b lib b"), unit("suite", "conformance"), unit("fuzz", "prefix")];
const [a, b, suite, fuzz] = us as [Unit, Unit, Unit, Unit];

function probe(rerun: string[], keep: string[]): Probe {
  return { name: "p", what: "", edit: { path: "x", insert: "" }, rerun, keep };
}

describe("a probe's verdict", () => {
  test("holds when every rerun unit moves and no other kept unit does", () => {
    const v = judge(probe(["^binary: a "], ["^binary: ", "^suite: "]), us, new Set([a]), new Set());
    expect(v.failures).toEqual([]);
    expect([v.rerun, v.kept, v.wide]).toEqual([1, 2, 0]);
  });

  test("names a rerun unit that holds and a kept unit that moves", () => {
    const v = judge(probe(["^binary: a "], ["^suite: "]), us, new Set([suite]), new Set());
    expect(v.failures).toEqual(["does not re-run: binary: a lib a", "re-runs: suite: conformance"]);
  });

  test("counts a kept unit keyed on everything as wide, and a pattern that matches nothing as an error", () => {
    const v = judge(probe(["^fuzz: "], ["^binary: b ", "^tsan: "]), us, new Set([fuzz, b]), new Set([b]));
    expect(v.wide).toBe(1);
    expect(v.failures).toEqual(["keep pattern matches no unit: ^tsan: "]);
  });

  test("holds a test check to its binaries: it re-runs with a moved one, and stays with none", () => {
    const check = (name: string, binaries: string[]): Unit => ({ ...unit("test", name), source: "verify record", binaries });
    const withA = check("a (filtered)", ["a lib a"]);
    const onlyB = check("b (filtered)", ["b lib b"]);
    const all = [...us, withA, onlyB];
    const held = judge(probe(["^binary: a "], ["^binary: ", "^test: "]), all, new Set([a, withA]), new Set());
    expect(held.failures).toEqual([]);
    expect([held.rerun, held.kept]).toEqual([2, 2]);
    const broken = judge(probe(["^binary: a "], ["^binary: ", "^test: "]), all, new Set([a, onlyB]), new Set());
    expect(broken.failures).toEqual(["does not re-run with its binaries: test: a (filtered)", "re-runs: test: b (filtered)"]);
  });
});

describe("what `nv why` prints", () => {
  test("a package's files at one tier are one line, and any other input is its own", () => {
    const part = (label: string, partition: string, tier: Part["tier"]): Part => ({ label, partition, tier, digest: "" });
    expect(
      whyLines([
        part("crates/nvs-host/src/a.rs", "crates", "card"),
        part("crates/nvs-host/src/b.rs", "crates", "card"),
        part("crates/nvs-host/Cargo.toml", "crates", "raw"),
        part("crates/nvs-host/tests/t.rs", "crate-tests", "raw"),
        part("tools/tsan.sh", "tools", "raw"),
      ]),
    ).toEqual([
      "crate-tests: 1 input(s)",
      "  crates/nvs-host/tests/t.rs  raw",
      "crates: 3 input(s)",
      "  crates/nvs-host/  2 file(s) at card",
      "  crates/nvs-host/Cargo.toml  raw",
      "tools: 1 input(s)",
      "  tools/tsan.sh  raw",
    ]);
  });
});

describe("a probe's edit", () => {
  test("goes in after the last of its anchors, or at the end, or makes a new file", async () => {
    const tree = (await Tree.read()).edited({ "x/probe.txt": "one two one two" });
    expect(applyEdit(tree, { path: "x/probe.txt", after: ["two", "one"], insert: "!" })).toEqual({ "x/probe.txt": "one two one! two" });
    expect(applyEdit(tree, { path: "x/probe.txt", insert: "." })).toEqual({ "x/probe.txt": "one two one two." });
    expect(applyEdit(tree, { path: "x/new.txt", create: "n" })).toEqual({ "x/new.txt": "n" });
    expect(() => applyEdit(tree, { path: "x/probe.txt", after: ["three"], insert: "" })).toThrow("not found");
    expect(() => applyEdit(tree, { path: "x/probe.txt", create: "" })).toThrow("exists");
  });
});
