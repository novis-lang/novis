import { describe, expect, test } from "bun:test";
import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { descendants, killTree, processTable, run } from "../lib/proc.ts";
import { sinceOverlay } from "../select/change.ts";
import { CovMap } from "../select/extract.ts";
import { ALL_NAMES, pathKeys, readsKeys, spawnKeys } from "../select/keys.ts";
import { advance, caseLabels, depInfoPaths, NO_ADVANCE_ENV } from "../select/record.ts";
import { type ChangeSet, query } from "../select/select.ts";
import { SelectStore } from "../select/store.ts";
import { digest } from "../keys/scan.ts";
import { scratch } from "./scratch.ts";

describe("a recorded tree with an overlay", () => {
  test("a path the overlay names is changed when its bytes moved, and put back as the commit holds it too", () => {
    const s = scratch();
    try {
      s.put("a.txt", "edited");
      s.put("b.txt", "as recorded");
      const overlay = { "a.txt": digest("older edit"), "b.txt": digest("as recorded"), "c.txt": digest("reverted since") };
      // git says a, b and d differ from the commit; c is as the commit holds it again.
      const changes = sinceOverlay(
        [
          { path: "a.txt", status: "modified" },
          { path: "b.txt", status: "modified" },
          { path: "d.txt", status: "added" },
        ],
        overlay,
        s.root,
      );
      expect(changes).toEqual([
        { path: "a.txt", status: "modified" },
        { path: "c.txt", status: "deleted" },
        { path: "d.txt", status: "added" },
      ]);
    } finally {
      s.cleanup();
    }
  });

  test("a path recorded as absent that is on disk now came into being", () => {
    const s = scratch();
    try {
      s.put("n.txt", "new");
      expect(sinceOverlay([], { "n.txt": null }, s.root)).toEqual([{ path: "n.txt", status: "added" }]);
      expect(sinceOverlay([], { "gone.txt": null }, s.root)).toEqual([]);
    } finally {
      s.cleanup();
    }
  });

  test("the store keeps the commit and the overlay together", () => {
    const st = new SelectStore(":memory:", "test-os");
    expect(st.overlay()).toEqual({});
    st.setBase("abc", { "x.rs": "d1", "y.rs": null });
    expect(st.base()).toBe("abc");
    expect(st.overlay()).toEqual({ "x.rs": "d1", "y.rs": null });
  });
});

function change(over: Partial<ChangeSet> = {}): ChangeSet {
  return { since: "base", changes: [], moved: new Map(), global: null, rustFiles: 0, itemChanges: 0, wideFiles: [], items: [], view: new Map(), tree: { commit: "c2", overlay: { "u.rs": "d" } }, ...over };
}

