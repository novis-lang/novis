// `bun nv select --full`: every atom that is not heavy runs and records, whatever changed, and a red one
// the selection should have picked is a selection miss (ADR 0226, § 6).
//
// The run takes the selection the change since the store's tree makes before anything runs, then runs:
//
// - every case of the case trees, through `recordCases`;
// - every proof program of the roster, judged on `target/proof` and recorded on the covws debug `nvs`
//   (`seed.ts` `seedProofs`);
// - every Rust test executable of the covws build (`seed.ts` `seedTests`);
// - the tools' `tsc` and every tools test file (`nvtests.ts`);
// - every check of the plan that a shut floor gate does not hold and that is its own atom, a fixture or a
//   command, `bun nv` ones among them, as the sweep runs it (`driver/runner.ts` `PlanSweep`).
//
// The heavy checks keep the floor gate's own cadence and are not run here. The store's tree then moves
// past the change, as after any sweep.
//
// A selection miss is an atom red in this run that the selection did not pick. The selection picks every
// atom last red, owed or never recorded, and every atom the tree names that the store does not know, so
// a miss is almost always one whose green verdict stayed green through a change the selection did not
// tie to it. Each miss is printed, kept in the store under `select-miss:<platform>:<atom>` until a full
// run finds the atom green again, and makes the command exit 1.

import { cpus } from "node:os";
import type { Check } from "../driver/accept.ts";
import { heldByGate, tiers } from "../driver/accept.ts";
import { holdOrigin } from "../driver/origin.ts";
import { PlanSweep } from "../driver/runner.ts";
import { needLine, sharedResources, takeSweepLock } from "../driver/sweep-lock.ts";
import { currentPlan } from "../lib/chain.ts";
import { buildCovws } from "../lib/covws.ts";
import { ROOT } from "../lib/paths.ts";
import { caseFiles, nvTestFiles, proofPrograms } from "./atoms.ts";
import type { How } from "./checks.ts";
import { runNvTest, runTsc } from "./nvtests.ts";
import { pool, Recorder, recordCases } from "./record.ts";
import { type Ctx, seedProofs, seedTests } from "./seed.ts";
import type { Selection } from "./select.ts";
import type { AtomKind, AtomRow, SelectStore, Verdict } from "./store.ts";

export interface FullOptions {
  root?: string;
  jobs?: number;
  say?: (line: string) => void;
  /** Only these kinds of the atoms named by the tree run; the plan's own checks run with `check`. */
  kinds?: ("case" | "proof" | "test" | "nvtest" | "check")[];
}

export interface Miss {
  id: string;
  /** When its last green run was recorded, in milliseconds since the epoch. */
  lastGreen: number;
}

export interface FullReport {
  /** The commit the store's tree was at before the run. */
  since: string;
  /** What the change since the store's tree selected before anything ran. */
  selection: Selection;
  /** Every atom this run recorded, with its verdict. */
  ran: Map<string, Verdict>;
  kinds: Partial<Record<AtomKind, { run: number; red: number }>>;
  misses: Miss[];
  owed: number;
  seconds: number;
  /** Set when the covws build failed, and nothing ran. */
  buildFailed?: string;
}

/** The slot a selection miss is kept under. */
export const missSlot = (platform: string, id: string) => `select-miss:${platform}:${id}`;

/**
 * The atoms red in `ran` that `selected` does not hold, with when `before` last recorded each green (0
 * for one that was never green). An atom that was red, owed or never recorded is picked by its verdict
 * or as new, so one of those that the selection did not pick is a miss too. `mutate.ts` judges a batch
 * by the same function.
 */
export function selectionMisses(before: Map<string, Pick<AtomRow, "verdict" | "lastRun">>, selected: Set<string>, ran: Map<string, Verdict>): Miss[] {
  const out: Miss[] = [];
  for (const [id, v] of ran) {
    if (v !== "red" || selected.has(id)) continue;
    const was = before.get(id);
    out.push({ id, lastGreen: was?.verdict === "green" ? was.lastRun : 0 });
  }
  return out.sort((a, b) => (a.id < b.id ? -1 : 1));
}

/** Keeps each miss in the store, and forgets the kept miss of every atom this run found green. */
export function keepMisses(store: SelectStore, misses: Miss[], ran: Map<string, Verdict>): void {
  store.transaction(() => {
    for (const [id, v] of ran) if (v === "green") store.deleteVerdict(missSlot(store.platform, id));
    for (const m of misses) store.putVerdict(missSlot(store.platform, m.id), "", JSON.stringify({ lastGreen: m.lastGreen }));
  });
}

/** Every miss the store keeps, by atom. */
export function keptMisses(store: SelectStore): Map<string, { lastGreen: number; at: number }> {
  const prefix = missSlot(store.platform, "");
  const out = new Map<string, { lastGreen: number; at: number }>();
  for (const [slot] of store.verdictsWithPrefix(prefix)) {
    const v = store.verdict(slot)!;
    let lastGreen = 0;
    try {
      lastGreen = Number((JSON.parse(v.verdict) as { lastGreen?: number }).lastGreen ?? 0);
    } catch {
      // A slot that does not read still names its atom.
    }
    out.set(slot.slice(prefix.length), { lastGreen, at: v.at });
  }
  return out;
}

/** The checks of `plan` a full run runs as the sweep does, in the order of their tiers: every one a shut
 * floor gate does not hold whose atom is its own (`howOf`, the check's group), and every setup command,
 * so a setup comes before the fixtures that read what it writes. The other checks are made of cases,
 * test binaries, proof programs and tools tests, which the full run runs whole. */
