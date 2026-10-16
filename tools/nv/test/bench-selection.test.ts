import { describe, expect, test } from "bun:test";
import { benchDef, benchFiles, benchOf } from "../select/atoms.ts";
import type { Moved } from "../select/items.ts";
import { advance } from "../select/record.ts";
import { splitBenches } from "../cmd/affected.ts";
import { type ChangeSet, discover, query, reachedByChange, touchesDef } from "../select/select.ts";
import { SelectStore } from "../select/store.ts";
import { scratch } from "./scratch.ts";

const SORT = "bench:benches/members/core/Arr/sort.nvs";
const LENGTH = "bench:benches/members/core/Str/length.nvs";

/** A store holding two recorded benches, each with the code its recording run reached, and a test binary. */
function store(): SelectStore {
  const s = new SelectStore(":memory:", "test-os");
  const rec = (id: string, keys: string[]) => s.recordRun(id, { def: "", verdict: "green", keys: new Map(keys.map((k) => [k, ""])) });
  rec(SORT, ["fn:crates/s/src/arr.rs#sort", "class:core\\arr"]);
  rec(LENGTH, ["fn:crates/s/src/str.rs#length", "class:core\\str"]);
  rec("test:other lib other", ["fn:crates/s/src/net.rs#connect"]);
  return s;
}

function change(moved: [string, string][]): ChangeSet {
  const m: Moved = new Map(moved.map(([k, path]) => [k, { path, how: "changed" as const }]));
  return {
    since: "base",
    changes: [...new Set(moved.map(([, p]) => p))].map((path) => ({ path, status: "modified" as const })),
    moved: m,
    global: null,
    rustFiles: 1,
    itemChanges: moved.length,
    wideFiles: [],
    items: [],
    view: new Map(),
  };
}

const benches = (s: SelectStore, c: ChangeSet) => [...query(s, c).selected.values()].filter((x) => x.kind === "bench").map((x) => x.id);

