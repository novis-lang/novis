import { describe, expect, test } from "bun:test";
import { parse } from "../cmd/select.ts";
import type { Moved } from "../select/items.ts";
import { failedLabels, seedable } from "../select/seed.ts";
import { type ChangeSet, counts, explain, isGlobal, query } from "../select/select.ts";
import { SelectStore } from "../select/store.ts";
import { join } from "node:path";
import { scratch } from "./scratch.ts";

/** A store holding four test-binary atoms, which are always on disk, and two `bun nv` checks. */
function store(): SelectStore {
  const s = new SelectStore(":memory:", "test-os");
  const rec = (id: string, keys: string[], verdict: "green" | "red" = "green") => s.recordRun(id, { def: "", verdict, keys: new Map(keys.map((k) => [k, ""])) });
  rec("test:proc lib proc", ["fn:crates/s/src/process.rs#Handle::wait", "class:core\\process"]);
  rec("test:math lib math", ["fn:crates/s/src/math.rs#gcd", "class:core\\math"]);
  rec("test:meta bin meta", ["class:*", "card:*"]);
  rec("test:red lib red", ["fn:crates/s/src/other.rs#x"], "red");
  rec("nv:reads-docs", ["file:docs/a.md", "mod:tools/nv/cmd/rules.ts"]);
  rec("nv:odd", ["*"]);
  s.ensureAtom("test:new lib new");
  return s;
}

function change(moved: [string, string][], global: string | null = null): ChangeSet {
  const m: Moved = new Map(moved.map(([k, path]) => [k, { path, how: "changed" as const }]));
  return {
    since: "base",
    changes: [...new Set(moved.map(([, p]) => p))].map((path) => ({ path, status: "modified" as const })),
    moved: m,
    global,
    rustFiles: 1,
    itemChanges: moved.length,
    wideFiles: [],
    items: [],
    view: new Map(),
  };
}

describe("the reverse-index query", () => {
  test("a fn edit selects the atoms that ran it, the never-recorded, the red and the unattributable", () => {
    const sel = query(store(), change([["fn:crates/s/src/process.rs#Handle::wait", "crates/s/src/process.rs"]]));
    const why = Object.fromEntries([...sel.selected.values()].map((s) => [s.id, s.why]));
    expect(why).toEqual({ "test:proc lib proc": "key", "test:red lib red": "red", "test:new lib new": "new", "nv:odd": "key" });
    expect(sel.selected.get("test:proc lib proc")!.keys[0]!.origin.path).toBe("crates/s/src/process.rs");
  });

  test("a class key selects its readers and every reader of the whole roster, and a card key only card readers", () => {
    const byClass = query(store(), change([["class:core\\math", "crates/s/src/math.rs"]]));
    expect([...byClass.selected.keys()].filter((id) => byClass.selected.get(id)!.why === "key").sort()).toEqual(["nv:odd", "test:math lib math", "test:meta bin meta"]);
    const byCard = query(store(), change([["card:core\\math", "crates/s/src/math.rs"]]));
    expect([...byCard.selected.keys()].filter((id) => byCard.selected.get(id)!.why === "key").sort()).toEqual(["nv:odd", "test:meta bin meta"]);
  });

  test("every class moved selects every atom that read a class", () => {
    const sel = query(store(), change([["class:*", "crates/s/src/registry.rs"]]));
    const keyed = [...sel.selected.values()].filter((s) => s.why === "key").map((s) => s.id).sort();
    expect(keyed).toEqual(["nv:odd", "test:math lib math", "test:meta bin meta", "test:proc lib proc"]);
  });

  test("a file prefix selects every atom holding a key of that file", () => {
    const sel = query(store(), change([["fn:crates/s/src/math.rs#", "crates/s/src/math.rs"]]));
    expect(sel.selected.get("test:math lib math")?.why).toBe("key");
    expect(sel.selected.has("test:proc lib proc")).toBe(false);
  });

  test("a global change selects every atom, and nothing moved selects only what never ran or was red", () => {
    const s = store();
    const all = query(s, change([["*", "Cargo.lock"]], "Cargo.lock"));
    expect(all.selected.size).toBe(7);
    expect(counts(all).byWhy.global).toEqual({ test: 5, nv: 2 });
    const none = query(s, change([]));
    expect([...none.selected.keys()].sort()).toEqual(["test:new lib new", "test:red lib red"]);
    expect(isGlobal("Cargo.lock") && isGlobal("crates/nvs-cli/Cargo.toml") && isGlobal("tools/nv-scan/src/items.rs")).toBe(true);
    expect(isGlobal("crates/nvs-cli/src/main.rs")).toBe(false);
  });

  test("a replayed change counts every atom whose definition it touches as redefined", () => {
    const s = store();
    const path = "tools/nv/test/select-query.test.ts";
    s.recordRun(`case:${path}`, { def: "whatever", verdict: "green", keys: new Map([["fn:x.rs#f", ""]]) });
    const replay = { ...change([["file:" + path, path]]), until: "later" };
    expect(query(s, replay).selected.get(`case:${path}`)?.why).toBe("def");
  });

  test("an owed atom stays selected with nothing moved, and a red one stays red when it is owed", () => {
    const s = store();
    expect(s.owe(["test:proc lib proc", "test:red lib red", "test:unknown lib x"])).toBe(1);
    expect(s.atom("test:red lib red")!.verdict).toBe("red");
    const sel = query(s, change([]));
    expect(sel.selected.get("test:proc lib proc")?.why).toBe("owed");
    s.recordRun("test:proc lib proc", { def: "", verdict: "green", keys: new Map() });
    expect(query(s, change([])).selected.has("test:proc lib proc")).toBe(false);
  });

  test("an atom discovered on disk and unknown to the store is new", () => {
    const sel = query(store(), change([]), { discovered: [{ id: "test:fresh test fresh", def: "" }] });
    expect(sel.selected.get("test:fresh test fresh")?.why).toBe("new");
    expect(sel.known.test).toBe(6);
  });

  test("a plan check's own atom whose recorded definition differs from the plan's runs, and one that matches does not", () => {
    const s = store();
    s.recordRun("check:a", { def: "d1", verdict: "green", keys: new Map([["file:x", ""]]) });
    s.recordRun("check:b", { def: "d2", verdict: "green", keys: new Map([["file:y", ""]]) });
    const sel = query(s, change([]), { defs: new Map([["check:a", "d1-changed"], ["check:b", "d2"]]) });
    expect(sel.selected.get("check:a")?.why).toBe("def");
    expect(sel.selected.has("check:b")).toBe(false);
  });

  test("the seed runs only the bun nv checks that read, and reads a case report's failures", () => {
    expect(seedable(["bun", "nv", "rules", "--check"])).toBe(true);
    expect(seedable(["bun", "nv", "proofs", "--gate"])).toBe(true);
    expect(seedable(["bun", "nv", "proofs", "--verify", "--group", "x"])).toBe(false);
    expect(seedable(["bun", "nv", "verify", "--list"])).toBe(false);
    expect(seedable(["bun", "nv", "rules", "--render"])).toBe(false);
    expect([...failedLabels("SKIP tests\\a.nvst — no php\r\nFAIL tests\\b\\c.nvst\r\n  want 1\n1 passed, 1 failed, 1 skipped\n")]).toEqual(["tests/b/c.nvst"]);
  });

  test("the command's arguments", () => {
    expect(parse(["--since", "HEAD~1", "--stats", "--json"])).toMatchObject({ since: "HEAD~1", stats: true, json: true });
    expect(parse(["--paths", "a.rs", "b.md", "--explain", "case:x"])).toMatchObject({ paths: ["a.rs", "b.md"], explain: "case:x" });
    expect(parse(["--seed", "--kinds", "case,test", "--limit", "3"])).toMatchObject({ seed: true, kinds: ["case", "test"], limit: 3 });
    expect(() => parse(["--kinds", "cases"])).toThrow("no atom kind");
    expect(() => parse(["--since"])).toThrow("needs a value");
  });

  test("explain names the key and where it came from, or why nothing reached the atom", () => {
    const s = store();
    const sel = query(s, change([["file:docs/a.md", "docs/a.md"]]));
    expect(explain(s, sel, "nv:reads-docs")[0]).toBe("nv:reads-docs: selected, because its footprint holds a key the change moved");
    expect(explain(s, sel, "nv:reads-docs")[1]).toContain("file:docs/a.md");
    const not = explain(s, sel, "test:math lib math");
    expect(not[0]).toContain("not selected");
    expect(not[1]).toBe("  docs/a.md changed; this atom did not read it");
  });
});

