// `bun nv sweep` with every runner handed in, so nothing builds and no check runs: what it writes to
// `.loop/last-sweep.md`, its exit code, and the floor gate's cadence over the store's meta.

import { afterAll, describe, expect, test } from "bun:test";
import { existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import type { AcceptanceResult, Check } from "../accept/accept.ts";
import { FLOOR_GATE_EVERY, FLOOR_META, LAST_SWEEP, type SweepCmdSeams, type SweepVoice, afterSession, sweep } from "../cmd/sweep.ts";
import type { Goal } from "../lib/chain.ts";
import { ROOT } from "../lib/paths.ts";

const TMP = join(ROOT, ".agent-tmp", `after-session-${process.pid}`);
afterAll(() => rmSync(TMP, { recursive: true, force: true }));

const SILENT: SweepVoice = { step() {}, note() {}, ledger() {}, phase() {} };

const LIGHT: Check = { id: "light", kind: "command", stage: 1, name: "light check" };
const HEAVY: Check = { id: "heavy", kind: "command", stage: 2, name: "heavy check", memoize: false };
const GOAL = { checks: [LIGHT, HEAVY], stages: [{ number: 1, title: "one" }, { number: 2, title: "two" }] } as unknown as Goal;

const verdict = (fail: string): { result: AcceptanceResult; secs: number } => ({ result: { fail, ran: 1, answered: 0, verdicts: new Map() }, secs: 1 });

/** Seams over a scratch root whose sweep always returns `fail`, and whose meta starts at `since`. */
function seams(name: string, fail: string, since = 0): Partial<SweepCmdSeams> & { root: string } {
  const root = join(TMP, name);
  mkdirSync(root, { recursive: true });
  const meta = new Map<string, number>([[FLOOR_META, since]]);
  return {
    root,
    voice: SILENT,
    plan: () => ({ slug: "demo", goal: GOAL }),
    meta: (key, missing) => meta.get(key) ?? missing,
    setMeta: (key, count) => void meta.set(key, count),
    fullRun: async () => "",
    sweep: async () => verdict(fail),
    benches: async () => "",
    disk: async () => "disk: freed nothing",
    docGate: async () => "",
    ownerGate: async () => "",
  };
}

const lastSweep = (root: string) => join(root, LAST_SWEEP);

describe("bun nv sweep", () => {
  test("nv sweep writes each red check to .loop/last-sweep.md", async () => {
    const fail = "light check [1 one]: exit 1 -- the first failing line\n       (and 1 later fixture(s) red)\n       also red: heavy check [2 two]: exit 2 -- another line";
    const s = seams("red", fail);
    expect(await sweep([], { ...s, ownerGate: async () => "a gap still names goal `demo`" })).toBe(0);
    const text = readFileSync(lastSweep(s.root), "utf8");
    expect(text).toContain("- `light` light check [1 one]: exit 1 -- the first failing line\n");
    expect(text).toContain("- `heavy` heavy check [2 two]: exit 2 -- another line\n");
    expect(text).not.toContain("later fixture");
    // A red scoped sweep does not reach the goal, so the gates do not run.
    expect(text).not.toContain("owner gate");
  });

  test("nv sweep removes .loop/last-sweep.md when the sweep is green", async () => {
    const s = seams("green", "");
    mkdirSync(join(s.root, ".loop"), { recursive: true });
    writeFileSync(lastSweep(s.root), "# red before\n");
    expect(await sweep([], s)).toBe(0);
    expect(existsSync(lastSweep(s.root))).toBe(false);
    // A gate that is red keeps the file, with the gate's finding in it.
    expect(await sweep([], { ...s, docGate: async () => "a broken intra-doc link" })).toBe(0);
    expect(readFileSync(lastSweep(s.root), "utf8")).toContain("- doc gate: a broken intra-doc link\n");
  });

  test("nv sweep exits 0 when a check is red or the sweep cannot run", async () => {
    const red = seams("exit-red", "light check [1 one]: exit 1 -- boom");
    expect(await sweep([], red)).toBe(0);
    expect(readFileSync(lastSweep(red.root), "utf8")).toContain("`light`");

    const none = seams("exit-none", "");
    expect(await sweep(["--goal", "missing"], { ...none, plan: () => null })).toBe(0);
    expect(readFileSync(lastSweep(none.root), "utf8")).toContain("no goal record for `missing`");

    const threw = seams("exit-threw", "");
    expect(await sweep([], { ...threw, sweep: () => Promise.reject(new Error("the covws build failed")) })).toBe(0);
    expect(readFileSync(lastSweep(threw.root), "utf8")).toContain("the sweep stopped: the covws build failed");

    const bad = seams("exit-bad", "");
    expect(await sweep(["--keep-runs", "many"], bad)).toBe(0);
    expect(readFileSync(lastSweep(bad.root), "utf8")).toContain("--keep-runs needs a value");
  });

  test("nv sweep opens the floor gate on every tenth sweep and holds the heavy checks otherwise", async () => {
    const s = seams("cadence", "light check [1 one]: exit 1 -- red, so no sweep reaches the goal");
    const swept: { ids: string[]; open: boolean }[] = [];
    let fullRuns = 0;
    let benchTurns = 0;
    const over: Partial<SweepCmdSeams> = {
      ...s,
      sweep: async (_goal, checks, _label, o) => {
        swept.push({ ids: checks.map((c) => c.id), open: o.gateOpen === true });
        return verdict("light check [1 one]: exit 1 -- red");
      },
      fullRun: async () => {
        fullRuns++;
        return "";
      },
      benches: async () => {
        benchTurns++;
        return "";
      },
    };
    for (let i = 0; i < FLOOR_GATE_EVERY * 2; i++) {
      await afterSession({ slug: "demo", goal: GOAL, done: false, session: "test", keepRuns: 1 }, SILENT, over);
    }
    const opened = swept.flatMap((x, i) => (x.open ? [i + 1] : []));
    expect(opened).toEqual([FLOOR_GATE_EVERY, FLOOR_GATE_EVERY * 2]);
    for (const x of swept) expect(x.ids).toEqual(x.open ? ["light", "heavy"] : ["light"]);
    expect(fullRuns).toBe(2);
    expect(benchTurns).toBe(2);
    expect(over.meta!(FLOOR_META, -1)).toBe(0);
  });
});
