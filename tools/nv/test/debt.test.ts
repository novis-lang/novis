// `bun nv debt`, the floor check every goal ends with, with every runner, the store and the clock handed
// in, so nothing builds and nothing runs: which debt it pays and in what order, that it pays all of it,
// the Linux legs it pays, the progress events it writes under looper, and its exit code.

import { afterAll, describe, expect, test } from "bun:test";
import { mkdirSync, readFileSync, rmSync } from "node:fs";
import { join } from "node:path";
import type { Check } from "../accept/accept.ts";
import type { LegName } from "../accept/legs.ts";
import { type DebtAtom, type DebtKind, type DebtSeams, type DebtTurn, type DebtVoice, debt, debtOf, debtTurns, legsTurn, payDebt } from "../cmd/debt.ts";
import type { LegsCmdSeams } from "../cmd/legs.ts";
import type { Goal } from "../lib/chain.ts";
import { ROOT } from "../lib/paths.ts";

const TMP = join(ROOT, ".agent-tmp", `debt-${process.pid}`);
afterAll(() => rmSync(TMP, { recursive: true, force: true }));

const SILENT: DebtVoice = { step() {}, note() {} };
const MINUTE = 60_000;

interface Run extends DebtSeams {
  /** The store's atoms, by id; a turn that pays one removes it, or marks a red bench run again. */
  store: Map<string, DebtAtom>;
  /** Each turn that started, in order. */
  turns: DebtTurn[];
  clock: { at: number };
}

/** Seams whose store holds `owed`, where every atom a turn pays takes `cost` milliseconds on the clock,
 * and an atom in `red` is red when it ran. */
function seams(owed: DebtAtom[], o: { cost?: number; red?: string[] } = {}): Run {
  const store = new Map(owed.map((a) => [a.id, a]));
  const turns: DebtTurn[] = [];
  const clock = { at: 1_000_000 };
  return {
    store,
    turns,
    clock,
    voice: SILENT,
    now: () => clock.at,
    debt: () => [...store.values()],
    pay: async (turn) => {
      turns.push(turn);
      const red: string[] = [];
      for (const id of turn.ids) {
        clock.at += o.cost ?? MINUTE;
        const a = store.get(id)!;
        if ((o.red ?? []).includes(id)) {
          red.push(id);
          if (a.kind === "bench") store.set(id, { ...a, verdict: "red", lastRun: clock.at });
          else store.delete(id);
        } else store.delete(id);
      }
      return red.length === 0 ? "" : `owed atoms: ${red.length} ran red: ${red.join(", ")}`;
    },
  };
}

/** An atom of the debt; a `heavy:` one is a leg. */
const atom = (id: string, verdict: "red" | "owed", lastRun: number): DebtAtom => ({ id, kind: id.startsWith("heavy:") ? "leg" : (id.slice(0, id.indexOf(":")) as DebtKind), verdict, lastRun });

const fixture = (file: string): Check => ({ id: `run-${file.replace(/\W+/g, "-")}`, kind: "exact", stage: 1, file, want: ["ok"] });

const LEGS_GOAL = {
  checks: [fixture("examples/a.nvs"), fixture("examples/b.nvs")],
  files: ["examples/a.nvs", "examples/b.nvs"],
  stages: [{ number: 1, title: "floor" }],
  env: { wsl: { targetDir: "/var/tmp/nvs-target-gate" } },
} as unknown as Goal;

/** The real legs turn over a fake WSL machine, whose store selects the legs `owed` names. */
function linux(owed: LegName[]) {
  const lines: string[] = [];
  const green = new Set<LegName>();
  const legs: Partial<LegsCmdSeams> = {
    plan: (slug) => ({ slug: slug ?? "demo", goal: LEGS_GOAL }),
    store: async () => ({ reached: (l) => owed.includes(l), green: (l) => void green.add(l), red: () => {}, done: () => {} }),
    hold: async () => () => {},
    machine: {
      platform: "win32",
      hasWsl: () => true,
      hasValgrind: () => true,
      shell: async (_where, line) => (lines.push(line), { code: 0, out: "ok\n", err: "" }),
      sync: async () => "",
      origin: async () => ({ line: "wsl origin: listening", close: () => {} }),
      profile: () => ({ cores: 4 }),
      jobs: () => 1,
      remember: () => {},
      now: () => 0,
    },
  };
  return { lines, green, legs };
}