describe("what the recording build cannot see", () => {
  const SQRT = "proof:docs/examples/core/Math/sqrt/01-square-roots.nvs";
  const ATTACK = "proof:tests/hostile/core/Attributes/all/01-one-roster-read-back-at-many-places.nvs";

  /** The store above with two recorded proof programs. */
  function withProofs(): SelectStore {
    const s = store();
    s.recordRun(SQRT, { def: "", verdict: "green", keys: new Map([["class:core\\math", ""]]) });
    s.recordRun(ATTACK, { def: "", verdict: "green", keys: new Map([["class:core\\attributes", ""]]) });
    return s;
  }

  test("a diverged atom is selected whatever changed, until its mark is cleared", () => {
    const s = withProofs();
    s.markDiverged(SQRT, "exit 101 on the recording run, 0 on the judged run");
    expect(s.divergences()).toEqual(new Map([[SQRT, "exit 101 on the recording run, 0 on the judged run"]]));
    expect(query(s, change([])).selected.get(SQRT)?.why).toBe("diverged");
    expect(query(s, change([["docs/a.md", "docs/a.md"]])).selected.get(SQRT)?.why).toBe("diverged");
    expect(query(s, change([])).selected.has(ATTACK)).toBe(false);
    expect(counts(query(s, change([]))).byWhy).toEqual({ new: { test: 1 }, red: { test: 1 }, diverged: { proof: 1 } });
    s.clearDiverged(SQRT);
    expect(s.divergences().size).toBe(0);
    expect(query(s, change([])).selected.has(SQRT)).toBe(false);
  });

  test("a mark belongs to its platform", () => {
    const dir = scratch();
    try {
      const file = join(dir.root, "select.sqlite");
      const here = new SelectStore(file, "test-os");
      here.markDiverged(SQRT, "stdout differs from the judged run's at line 2");
      here.close();
      const there = new SelectStore(file, "other-os");
      expect(there.divergences().size).toBe(0);
      there.close();
      const again = new SelectStore(file, "test-os");
      expect([...again.divergences().keys()]).toEqual([SQRT]);
      again.close();
    } finally {
      dir.cleanup();
    }
  });

  test("code only an optimized build compiles selects every proof program and nothing else by itself", () => {
    const sel = query(withProofs(), change([["profile:optimized", "crates/s/src/alloc.rs"]]));
    const keyed = [...sel.selected.values()].filter((x) => x.why === "key").map((x) => x.id).sort();
    expect(keyed).toEqual([ATTACK, SQRT, "nv:odd"].sort());
    expect(sel.selected.get(SQRT)!.keys[0]!.origin.path).toBe("crates/s/src/alloc.rs");
  });
});
