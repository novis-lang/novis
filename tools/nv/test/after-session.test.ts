// `bun nv sweep`, looper's hook after every session, with every runner, the store, the clock and `gh`
// handed in, so nothing builds and nothing runs: which debt it pays and in what order, that the budget
// starts no turn once it is spent, the Linux legs it pays, what it reads of the nightly CI run, the lines
// it writes into `.loop/last-sweep.md` beside looper's, and its exit code.

import { afterAll, describe, expect, test } from "bun:test";
import { existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import type { Check } from "../accept/accept.ts";
import type { LegName } from "../accept/legs.ts";
import type { LegsCmdSeams } from "../cmd/legs.ts";
import {
  DEBT_BUDGET_MS,
  type DebtAtom,
  type DebtKind,
  type DebtTurn,
  FLOOR_LINE,
  type GhRun,
  LAST_SWEEP,
  NIGHTLY_LINE,
  type SweepCmdSeams,
  type SweepVoice,
  debtOf,
  debtTurns,
  legsTurn,
  payDebt,
  sweep,
} from "../cmd/sweep.ts";
import type { Goal } from "../lib/chain.ts";
import { ROOT } from "../lib/paths.ts";

const TMP = join(ROOT, ".agent-tmp", `after-session-${process.pid}`);
afterAll(() => rmSync(TMP, { recursive: true, force: true }));

const SILENT: SweepVoice = { step() {}, note() {} };
const MINUTE = 60_000;

/** What looper's own sweep wrote into the file just before the hook. */
const LOOPER_LINE = "- `light` is red: exit 1, want 0 (log: checks/light.log)";

const RUN = "https://github.com/example/novis/actions/runs";

/** `gh` answering the nightly run list with `runs`, and each run's jobs from `jobs`. */
function ghOf(runs: { databaseId: number; status: string; conclusion: string }[], jobs: Record<number, { name: string; conclusion: string }[]> = {}) {
  const asked: string[][] = [];
  const gh = async (args: string[]): Promise<GhRun> => {
    asked.push(args);
    if (args[1] === "list") return { code: 0, stdout: JSON.stringify(runs.map((r) => ({ ...r, url: `${RUN}/${r.databaseId}` }))), stderr: "" };
    return { code: 0, stdout: JSON.stringify({ jobs: jobs[Number(args[2])] ?? [] }), stderr: "" };
  };
  return { gh, asked };
}

const GREEN_NIGHTLY = ghOf([{ databaseId: 1, status: "completed", conclusion: "success" }]).gh;

interface Hook extends SweepCmdSeams {
  /** The store's atoms, by id; a turn that pays one removes it, or marks a red bench run again. */
  store: Map<string, DebtAtom>;
  /** Each turn that started, in order. */
  turns: DebtTurn[];
  /** Each deadline a turn was handed. */
  deadlines: number[];
  clock: { at: number };
}

/**
 * Seams over a scratch root whose store holds `debt`, where every atom a turn pays takes `cost`
 * milliseconds on the clock. A turn starts no atom once the clock reaches its deadline, as each runner
 * does, and an atom in `red` is red when it ran.
 */
function hook(name: string, debt: DebtAtom[], o: { cost?: number; red?: string[]; nightly?: SweepCmdSeams["gh"] } = {}): Hook {
  const root = join(TMP, name);
  rmSync(root, { recursive: true, force: true });
  mkdirSync(root, { recursive: true });
  const store = new Map(debt.map((a) => [a.id, a]));
  const turns: DebtTurn[] = [];
  const deadlines: number[] = [];
  const clock = { at: 1_000_000 };
  return {
    root,
    store,
    turns,
    deadlines,
    clock,
    voice: SILENT,
    now: () => clock.at,
    debt: () => [...store.values()],
    pay: async (turn, deadline) => {
      turns.push(turn);
      deadlines.push(deadline);
      const red: string[] = [];
      for (const id of turn.ids) {
        if (clock.at >= deadline) break;
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
    gh: o.nightly ?? GREEN_NIGHTLY,
    disk: async () => "disk: freed nothing",
  };
}

/** An atom of the debt; a `heavy:` one is a leg. */
const atom = (id: string, verdict: "red" | "owed", lastRun: number): DebtAtom => ({ id, kind: id.startsWith("heavy:") ? "leg" : (id.slice(0, id.indexOf(":")) as DebtKind), verdict, lastRun });

const lastSweep = (root: string) => join(root, LAST_SWEEP);

/** Writes the file as looper leaves it, with `extra` lines after its own. */
function looperWrote(root: string, ...extra: string[]): void {
  mkdirSync(join(root, ".loop"), { recursive: true });
  writeFileSync(lastSweep(root), `${[LOOPER_LINE, ...extra].join("\n")}\n`);
}

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
  const seams: Partial<LegsCmdSeams> = {
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
  return { lines, green, seams };
}

describe("bun nv sweep: the floor debt", () => {
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

  test("the budget starts no atom once it is spent, and what it did not reach stays owed", async () => {
    // Each atom takes four minutes: the third proof starts before the deadline and runs to its end, and
    // no turn starts after it.
    const h = hook("budget", [atom("proof:a.nvs", "owed", 1), atom("proof:b.nvs", "owed", 2), atom("proof:c.nvs", "owed", 3), atom("proof:d.nvs", "owed", 4), atom("bench:x.nvs", "owed", 5), atom("heavy:wsl leg", "owed", 6)], { cost: 4 * MINUTE });
    const start = h.clock.at;
    const steps: string[] = [];
    const out = await payDebt({ keepRuns: 1 }, { step: (l) => steps.push(l), note() {} }, h);
    expect(h.deadlines).toEqual([start + DEBT_BUDGET_MS]);
    expect(h.turns.map((t) => t.runner)).toEqual(["full"]);
    expect(out.paid.map((a) => a.id)).toEqual(["proof:a.nvs", "proof:b.nvs", "proof:c.nvs"]);
    expect([...h.store.keys()]).toEqual(["proof:d.nvs", "bench:x.nvs", "heavy:wsl leg"]);
    expect(out.left.map((a) => a.id)).toEqual(["proof:d.nvs", "bench:x.nvs", "heavy:wsl leg"]);
    expect(steps).toContain("floor debt: the budget is spent, so the rest waits for the next run");
    expect(steps).toContain("floor debt: paid 3 (3 proof) in 12m00s; left 3 (1 bench, 1 leg, 1 proof)");
  });

  test("with budget to spare every turn runs, the red bench first", async () => {
    const h = hook("spare", [atom("proof:a.nvs", "owed", 1), atom("bench:red.nvs", "red", 9), atom("heavy:wsl leg", "owed", 2)]);
    const out = await payDebt({ keepRuns: 1 }, SILENT, h);
    expect(h.turns.map((t) => t.runner)).toEqual(["bench", "full", "leg"]);
    expect(out.left).toEqual([]);
  });

  test("an owed leg runs the Linux legs the store selects, and no leg debt runs none", async () => {
    /** The hook's seams with the real legs turn over `machine`, and nothing run for any other turn. */
    const withLegs = (h: Hook, machine: ReturnType<typeof linux>): Hook => ({
      ...h,
      pay: (turn, _deadline, slug, note) => (turn.runner === "leg" ? legsTurn(slug, note, false, machine.seams) : Promise.resolve("")),
    });
    const l = linux(["wsl leg"]);
    await payDebt({ slug: "demo", keepRuns: 1 }, SILENT, withLegs(hook("legs", [atom("heavy:wsl leg", "owed", 1)]), l));
    expect(l.lines.some((line) => line.includes("cargo build --quiet"))).toBe(true);
    expect(l.green).toEqual(new Set<LegName>(["wsl leg"]));
    expect(l.lines.some((line) => line.includes("valgrind --error-exitcode"))).toBe(false);

    const none = linux([]);
    await payDebt({ slug: "demo", keepRuns: 1 }, SILENT, withLegs(hook("legs-none", [atom("proof:a.nvs", "owed", 1)]), none));
    expect(none.lines).toEqual([]);
  });

  test("a red the debt run found is a floor line beside looper's, and an earlier run's line goes", async () => {
    const h = hook("reds", [atom("proof:a.nvs", "owed", 1), atom("proof:b.nvs", "owed", 2)], { red: ["proof:b.nvs"] });
    looperWrote(h.root, `${FLOOR_LINE}benches: an earlier run's red`);
    expect(await sweep([], h)).toBe(0);
    expect(readFileSync(lastSweep(h.root), "utf8")).toBe(`${LOOPER_LINE}\n${FLOOR_LINE}owed atoms: 1 ran red: proof:b.nvs\n`);

    const green = hook("reds-green", [atom("proof:a.nvs", "owed", 1)]);
    mkdirSync(join(green.root, ".loop"), { recursive: true });
    writeFileSync(lastSweep(green.root), `${FLOOR_LINE}benches: an earlier run's red\n`);
    expect(await sweep([], green)).toBe(0);
    expect(existsSync(lastSweep(green.root))).toBe(false);
  });

  test("nv sweep exits 0 on a bad argument and on a turn that threw, says why, and still reads the nightly run", async () => {
    const failed = ghOf([{ databaseId: 7, status: "completed", conclusion: "failure" }], { 7: [{ name: "lint", conclusion: "failure" }] });
    const bad = hook("exit-bad", [], { nightly: failed.gh });
    expect(await sweep(["--keep-runs", "many"], bad)).toBe(0);
    expect(readFileSync(lastSweep(bad.root), "utf8")).toBe(
      `${FLOOR_LINE}could not run: \`bun nv sweep\` was started with a bad argument: --keep-runs needs a value\n${NIGHTLY_LINE}job \`lint\` ended \`failure\`: ${RUN}/7\n`,
    );
    expect(bad.turns).toEqual([]);

    const threw = hook("exit-threw", [atom("heavy:wsl leg", "owed", 1)]);
    looperWrote(threw.root);
    expect(await sweep(["--goal", "demo"], { ...threw, pay: () => Promise.reject(new Error("the WSL build failed")) })).toBe(0);
    expect(readFileSync(lastSweep(threw.root), "utf8")).toBe(`${LOOPER_LINE}\n${FLOOR_LINE}could not run: the debt run stopped: the WSL build failed\n`);
    // Nothing was paid, so the leg is still owed for the next run.
    expect([...threw.store.keys()]).toEqual(["heavy:wsl leg"]);
  });
});

describe("bun nv sweep: the nightly CI run", () => {
  test("it asks gh for the dispatched runs of ci.yml on main, and nothing else", async () => {
    const g = ghOf([{ databaseId: 1, status: "completed", conclusion: "success" }]);
    await sweep([], { ...hook("nightly-ask", []), gh: g.gh });
    expect(g.asked).toEqual([["run", "list", "--workflow", "ci.yml", "--branch", "main", "--event", "workflow_dispatch", "--limit", "5", "--json", "databaseId,status,conclusion,url"]]);
  });

  test("a failed run writes one line per failed job with the run's URL", async () => {
    const g = ghOf([{ databaseId: 42, status: "completed", conclusion: "failure" }], {
      42: [
        { name: "docs", conclusion: "failure" },
        { name: "what changed", conclusion: "success" },
        { name: "test / linux-x86_64", conclusion: "timed_out" },
        { name: "browser", conclusion: "skipped" },
      ],
    });
    const h = hook("nightly-red", [], { nightly: g.gh });
    looperWrote(h.root, `${NIGHTLY_LINE}job \`old\` ended \`failure\`: ${RUN}/41`);
    expect(await sweep([], h)).toBe(0);
    expect(readFileSync(lastSweep(h.root), "utf8")).toBe(
      [LOOPER_LINE, `${NIGHTLY_LINE}job \`docs\` ended \`failure\`: ${RUN}/42`, `${NIGHTLY_LINE}job \`test / linux-x86_64\` ended \`timed_out\`: ${RUN}/42`, ""].join("\n"),
    );
  });

  test("a run still in progress changes nothing: the newest completed run still decides", async () => {
    const g = ghOf(
      [
        { databaseId: 43, status: "in_progress", conclusion: "" },
        { databaseId: 42, status: "completed", conclusion: "failure" },
      ],
      { 42: [{ name: "docs", conclusion: "failure" }] },
    );
    const h = hook("nightly-running", [], { nightly: g.gh });
    looperWrote(h.root, `${NIGHTLY_LINE}job \`docs\` ended \`failure\`: ${RUN}/42`);
    expect(await sweep([], h)).toBe(0);
    expect(readFileSync(lastSweep(h.root), "utf8")).toBe(`${LOOPER_LINE}\n${NIGHTLY_LINE}job \`docs\` ended \`failure\`: ${RUN}/42\n`);
    expect(g.asked.some((a) => a[1] === "view" && a[2] === "43")).toBe(false);
  });

  test("a newer green run removes the lines, and the file once nothing is left", async () => {
    const h = hook("nightly-green", []);
    looperWrote(h.root, `${NIGHTLY_LINE}job \`docs\` ended \`failure\`: ${RUN}/42`);
    expect(await sweep([], h)).toBe(0);
    expect(readFileSync(lastSweep(h.root), "utf8")).toBe(`${LOOPER_LINE}\n`);

    const alone = hook("nightly-green-alone", []);
    mkdirSync(join(alone.root, ".loop"), { recursive: true });
    writeFileSync(lastSweep(alone.root), `${NIGHTLY_LINE}job \`docs\` ended \`failure\`: ${RUN}/42\n`);
    expect(await sweep([], alone)).toBe(0);
    expect(existsSync(lastSweep(alone.root))).toBe(false);
  });

  test("gh that fails, is not logged in or is missing writes one line, and the hook exits 0", async () => {
    const out = hook("nightly-out", [], { nightly: async () => ({ code: 4, stdout: "", stderr: "To get started with GitHub CLI, please run:  gh auth login\n" }) });
    expect(await sweep([], out)).toBe(0);
    expect(readFileSync(lastSweep(out.root), "utf8")).toBe(`${NIGHTLY_LINE}could not be read: \`gh run list\` exited 4: To get started with GitHub CLI, please run:  gh auth login\n`);

    const missing = hook("nightly-missing", [], { nightly: () => Promise.reject(new Error("Executable not found in $PATH: \"gh\"")) });
    looperWrote(missing.root);
    expect(await sweep([], missing)).toBe(0);
    expect(readFileSync(lastSweep(missing.root), "utf8")).toBe(`${LOOPER_LINE}\n${NIGHTLY_LINE}could not be read: \`gh\` failed: Executable not found in $PATH: "gh"\n`);

    const garbled = hook("nightly-garbled", [], { nightly: async () => ({ code: 0, stdout: "<html>", stderr: "" }) });
    expect(await sweep([], garbled)).toBe(0);
    const text = readFileSync(lastSweep(garbled.root), "utf8");
    expect(text.startsWith(`${NIGHTLY_LINE}could not be read: \`gh\` failed: `)).toBe(true);
    expect(text.split("\n").filter((l) => l !== "")).toHaveLength(1);
  });
});
