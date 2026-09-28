import { Database } from "bun:sqlite";
import { describe, expect, test } from "bun:test";
import { existsSync, rmSync } from "node:fs";
import { join } from "node:path";
import { pack, SelectStore, unpack } from "../select/store.ts";
import { scratch } from "./scratch.ts";

const store = () => new SelectStore(":memory:", "test-os");

describe("the select store", () => {
  test("a closed store lets go of its file, however many statements it prepared", () => {
    const t = scratch();
    const file = join(t.root, "select.sqlite");
    const s = new SelectStore(file, "test-os");
    s.recordRun("case:tests/a.nvst", { def: "d", verdict: "green", keys: new Map([["fn:a.rs#f", "x"], ["class:core\\math", ""]]) });
    s.recordRun("test:x lib x", { def: "", verdict: "red", keys: new Map([["tree:docs", ""]]) });
    s.atoms();
    s.atoms("case");
    s.atom("case:tests/a.nvst");
    s.footprint("case:tests/a.nvst");
    s.owe(["case:tests/a.nvst"]);
    s.putVerdict("slot:a", "", "x");
    s.verdictsWithPrefix("slot:");
    s.markDiverged("case:tests/a.nvst", "why");
    s.divergences();
    s.clearDiverged("case:tests/a.nvst");
    s.keysWithPrefix("fn:");
    s.atomsUnderIds(s.keyIdsOf(["fn:a.rs#f"]));
    s.keysUnder("tree:", "test");
    s.setBase("c1", {});
    s.base();
    s.stats();
    s.prune(["fn:a.rs#f"]);
    s.removeAtom("test:x lib x");
    // `bun:sqlite` caches twenty statements of its own; this store has prepared more than that.
    expect((s as unknown as { statements: Map<string, unknown> }).statements.size).toBeGreaterThan(20);
    s.close();
    // The last connection to close checkpoints the WAL and deletes it, and the directory goes at once.
    expect(existsSync(`${file}-wal`)).toBe(false);
    rmSync(t.root, { recursive: true });
    expect(existsSync(t.root)).toBe(false);
  });

  test("a key list survives the blob it is kept in", () => {
    const ids = [1, 2, 3, 127, 128, 129, 16_384, 2_000_000];
    expect(unpack(pack(ids))).toEqual(ids);
    expect(unpack(pack([]))).toEqual([]);
  });

  test("a run widens a footprint, and a run of a new definition starts it again", () => {
    const s = store();
    s.recordRun("case:tests/a.nvst", { def: "d1", verdict: "green", keys: new Map([["fn:a.rs#f", "x"], ["class:core\\math", ""]]) });
    s.recordRun("case:tests/a.nvst", { def: "d1", verdict: "green", keys: new Map([["fn:a.rs#g", "y"]]) });
    expect(s.footprint("case:tests/a.nvst")).toEqual(["class:core\\math", "fn:a.rs#f", "fn:a.rs#g"]);
    s.recordRun("case:tests/a.nvst", { def: "d2", verdict: "red", keys: new Map([["fn:a.rs#h", ""]]) });
    expect(s.footprint("case:tests/a.nvst")).toEqual(["fn:a.rs#h"]);
    expect(s.atom("case:tests/a.nvst")).toMatchObject({ kind: "case", def: "d2", verdict: "red", keys: 1 });
    expect(s.atomsUnder(["fn:a.rs#f"]).size).toBe(0);
    expect(s.keyDigest("fn:a.rs#g")).toBe("y");
  });

  test("a run whose use was read takes `*` out of the footprint and keeps what the earlier runs recorded", () => {
    const s = store();
    s.recordRun("test:a lib a", { def: "d", verdict: "green", keys: new Map([["fn:a.rs#f", ""]]) });
    s.recordRun("test:a lib a", { def: "d", verdict: "green", keys: new Map([["*", ""]]) });
    expect(s.footprint("test:a lib a")).toEqual(["*", "fn:a.rs#f"]);
    s.recordRun("test:a lib a", { def: "d", verdict: "green", keys: new Map([["fn:a.rs#g", ""]]) });
    expect(s.footprint("test:a lib a")).toEqual(["fn:a.rs#f", "fn:a.rs#g"]);
    expect(s.atomsUnder(["*"]).size).toBe(0);
  });

  test("a `bun nv` check's run with a reads log is its whole footprint, and one without keeps the old one", () => {
    const s = store();
    s.recordRun("nv:a", { def: "d", verdict: "green", keys: new Map([["dir:data/gaps", ""], ["file:a.json", ""]]) });
    s.recordRun("nv:a", { def: "d", verdict: "green", keys: new Map([["*", ""]]) });
    expect(s.footprint("nv:a")).toEqual(["*", "dir:data/gaps", "file:a.json"]);
    s.recordRun("nv:a", { def: "d", verdict: "green", keys: new Map([["file:b.json", ""]]) });
    expect(s.footprint("nv:a")).toEqual(["file:b.json"]);
    expect(s.atomsUnder(["dir:data/gaps", "*"]).size).toBe(0);
  });

  test("the reverse index answers which atoms hold a key, and only on its own platform", () => {
    const s = store();
    s.recordRun("test:a lib a", { def: "", verdict: "green", keys: new Map([["fn:a.rs#f", ""], ["file:x.txt", ""]]) });
    s.recordRun("test:b lib b", { def: "", verdict: "green", keys: new Map([["fn:a.rs#g", ""], ["file:x.txt", ""]]) });
    const under = s.atomsUnder(["file:x.txt", "fn:a.rs#g", "fn:none.rs#z"]);
    expect([...under.keys()].sort()).toEqual(["test:a lib a", "test:b lib b"]);
    expect(under.get("test:b lib b")!.sort()).toEqual(["file:x.txt", "fn:a.rs#g"]);
    expect([...s.keysWithPrefix("fn:a.rs#").keys()].sort()).toEqual(["fn:a.rs#f", "fn:a.rs#g"]);
    const other = new SelectStore(":memory:", "other-os");
    expect(other.atomsUnder(["file:x.txt"]).size).toBe(0);
  });

  test("a vanished item is pruned from every footprint", () => {
    const s = store();
    s.recordRun("test:a lib a", { def: "", verdict: "green", keys: new Map([["fn:a.rs#f", ""], ["fn:a.rs#gone", ""]]) });
    expect(s.prune(["fn:a.rs#gone"])).toBe(1);
    expect(s.footprint("test:a lib a")).toEqual(["fn:a.rs#f"]);
    expect(s.atom("test:a lib a")!.keys).toBe(1);
    s.removeAtom("test:a lib a");
    expect(s.atom("test:a lib a")).toBeNull();
    expect(s.atomsUnder(["fn:a.rs#f"]).size).toBe(0);
  });

  test("the base tree, the items and the counts are per platform", () => {
    const s = store();
    s.setBase("abc");
    s.replaceItems([{ file: "a.rs", parsed: true, raw: "r", items: [] }]);
    expect(s.base()).toBe("abc");
    expect(s.items("a.rs")?.raw).toBe("r");
    s.replaceItems([{ file: "b.rs", parsed: true, raw: "r2", items: [] }]);
    expect(s.itemFiles()).toEqual(["b.rs"]);
    s.ensureAtom("proof:docs/x.nvs");
    expect(s.stats()).toMatchObject({ atoms: { proof: 1 }, recorded: { proof: 0 }, red: 0 });
  });

  test("a run's judged and recorded times are kept per atom, and a new run keeps them", () => {
    const s = store();
    s.recordRun("proof:docs/a.nvs", { def: "d", verdict: "green", keys: new Map([["file:docs/a.nvs", ""]]) });
    s.ensureAtom("proof:docs/b.nvs");
    s.setDurations("proof:docs/a.nvs", 120.4, 950.6);
    s.setDurations("proof:docs/unknown.nvs", 1, 1);
    expect([...s.durations("proof")]).toEqual([["proof:docs/a.nvs", { judged: 120, recorded: 951 }]]);
    s.recordRun("proof:docs/a.nvs", { def: "d", verdict: "red", keys: new Map() });
    expect(s.durations("proof").get("proof:docs/a.nvs")).toEqual({ judged: 120, recorded: 951 });
    expect(s.durations("case").size).toBe(0);
  });

  test("a store made before the time columns gains them and keeps every atom", () => {
    const t = scratch();
    const file = join(t.root, "select.sqlite");
    try {
      const old = new Database(file);
      old.exec("CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL); INSERT INTO meta VALUES ('schema', '1');");
      old.exec(
        "CREATE TABLE atoms (n INTEGER PRIMARY KEY, id TEXT NOT NULL, platform TEXT NOT NULL, kind TEXT NOT NULL, def TEXT NOT NULL DEFAULT '', " +
          "verdict TEXT NOT NULL DEFAULT '', last_run INTEGER NOT NULL DEFAULT 0, nkeys INTEGER NOT NULL DEFAULT 0, keys BLOB, UNIQUE (id, platform));",
      );
      old.exec("INSERT INTO atoms (id, platform, kind, verdict) VALUES ('proof:docs/a.nvs', 'test-os', 'proof', 'green');");
      old.close();
      const s = new SelectStore(file, "test-os");
      expect(s.atom("proof:docs/a.nvs")?.verdict).toBe("green");
      expect(s.durations("proof").size).toBe(0);
      s.setDurations("proof:docs/a.nvs", 5, 50);
      s.close();
      const again = new SelectStore(file, "test-os");
      expect(again.durations("proof").get("proof:docs/a.nvs")).toEqual({ judged: 5, recorded: 50 });
      again.close();
    } finally {
      t.cleanup();
    }
  });
});
