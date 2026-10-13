// `bun nv sweep`, the floor gate, with every runner handed in, so nothing builds and nothing runs: which
// steps a shut and an open gate start, that an open gate runs both Linux legs whatever changed, the count
// in the store's meta, the lines it writes into `.loop/last-sweep.md` beside looper's, and its exit code.

import { afterAll, describe, expect, test } from "bun:test";
import { existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import type { Check } from "../accept/accept.ts";
import type { LegName } from "../accept/legs.ts";
import type { LegsCmdSeams } from "../cmd/legs.ts";
import { FLOOR_GATE_EVERY, FLOOR_LINE, FLOOR_META, LAST_SWEEP, type SweepCmdSeams, type SweepVoice, floorGate, legsTurn, sweep } from "../cmd/sweep.ts";
import type { Goal } from "../lib/chain.ts";
import { ROOT } from "../lib/paths.ts";

const TMP = join(ROOT, ".agent-tmp", `after-session-${process.pid}`);
afterAll(() => rmSync(TMP, { recursive: true, force: true }));

const SILENT: SweepVoice = { step() {}, note() {} };

/** What looper's own sweep wrote into the file just before the hook. */
const LOOPER_LINE = "- `light` is red: exit 1, want 0 (log: checks/light.log)";

interface Gate extends Partial<SweepCmdSeams> {
  root: string;
  /** Each step the gate started, in order: `full`, `legs <slug>`, `benches`, `disk`. */
  ran: string[];
  count(): number;
}

/** Seams over a scratch root whose meta holds `since`, and whose steps return `reds`. */
function gate(name: string, since: number, reds: { full?: string; legs?: string; benches?: string } = {}): Gate {
  const root = join(TMP, name);
  rmSync(root, { recursive: true, force: true });
  mkdirSync(root, { recursive: true });
  const meta = new Map<string, number>([[FLOOR_META, since]]);
  const ran: string[] = [];
  return {
    root,
    ran,
    count: () => meta.get(FLOOR_META)!,
    voice: SILENT,
    meta: (key, missing) => meta.get(key) ?? missing,
    setMeta: (key, count) => void meta.set(key, count),
    fullRun: async () => (ran.push("full"), reds.full ?? ""),
    legs: async (slug) => (ran.push(`legs ${slug ?? "-"}`), reds.legs ?? ""),
    benches: async () => (ran.push("benches"), reds.benches ?? ""),
    disk: async () => (ran.push("disk"), "disk: freed nothing"),
  };
}

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

/** The real legs turn over a fake WSL machine, whose store says the change reached neither leg. */
function linux() {
  const lines: string[] = [];
  const green = new Set<LegName>();
  const seams: Partial<LegsCmdSeams> = {
    plan: (slug) => ({ slug: slug ?? "demo", goal: LEGS_GOAL }),
    store: async () => ({ reached: () => false, green: (l) => void green.add(l), red: () => {}, done: () => {} }),
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
  return { lines, green, legs: (slug: string | undefined, note: (line: string) => void) => legsTurn(slug, note, seams) };
}

describe("bun nv sweep", () => {
  test("an open gate runs both Linux legs over every fixture, though the store says the change reached neither", async () => {
    const l = linux();
    const g = gate("legs-open", FLOOR_GATE_EVERY - 1);
    expect((await floorGate({ slug: "demo", keepRuns: 1 }, SILENT, { ...g, legs: l.legs })).open).toBe(true);
    expect(l.lines.some((line) => line.includes("cargo build --quiet"))).toBe(true);
    expect(l.lines.filter((line) => line.includes("valgrind --error-exitcode"))).toHaveLength(2);
    expect(l.green).toEqual(new Set<LegName>(["wsl leg", "valgrind sweep"]));
  });

  test("a shut gate runs neither Linux leg", async () => {
    const l = linux();
    const g = gate("legs-shut", 0);
    expect((await floorGate({ slug: "demo", keepRuns: 1 }, SILENT, { ...g, legs: l.legs })).open).toBe(false);
    expect(l.lines).toEqual([]);
    expect(l.green.size).toBe(0);
  });

  test("a shut gate counts the session and runs the disk sweep alone", async () => {
    const g = gate("shut", 3);
    looperWrote(g.root, `${FLOOR_LINE}benches: an earlier run's red`);
    const out = await floorGate({ keepRuns: 1 }, SILENT, g);
    expect(out).toEqual({ open: false, reds: [] });
    expect(g.ran).toEqual(["disk"]);
    expect(g.count()).toBe(4);
    // looper's line stays as it wrote it, and the earlier run's floor line is gone.
    expect(readFileSync(lastSweep(g.root), "utf8")).toBe(`${LOOPER_LINE}\n`);
  });

  test("an open gate runs the full run, the legs and the benches, and sets the count back to 0", async () => {
    const g = gate("open", FLOOR_GATE_EVERY - 1, {
      full: "full run: 1 selection miss(es), red now and not selected by the change that broke them: proof:a.nvs",
      legs: "wsl leg: 1 fixture red\n       (and 2 later fixture(s) red)\n       also red: valgrind sweep: 1 fixture leaked",
    });
    looperWrote(g.root);
    const out = await floorGate({ slug: "demo", keepRuns: 1 }, SILENT, g);
    expect(out.open).toBe(true);
    expect(g.ran).toEqual(["full", "legs demo", "benches", "disk"]);
    expect(g.count()).toBe(0);
    expect(readFileSync(lastSweep(g.root), "utf8")).toBe(
      [
        LOOPER_LINE,
        `${FLOOR_LINE}full run: 1 selection miss(es), red now and not selected by the change that broke them: proof:a.nvs`,
        `${FLOOR_LINE}wsl leg: 1 fixture red`,
        `${FLOOR_LINE}valgrind sweep: 1 fixture leaked`,
        "",
      ].join("\n"),
    );
  });

  test("the gate opens on every tenth run", async () => {
    const g = gate("cadence", 0);
    const opened: number[] = [];
    for (let i = 1; i <= FLOOR_GATE_EVERY * 2; i++) if ((await floorGate({ keepRuns: 1 }, SILENT, g)).open) opened.push(i);
    expect(opened).toEqual([FLOOR_GATE_EVERY, FLOOR_GATE_EVERY * 2]);
    expect(g.ran.filter((r) => r === "full")).toHaveLength(2);
    expect(g.ran.filter((r) => r === "benches")).toHaveLength(2);
    expect(g.ran.filter((r) => r === "disk")).toHaveLength(FLOOR_GATE_EVERY * 2);
    // A store that holds no count opens the gate on its first run.
    const fresh = gate("cadence-fresh", 0);
    expect((await floorGate({ keepRuns: 1 }, SILENT, { ...fresh, meta: (_key, missing) => missing })).open).toBe(true);
  });

  test("a green gate removes its own lines, and the file once nothing is left", async () => {
    const g = gate("green", FLOOR_GATE_EVERY - 1);
    looperWrote(g.root, `${FLOOR_LINE}benches: an earlier run's red`);
    expect(await sweep([], g)).toBe(0);
    expect(readFileSync(lastSweep(g.root), "utf8")).toBe(`${LOOPER_LINE}\n`);

    const alone = gate("green-alone", FLOOR_GATE_EVERY - 1);
    mkdirSync(join(alone.root, ".loop"), { recursive: true });
    writeFileSync(lastSweep(alone.root), `${FLOOR_LINE}benches: an earlier run's red\n`);
    expect(await sweep([], alone)).toBe(0);
    expect(existsSync(lastSweep(alone.root))).toBe(false);
  });

  test("nv sweep exits 0 on a bad argument and on a step that threw, and says why in the file", async () => {
    const bad = gate("exit-bad", 0);
    expect(await sweep(["--keep-runs", "many"], bad)).toBe(0);
    expect(readFileSync(lastSweep(bad.root), "utf8")).toContain(`${FLOOR_LINE}could not run: \`bun nv sweep\` was started with a bad argument: --keep-runs needs a value`);
    expect(bad.ran).toEqual([]);

    const threw = gate("exit-threw", FLOOR_GATE_EVERY - 1);
    looperWrote(threw.root);
    expect(await sweep(["--goal", "demo"], { ...threw, legs: () => Promise.reject(new Error("the WSL build failed")) })).toBe(0);
    expect(readFileSync(lastSweep(threw.root), "utf8")).toBe(`${LOOPER_LINE}\n${FLOOR_LINE}could not run: the gate stopped: the WSL build failed\n`);
    // The count stays where it was, so the next run opens the gate again.
    expect(threw.count()).toBe(FLOOR_GATE_EVERY - 1);
  });
});
