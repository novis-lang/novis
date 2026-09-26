import { afterAll, describe, expect, test } from "bun:test";
import { existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { ROOT } from "../lib/paths.ts";
import { keepMisses, keptMisses, selectionMisses } from "../select/full.ts";
import { applyEdits, attribute, describeBatch, type Edit, mutateBatch, mutateBatches, overlap, removeScratch, revertEdits, unselectedReds, withStoreCopy } from "../select/mutate.ts";
import type { ChangeSet, Selected, Selection } from "../select/select.ts";
import { kindOfAtom, SelectStore, STORE_ENV, type Verdict } from "../select/store.ts";

const DIR = join(ROOT, ".agent-tmp", `select-mutate-test-${process.pid}`);
let n = 0;

/** A fixture tree of its own under `.agent-tmp/`, with `a.txt` (LF) and `b.txt` (CRLF). */
function tree(): string {
  const root = join(DIR, `t${++n}`);
  mkdirSync(join(root, "src"), { recursive: true });
  writeFileSync(join(root, "a.txt"), "one\ntwo\nthree\n");
  writeFileSync(join(root, "src", "b.txt"), "alpha\r\nbeta\r\ngamma\r\n");
  return root;
}

const read = (root: string, f: string) => readFileSync(join(root, f), "utf8");

function selection(ids: string[]): Selection {
  const selected = new Map<string, Selected>(ids.map((id) => [id, { id, kind: kindOfAtom(id), why: "key", keys: [] }]));
  return { change: {} as ChangeSet, selected, known: {}, gone: [] };
}

afterAll(() => rmSync(DIR, { recursive: true, force: true }));

describe("the mutation harness", () => {
  test("edits apply, a CRLF file matches an LF find, a new file is created, and revert puts every byte back", () => {
    const root = tree();
    const before = [readFileSync(join(root, "a.txt")), readFileSync(join(root, "src", "b.txt"))];
    const applied = applyEdits(
      [
        { file: "a.txt", find: "two", replace: "2", note: "" },
        { file: "src/b.txt", find: "alpha\nbeta", replace: "ALPHA\nBETA", note: "" },
        { file: "src/new.txt", find: "", replace: "made", note: "" },
        { file: "a.txt", find: "three", replace: "3", note: "" },
      ],
      root,
    );
    expect(read(root, "a.txt")).toBe("one\n2\n3\n");
    expect(read(root, "src/b.txt")).toBe("ALPHA\r\nBETA\r\ngamma\r\n");
    expect(read(root, "src/new.txt")).toBe("made");
    expect(applied.map((a) => a.file)).toEqual(["a.txt", "src/b.txt", "src/new.txt"]);
    revertEdits(applied, root);
    expect(readFileSync(join(root, "a.txt"))).toEqual(before[0]!);
    expect(readFileSync(join(root, "src", "b.txt"))).toEqual(before[1]!);
    expect(existsSync(join(root, "src", "new.txt"))).toBe(false);
  });

  test("a find that is absent or occurs twice throws before any file is written", () => {
    const root = tree();
    writeFileSync(join(root, "twice.txt"), "x x");
    const first: Edit = { file: "a.txt", find: "one", replace: "1", note: "" };
    expect(() => applyEdits([first, { file: "twice.txt", find: "x", replace: "y", note: "" }], root)).toThrow(/occurs 2 times/);
    expect(read(root, "a.txt")).toBe("one\ntwo\nthree\n");
    expect(() => applyEdits([first, { file: "a.txt", find: "four", replace: "4", note: "" }], root)).toThrow(/occurs 0 times/);
    expect(read(root, "a.txt")).toBe("one\ntwo\nthree\n");
    expect(() => applyEdits([{ file: "a.txt", find: "", replace: "z", note: "" }], root)).toThrow(/exists/);
  });

  test("the subset check names every red atom the selection did not pick", () => {
    const ran = new Map<string, Verdict>([
      ["case:tests/a.nvst", "red"],
      ["case:tests/b.nvst", "green"],
      ["test:x lib x", "red"],
      ["nvtest:tools/nv/test/q.test.ts", "red"],
    ]);
    expect(unselectedReds(ran, new Set(["case:tests/a.nvst", "case:tests/b.nvst"]))).toEqual(["nvtest:tools/nv/test/q.test.ts", "test:x lib x"]);
    expect(unselectedReds(ran, new Set(ran.keys()))).toEqual([]);
  });

  test("a run on the store's copy writes nothing into the real store, and the copy is gone afterwards", async () => {
    const root = tree();
    const file = join(root, "real.sqlite");
    const real = new SelectStore(file, "test-os");
    real.recordRun("case:tests/a.nvst", { def: "d", verdict: "green", keys: new Map([["fn:a.rs#f", ""]]) });
    real.close();
    const scratch = join(root, "copy");
    const was = process.env[STORE_ENV];
    await withStoreCopy(file, scratch, async (store) => {
      expect(process.env[STORE_ENV]).toBe(join(scratch, "select.sqlite"));
      const copy = new SelectStore(store.file, "test-os");
      copy.recordRun("case:tests/a.nvst", { def: "d", verdict: "red", keys: new Map() });
      copy.recordRun("case:tests/b.nvst", { def: "d", verdict: "red", keys: new Map() });
      copy.close();
    });
    expect(process.env[STORE_ENV]).toBe(was);
    expect(existsSync(scratch)).toBe(false);
    const after = new SelectStore(file, "test-os");
    expect(after.atom("case:tests/a.nvst")?.verdict).toBe("green");
    expect(after.atom("case:tests/b.nvst")).toBeNull();
    after.close();
  });

  test("the scratch directory is deleted again while it is busy, and any other error is thrown at once", () => {
    const busy = (code: string) => Object.assign(new Error(code), { code });
    let calls = 0;
    removeScratch("x", { delayMs: 0, rm: () => { if (++calls < 3) throw busy("EBUSY"); } });
    expect(calls).toBe(3);
    calls = 0;
    expect(() => removeScratch("x", { tries: 4, delayMs: 0, rm: () => { calls++; throw busy("EBUSY"); } })).toThrow("EBUSY");
    expect(calls).toBe(4);
    calls = 0;
    expect(() => removeScratch("x", { delayMs: 0, rm: () => { calls++; throw busy("ENOENT"); } })).toThrow("ENOENT");
    expect(calls).toBe(1);
  });

  test("a batch's report counts its red atoms by kind", () => {
    const lines = describeBatch({ index: 1, edits: [], selected: { case: 2 }, red: ["case:a.nvst", "case:b.nvst", "proof:c.nvs"], misses: [] });
    expect(lines).toContain("  selected: 2 case");
    expect(lines).toContain("  red: 3 (2 case, 1 proof): case:a.nvst, case:b.nvst, proof:c.nvs");
  });

  test("a batch applies its edits for the run, reports the red atoms not selected, and reverts", async () => {
    const root = tree();
    const file = join(root, "real.sqlite");
    new SelectStore(file).close();
    const edits: Edit[] = [{ file: "a.txt", find: "two", replace: "TWO", note: "a changed word" }];
    let seen = "";
    const rep = await mutateBatch(1, edits, {
      root,
      storeFile: file,
      scratch: join(root, "copy"),
      status: () => "",
      run: async () => {
        seen = read(root, "a.txt");
        return {
          selection: selection(["case:tests/a.nvst", "test:x lib x"]),
          ran: new Map<string, Verdict>([
            ["case:tests/a.nvst", "red"],
            ["case:tests/c.nvst", "red"],
          ]),
        };
      },
      explain: (_s, _sel, id) => [`${id}: not selected`],
    });
    expect(seen).toBe("one\nTWO\nthree\n");
    expect(read(root, "a.txt")).toBe("one\ntwo\nthree\n");
    expect(rep.failed).toBeUndefined();
    expect(rep.selected).toEqual({ case: 1, test: 1 });
    expect(rep.red).toEqual(["case:tests/a.nvst", "case:tests/c.nvst"]);
    expect(rep.misses).toEqual([{ id: "case:tests/c.nvst", explain: ["case:tests/c.nvst: not selected"] }]);
  });

  test("batches overlap when their edits share text of one file, and not when they only touch", () => {
    const root = tree();
    const e = (file: string, find: string): Edit => ({ file, find, replace: `${find}!`, note: "" });
    expect(overlap([{ index: 1, edits: [e("a.txt", "one")] }, { index: 2, edits: [e("a.txt", "three")] }, { index: 3, edits: [e("src/b.txt", "beta")] }], root)).toBeNull();
    expect(overlap([{ index: 1, edits: [e("a.txt", "one\ntwo")] }, { index: 2, edits: [e("a.txt", "two\nthree")] }], root)).toBe("batch 1 and batch 2 both edit the same text of a.txt");
    // A batch's span runs from its first found text to its last, so a batch editing between them overlaps.
    expect(overlap([{ index: 1, edits: [e("a.txt", "one"), e("a.txt", "three")] }, { index: 2, edits: [e("a.txt", "two")] }], root)).toMatch(/batch 1 and batch 2/);
    // The CRLF file is matched with its own line ends.
    expect(overlap([{ index: 1, edits: [e("src/b.txt", "alpha\nbeta")] }, { index: 2, edits: [e("src/b.txt", "beta")] }], root)).toMatch(/src\/b\.txt/);
    // A created file is the whole file.
    expect(overlap([{ index: 1, edits: [{ file: "new.txt", find: "", replace: "x", note: "" }] }, { index: 2, edits: [{ file: "new.txt", find: "", replace: "y", note: "" }] }], root)).toMatch(/new\.txt/);
  });

  test("batches run at once apply every edit for one run, check the combined selection, trace red atoms to each batch, and revert", async () => {
    const root = tree();
    const file = join(root, "real.sqlite");
    new SelectStore(file).close();
    const parts = [
      { index: 2, edits: [{ file: "a.txt", find: "two", replace: "TWO", note: "a word" }] },
      { index: 4, edits: [{ file: "src/b.txt", find: "gamma", replace: "GAMMA", note: "a CRLF word" }] },
    ];
    let seen: string[] = [];
    let runs = 0;
    const sel = selection(["case:tests/a.nvst", "case:tests/b.nvst"]);
    sel.selected.get("case:tests/a.nvst")!.keys.push({ key: "file:a.txt", origin: { path: "a.txt", how: "changed" } });
    sel.selected.get("case:tests/b.nvst")!.keys.push({ key: "file:src/b.txt", origin: { path: "src/b.txt", how: "changed" } });
    const rep = await mutateBatches(parts, {
      root,
      storeFile: file,
      scratch: join(root, "copy"),
      status: () => "",
      run: async () => {
        runs++;
        seen = [read(root, "a.txt"), read(root, "src/b.txt")];
        return {
          selection: sel,
          ran: new Map<string, Verdict>([
            ["case:tests/a.nvst", "red"],
            ["case:tests/b.nvst", "red"],
            ["case:tests/c.nvst", "red"],
          ]),
        };
      },
    });
    expect(runs).toBe(1);
    expect(seen).toEqual(["one\nTWO\nthree\n", "alpha\r\nbeta\r\nGAMMA\r\n"]);
    expect(read(root, "a.txt")).toBe("one\ntwo\nthree\n");
    expect(read(root, "src/b.txt")).toBe("alpha\r\nbeta\r\ngamma\r\n");
    expect(rep.failed).toBeUndefined();
    expect(rep.red).toEqual(["case:tests/a.nvst", "case:tests/b.nvst", "case:tests/c.nvst"]);
    expect(rep.misses.map((m) => m.id)).toEqual(["case:tests/c.nvst"]);
    expect(rep.parts?.map((p) => [p.index, p.red])).toEqual([
      [2, ["case:tests/a.nvst"]],
      [4, ["case:tests/b.nvst"]],
    ]);
    const lines = describeBatch(rep);
    expect(lines[0]).toBe("batches 2, 4 at once: 2 edit(s)");
    expect(lines).toContain("  batch 2: 1 edit(s), 1 red atom(s) traced to its files: case:tests/a.nvst");
    expect(lines).toContain("  MISS  case:tests/c.nvst went red and was not selected");

    let ran = false;
    const clash = await mutateBatches([parts[0]!, { index: 3, edits: [{ file: "a.txt", find: "one\ntwo", replace: "x", note: "" }] }], {
      root,
      storeFile: file,
      scratch: join(root, "copy"),
      status: () => "",
      run: async () => {
        ran = true;
        return { selection: selection([]), ran: new Map() };
      },
    });
    expect(ran).toBe(false);
    expect(clash.failed).toBe("the batches cannot be applied at once: batch 2 and batch 3 both edit the same text of a.txt");
    expect(read(root, "a.txt")).toBe("one\ntwo\nthree\n");
  });

  test("a red atom a batch's own file names is traced to that batch", () => {
    const parts = [{ index: 1, edits: [{ file: "docs/examples/x/01-a.out", find: "1", replace: "2", note: "" }] }];
    expect(attribute(parts, selection([]), ["proof:docs/examples/x/01-a.nvs", "case:tests/z.nvst"])).toEqual([{ index: 1, red: ["proof:docs/examples/x/01-a.nvs"] }]);
  });

  test("a run that throws still reverts, and a tree left changed fails the batch", async () => {
    const root = tree();
    const file = join(root, "real.sqlite");
    new SelectStore(file).close();
    const edits: Edit[] = [{ file: "a.txt", find: "one", replace: "ONE", note: "" }];
    const thrown = await mutateBatch(1, edits, { root, storeFile: file, scratch: join(root, "copy"), status: () => "", run: async () => Promise.reject(new Error("boom")) });
    expect(thrown.failed).toBe("boom");
    expect(read(root, "a.txt")).toBe("one\ntwo\nthree\n");
    let calls = 0;
    const left = await mutateBatch(1, edits, {
      root,
      storeFile: file,
      scratch: join(root, "copy"),
      status: () => (calls++ === 0 ? "" : "?? stray.txt\n"),
      run: async () => ({ selection: selection([]), ran: new Map<string, Verdict>([["case:tests/a.nvst", "red"]]) }),
    });
    expect(left.failed).toMatch(/not as it was/);
    const none = await mutateBatch(1, edits, { root, storeFile: file, scratch: join(root, "copy"), status: () => "", run: async () => ({ selection: selection([]), ran: new Map() }) });
    expect(none.failed).toMatch(/no atom went red/);
  });
});

describe("the full run's selection misses", () => {
  test("a miss is red now and not selected, whatever it was before, and --full and --mutate count the same misses", () => {
    const before = new Map([
      ["case:tests/a.nvst", { verdict: "green" as Verdict, lastRun: 5 }],
      ["case:tests/b.nvst", { verdict: "green" as Verdict, lastRun: 6 }],
      ["case:tests/c.nvst", { verdict: "red" as Verdict, lastRun: 7 }],
      ["case:tests/d.nvst", { verdict: "owed" as Verdict, lastRun: 8 }],
    ]);
    const ran = new Map<string, Verdict>([
      ["case:tests/a.nvst", "red"],
      ["case:tests/b.nvst", "red"],
      ["case:tests/c.nvst", "red"],
      ["case:tests/d.nvst", "red"],
      ["case:tests/new.nvst", "red"],
    ]);
    const selected = new Set(["case:tests/b.nvst"]);
    const misses = selectionMisses(before, selected, ran);
    expect(misses).toEqual([
      { id: "case:tests/a.nvst", lastGreen: 5 },
      { id: "case:tests/c.nvst", lastGreen: 0 },
      { id: "case:tests/d.nvst", lastGreen: 0 },
      { id: "case:tests/new.nvst", lastGreen: 0 },
    ]);
    expect(unselectedReds(ran, selected)).toEqual(misses.map((m) => m.id));
  });

  test("a kept miss stays until a full run finds its atom green", () => {
    const s = new SelectStore(":memory:", "test-os");
    keepMisses(s, [{ id: "case:tests/a.nvst", lastGreen: 5 }], new Map([["case:tests/a.nvst", "red"]]));
    expect([...keptMisses(s).keys()]).toEqual(["case:tests/a.nvst"]);
    keepMisses(s, [], new Map([["case:tests/b.nvst", "green"]]));
    expect(keptMisses(s).get("case:tests/a.nvst")?.lastGreen).toBe(5);
    keepMisses(s, [], new Map([["case:tests/a.nvst", "green"]]));
    expect(keptMisses(s).size).toBe(0);
    s.close();
  });
});