export function ownChecks(plan: Check[], label: (n: number) => string, howOf: (c: Check) => How | undefined): Check[] {
  const own = plan.filter((c) => {
    if (c.setup === true) return true;
    if (heldByGate(c)) return false;
    const how = howOf(c);
    return how === "fixture" || how === "command" || how === "nv";
  });
  return tiers(own, label).flatMap((t) => t.checks);
}

/** Runs every atom that is not heavy on `store`, records each, moves the store's tree, and reports the
 * selection misses. The caller closes the store. */
export async function fullRun(store: SelectStore, opts: FullOptions = {}): Promise<FullReport> {
  const root = opts.root ?? ROOT;
  const say = opts.say ?? ((line: string) => console.log(line));
  const jobs = opts.jobs ?? Math.max(2, Math.floor(cpus().length / 2));
  const kinds = new Set(opts.kinds ?? ["case", "proof", "test", "nvtest", "check"]);
  const started = Date.now();
  const since = store.base() ?? "";
  const before = new Map(store.atoms().map((a) => [a.id, a]));

  const found = currentPlan(root);
  const plan = (found?.goal.checks ?? []) as Check[];
  const labels = new Map((found?.goal.stages ?? []).map((s) => [s.number, `${s.number} ${s.title}`]));
  const label = (n: number) => labels.get(n) ?? String(n);
  const sweep = await PlanSweep.open(plan, label, { full: true, store, say, onRun: (what) => say(`select: ${what}`) });
  const selection = sweep.sel;
  const report: FullReport = { since, selection, ran: new Map(), kinds: {}, misses: [], owed: 0, seconds: 0 };

  let build: Awaited<ReturnType<typeof buildCovws>>;
  try {
    say("select: building covws");
    build = await buildCovws({ tests: kinds.has("test") });
  } catch (e) {
    sweep.discard();
    report.buildFailed = (e as Error).message.split("\n").slice(0, 20).join("\n");
    report.seconds = Math.round((Date.now() - started) / 1000);
    return report;
  }

  const r = await Recorder.open(store, sweep.change.view, sweep.graph, "full", { root, say });
  const ctx: Ctx = { r, nvs: build.nvs, jobs, say, tally: () => {}, root };
  try {
    if (kinds.has("case")) {
      await recordCases(r, build.nvs, caseFiles(root), { jobs, onBatch: (done, total) => say(`select: cases ${done}/${total}`), root });
    }
    if (kinds.has("proof")) await seedProofs(ctx, await proofPrograms(build.nvs));
    if (kinds.has("test")) await seedTests(ctx, [...build.tests].flatMap(([pkg, ts]) => ts.map((t) => ({ pkg, t }))));
    if (kinds.has("nvtest")) {
      say("select: tsc --noEmit");
      await runTsc(r);
      const files = nvTestFiles(root);
      const width = Math.max(1, Math.min(4, Math.floor((cpus().length || 4) / 4)));
      let done = 0;
      await pool(files, width, async (f) => {
        await runNvTest(r, f);
        say(`select: tools tests ${++done}/${files.length}`);
      });
    }
    // The plan checks take the sweep lock and hold the origin up by the rule a sweep follows
    // (`driver/sweep-lock.ts`): a fixture of `examples/http.nvs` needs both. No leg runs here, and a
    // heavy check only when it is a setup.
    if (kinds.has("check")) {
      const own = ownChecks(plan, label, (c) => sweep.groups.get(c.id)?.how);
      const need = sharedResources(own, false, () => false);
      say(`select: ${needLine(need)}`);
      const lock = need.lock ? await takeSweepLock({ note: (l) => say(`select: ${l}`) }) : null;
      const origin = need.origin ? await holdOrigin() : null;
      try {
        if (origin !== null) say(`select: ${origin.line}`);
        for (const c of own) await sweep.check(c);
      } finally {
        origin?.close();
        lock?.release();
      }
    }
  } finally {
    r.close();
  }

  for (const a of store.atoms()) {
    if (a.lastRun < started || (a.verdict !== "green" && a.verdict !== "red")) continue;
    report.ran.set(a.id, a.verdict);
    const k = (report.kinds[a.kind] ??= { run: 0, red: 0 });
    k.run++;
    if (a.verdict === "red") k.red++;
  }
  report.owed = sweep.close().owed;
  report.misses = selectionMisses(before, new Set(selection.selected.keys()), report.ran);
  keepMisses(store, report.misses, report.ran);
  report.seconds = Math.round((Date.now() - started) / 1000);
  return report;
}

/** The lines `bun nv select --full` prints for a report. */
export function describeFull(rep: FullReport): string[] {
  if (rep.buildFailed) return [`select: the covws build failed, so nothing ran:`, rep.buildFailed];
  const picked = rep.selection.selected.size;
  const lines = [`select: full run from ${rep.since.slice(0, 12) || "no recorded tree"} in ${rep.seconds}s; the selection before it picked ${picked} atom(s)`];
  for (const [kind, k] of Object.entries(rep.kinds)) lines.push(`  ${kind.padEnd(6)} ${k.run} run, ${k.red} red`);
  if (rep.owed > 0) lines.push(`  ${rep.owed} atom(s) the change reached did not run here, and are owed`);
  if (rep.misses.length === 0) lines.push("select: no selection miss");
  else {
    lines.push(`select: ${rep.misses.length} selection miss(es): red now, and not picked by the selection`);
    for (const m of rep.misses) lines.push(`  miss  ${m.id}  (${m.lastGreen ? `green at ${new Date(m.lastGreen).toISOString()}` : "never recorded green"})`);
  }
  return lines;
}
