// A run counts the moment each atom's run is recorded: the stamp of the tree it was taken at, read by
// the next selection, cleared when the base moves past it.

import { describe, expect, test } from "bun:test";
import { Database } from "bun:sqlite";
import { join } from "node:path";
import { scanItems } from "../keys/scan.ts";
import type { Result } from "../proofs/run.ts";
import { judgeKept } from "../proofs/select.ts";
import { proofId } from "../select/atoms.ts";
import { advance } from "../select/record.ts";
import { computeChange, query, type Selection } from "../select/select.ts";
import { type RunTree, SelectStore } from "../select/store.ts";
import { scratch, type Scratch } from "./scratch.ts";

const LIB = "crates/demo/src/lib.rs";
const fns = (a: number, b: number, c: number) => `pub fn a() -> u8 {\n    ${a}\n}\n\npub fn b() -> u8 {\n    ${b}\n}\n\npub fn c() -> u8 {\n    ${c}\n}\n`;
const A = "test:demo test a";
const B = "test:demo test b";
const C = "test:demo test c";

/** A repository holding `LIB` with `fns(1, 2, 3)` committed, and a store whose base is that commit and
 * whose atoms A, B and C ran `a`, `b` and `c` green there. A store `file` goes under `.git/`, where its
 * bytes are no part of the tree. */
function repo(file = ":memory:"): { t: Scratch; s: SelectStore; select: () => Promise<Selection>; done: () => void } {
  const t = scratch();
  const git = (...args: string[]) => {
    const r = Bun.spawnSync(["git", ...args], { cwd: t.root, stdout: "pipe", stderr: "pipe" });
    if (r.exitCode !== 0) throw new Error(`git ${args.join(" ")}: ${r.stderr.toString()}`);
    return r.stdout.toString().trim();
  };
  git("init", "-q");
  git("config", "user.email", "test@example.com");
  git("config", "user.name", "test");
  git("config", "core.hooksPath", ".no-hooks");
  t.put(LIB, fns(1, 2, 3));
  git("add", ".");
  git("commit", "-q", "-m", "start");
  const s = new SelectStore(file === ":memory:" ? file : join(t.root, ".git", file), "test-os");
  s.setItems(scanItems([LIB], t.root));
  s.setBase(git("rev-parse", "HEAD"));
  for (const [id, f] of [[A, "a"], [B, "b"], [C, "c"]] as const) s.recordRun(id, { def: "", verdict: "green", keys: new Map([[`fn:${LIB}#${f}`, ""]]) });
  const select = async () => query(s, await computeChange(s, { graph: null, root: t.root }), { discovered: [A, B, C] });
  return {
    t,
    s,
    select,
    done: () => {
      s.close();
      t.cleanup();
    },
  };
}

const ids = (sel: Selection) => [...sel.selected.keys()].sort();
const green = (s: SelectStore, id: string) => s.recordRun(id, { def: "", verdict: "green", keys: new Map([[`fn:${LIB}#${id.slice(-1)}`, ""]]) });

