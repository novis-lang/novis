// `bun nv sweep`, the floor gate, with every runner handed in, so nothing builds and nothing runs: which
// steps a shut and an open gate start, the count in the store's meta, the lines it writes into
// `.loop/last-sweep.md` beside looper's, and its exit code.

import { afterAll, describe, expect, test } from "bun:test";
import { existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { FLOOR_GATE_EVERY, FLOOR_LINE, FLOOR_META, LAST_SWEEP, type SweepCmdSeams, type SweepVoice, floorGate, sweep } from "../cmd/sweep.ts";
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

describe("bun nv sweep", () => {
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
