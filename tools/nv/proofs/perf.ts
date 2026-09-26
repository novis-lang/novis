// The perf ledger: `bun nv proofs --record-perf` measures a feature's bench and appends one record to
// `docs/perf/members.ndjson`, and `--perf-report` writes `docs/perf/members.md` from every record in it.
// `rule:testing/member-perf-ledger` is what a record holds and when it is current, and
// `benches/members/README.md` is what a bench program is.
//
// A record is four counts and one clock. The counts come from one `nvs run --count` and are the same on
// every machine. The clock is the fastest of `--reps` timed runs, with the empty program's fastest run
// subtracted, and `ratio` divides it by the calibration program's cost per iteration, measured in the same
// sweep. A bench's `// bench:` lines say what it expects, and a record whose figures miss them is not
// written unless the bench carries a `proof: gap` marker: the record then carries the findings.
//
// The measurements run on the release binary, never the proof binary, because a clock taken on a build
// with a different link is not the clock a shipped program sees.

import { createHash } from "node:crypto";
import { appendFileSync, existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";
import { dirty, head, lastCommit } from "../lib/git.ts";
import { abs } from "../lib/paths.ts";
import { progress } from "../lib/progress.ts";
import { fixed, general } from "../lib/py.ts";
import { fingerprint, implHash, knownGap, LEDGER, ledgerRecords, owed, type Policy, type Proofs, type Skips } from "./collect.ts";
import { benchFile, implFile, read, type Entry } from "./roster.ts";
import { skipReason, spawnProof } from "./run.ts";

export const PERF_REPORT = "docs/perf/members.md";
const CALIBRATION = "benches/members/_calibration";
const POLICY_FILE = "data/proofs/policy.json";

/** `// bench: iterations 200000` inside a bench program. */
const ITER_RE = /(?:\/\/|#)\s*bench:\s*iterations\s+([0-9_]+)/;
/** `// bench: allocations 0` and its siblings: what the bench expects per operation. Met to within a
 * hundredth, so a one-off set-up allocation over many iterations rounds away and a per-call one does not. */
const EXPECT_RE = /(?:\/\/|#)\s*bench:\s*(allocations|calls|statements|bytes)\s+([0-9_]+)/g;
/** `// bench: complexity constant` in a bench, and `// bench: scale 10` in its `<name>.scale.nvs` sibling,
 * whose input is that many times the bench's. The ratio of their per-operation clocks, taken seconds
 * apart on one machine, is the one wall-clock check that holds on any machine. */
const COMPLEXITY_RE = /(?:\/\/|#)\s*bench:\s*complexity\s+(constant|linear)/;
const SCALE_RE = /(?:\/\/|#)\s*bench:\s*scale\s+([0-9._]+)/;
/** The one line `nvs run --count` prints on standard error at exit. */
const COUNT_LINE_RE = /^count: statements=(\d+) calls=(\d+) allocations=(\d+) bytes=(\d+)/m;
const COUNTS = ["statements", "calls", "allocations", "bytes"] as const;
/** How far a scaling ratio may sit above what its declared complexity predicts: a factor, because a
 * same-run wall-clock ratio is honest to a factor and not to a percent. */
const SCALE_TOLERANCE = 3;
/** The least a calibration iteration may cost. Every `units` figure divides by it, so a calibration that
 * measured nothing, on a machine busy enough that the empty program's fastest run lands above the unit
 * program's, is refused rather than recorded. A tenth of a nanosecond is under one clock cycle. */
const MIN_UNIT_NS = 0.1;
const TIMEOUT_MS = 600_000;

type Rec = Record<string, unknown>;

/** A failure that stops the sweep, with the line that says why. */
class PerfError extends Error {}

const firstLine = (text: string) => text.trim().split(/\r?\n/)[0] ?? "";

/** What `recordPerf` is measuring now, which the progress text starts with: `calibrating`, `feature 3/12`. */
let measuring = "";

/** (the fastest, the median) of `reps` runs, in nanoseconds. The fastest is the figure, and the median
 * rides beside it so a later delta can be read against the spread it was taken in. */
async function timeProgram(nvs: string, path: string, reps: number): Promise<[number, number]> {
  const spent: number[] = [];
  for (let i = 0; i < reps; i++) {
    progress(`proofs perf: ${measuring}, ${path}, run ${i + 1}/${reps}`);
    const out = await spawnProof([nvs, "run", path], path, TIMEOUT_MS);
    spent.push(out.ms * 1e6);
    if (out.timedOut) throw new PerfError(`${path} timed out after ${TIMEOUT_MS / 1000}s`);
    if (out.code !== 0) throw new PerfError(`${path} exited ${out.code}: ${firstLine(out.stderr)}`);
  }
  spent.sort((a, b) => a - b);
  return [spent[0]!, spent[Math.floor(spent.length / 2)]!];
}

/** `rule:testing/bench-counters`'s four totals for one run of `path`. One run, because the answer is the
 * same every time. */
async function countProgram(nvs: string, path: string): Promise<Record<string, number>> {
  const out = await spawnProof([nvs, "run", "--count", path], path, TIMEOUT_MS);
  if (out.code !== 0) throw new PerfError(`${path} exited ${out.code} under --count: ${firstLine(out.stderr)}`);
  const m = COUNT_LINE_RE.exec(out.stderr.replace(/\r\n/g, "\n"));
  if (!m) throw new PerfError(`${path}: \`nvs run --count\` printed no count line -- is ${nvs} built from this tree?`);
  return Object.fromEntries(COUNTS.map((k, i) => [k, Number(m[i + 1])]));
}

const number = (digits: string) => Number(digits.replace(/_/g, ""));

function iterationsOf(path: string): number {
  const m = ITER_RE.exec(read(path));
  if (!m) throw new PerfError(`${path} declares no \`// bench: iterations N\``);
  return number(m[1]!);
}

/** What a bench declares per operation, as `{ allocations: 0, calls: 1 }`, or nothing. */
function expectationsOf(path: string): Record<string, number> {
  return Object.fromEntries([...read(path).matchAll(EXPECT_RE)].map((m) => [m[1]!, number(m[2]!)]));
}

const round = (x: number, digits: number) => Math.round(x * 10 ** digits) / 10 ** digits;

/** (the empty program's floor in ns, the calibration unit's ns per iteration), taken in this sweep. The
 * floor is subtracted so a figure is the work and not the CLI's start-up, and the unit is what a ratio is
 * expressed in so the number means something on another machine. */
async function calibrate(nvs: string, reps: number): Promise<[number, number]> {
  const baseline = `${CALIBRATION}/baseline.nvs`;
  const unit = `${CALIBRATION}/unit.nvs`;
  for (const p of [baseline, unit]) if (!existsSync(abs(p))) throw new PerfError(`the calibration program ${p} is missing`);
  const [floor] = await timeProgram(nvs, baseline, reps);
  const [unitTotal] = await timeProgram(nvs, unit, reps);
  const unitNs = (unitTotal - floor) / iterationsOf(unit);
  if (unitNs < MIN_UNIT_NS) {
    throw new PerfError(
      `the calibration did not measure anything -- the unit program's fastest run (${fixed(unitTotal / 1e6, 1)} ms) is not enough above ` +
        `the empty program's (${fixed(floor / 1e6, 1)} ms) to price one iteration at ${MIN_UNIT_NS} ns. ` +
        "Something else on this machine is taking the CPU; re-run --record-perf when it is idle",
    );
  }
  return [floor, unitNs];
}

/** One feature's figures, and every way they fall short of what its bench declared. */
async function measureOne(nvs: string, bench: string, reps: number, floor: number, base: Record<string, number>): Promise<[Rec, string[]]> {
  const iters = iterationsOf(bench);
  const [total, median] = await timeProgram(nvs, bench, reps);
  const nsPerOp = Math.max(0, (total - floor) / iters);
  const counts = await countProgram(nvs, bench);
  const perOp = Object.fromEntries(COUNTS.map((k) => [k, round(Math.max(0, counts[k]! - base[k]!) / iters, 3)])) as Record<string, number>;
  const fig: Rec = { iterations: iters, ns_per_op: round(nsPerOp, 3), median_ns_per_op: round(Math.max(0, (median - floor) / iters), 3), ...perOp };
  const findings: string[] = [];
  const expected = expectationsOf(bench);
  if (Object.keys(expected).length) fig.expected = expected;
  for (const [k, want] of Object.entries(expected)) {
    if (Math.abs(perOp[k]! - want) > 0.01) findings.push(`declares \`${k} ${want}\` per op and did ${fixed(perOp[k]!, 3)}`);
  }
  const complexity = COMPLEXITY_RE.exec(read(bench))?.[1];
  if (complexity) fig.complexity = complexity;
  const sibling = bench.replace(/\.nvs$/, ".scale.nvs");
  if (existsSync(abs(sibling))) {
    const k = SCALE_RE.exec(read(sibling));
    if (!k) throw new PerfError(`${sibling} declares no \`// bench: scale K\``);
    const scale = number(k[1]!);
    const [scaledTotal] = await timeProgram(nvs, sibling, reps);
    const scaled = Math.max(0, (scaledTotal - floor) / iterationsOf(sibling));
    const ratio = nsPerOp > 0 ? scaled / nsPerOp : Infinity;
    Object.assign(fig, { scale, scale_ns_per_op: round(scaled, 3), scale_ratio: round(ratio, 3) });
    // An upper bound only. A linear member over a small input is dominated by its fixed per-call cost and
    // looks nearly constant, which is not a bug. Growing faster than declared is the finding.
    const predicted = complexity === "constant" ? 1 : complexity === "linear" ? scale : null;
    if (predicted !== null && ratio > predicted * SCALE_TOLERANCE) {
      findings.push(
        `declares \`complexity ${complexity}\` and costs ${fixed(ratio, 1)}x per op on ${general(scale)}x the input, past the ${general(predicted * SCALE_TOLERANCE)}x that allows`,
      );
    }
  }
  return [fig, findings];
}

/** The fields a record holds as Python floats, written with a `.0` when whole so every line of the ledger
 * reads alike. */
const FLOATS = new Set(["ns_per_op", "median_ns_per_op", "statements", "calls", "allocations", "bytes", "scale", "scale_ns_per_op", "scale_ratio", "unit_ns", "ratio"]);

/** One ledger line, as Python's `json.dumps` wrote the lines before it. `field` is the record's own key a
 * value sits under, and is empty inside a nested value. */
function ledgerLine(value: unknown, field = "", top = true): string {
  if (Array.isArray(value)) return `[${value.map((v) => ledgerLine(v, "", false)).join(", ")}]`;
  if (typeof value === "object" && value !== null) {
    return `{${Object.entries(value)
      .map(([k, v]) => `${ledgerLine(k)}: ${ledgerLine(v, top ? k : "", false)}`)
      .join(", ")}}`;
  }
  if (typeof value === "number" && FLOATS.has(field) && Number.isInteger(value)) return `${value}.0`;
  if (typeof value === "number" && !Number.isFinite(value)) return value > 0 ? "Infinity" : "-Infinity";
  return JSON.stringify(value).replace(/[\u0080-￿]/g, (c) => `\\u${c.charCodeAt(0).toString(16).padStart(4, "0")}`);
}

export interface RecordOptions {
  reps: number;
  note: string;
  force: boolean;
}

const ljust = (s: string, n: number) => s + " ".repeat(Math.max(0, n - s.length));
const rjust = (s: string, n: number) => " ".repeat(Math.max(0, n - s.length)) + s;

/**
 * `--record-perf`: measures and appends. By default only what has no current figure, so running it at the
 * end of a slice re-measures what that slice changed and nothing else. `force` re-measures everything in
 * scope, for when the question is the machine and not the code. A bench this host does not run, by its
 * `requires:` line, is left out as though it were not on disk. Returns the exit status.
 */
export async function recordPerf(out: string[], nvs: string, entries: Entry[], proofs: Map<string, Proofs>, policy: Policy, skips: Skips, opts: RecordOptions): Promise<number> {
  let todo = entries.filter((e) => existsSync(abs(benchFile(e))) && skipReason(read(benchFile(e))) === null);
  if (!opts.force) {
    todo = todo.filter((e) => "perf" in owed(e, proofs.get(e.id)!, policy, skips));
    if (todo.length === 0) {
      out.push("nv proofs perf: every bench in scope already has a current figure (--force re-measures anyway).");
      return 0;
    }
  }
  if (todo.length === 0) {
    out.push("nv proofs perf: no bench programs in scope -- nothing to measure.");
    return 0;
  }
  if (!nvs.replace(/\\/g, "/").includes("release")) {
    out.push(`nv proofs perf: refusing to measure with ${nvs} -- build a release binary (\`cargo build --release -p nvs-cli\`).`);
    return 1;
  }
  const fp = fingerprint();
  const commit = (await head()).slice(0, 12);
  const isDirty = (await dirty()).length > 0;
  const binary = createHash("sha1").update(readFileSync(nvs)).digest("hex").slice(0, 12);
  const print = (line: string) => console.log(line);
  let floor: number;
  let unitNs: number;
  let base: Record<string, number>;
  try {
    measuring = "calibrating";
    [floor, unitNs] = await calibrate(nvs, opts.reps);
    base = await countProgram(nvs, `${CALIBRATION}/baseline.nvs`);
  } catch (e) {
    if (!(e instanceof PerfError)) throw e;
    out.push(`nv proofs perf: ${e.message}`);
    return 1;
  }
  // A sweep takes minutes, so each line is printed as its feature finishes.
  print(`nv proofs perf: ${todo.length} features, ${opts.reps} reps, unit = ${fixed(unitNs, 1)} ns/iteration on ${fp.cpu} (${fp.id}), binary ${binary}`);
  print(`  ${ljust("feature", 44)} ${rjust("ns/op", 10)} ${rjust("units", 9)}  ${rjust("stmts", 7)} ${rjust("calls", 7)} ${rjust("allocs", 7)} ${rjust("bytes", 9)}`);
  const lines: string[] = [];
  let failed = 0;
  for (const [i, e] of [...todo].sort((a, b) => (a.id < b.id ? -1 : a.id > b.id ? 1 : 0)).entries()) {
    const bench = benchFile(e);
    measuring = `feature ${i + 1}/${todo.length}`;
    let fig: Rec;
    let findings: string[];
    try {
      [fig, findings] = await measureOne(nvs, bench, opts.reps, floor, base);
    } catch (err) {
      if (!(err instanceof PerfError)) throw err;
      out.push(`  FAIL  ${e.id}: ${err.message}`);
      return 1;
    }
    const gap = knownGap(read(bench));
    if (findings.length && !gap) {
      failed++;
      print(`  FAIL  ${e.id}: ${findings.join("; ")}`);
      print("        fix it, or mark the bench `// proof: gap <gap id>`; the figure is not recorded until one of those lands.");
      continue;
    }
    const impl = implFile(e);
    const rec: Rec = {
      at: new Date().toISOString().replace(/\.\d+Z$/, "Z"),
      id: e.id,
      kind: e.kind,
      group: e.group,
      commit,
      dirty: isDirty,
      binary,
      impl_commit: impl ? (await lastCommit(impl)).slice(0, 12) : "",
      impl_hash: impl ? implHash(impl) : "",
      machine: fp.id,
      cpu: fp.cpu,
      os: fp.os,
      cores: fp.cores,
      reps: opts.reps,
      ...fig,
      unit_ns: round(unitNs, 3),
      ratio: round((fig.ns_per_op as number) / unitNs, 4),
      note: opts.note,
    };
    if (findings.length) rec.known_gap = { gap, findings };
    lines.push(ledgerLine(rec));
    const n = (k: string, d: number, w: number) => rjust(fixed(fig[k] as number, d), w);
    print(
      `  ${ljust(e.id, 44)} ${n("ns_per_op", 1, 10)} ${rjust(fixed(rec.ratio as number, 3), 9)}  ${n("statements", 2, 7)} ${n("calls", 2, 7)} ${n("allocations", 2, 7)} ${n("bytes", 1, 9)}` +
        ("scale" in fig ? `   scale x${general(fig.scale as number)}: ${fixed(fig.scale_ratio as number, 2)}x` : "") +
        (findings.length ? `   known-gap: ${findings.join("; ")}` : ""),
    );
  }
  if (lines.length) {
    mkdirSync(dirname(abs(LEDGER)), { recursive: true });
    appendFileSync(abs(LEDGER), lines.join("\n") + "\n");
  }
  out.push(`nv proofs perf: ${lines.length} records appended to ${LEDGER}` + (failed ? `, ${failed} bench(es) missed what they declared and were not recorded` : ""));
  return failed ? 1 : 0;
}

/** `report` in the policy file, over its defaults. `outlierFactor` is how far above its group's median
 * a `units` figure sits before the report lists it. `ceiling` is a `units` figure per declared complexity,
 * past which a feature is listed whatever its neighbours cost. Both are advisory and never a gate. */
function reportPolicy(): { outlierFactor: number; ceiling: Record<string, number> } {
  const policy = { outlierFactor: 5, ceiling: {} as Record<string, number> };
  if (!existsSync(abs(POLICY_FILE))) return policy;
  const table = (JSON.parse(read(POLICY_FILE)).report ?? {}) as Record<string, unknown>;
  if ("outlierFactor" in table) policy.outlierFactor = Number(table.outlierFactor);
  for (const [k, v] of Object.entries((table.ceiling ?? {}) as Record<string, unknown>)) policy.ceiling[k] = Number(v);
  return policy;
}

/** A count's change against the record before it, signed, or blank when there is no earlier count or it
 * did not move. Any machine's record is the earlier one, because a count is the same everywhere. */
function countDelta(last: Rec, prev: Rec | null, key: string): string {
  if (prev === null || !(key in prev) || !(key in last)) return "";
  const change = (last[key] as number) - (prev[key] as number);
  return Math.abs(change) < 0.0005 ? "" : signed(change, 3);
}

const signed = (x: number, digits: number) => (x >= 0 ? "+" : "") + fixed(x, digits);
const byKey = (a: string, b: string) => (a < b ? -1 : a > b ? 1 : 0);
const num = (v: unknown) => (typeof v === "number" ? v : 0);

/**
 * `--perf-report`: writes `docs/perf/members.md`, the ledger's front page, whole. It has three parts. What
 * the program did: one row per feature, the same on every machine, each count diffed against the record
 * before it. The candidates: features whose `units` figure sits far above their group's median, past the
 * ceiling for their declared complexity, or recorded as a known gap. Then the clock, one table per
 * machine, each reading against the one before it on that same machine only.
 */
export function perfReport(out: string[]): number {
  if (!existsSync(abs(LEDGER))) {
    out.push(`nv proofs: ${LEDGER} does not exist yet -- run --record-perf first.`);
    return 1;
  }
  const byFeature = ledgerRecords();
  if (byFeature.size === 0) {
    out.push(`nv proofs: ${LEDGER} holds no records -- run --record-perf first.`);
    return 1;
  }
  // (machine, feature) -> records, oldest first, in the order the ledger first names each pair.
  const history = new Map<string, { mid: string; fid: string; recs: Rec[] }>();
  for (const [fid, recs] of byFeature) {
    for (const rec of recs) {
      const mid = String(rec.machine ?? "");
      const k = `${mid}\0${fid}`;
      if (!history.has(k)) history.set(k, { mid, fid, recs: [] });
      history.get(k)!.recs.push(rec);
    }
  }
  const machines = new Map<string, Rec>();
  for (const { mid, recs } of history.values()) if (!machines.has(mid)) machines.set(mid, recs.at(-1)!);
  const report = reportPolicy();

  const doc = [
    "# Measured cost, feature by feature",
    "",
    "**Generated by `bun nv proofs --perf-report` — never edited.**",
    `[\`${LEDGER}\`](members.ndjson) is the append-only ledger this is the front page of;`,
    "`tools/nv/proofs/perf.ts` owns how a figure is taken and `benches/members/README.md` owns what a",
    "bench program is. `rule:testing/member-perf-ledger` is what the columns mean.",
    "",
    "Every figure is Novis against **itself**: there is no PHP column here and there never will",
    "be — [`benches/userland/`](../../benches/userland/README.md) owns the cross-engine",
    "comparison. The **counts** — statements, calls, allocations, bytes, each per operation",
    "with the empty program's share subtracted — are what the program did, and are the same on",
    "every machine for the same commit, so their `Δ` is against the previous record wherever it",
    "was taken. A `Core` member is one call however much it does inside, so a count sees what",
    "the program asked for and not what the member cost. The **clock** — `ns/op`, the fastest",
    "of the reps, with `median` beside it — is wall clock on the machine named in the heading,",
    "and its `Δ` is against the previous reading **on that machine** only; `units` divides it by",
    "the calibration program measured in the same sweep and travels to about a tenth",
    "(`rule:testing/perf-two-mechanisms`). A clock `Δ` inside the spread the median shows is",
    "marked `~`: it is the machine, not the code.",
    "",
    "## What the program did, per operation",
    "",
    "| Feature | statements | calls | allocations | bytes | Δ allocations | Δ bytes | Declares | Commit |",
    "|---|---:|---:|---:|---:|---:|---:|---|---|",
  ];
  for (const fid of [...byFeature.keys()].sort(byKey)) {
    const recs = byFeature.get(fid)!.filter((r) => "allocations" in r);
    if (recs.length === 0) continue;
    const last = recs.at(-1)!;
    const prev = recs.length > 1 ? recs.at(-2)! : null;
    let declares = Object.entries((last.expected ?? {}) as Record<string, number>)
      .map(([k, v]) => `${k} ${v}`)
      .join(", ");
    if (last.complexity) declares = [`complexity ${last.complexity}`, declares].filter((x) => x).join(", ");
    doc.push(
      `| \`${fid}\` | ${fixed(num(last.statements), 2)} | ${fixed(num(last.calls), 2)} | ${fixed(num(last.allocations), 2)} | ${fixed(num(last.bytes), 1)} | ` +
        `${countDelta(last, prev, "allocations")} | ${countDelta(last, prev, "bytes")} | ${declares} | ${last.commit ?? ""} |`,
    );
  }
  doc.push("");

  // The candidates are read off this machine's latest clock per feature, group by group.
  const me = fingerprint().id;
  const latest = new Map<string, Rec>();
  for (const { mid, fid, recs } of history.values()) if (mid === me) latest.set(fid, recs.at(-1)!);
  const groups = new Map<string, Rec[]>();
  for (const rec of latest.values()) {
    const g = String(rec.group ?? "");
    if (!groups.has(g)) groups.set(g, []);
    groups.get(g)!.push(rec);
  }
  const candidates: string[] = [];
  for (const group of [...groups.keys()].sort(byKey)) {
    const recs = groups.get(group)!;
    const ratios = recs.map((r) => num(r.ratio)).sort((a, b) => a - b);
    const median = ratios.length ? ratios[Math.floor(ratios.length / 2)]! : 0;
    for (const rec of [...recs].sort((a, b) => byKey(String(a.id ?? ""), String(b.id ?? "")))) {
      const why: string[] = [];
      const ratio = num(rec.ratio);
      if (recs.length >= 3 && median > 0 && ratio > report.outlierFactor * median) {
        why.push(`${fixed(ratio / median, 1)}x its group's median of ${fixed(median, 1)} units`);
      }
      const ceiling = report.ceiling[String(rec.complexity ?? "")];
      if (ceiling !== undefined && ratio > ceiling) why.push(`${fixed(ratio, 1)} units against a \`${rec.complexity}\` ceiling of ${general(ceiling)}`);
      const gap = rec.known_gap as { findings: string[] } | undefined;
      if (gap) why.push("recorded as known-gap: " + gap.findings.join("; "));
      if (why.length) candidates.push(`| \`${rec.id}\` | ${group} | ${why.join(" — ")} |`);
    }
  }
  doc.push(
    "## Candidates",
    "",
    `Advisory, never a gate: a \`units\` figure more than ${general(report.outlierFactor)}x its`,
    "group's median on this machine, a figure past the ceiling `data/proofs/policy.json`",
    "sets for its declared complexity, or a bench recorded as `known-gap`. A row here is where",
    "a person with a profiler looks first; it is not a verdict.",
    "",
  );
  if (candidates.length) doc.push("| Feature | Group | Why |", "|---|---|---|", ...candidates);
  else doc.push("*(none on this machine's latest readings)*");
  doc.push("");

  for (const mid of [...machines.keys()].sort(byKey)) {
    const sample = machines.get(mid)!;
    const rows = [...history.values()].filter((h) => h.mid === mid).sort((a, b) => byKey(a.fid, b.fid));
    doc.push(
      `## ${sample.cpu ?? "unknown CPU"} · ${sample.os ?? "?"} · ${sample.cores ?? "?"} cores  (\`${mid}\`)`,
      "",
      "| Feature | ns/op | median | units | Δ | Scaling | Measured at | Implementation |",
      "|---|---:|---:|---:|---:|---|---|---|",
    );
    for (const { fid, recs } of rows) {
      const last = recs.at(-1)!;
      const prev = recs.length > 1 ? recs.at(-2)! : null;
      let delta = "";
      if (prev && prev.ns_per_op) {
        const change = ((num(last.ns_per_op) - num(prev.ns_per_op)) / num(prev.ns_per_op)) * 100;
        delta = `${signed(change, 1)}%`;
        const median = last.median_ns_per_op;
        if (median && num(last.ns_per_op) > 0) {
          const spread = ((num(median) - num(last.ns_per_op)) / num(last.ns_per_op)) * 100;
          if (Math.abs(change) <= spread) delta += " ~";
        }
      }
      const medianCell = "median_ns_per_op" in last ? fixed(num(last.median_ns_per_op), 1) : "";
      const scaling = "scale" in last ? `x${general(num(last.scale))} → ${fixed(num(last.scale_ratio), 2)}x` : "";
      doc.push(
        `| \`${fid}\` | ${fixed(num(last.ns_per_op), 1)} | ${medianCell} | ${fixed(num(last.ratio), 3)} | ${delta} | ${scaling} | ` +
          `${last.commit ?? ""} | ${last.impl_hash || last.impl_commit || ""} |`,
      );
    }
    doc.push("");
  }
  writeFileSync(abs(PERF_REPORT), doc.join("\n"));
  const total = [...history.values()].reduce((n, h) => n + h.recs.length, 0);
  out.push(`nv proofs: wrote ${PERF_REPORT} (${total} records, ${machines.size} machines)`);
  return 0;
}
