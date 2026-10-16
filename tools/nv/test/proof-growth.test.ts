import { afterAll, expect, test } from "bun:test";
import { mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { type Batch, growthOf, type Judged, type Measure, type Options, overBudget, oversized, withIterations } from "../cmd/scaling.ts";
import { stepCommands } from "../cmd/verify.ts";
import { ROOT } from "../lib/paths.ts";
import { COUNTS, ProgramFailed } from "../proofs/perf.ts";

// The perf proof's growth half, `growthOf`, over fixture benches whose counts a fake measure gives as a
// function of the batch or the input size, so no binary runs and every verdict is exact.

const DIR = `.agent-tmp/proof-growth-${process.pid}`;
const SCRATCH = join(ROOT, `${DIR}-scratch`);
mkdirSync(join(ROOT, DIR), { recursive: true });
afterAll(() => {
  rmSync(join(ROOT, DIR), { recursive: true, force: true });
  rmSync(SCRATCH, { recursive: true, force: true });
});

/** A bench under the fixture folder, declaring `complexity` when it is given, and its path. */
function bench(name: string, complexity: string | null): string {
  const lines = ["<?nvs", "// bench: iterations 100000", ...(complexity ? [`// bench: complexity ${complexity}`] : []), 'echo Bench::run(100000), "\\n";', ""];
  writeFileSync(join(ROOT, DIR, `${name}.nvs`), lines.join("\n"));
  return `${DIR}/${name}.nvs`;
}

/** The `.scale.nvs` sibling of `name`, ramped from 64 to 1024. */
function sibling(name: string): void {
  writeFileSync(join(ROOT, DIR, `${name}.scale.nvs`), ["<?nvs", "// bench: start 64", "// bench: max 1024", 'echo Bench::run(64), "\\n";', ""].join("\n"));
}

/** A measure whose every count is `ops(n)` at batch `n` of a bench and `size(n)` at size `n` of a
 * sibling, with the clock at zero. `callgrind` counts each run it would start. */
function fake(ops: (n: number) => number, size: (n: number) => number = ops, callgrind?: (n: number) => number) {
  const started = { callgrind: 0 };
  const measure: Measure = {
    keys: COUNTS,
    rewrites: true,
    take: async (copy, _bench, _opts, n) => {
      const c = 1000 + (copy.endsWith(".scale.nvs") ? size(n) : ops(n));
      return { counts: Object.fromEntries(COUNTS.map((k) => [k, c])), ns: 0 };
    },
    clock: async () => 0,
    callgrind: async (_copy, _bench, _opts, n) => {
      started.callgrind++;
      if (!callgrind) throw new Error("callgrind started");
      return callgrind(n);
    },
    judge: (_batches: Batch[]) => null,
  };
  return { measure, started };
}

const opts = (callgrind: boolean): Options => ({ nvs: "nvs", reps: 1, wslNvs: "nvs", callgrind, scratch: SCRATCH });

/** Two per operation at one batch and half at the next, so the counts never agree and never grow. */
const seesaw = (n: number) => n * (Math.log2(n) % 2 === 0 ? 1 : 1.5);

test("a bench whose cost per operation rises as its batches double fails its perf proof", async () => {
  const flat = await growthOf(bench("flat", "constant"), opts(false), true, fake((n) => 5 * n).measure);
  expect(flat.findings).toEqual([]);
  expect(flat.ramps.map((j) => j.verdict)).toEqual(["flat"]);
  const rising = await growthOf(bench("rising", "constant"), opts(false), true, fake((n) => n * n).measure);
  expect(rising.ramps[0]!.verdict).toBe("grows");
  expect(rising.findings).toHaveLength(1);
  expect(rising.findings[0]).toContain("its cost per operation rises as its batches double");
});

test("a bench that grows faster than its declared complexity fails its perf proof", async () => {
  bench("linear", "linear");
  sibling("linear");
  const within = await growthOf(`${DIR}/linear.nvs`, opts(false), true, fake((n) => 5 * n, (n) => 7 * n).measure);
  expect(within.findings).toEqual([]);
  expect(within.ramps.map((j) => j.verdict)).toEqual(["flat", "flat"]);
  const faster = await growthOf(`${DIR}/linear.nvs`, opts(false), true, fake((n) => 5 * n, (n) => n * n).measure);
  expect(faster.ramps.map((j) => j.verdict)).toEqual(["flat", "grows"]);
  expect(faster.findings[0]).toContain("grows faster than `complexity linear`");
  // A complexity other than `constant` needs its sibling, and `quadratic` is never a complexity.
  const alone = await growthOf(bench("lonely", "nlogn"), opts(false), true, fake((n) => 5 * n).measure);
  expect(alone.findings[0]).toContain("has no");
  const quadratic = await growthOf(bench("quadratic", "quadratic"), opts(false), true, fake((n) => 5 * n).measure);
  expect(quadratic.findings[0]).toContain("is not one of");
  // A sibling that exits with an error at a size cannot be ramped, and that is a finding too.
  const broken = (n: number) => {
    if (n > 256) throw new ProgramFailed(`the sibling exited 1 at size ${n}`);
    return 7 * n;
  };
  const failing = await growthOf(`${DIR}/linear.nvs`, opts(false), true, fake((n) => 5 * n, broken).measure);
  expect(failing.ramps.map((j) => j.verdict)).toEqual(["flat", "invalid"]);
  expect(failing.findings[0]).toContain("cannot be ramped");
});

test("a bench with no declared complexity fails its perf proof", async () => {
  const path = bench("undeclared", null);
  const required = await growthOf(path, opts(false), true, fake((n) => 5 * n).measure);
  expect(required.findings).toEqual(["declares no `// bench: complexity`"]);
  // Before the backfill the proof runs with `required` off, and the ramp still judges the batches.
  const gated = await growthOf(path, opts(false), false, fake((n) => n * n).measure);
  expect(gated.findings).toHaveLength(1);
  expect(gated.findings[0]).toContain("rises as its batches double");
});

test("valgrind runs only when the ramp shows no clear growth", async () => {
  const path = bench("seesaw", "constant");
  const clear = fake((n) => 5 * n, undefined, (n) => 100 * n);
  expect((await growthOf(path, opts(true), true, clear.measure)).findings).toEqual([]);
  expect(clear.started.callgrind).toBe(0);
  const unclear = fake(seesaw, undefined, (n) => 100 * n);
  const settled = await growthOf(path, opts(true), true, unclear.measure);
  expect(unclear.started.callgrind).toBeGreaterThan(0);
  expect(settled.ramps[0]!.verdict).toBe("flat");
  // With callgrind off the same ramp is reported as unclear, starts nothing, and fails nothing.
  const off = fake(seesaw);
  const reported = await growthOf(path, opts(false), true, off.measure);
  expect(off.started.callgrind).toBe(0);
  expect(reported.ramps[0]!.verdict).toBe("unclear");
  expect(reported.findings).toEqual([]);
});

test("callgrind sets a bench's iterations, and a later run at that N starts no valgrind", async () => {
  const path = bench("threshold", "constant");
  const first = fake(seesaw, undefined, (n) => 100 * n);
  const measured = await growthOf(path, opts(true), true, first.measure);
  // The instructions agree from the fourth batch, 128, so N becomes twice that.
  expect(measured.ramps[0]!.threshold).toBe(128);
  const source = readFileSync(join(ROOT, path), "utf8");
  expect(source).toContain("// bench: iterations 256 callgrind\n");
  expect(source).toContain("echo Bench::run(256),");
  // A later run ramps only up to that batch, and its counts settle with no valgrind.
  const later = fake((n) => 5 * n, undefined, (n) => 100 * n);
  const again = await growthOf(path, opts(true), true, later.measure);
  expect(later.started.callgrind).toBe(0);
  expect(again.ramps[0]!.verdict).toBe("flat");
  expect(Math.max(...again.ramps[0]!.sizes)).toBe(128);
  expect(readFileSync(join(ROOT, path), "utf8")).toBe(source);
});

test("--sized fails a bench whose N is more than four times the batch its ramp settled at", () => {
  const judged = (verdict: Judged["verdict"], sizes: number[], threshold?: number): Judged => ({ bench: "b.nvs", verdict, sizes, slopes: {}, clock: null, notes: [], ...(threshold === undefined ? {} : { threshold }) });
  // A bench `--lower` set to twice one ramp's batch passes when the next ramp settles a doubling earlier.
  expect(oversized(judged("flat", [16, 32, 64, 128]), 512)).toBeNull();
  expect(oversized(judged("flat", [16, 32, 64, 128]), 513)).toContain("N is at most 512, and `--lower` writes 256");
  // Callgrind's threshold is where the ramp settled, though the ramp ran every batch to the ceiling.
  expect(oversized(judged("flat", [16, 32, 64, 128, 256, 512, 1024, 2048, 4096], 128), 100000)).toContain("settled at 128");
  // A ramp that did not settle keeps its N.
  expect(oversized(judged("unclear", [16, 32, 64, 128, 256, 512, 1024, 2048, 4096]), 100000)).toBeNull();
});

test("--lower rewrites N, and notes callgrind only when callgrind settled the ramp", () => {
  const source = "// bench: iterations 400_000\n// bench: complexity constant\necho Bench::run(400_000);\n";
  expect(withIterations(source, 400000, 2048, false)).toBe("// bench: iterations 2048\n// bench: complexity constant\necho Bench::run(2048);\n");
  expect(withIterations(source, 400000, 256)).toContain("// bench: iterations 256 callgrind\n");
});

test("--budget lets a share rise only for a new bench or an N raised with a reason", () => {
  const was = { total: 300, benches: { "a.nvs": 100, "b.nvs": 200 } };
  expect(overBudget(was, { "a.nvs": 90, "b.nvs": 200, "c.nvs": 5000 }, {})).toEqual([]);
  expect(overBudget(was, { "a.nvs": 101, "b.nvs": 200 }, {})[0]).toContain("a.nvs runs 101 statements and its budget is 100");
  expect(overBudget(was, { "a.nvs": 101, "b.nvs": 200 }, { "a.nvs": "the ramp needs 4096" })).toEqual([]);
});

test("nv verify never runs valgrind over a bench", () => {
  const commands = stepCommands();
  expect(commands.length).toBeGreaterThan(0);
  for (const [name, command] of commands) {
    expect(`${name}: ${command}`).not.toMatch(/valgrind|callgrind|nv scaling|--record-perf/);
  }
});
