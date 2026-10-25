// `bun nv debt`, the floor check every goal ends with, with the runner, the store and the clock handed
// in, so nothing builds and nothing runs: which debt it pays and in what order, that it pays all of it,
// the progress events it writes under looper, and its exit code.

import { afterAll, describe, expect, test } from "bun:test";
import { mkdirSync, readFileSync, rmSync } from "node:fs";
import { join } from "node:path";
import { type DebtAtom, type DebtKind, type DebtSeams, type DebtVoice, debt, debtOf, debtOrder, payDebt } from "../cmd/debt.ts";
import { ROOT } from "../lib/paths.ts";

const TMP = join(ROOT, ".agent-tmp", `debt-${process.pid}`);
afterAll(() => rmSync(TMP, { recursive: true, force: true }));

const SILENT: DebtVoice = { step() {}, note() {} };
const MINUTE = 60_000;

interface Run extends DebtSeams {
  /** The store's atoms, by id; a round that pays one removes it. */
  store: Map<string, DebtAtom>;
  /** The ids each round ran, in order. */
  turns: string[][];
  clock: { at: number };
}

/** Seams whose store holds `owed`, where every atom a round pays takes `cost` milliseconds on the clock,
 * and an atom in `red` is red when it ran. */
function seams(owed: DebtAtom[], o: { cost?: number; red?: string[] } = {}): Run {
  const store = new Map(owed.map((a) => [a.id, a]));
  const turns: string[][] = [];
  const clock = { at: 1_000_000 };
  return {
    store,
    turns,
    clock,
    voice: SILENT,
    now: () => clock.at,
    debt: () => [...store.values()],
    pay: async (ids) => {
      turns.push(ids);
      const red: string[] = [];
      for (const id of ids) {
        clock.at += o.cost ?? MINUTE;
        if ((o.red ?? []).includes(id)) red.push(id);
        store.delete(id);
      }
      return red.length === 0 ? "" : `owed atoms: ${red.length} ran red: ${red.join(", ")}`;
    },
  };
}

/** An atom of the debt, its kind read off its id. */
const atom = (id: string, lastRun: number): DebtAtom => ({ id, kind: id.slice(0, id.indexOf(":")) as DebtKind, lastRun });