describe("a run killed halfway", () => {
  test("keeps every atom it finished: at the same tree only the rest is selected", async () => {
    const r = repo();
    try {
      r.t.put(LIB, fns(10, 20, 3));
      expect(ids(await r.select())).toEqual([A, B]);
      green(r.s, A);
      // No `advance`: the run was killed here.
      expect(ids(await r.select())).toEqual([B]);
      expect(r.s.stamps().has(A)).toBe(true);
      expect(r.s.stamps().has(B)).toBe(false);
    } finally {
      r.done();
    }
  }, 180_000);

  test("a further edit selects a finished atom only when the edit since its own run reaches it", async () => {
    const r = repo();
    try {
      r.t.put(LIB, fns(10, 20, 3));
      await r.select();
      green(r.s, A);
      r.t.put(LIB, fns(10, 20, 30));
      // A ran on `a` as it is, and only `c` moved since; B never ran and C moved since the base.
      expect(ids(await r.select())).toEqual([B, C]);
      r.t.put(LIB, fns(11, 20, 30));
      expect(ids(await r.select())).toEqual([A, B, C]);
    } finally {
      r.done();
    }
  }, 180_000);

  test("putting a file back as the base had it still selects the atom that ran on the edit", async () => {
    const r = repo();
    try {
      r.t.put(LIB, fns(10, 2, 3));
      await r.select();
      green(r.s, A);
      r.t.put(LIB, fns(1, 2, 3));
      // The tree is the base again, so nothing moved since the base; `a` moved since A's own run.
      expect(ids(await r.select())).toEqual([A]);
    } finally {
      r.done();
    }
  }, 180_000);

  test("an atom stamped with a tree that cannot be compared is selected", async () => {
    const r = repo();
    try {
      const gone: RunTree = { commit: "0".repeat(40), overlay: {}, items: [], markers: {} };
      r.s.runAt("a tree git cannot name", () => gone);
      green(r.s, A);
      const sel = await r.select();
      expect(ids(sel)).toEqual([A]);
      expect(sel.selected.get(A)?.why).toBe("global");
    } finally {
      r.done();
    }
  }, 180_000);

  test("a base commit a rewritten history removed selects everything but what ran on the tree as it is", async () => {
    const r = repo();
    try {
      r.t.put(LIB, fns(10, 20, 3));
      await r.select();
      green(r.s, A);
      r.s.setBase("0".repeat(40));
      const change = await computeChange(r.s, { graph: null, root: r.t.root });
      expect(change.global).toBe(`the recorded tree ${"0".repeat(12)} is no longer in the repository`);
      expect(change.full).toBe(true);
      const sel = query(r.s, change, { discovered: [A, B, C] });
      expect(ids(sel)).toEqual([B, C]);
      expect(sel.selected.get(B)?.why).toBe("global");
      // A green run moves the base off the lost commit.
      advance(r.s, change, sel, new Set([B, C]), null, r.t.root);
      expect(r.s.base()).toBe(change.tree!.commit);
      expect(ids(await r.select())).toEqual([]);
    } finally {
      r.done();
    }
  }, 180_000);

  test("a red atom stays selected, and so does an owed one", async () => {
    const r = repo();
    try {
      r.t.put(LIB, fns(10, 20, 3));
      await r.select();
      r.s.recordRun(A, { def: "", verdict: "red", keys: new Map([[`fn:${LIB}#a`, ""]]) });
      green(r.s, B);
      r.s.owe([B]);
      const sel = await r.select();
      expect(sel.selected.get(A)?.why).toBe("red");
      expect(sel.selected.get(B)?.why).toBe("owed");
    } finally {
      r.done();
    }
  }, 180_000);
});

describe("moving the base", () => {
  test("clears the stamps it covers and deletes the trees no atom is stamped with", async () => {
    const r = repo();
    try {
      r.t.put(LIB, fns(10, 20, 3));
      await r.select();
      green(r.s, A);
      r.t.put(LIB, fns(10, 20, 30));
      const c = await computeChange(r.s, { graph: null, root: r.t.root });
      const sel = query(r.s, c, { discovered: [A, B, C] });
      expect(ids(sel)).toEqual([B, C]);
      green(r.s, B);
      // C is selected and does not run, so it is owed; A was not selected and holds on this tree.
      advance(r.s, c, sel, new Set([B]), null, r.t.root);
      expect(r.s.stamps().size).toBe(0);
      expect(r.s.atom(C)?.verdict).toBe("owed");
      expect((r.s.db.query("SELECT COUNT(*) AS n FROM trees").get() as { n: number }).n).toBe(0);
      expect(ids(await r.select())).toEqual([C]);
    } finally {
      r.done();
    }
  }, 180_000);

  test("keeps a stamp written after the selection it moves by was read", async () => {
    const r = repo("select.sqlite");
    try {
      r.t.put(LIB, fns(10, 2, 3));
      const c = await computeChange(r.s, { graph: null, root: r.t.root });
      const sel = query(r.s, c, { discovered: [A, B, C] });
      green(r.s, A);
      // Another process records C on a tree of its own once the selection was read.
      const other = new SelectStore(r.s.file, "test-os");
      try {
        other.runAt("another tree", () => ({ commit: c.tree!.commit, overlay: { "x.txt": "1" }, items: [], markers: {} }));
        green(other, C);
      } finally {
        other.close();
      }
      advance(r.s, c, sel, new Set([A]), null, r.t.root);
      expect([...r.s.stamps().keys()]).toEqual([C]);
      expect(r.s.treeN("another tree")).not.toBeNull();
    } finally {
      r.done();
    }
  }, 180_000);
});

describe("a judged verdict", () => {
  test("survives a kill before its atom is recorded, and only on the tree it was judged at", async () => {
    const r = repo("select.sqlite");
    try {
      r.t.put(LIB, fns(10, 2, 3));
      await r.select();
      const key = r.s.judgedKey("opts")!;
      r.s.putJudged(A, key, { verdict: "ok" });
      // The process is killed: a new one opens the store and reads the same tree.
      r.s.close();
      const s2 = new SelectStore(r.s.file, "test-os");
      try {
        query(s2, await computeChange(s2, { graph: null, root: r.t.root }));
        expect(s2.judgedKey("opts")).toBe(key);
        expect(s2.judged<{ verdict: string }>(A, key)?.verdict).toBe("ok");
        expect(s2.judged(A, s2.judgedKey("other opts")!)).toBeNull();
        green(s2, A);
        expect(s2.judged(A, key)).toBeNull();
        s2.putJudged(B, key, { verdict: "ok" });
        r.t.put(LIB, fns(10, 20, 3));
        const c = await computeChange(s2, { graph: null, root: r.t.root });
        advance(s2, c, query(s2, c), new Set(), null, r.t.root);
        expect(s2.judged(B, key)).toBeNull();
      } finally {
        s2.close();
      }
    } finally {
      r.t.cleanup();
    }
  }, 180_000);
});

