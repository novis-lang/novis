// The `component:` keys: what a built-in component's change moves, how a footprint log names one, and
// how a footprint recorded before the kind existed is read.

import { describe, expect, test } from "bun:test";
import { ALL_COMPONENTS, BUILTINS_READ, builtinComponents, componentKeys, logKeys } from "../select/keys.ts";
import type { Moved } from "../select/items.ts";
import { BUILTIN_LOADER, type ChangeSet, query } from "../select/select.ts";
import { SelectStore } from "../select/store.ts";
import { scratch } from "./scratch.ts";

const COMPONENTS = new Set(["pdf", "spreadsheet"]);

/** A change that moves `keys`, each from `path`. */
function change(keys: string[], path: string): ChangeSet {
  const moved: Moved = new Map(keys.map((k) => [k, { path, how: "path" as const }]));
  return { since: "base", changes: [{ path, status: "modified" }], moved, global: null, rustFiles: 0, itemChanges: 0, wideFiles: [], items: [], view: new Map() };
}

/** The atoms `keys` select, by key alone. */
function keyed(s: SelectStore, keys: string[], path: string): string[] {
  const sel = query(s, change(keys, path));
  return [...sel.selected.values()].filter((a) => a.why === "key").map((a) => a.id).sort();
}

const LOADER = `${BUILTIN_LOADER}load.rs#Builtin::extension`;

/** Footprints recorded with component keys, one recorded before them, and one that never loaded a
 * component. */
function store(): SelectStore {
  const s = new SelectStore(":memory:", "test-os");
  const rec = (id: string, keys: string[]) => s.recordRun(id, { def: "", verdict: "green", keys: new Map(keys.map((k) => [k, ""])) });
  rec("test:sheet test sheet", [LOADER, BUILTINS_READ, "component:spreadsheet"]);
  rec("test:pdf test pdf", [LOADER, BUILTINS_READ, "component:pdf"]);
  rec("test:plain test plain", [LOADER, BUILTINS_READ]);
  rec("test:meta test meta", [LOADER, BUILTINS_READ, ALL_COMPONENTS]);
  rec("test:old test old", [LOADER, "fn:crates/s/src/a.rs#run"]);
  rec("test:ldap test ldap", ["fn:crates/ldap/src/proto.rs#Filter::from_ber"]);
  return s;
}

describe("the component keys a change moves", () => {
  test("a path of a component's crate or its WIT file moves that component alone", () => {
    expect(componentKeys("extensions/spreadsheet/src/read/cfb.rs", COMPONENTS)).toEqual(["component:spreadsheet"]);
    expect(componentKeys("extensions/spreadsheet/manifest.json", COMPONENTS)).toEqual(["component:spreadsheet"]);
    expect(componentKeys("wit/spreadsheet.wit", COMPONENTS)).toEqual(["component:spreadsheet"]);
  });

  test("the shared world, a directory that is no component, and a WIT file of none move every component", () => {
    expect(componentKeys("wit/nvs-ext/world.wit", COMPONENTS)).toEqual([ALL_COMPONENTS]);
    expect(componentKeys("extensions/fonts/LICENSE", COMPONENTS)).toEqual([ALL_COMPONENTS]);
    expect(componentKeys("wit/other.wit", COMPONENTS)).toEqual([ALL_COMPONENTS]);
  });

  test("a path outside extensions/ and wit/ moves no component", () => {
    expect(componentKeys("crates/nvs-ext/src/load.rs", COMPONENTS)).toEqual([]);
    expect(componentKeys("docs/extensions/spreadsheet.md", COMPONENTS)).toEqual([]);
  });

  test("a component is a directory under extensions/ that holds a manifest.json", () => {
    const root = scratch();
    try {
      root.put("extensions/pdf/manifest.json", "{}");
      root.put("extensions/fonts/LICENSE", "");
      expect([...builtinComponents(root.root)]).toEqual(["pdf"]);
    } finally {
      root.cleanup();
    }
  });
});

describe("a footprint log's component lines", () => {
  test("a component line is its key, every component is the wild one, and the table's line is the marker", () => {
    const keys = logKeys("component\tspreadsheet\ncomponent\t*\nbuiltins\tread\n");
    expect([...keys].sort()).toEqual([BUILTINS_READ, ALL_COMPONENTS, "component:spreadsheet"].sort());
  });
});

describe("the atoms a component change selects", () => {
  test("a component's key selects what used it, every reader of all of them, and a footprint older than the kind", () => {
    expect(keyed(store(), ["component:spreadsheet"], "extensions/spreadsheet/src/lib.rs")).toEqual(["test:meta test meta", "test:old test old", "test:sheet test sheet"]);
  });

  test("every component moved selects every footprint that holds one, and the older ones", () => {
    expect(keyed(store(), [ALL_COMPONENTS], "wit/nvs-ext/world.wit")).toEqual(["test:meta test meta", "test:old test old", "test:pdf test pdf", "test:sheet test sheet"]);
  });

  test("a change that moves no component key selects no footprint by the loader alone", () => {
    expect(keyed(store(), ["file:docs/a.md"], "docs/a.md")).toEqual([]);
  });
});