describe("bun nv debt", () => {
  test("an owed atom of any kind but a bench or a heavy check is debt, and a red one only when verify never runs it", () => {
    const got = debtOf([
      { id: "bench:benches/members/a.nvs", kind: "bench", verdict: "red", lastRun: 1 },
      { id: "bench:benches/members/b.nvs", kind: "bench", verdict: "owed", lastRun: 1 },
      { id: "proof:docs/examples/a.nvs", kind: "proof", verdict: "owed", lastRun: 1 },
      { id: "proof:docs/examples/b.nvs", kind: "proof", verdict: "red", lastRun: 1 },
      { id: "case:tests/x/a.nvst", kind: "case", verdict: "green", lastRun: 1 },
      { id: "test:nvs-cli test a", kind: "test", verdict: "red", lastRun: 1 },
      { id: "heavy:wsl leg", kind: "heavy", verdict: "owed", lastRun: 1 },
      { id: "heavy:valgrind sweep", kind: "heavy", verdict: "red", lastRun: 1 },
      { id: "heavy:release-guard", kind: "heavy", verdict: "owed", lastRun: 1 },
      { id: "step:nv", kind: "step", verdict: "owed", lastRun: 1 },
    ]);
    expect(got.map((a) => `${a.kind} ${a.id}`)).toEqual(["proof proof:docs/examples/a.nvs", "proof proof:docs/examples/b.nvs"]);
  });

  test("the owed atoms go by last run, oldest first, then by id", () => {
    const order = debtOrder([atom("proof:p.nvs", 50), atom("case:tests/x/c.nvst", 10), atom("proof:b.nvs", 30), atom("test:nvs-cli test a", 30)]);
    expect(order).toEqual(["case:tests/x/c.nvst", "proof:b.nvs", "test:nvs-cli test a", "proof:p.nvs"]);
  });

  test("a round runs however long the debt takes, and says its time", async () => {
    const s = seams([atom("proof:a.nvs", 1), atom("proof:b.nvs", 2), atom("case:tests/x/c.nvst", 3)], { cost: 30 * MINUTE });
    const steps: string[] = [];
    const out = await payDebt({ ...s, voice: { step: (l) => steps.push(l), note() {} } });
    expect(s.turns).toEqual([["proof:a.nvs", "proof:b.nvs", "case:tests/x/c.nvst"]]);
    expect(out.left).toEqual([]);
    expect(steps).toContain("floor debt: paid 2 proof, 1 case in 90m00s");
    expect(steps).toContain("floor debt: paid 3 (2 proof, 1 case) in 90m00s; left 0");
  });

  test("an atom a turn's tree move owes is paid by a further round, and one a round left unpaid ends the rounds", async () => {
    const s = seams([atom("proof:a.nvs", 1)]);
    const pay = s.pay;
    let moved = false;
    const steps: string[] = [];
    // The first turn's tree move owes two atoms the change reached and the turn did not run.
    const out = await payDebt({
      ...s,
      pay: async (ids, ...rest) => {
        const r = await pay(ids, ...rest);
        if (!moved) {
          moved = true;
          for (const a of [atom("nvtest:tools/nv/test/x.test.ts", 5), atom("test:nvs-cli test b", 6)]) s.store.set(a.id, a);
        }
        return r;
      },
      voice: { step: (l) => steps.push(l), note() {} },
    });
    expect(s.turns).toEqual([["proof:a.nvs"], ["nvtest:tools/nv/test/x.test.ts", "test:nvs-cli test b"]]);
    expect(out.left).toEqual([]);
    expect(steps).toContain("floor debt: 2 more atom(s), 1 nvtest, 1 test, owed by the run's own tree move");
    expect(steps.at(-1)).toBe("floor debt: paid 3 (1 nvtest, 1 proof, 1 test) in 3m00s; left 0");

    const stuck = seams([atom("proof:a.nvs", 1)]);
    const named: string[] = [];
    const once = await payDebt({ ...stuck, pay: async (ids) => (stuck.turns.push(ids), ""), voice: { step: (l) => named.push(l), note() {} } });
    expect(stuck.turns.length).toBe(1);
    expect(once.left.map((a) => a.id)).toEqual(["proof:a.nvs"]);
    expect(named.at(-1)).toBe("floor debt: still owed: proof:a.nvs");
  });


  test("it exits 0 when the debt is paid green, 1 on a red or debt left, and 2 on a bad argument", async () => {
    expect(await debt([], seams([atom("proof:a.nvs", 1)]))).toBe(0);
    expect(await debt([], seams([]))).toBe(0);
    expect(await debt([], seams([atom("proof:a.nvs", 1)], { red: ["proof:a.nvs"] }))).toBe(1);
    const stuck = seams([atom("proof:a.nvs", 1)]);
    expect(await debt([], { ...stuck, pay: async () => "" })).toBe(1);
    expect(await debt(["--budget", "5"], seams([]))).toBe(2);
    expect(await debt(["--goal", "demo"], seams([]))).toBe(2);
  });

  test("under looper, each debt atom is an item, one its runner did not run ends skipped, and the run ends with done", async () => {
    mkdirSync(TMP, { recursive: true });
    const file = join(TMP, "events.ndjson");
    const s = seams([atom("proof:a.nvs", 1), atom("proof:b.nvs", 2)]);
    const was = process.env.LOOPER_EVENTS;
    process.env.LOOPER_EVENTS = file;
    try {
      // The runner pays the first atom and never reaches the second.
      expect(await debt([], { ...s, pay: async () => (s.store.delete("proof:a.nvs"), "") })).toBe(1);
    } finally {
      if (was === undefined) delete process.env.LOOPER_EVENTS;
      else process.env.LOOPER_EVENTS = was;
    }
    const got = readFileSync(file, "utf8")
      .split("\n")
      .filter((l) => l !== "")
      .map((l) => JSON.parse(l) as { event: string; id?: string; parent?: string; kind?: string; result?: string });
    const said = got.map((e) => `${e.event}${e.id ? ` ${e.id}` : ""}${e.result ? ` ${e.result}` : ""}`);
    expect(said).toEqual(["add debt", "add proof:a.nvs", "add proof:b.nvs", "planned", "start debt", "end proof:a.nvs", "end proof:b.nvs skipped", "end debt", "done red"]);
    expect(got[1]).toMatchObject({ parent: "debt", kind: "proof" });
  });
});
