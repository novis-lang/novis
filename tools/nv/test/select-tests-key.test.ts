// An added test and the test binaries it reaches (`select/select.ts`): the test binaries it is compiled
// into (`keys/graph.ts` `landsIn`), what counts as a test, and how a footprint that holds only its
// package's `tests:` key is narrowed to the binaries its atom runs.

import { describe, expect, test } from "bun:test";
import { type Graph, landsIn } from "../keys/graph.ts";
import type { Item } from "../keys/scan.ts";
import type { Moved } from "../select/items.ts";
import { pkgTestsKey, testsKey } from "../select/keys.ts";
import { testKeys } from "../select/seed.ts";
import { type ChangeSet, isTest, query } from "../select/select.ts";
import { SelectStore } from "../select/store.ts";

const graph: Graph = new Map([
  [
    "demo",
    {
      name: "demo",
      dir: "crates/demo",
      deps: new Map(),
      targets: [
        { kind: "lib", name: "demo", src: "crates/demo/src/lib.rs", test: true },
        { kind: "bin", name: "demo", src: "crates/demo/src/main.rs", test: true },
        { kind: "test", name: "queue", src: "crates/demo/tests/queue.rs", test: true },
        { kind: "test", name: "nested", src: "crates/demo/tests/nested/main.rs", test: true },
        { kind: "bench", name: "speed", src: "crates/demo/benches/speed.rs", test: false },
      ],
    },
  ],
]);

const LIB = "demo lib demo";
const BIN = "demo bin demo";
const QUEUE = "demo test queue";
const NESTED = "demo test nested";

describe("the test binaries an added test is compiled into", () => {
  test("a file under src/ lands in the library and binary tests, and an integration test file in its own binary", () => {
    expect(landsIn(graph, "demo", "crates/demo/src/serialize.rs").sort()).toEqual([BIN, LIB]);
    expect(landsIn(graph, "demo", "crates/demo/src/lib.rs")).toEqual([LIB]);
    expect(landsIn(graph, "demo", "crates/demo/tests/queue.rs")).toEqual([QUEUE]);
    expect(landsIn(graph, "demo", "crates/demo/tests/nested/helpers.rs")).toEqual([NESTED]);
  });

  test("a module integration tests share lands in every test binary of the package", () => {
    expect(landsIn(graph, "demo", "crates/demo/tests/common/mod.rs").sort()).toEqual([BIN, LIB, NESTED, QUEUE]);
  });

  test("a test-only function or macro call is a test, and a use line or a test-only type is not", () => {
    const item = (kind: string, t: boolean): Item => ({ id: "x", kind, start: 1, end: 1, test: t, digest: "", refs: [], defines: [] });
    expect(isTest(item("fn", true))).toBe(true);
    expect(isTest(item("macro", true))).toBe(true);
    expect(isTest(item("use", true))).toBe(false);
    expect(isTest(item("struct", true))).toBe(false);
    expect(isTest(item("fn", false))).toBe(false);
  });

  test("a test binary's footprint holds its own key", () => {
    expect([...testKeys(null, QUEUE).keys()]).toEqual(["tests:demo/test/queue"]);
    expect(testsKey(LIB)).toBe("tests:demo/lib/demo");
  });
});

describe("a footprint that holds only its package's key", () => {
  function store(): SelectStore {
    const s = new SelectStore(":memory:", "test-os");
    const rec = (id: string, keys: string[]) => s.recordRun(id, { def: "", verdict: "green", keys: new Map(keys.map((k) => [k, ""])) });
    rec(`test:${LIB}`, [pkgTestsKey("demo")]);
    rec(`test:${QUEUE}`, [pkgTestsKey("demo")]);
    rec("heavy:matrix", [pkgTestsKey("demo")]);
    rec("heavy:unknown", [pkgTestsKey("demo")]);
    rec("heavy:recorded-again", [pkgTestsKey("demo"), testsKey(QUEUE)]);
    rec(`test:${NESTED}`, [testsKey(NESTED)]);
    return s;
  }
  const change = (keys: string[]): ChangeSet => {
    const moved: Moved = new Map(keys.map((k) => [k, { path: "crates/demo/src/serialize.rs", item: "tests::added", how: "added" as const }]));
    return { since: "base", changes: [{ path: "crates/demo/src/serialize.rs", status: "modified" }], moved, global: null, rustFiles: 1, itemChanges: 1, wideFiles: [], items: [], view: new Map() };
  };
  const ran = new Map([["heavy:matrix", [QUEUE]]]);
  const picked = (keys: string[]) => [...query(store(), change(keys), { ran }).selected.keys()].sort();

  test("a unit test added under src/ reaches the library binary and no twin that runs only integration tests", () => {
    expect(picked([testsKey(LIB), testsKey(BIN)])).toEqual(["heavy:unknown", `test:${LIB}`]);
  });

  test("a test added to tests/queue.rs reaches that binary and every twin that runs it", () => {
    expect(picked([testsKey(QUEUE)])).toEqual(["heavy:matrix", "heavy:recorded-again", "heavy:unknown", `test:${QUEUE}`]);
  });

  test("a footprint that names its binaries is reached by those keys alone", () => {
    expect(picked([testsKey(NESTED)])).toEqual(["heavy:unknown", `test:${NESTED}`]);
  });
});