describe("moving the base past a green run", () => {
  test("every selected atom the run did not run is owed, the tree is recorded, and vanished items leave the footprints", () => {
    const st = new SelectStore(":memory:", "test-os");
    st.recordRun("test:a lib a", { def: "", verdict: "green", keys: new Map([["fn:crates/x/src/a.rs#f", ""], ["fn:crates/x/src/a.rs#gone", ""]]) });
    st.recordRun("test:b lib b", { def: "", verdict: "green", keys: new Map([["fn:crates/x/src/a.rs#f", ""]]) });
    st.recordRun("test:r lib r", { def: "", verdict: "red", keys: new Map([["fn:crates/x/src/a.rs#f", ""]]) });
    const item = { id: "gone", kind: "fn", start: 1, end: 2, digest: "x", refs: [], defines: [] } as never;
    const now = { file: "crates/x/src/a.rs", raw: "r", parsed: true, items: [] } as never;
    const c = change({
      changes: [{ path: "crates/x/src/a.rs", status: "modified" }],
      moved: new Map([["fn:crates/x/src/a.rs#f", { path: "crates/x/src/a.rs", how: "changed" }]]),
      items: [{ file: "crates/x/src/a.rs", id: "gone", how: "removed", item, was: item }],
      view: new Map([["crates/x/src/a.rs", now]]),
    });
    const sel = query(st, c);
    expect([...sel.selected.keys()].sort()).toEqual(["test:a lib a", "test:b lib b", "test:r lib r"]);
    const { owed } = advance(st, c, sel, new Set(["test:a lib a"]), null);
    expect(owed).toBe(1);
    expect(st.base()).toBe("c2");
    expect(st.overlay()).toEqual({ "u.rs": "d" });
    expect(st.atom("test:a lib a")!.verdict).toBe("green");
    expect(st.atom("test:b lib b")!.verdict).toBe("owed");
    expect(st.atom("test:r lib r")!.verdict).toBe("red");
    expect(st.footprint("test:a lib a")).toEqual(["fn:crates/x/src/a.rs#f"]);
    expect(st.items("crates/x/src/a.rs")).toMatchObject({ file: "crates/x/src/a.rs" });
    // With the base moved the change is gone, and only what is owed or red is selected.
    const after = query(st, change());
    expect(Object.fromEntries([...after.selected.values()].map((x) => [x.id, x.why]))).toEqual({ "test:b lib b": "owed", "test:r lib r": "red" });
  });

  test("a change replayed from history records no tree", () => {
    const st = new SelectStore(":memory:", "test-os");
    const { tree: _, ...replay } = change();
    advance(st, replay, null, new Set(), null);
    expect(st.base()).toBeNull();
  });

  test("a red run moves the tree too: what ran red stays red, and the next run repeats only it", () => {
    const st = new SelectStore(":memory:", "test-os");
    const key = "fn:crates/x/src/a.rs#f";
    for (const id of ["test:a lib a", "test:b lib b"]) st.recordRun(id, { def: "", verdict: "green", keys: new Map([[key, ""]]) });
    const c = change({ changes: [{ path: "crates/x/src/a.rs", status: "modified" }], moved: new Map([[key, { path: "crates/x/src/a.rs", how: "changed" }]]) });
    const sel = query(st, c);
    st.recordRun("test:a lib a", { def: "", verdict: "red", keys: new Map([[key, ""]]) });
    st.recordRun("test:b lib b", { def: "", verdict: "green", keys: new Map([[key, ""]]) });
    advance(st, c, sel, new Set(["test:a lib a", "test:b lib b"]), null);
    expect(st.base()).toBe("c2");
    const next = query(st, change());
    expect(Object.fromEntries([...next.selected.values()].map((x) => [x.id, x.why]))).toEqual({ "test:a lib a": "red" });
  });

  test("a run another recording run started leaves the tree to it", () => {
    const st = new SelectStore(":memory:", "test-os");
    st.recordRun("test:a lib a", { def: "", verdict: "green", keys: new Map([["file:x", ""]]) });
    const c = change({ moved: new Map([["file:x", { path: "x", how: "path" }]]) });
    const sel = query(st, c);
    process.env[NO_ADVANCE_ENV] = "1";
    try {
      expect(advance(st, c, sel, new Set(), null)).toEqual({ owed: 0 });
    } finally {
      delete process.env[NO_ADVANCE_ENV];
    }
    expect(st.base()).toBeNull();
    expect(st.atom("test:a lib a")!.verdict).toBe("green");
  });
});

describe("process trees", () => {
  test("the process table holds this process, and a tree is killed whole", async () => {
    const table = processTable();
    expect(table.some(([pid]) => pid === process.pid)).toBe(true);
    const argv = process.platform === "win32" ? ["cmd", "/c", "ping -n 60 127.0.0.1 > NUL"] : ["sh", "-c", "sleep 60 & wait"];
    const child = Bun.spawn(argv, { stdout: "ignore", stderr: "ignore" });
    let below: number[] = [];
    for (let i = 0; i < 50 && below.length === 0; i++) {
      await Bun.sleep(100);
      below = descendants(child.pid);
    }
    expect(below.length).toBeGreaterThan(0);
    killTree(child.pid);
    await child.exited;
    await Bun.sleep(200);
    const left = new Set(processTable().map(([pid]) => pid));
    expect(below.filter((p) => left.has(p))).toEqual([]);
  });

  test("a run past its timeout kills what its program started", async () => {
    const argv = process.platform === "win32" ? ["cmd", "/c", "ping -n 60 127.0.0.1 > NUL"] : ["sh", "-c", "sleep 60 & wait"];
    const started = performance.now();
    const r = await run(argv, { timeoutMs: 1_000 });
    expect(r.timedOut).toBe(true);
    expect(performance.now() - started).toBeLessThan(30_000);
  });
});