describe("bun nv debt", () => {
  test("a bench owed or red is debt, and of every other kind an owed atom alone", () => {
    const got = debtOf([
      { id: "bench:benches/members/a.nvs", kind: "bench", verdict: "red", lastRun: 1 },
      { id: "bench:benches/members/b.nvs", kind: "bench", verdict: "owed", lastRun: 1 },
      { id: "proof:docs/examples/a.nvs", kind: "proof", verdict: "owed", lastRun: 1 },
      { id: "proof:docs/examples/b.nvs", kind: "proof", verdict: "red", lastRun: 1 },
      { id: "case:tests/x/a.nvst", kind: "case", verdict: "green", lastRun: 1 },
      { id: "heavy:wsl leg", kind: "heavy", verdict: "owed", lastRun: 1 },
      { id: "heavy:release-guard", kind: "heavy", verdict: "owed", lastRun: 1 },
      { id: "step:nv", kind: "step", verdict: "owed", lastRun: 1 },
    ]);
    expect(got.map((a) => `${a.kind} ${a.id}`)).toEqual(["bench bench:benches/members/a.nvs", "bench bench:benches/members/b.nvs", "proof proof:docs/examples/a.nvs", "leg heavy:wsl leg"]);
  });

  test("red atoms go first, then the owed ones by last run, oldest first, one turn per runner", () => {
    const turns = debtTurns([
      atom("proof:p.nvs", "owed", 50),
      atom("case:tests/x/c.nvst", "owed", 10),
      atom("bench:new.nvs", "owed", 30),
      atom("bench:red.nvs", "red", 90),
      atom("heavy:wsl leg", "owed", 20),
    ]);
    expect(turns).toEqual([
      { runner: "bench", ids: ["bench:red.nvs"] },
      { runner: "full", ids: ["case:tests/x/c.nvst", "proof:p.nvs"] },
      { runner: "leg", ids: ["heavy:wsl leg"] },
      { runner: "bench", ids: ["bench:new.nvs"] },
    ]);
  });

  test("every turn runs however long the debt takes, and each turn says its time", async () => {
    const s = seams([atom("proof:a.nvs", "owed", 1), atom("proof:b.nvs", "owed", 2), atom("bench:red.nvs", "red", 9), atom("heavy:wsl leg", "owed", 3)], { cost: 30 * MINUTE });
    const steps: string[] = [];
    const out = await payDebt(undefined, { ...s, voice: { step: (l) => steps.push(l), note() {} } });
    expect(s.turns.map((t) => t.runner)).toEqual(["bench", "full", "leg"]);
    expect(out.left).toEqual([]);
    expect(steps).toContain("floor debt: paid 2 proof in 60m00s, by the full run's runners");
    expect(steps).toContain("floor debt: paid 4 (2 proof, 1 bench, 1 leg) in 120m00s; left 0");
  });

  test("an owed leg runs the Linux legs the store selects, and no leg debt runs none", async () => {
    /** The seams with the real legs turn over `machine`, and nothing run for any other turn. */
    const withLegs = (s: Run, machine: ReturnType<typeof linux>): Run => ({
      ...s,
      pay: (turn, slug, note) => (turn.runner === "leg" ? legsTurn(slug, note, false, machine.legs) : Promise.resolve("")),
    });
    const l = linux(["wsl leg"]);
    await payDebt("demo", withLegs(seams([atom("heavy:wsl leg", "owed", 1)]), l));
    expect(l.lines.some((line) => line.includes("cargo build --quiet"))).toBe(true);
    expect(l.green).toEqual(new Set<LegName>(["wsl leg"]));
    expect(l.lines.some((line) => line.includes("valgrind --error-exitcode"))).toBe(false);

    const none = linux([]);
    await payDebt("demo", withLegs(seams([atom("proof:a.nvs", "owed", 1)]), none));
    expect(none.lines).toEqual([]);
  });

  test("it exits 0 when the debt is paid green, 1 on a red or debt left, and 2 on a bad argument", async () => {
    expect(await debt([], seams([atom("proof:a.nvs", "owed", 1)]))).toBe(0);
    expect(await debt([], seams([]))).toBe(0);
    expect(await debt([], seams([atom("proof:a.nvs", "owed", 1)], { red: ["proof:a.nvs"] }))).toBe(1);
    expect(await debt(["--goal", "demo"], seams([atom("bench:x.nvs", "owed", 1)], { red: ["bench:x.nvs"] }))).toBe(1);
    const stuck = seams([atom("proof:a.nvs", "owed", 1)]);
    expect(await debt([], { ...stuck, pay: async () => "" })).toBe(1);
    expect(await debt(["--budget", "5"], seams([]))).toBe(2);
    expect(await debt(["--goal"], seams([]))).toBe(2);
  });

  test("under looper, each debt atom is an item, one its runner did not run ends skipped, and the run ends with done", async () => {
    mkdirSync(TMP, { recursive: true });
    const file = join(TMP, "events.ndjson");
    const s = seams([atom("proof:a.nvs", "owed", 1), atom("proof:b.nvs", "owed", 2)]);
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
