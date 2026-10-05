import { describe, expect, test } from "bun:test";
import { benchDef, benchFiles, benchOf } from "../select/atoms.ts";
import type { Moved } from "../select/items.ts";
import { advance } from "../select/record.ts";
import { type ChangeSet, query } from "../select/select.ts";
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

  test("the bench tree lists each bench once, and a sibling's edit is its bench's definition", () => {
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