describe("what a started program read", () => {
  test("git keeps no log: a listing holds every name, a grep its extensions or the whole tree, and history nothing", () => {
    expect(spawnKeys(["C:\\Program Files\\Git\\bin\\git.exe", "ls-files", "-z"])).toEqual([ALL_NAMES]);
    expect(spawnKeys(["git", "grep", "-h", "-o", "fn x", "--", "*.rs"])).toEqual(["ext:rs"]);
    expect(spawnKeys(["git", "grep", "-l", "x", "--", "*.rs", "*.TOML", "*.rs"])).toEqual(["ext:rs", "ext:toml"]);
    expect(spawnKeys(["git", "grep", "fn x", "*.rs"])).toEqual(["tree:."]);
    expect(spawnKeys(["git", "grep", "x", "--", "crates/*.rs"])).toEqual(["tree:."]);
    expect(spawnKeys(["git", "grep", "-f", "patterns.txt", "--", "*.rs"])).toEqual(["tree:."]);
    expect(spawnKeys(["git", "grep", "x", "--"])).toEqual(["tree:."]);
    expect(spawnKeys(["git", "-c", "core.quotepath=off", "status", "--porcelain"])).toEqual(["tree:."]);
    expect(spawnKeys(["git", "rev-parse", "HEAD"])).toEqual([]);
    expect(spawnKeys(["git", "-C", "/elsewhere/scratch", "ls-files"])).toEqual([]);
    expect(spawnKeys(["bun", "nv", "rules"])).toEqual([]);
    const keys = readsKeys({ files: ["a.md"], exists: [], dirs: [], spawns: [["git", "ls-files"]] });
    expect([...keys].sort()).toEqual([ALL_NAMES, "file:a.md"]);
  });

  test("a path that came or went moves every name, and an edit does not", () => {
    expect(pathKeys("docs/new.md", true)).toContain(ALL_NAMES);
    expect(pathKeys("docs/old.md", false)).not.toContain(ALL_NAMES);
  });
});

describe("the readers a recorded run needs", () => {
  test("a case report's failures and skips are read by label", () => {
    const out = "SKIP tests\\a.nvst — no php\r\nFAIL tests\\b\\c.nvst\r\n  want 1\n1 passed, 1 failed, 1 skipped\n";
    expect([...caseLabels(out, "FAIL")]).toEqual(["tests/b/c.nvst"]);
    expect([...caseLabels(out, "SKIP")]).toEqual(["tests/a.nvst"]);
  });

  test("a dep-info file's paths are read, an escaped space kept and a path outside the repository dropped", () => {
    const s = scratch();
    try {
      s.put("deps/a.d", `${s.root}/deps/liba.rmeta: ${s.root}/src/lib.rs ${s.root}/src/my\\ file.rs /elsewhere/x.rs\n\n${s.root}/src/lib.rs:\n`);
      const repo = (p: string) => {
        const n = p.replace(/\\/g, "/");
        const r = s.root.replace(/\\/g, "/");
        return n.startsWith(`${r}/`) ? n.slice(r.length + 1) : null;
      };
      expect([...depInfoPaths(join(s.root, "deps"), repo)].sort()).toEqual(["src/lib.rs", "src/my file.rs"]);
    } finally {
      s.cleanup();
    }
  });

  test("a rebuilt object's coverage map replaces its old one on disk", async () => {
    const s = scratch();
    try {
      s.put("obj.bin", "one");
      const dir = join(s.root, "covmap");
      const cm = new CovMap(dir);
      const load = (cm as unknown as { replaced(o: string, c: string): void }).replaced.bind(cm);
      s.put("covmap/first.json", "{}");
      load(join(s.root, "obj.bin"), join(dir, "first.json"));
      s.put("covmap/second.json", "{}");
      load(join(s.root, "obj.bin"), join(dir, "second.json"));
      expect(existsSync(join(dir, "first.json"))).toBe(false);
      expect(JSON.parse(readFileSync(join(dir, "index.json"), "utf8"))[join(s.root, "obj.bin")]).toBe(join(dir, "second.json"));
    } finally {
      s.cleanup();
    }
  });
});