describe("a proof program judged and not recorded", () => {
  const SQRT = "docs/examples/core/Math/sqrt/01-square-roots.nvs";
  const opts = { valgrind: false, strict: false, unlogged: true };

  test("keeps its verdict the moment it is judged, and is not judged again on the same tree", async () => {
    const s = new SelectStore(":memory:", "test-os");
    try {
      s.runAt("t", () => ({ commit: "c", overlay: {}, items: [], markers: {} }));
      // `bun` stands in for `nvs`: it cannot run the program, so the verdict is a failure.
      const first = await judgeKept(s, { path: process.execPath, key: "k", runs: "k", recorded: true }, [{ what: "examples", path: SQRT }], opts);
      const judged = first.results.get(`examples:${SQRT}`)!;
      expect(s.judged<Result>(proofId(SQRT), s.judgedKey("proof native lenient")!)).toEqual(judged);
      // A binary that does not exist would fail for another reason, so the same reason means no run.
      const again = await judgeKept(s, { path: "no-such-nvs", key: "k", runs: "k", recorded: true }, [{ what: "examples", path: SQRT }], opts);
      expect(again.results.get(`examples:${SQRT}`)).toEqual(judged);
      expect(again.width).toBeGreaterThan(0);
      s.runAt("u", () => ({ commit: "c", overlay: { "x.txt": "1" }, items: [], markers: {} }));
      const elsewhere = await judgeKept(s, { path: "no-such-nvs", key: "k", runs: "k", recorded: true }, [{ what: "examples", path: SQRT }], opts);
      expect(elsewhere.results.get(`examples:${SQRT}`)?.why).not.toBe(judged.why);
    } finally {
      s.close();
    }
  }, 120_000);

  test("has no result and keeps no verdict when a stop comes before its run", async () => {
    const s = new SelectStore(":memory:", "test-os");
    try {
      s.runAt("t", () => ({ commit: "c", overlay: {}, items: [], markers: {} }));
      const other = "docs/examples/core/Math/sqrt/02-negative-and-special-values.nvs";
      // Each worker asks before it takes a program, so only the first ask lets one start.
      let asked = 0;
      const pass = await judgeKept(s, { path: process.execPath, key: "k", runs: "k", recorded: true }, [{ what: "examples", path: SQRT }, { what: "examples", path: other }], { ...opts, stop: () => asked++ > 0 });
      expect([...pass.results.keys()]).toEqual([`examples:${SQRT}`]);
      expect(s.judged<Result>(proofId(other), s.judgedKey("proof native lenient")!)).toBeNull();
    } finally {
      s.close();
    }
  }, 120_000);
});

describe("a store made before stamps", () => {
  test("gains the column and the table, and keeps its atoms", () => {
    const t = scratch();
    try {
      const file = join(t.root, "old.sqlite");
      const db = new Database(file);
      db.exec("CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL); INSERT INTO meta VALUES ('schema', '1');");
      db.exec("CREATE TABLE atoms (n INTEGER PRIMARY KEY, id TEXT NOT NULL, platform TEXT NOT NULL, kind TEXT NOT NULL, def TEXT NOT NULL DEFAULT '', verdict TEXT NOT NULL DEFAULT '', last_run INTEGER NOT NULL DEFAULT 0, nkeys INTEGER NOT NULL DEFAULT 0, keys BLOB, UNIQUE (id, platform));");
      db.exec("INSERT INTO atoms (id, platform, kind, verdict) VALUES ('test:x lib x', 'test-os', 'test', 'green');");
      db.close();
      const s = new SelectStore(file, "test-os");
      try {
        expect(s.atom("test:x lib x")?.verdict).toBe("green");
        expect(s.stamps().size).toBe(0);
        s.runAt("t", () => ({ commit: "c", overlay: {}, items: [], markers: {} }));
        s.setVerdict("test:x lib x", "green");
        expect(s.stamps().get("test:x lib x")).toBe(s.treeN("t")!);
      } finally {
        s.close();
      }
    } finally {
      t.cleanup();
    }
  });
});