describe("the bench atom", () => {
  test("a change to code a bench reaches selects that bench", () => {
    expect(benches(store(), change([["fn:crates/s/src/arr.rs#sort", "crates/s/src/arr.rs"]]))).toEqual([SORT]);
    expect(benches(store(), change([["class:core\\str", "crates/s/src/str.rs"]]))).toEqual([LENGTH]);
  });

  test("a change no bench reaches selects no bench", () => {
    const s = store();
    const sel = query(s, change([["fn:crates/s/src/net.rs#connect", "crates/s/src/net.rs"]]));
    expect([...sel.selected.keys()]).toEqual(["test:other lib other"]);
    expect(benches(s, change([]))).toEqual([]);
  });

  test("a bench whose figure the change made stale is owed until it runs", () => {
    const t = scratch();
    const s = store();
    try {
      const c = { ...change([["fn:crates/s/src/arr.rs#sort", "crates/s/src/arr.rs"]]), tree: { commit: "c1", overlay: {} } };
      const sel = query(s, c);
      // The run that moves the tree ran nothing, so the bench it reached is owed.
      advance(s, c, sel, new Set(), null, t.root);
      expect(s.atom(SORT)!.verdict).toBe("owed");
      expect(query(s, change([])).selected.get(SORT)?.why).toBe("owed");
      expect(s.atom(LENGTH)!.verdict).toBe("green");
      s.recordRun(SORT, { def: "", verdict: "green", keys: new Map([["fn:crates/s/src/arr.rs#sort", ""]]) });
      expect(benches(s, change([]))).toEqual([]);
    } finally {
      s.close();
      t.cleanup();
    }
  });

  test("a change reaches an owed bench only through a key it holds, and a bench left out stays owed with its items", () => {
    const t = scratch();
    const s = store();
    const ADDED = "bench:benches/members/core/Arr/added.nvs";
    const UNRECORDED = "bench:benches/members/core/Arr/old.nvs";
    try {
      const sortKey = "fn:crates/s/src/arr.rs#sort";
      const first = { ...change([[sortKey, "crates/s/src/arr.rs"]]), tree: { commit: "c1", overlay: {} } };
      advance(s, first, query(s, first), new Set(), null, t.root);
      // Owed for no key, as a run that cannot name its changed items leaves an atom.
      s.owe([LENGTH]);
      expect(s.atom(SORT)!.owedItems).toEqual([sortKey]);
      expect(s.atom(LENGTH)!.owedItems).toBeNull();

      const moved = change([["class:core\\str", "crates/s/src/str.rs"]]);
      const c = { ...moved, changes: [...moved.changes, { path: "benches/members/core/Arr/added.nvs", status: "added" as const }], tree: { commit: "c2", overlay: {} } };
      const sel = query(s, c, { discovered: [SORT, LENGTH, ADDED, UNRECORDED] });
      const touched = touchesDef(c);
      const reached = (id: string) => reachedByChange(sel.selected.get(id)!, touched);
      // `LENGTH` was owed from before, and the change also moved a key its footprint holds.
      expect(sel.selected.get(LENGTH)!.why).toBe("owed");
      expect(reached(LENGTH)).toBe(true);
      expect(reached(SORT)).toBe(false);
      expect(reached(ADDED)).toBe(true);
      expect(reached(UNRECORDED)).toBe(false);
      const split = splitBenches(sel.selected.values(), touched);
      expect(split.judged.map((b) => b.id)).toEqual([ADDED, LENGTH]);
      expect(split.before).toBe(2);

      // The run judged `LENGTH` alone, so `SORT` keeps the items it was owed for.
      s.recordRun(LENGTH, { def: "", verdict: "green", keys: new Map([["class:core\\str", ""]]) });
      advance(s, c, sel, new Set([LENGTH]), null, t.root);
      expect(s.atom(SORT)!.verdict).toBe("owed");
      expect(s.atom(SORT)!.owedItems).toEqual([sortKey]);
      expect(s.atom(LENGTH)!.verdict).toBe("green");
    } finally {
      s.close();
      t.cleanup();
    }
  });

  test("the bench tree lists each bench once, discovered as an atom, and a sibling's edit is its bench's definition", () => {
    const t = scratch();
    try {
      t.put("benches/members/core/Arr/sort.nvs", "// bench: iterations 32\n");
      t.put("benches/members/core/Arr/sort.scale.nvs", "a");
      t.put("benches/members/core/Arr/sort.twin.nvs", "");
      t.put("benches/members/core/Arr/sort.nvsr", "");
      t.put("benches/members/lang/autoload.nvs", "");
      t.put("benches/members/lang/autoload/Helper.nvs", "");
      t.put("benches/members/_calibration/unit.nvs", "");
      expect(benchFiles(t.root)).toEqual(["benches/members/core/Arr/sort.nvs", "benches/members/lang/autoload.nvs"]);
      expect(discover(null, t.root).atoms.filter((a) => a.startsWith("bench:"))).toEqual(["bench:benches/members/core/Arr/sort.nvs", "bench:benches/members/lang/autoload.nvs"]);
      for (const p of ["sort.scale.nvs", "sort.twin.nvs", "sort.nvsr", "sort.in", "sort.nvs"]) expect(benchOf(`benches/members/core/Arr/${p}`)).toBe("benches/members/core/Arr/sort.nvs");
      expect(benchOf("benches/members/README.md")).toBeNull();
      expect(benchOf("docs/examples/core/Arr/sort/01-x.nvs")).toBeNull();
      const before = benchDef("benches/members/core/Arr/sort.nvs", t.root);
      t.put("benches/members/core/Arr/sort.scale.nvs", "b");
      expect(benchDef("benches/members/core/Arr/sort.nvs", t.root)).not.toBe(before);
    } finally {
      t.cleanup();
    }
  });
});
