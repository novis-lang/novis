// Recording the runs a sweep, `nv verify` or `nv proofs` makes, and moving the recorded tree past a
// change once a run over it is green.
//
// Every run of an atom records its footprint (`Recorder`): each process it starts writes its coverage
// counters to `LLVM_PROFILE_FILE` and its footprint log to `NVS_FOOTPRINT_LOG`, both under the atom's
// own directory, and `take` turns them into keys, records the run in the store and deletes them. The
// compile cache starts empty for a recorded run, so the code generator's functions run and are seen:
// a test binary gets a cache directory of its own, and a case or a proof program runs with the cache
// off. Scratch goes under `.agent-tmp/select-rec/<label>-<pid>/`, which `close` deletes.
//
// `advance` records the tree a run was selected against, green or red: the items of every changed Rust
// file, the digests of what the build scripts generated, and the commit and overlay the change was read
// at. The base is shared by every kind of atom, and a run runs only some of them, so each atom the
// change selected and this run did not run is marked `owed` first: it stays selected until a run of it
// is green, and moving the base can lose nothing. An atom that ran red stays `red`, selected again.
// `owed` is written over whatever verdict the store holds, so an atom another process ran green after
// this run read its selection is owed too and runs once more: a concurrent run costs a run, never a miss.
//
// A run counts before `advance`: `record` writes each atom's run the moment it ends, stamped with the
// tree the run's selection was read at (`store.ts`), so a run that is killed, times out or crashes
// loses only the atoms still running, and the next selection reads each stamped atom against the change
// since its own tree. `advance` then clears the stamps the new base covers (`settle`).

import { copyFileSync, mkdirSync, readdirSync, readFileSync, rmSync, statSync, writeFileSync } from "node:fs";
import { cpus } from "node:os";
import { basename, join } from "node:path";
import type { FileItems } from "../keys/scan.ts";
import { scanItems } from "../keys/scan.ts";
import type { Graph } from "../keys/graph.ts";
import { COVWS_TARGET, hostTriple } from "../lib/covws.ts";
import { abs, ROOT } from "../lib/paths.ts";
import { run } from "../lib/proc.ts";
import { recordName } from "../proofs/run.ts";
import { caseDef, caseId, treeOf } from "./atoms.ts";
import { buildScripts, type Generated, generatedDigest, generatedIncludes, generatedMeta } from "./build.ts";
import { snapshot, treeSig } from "./change.ts";
import { CovMap, type Extracted, extract, ItemIndex, type Recorded, recordedIn, recordsIn } from "./extract.ts";
import { fileWild, fnKey, WILD } from "./keys.ts";
import { type ChangeSet, changedItems, embedders, graphScope, recordOverlayMarkers, rustFiles, type Selection } from "./select.ts";
import type { Keyed, SelectStore, Verdict } from "./store.ts";

export const REC_ROOT = ".agent-tmp/select-rec";

/**
 * Makes `dir`, writable by this account, the administrators and the system alone. `nvs` refuses to
 * write into a directory an ordinary group may write to, and a directory under this repository inherits
 * whatever its drive grants, so a test that makes its scratch under the temporary directory would fail
 * for the place rather than for what it tests.
 */
export async function privateDir(dir: string): Promise<void> {
  mkdirSync(dir, { recursive: true });
  if (process.platform !== "win32") return;
  const user = process.env.USERNAME ?? "";
  const r = await run(["icacls", dir, "/inheritance:r", "/grant:r", "*S-1-5-18:(OI)(CI)F", "*S-1-5-32-544:(OI)(CI)F", `${user}:(OI)(CI)F`], { timeoutMs: 60_000 });
  if (r.code !== 0) throw new Error(`icacls ${dir} failed:\n${r.stdout}${r.stderr}`);
}

/** Runs `body` over `items`, `width` at a time. */
export async function pool<T>(items: T[], width: number, body: (item: T) => Promise<void>): Promise<void> {
  let next = 0;
  const worker = async () => {
    while (next < items.length) await body(items[next++]!);
  };
  await Promise.all(Array.from({ length: Math.max(1, Math.min(width, items.length)) }, worker));
}

export function chunks<T>(items: T[], size: number): T[][] {
  const out: T[][] = [];
  for (let i = 0; i < items.length; i += size) out.push(items.slice(i, i + size));
  return out;
}

/** At most `width` bodies at once, in the order they were asked for. */
function limiter(width: number): <T>(body: () => Promise<T>) => Promise<T> {
  let active = 0;
  const waiting: (() => void)[] = [];
  return async (body) => {
    if (active >= width) await new Promise<void>((r) => waiting.push(r));
    active++;
    try {
      return await body();
    } finally {
      active--;
      waiting.shift()?.();
    }
  };
}

/** The items a recorded run's coverage is mapped against, when no change was read: the stored scan, or
 * a scan of every Rust file for a store that has none. */
export async function currentView(store: SelectStore, root: string = ROOT): Promise<Map<string, FileItems>> {
  const stored = store.allItems();
  if (stored.size > 0) return stored;
  return new Map(scanItems(await rustFiles(root), root).map((f) => [f.file, f]));
}

export class Recorder {
  readonly covmap = new CovMap();
  index: ItemIndex;
  generated: Generated[];
  /** This run's scratch directory, absolute. */
  readonly dir: string;
  /** The temporary directory every recorded process gets, which `nvs` accepts to write in. */
  readonly tmpEnv: Record<string, string>;
  private readonly merge: string;
  private readonly gate: <T>(body: () => Promise<T>) => Promise<T>;

  private constructor(
    readonly store: SelectStore,
    view: Map<string, FileItems>,
    generated: Generated[],
    dir: string,
    readonly say: (line: string) => void,
    extractWidth: number,
  ) {
    this.index = new ItemIndex(view.values());
    this.generated = generated;
    this.dir = dir;
    this.tmpEnv = { TMP: join(dir, "tmp"), TEMP: join(dir, "tmp"), TMPDIR: join(dir, "tmp") };
    this.merge = join(dir, "_merge");
    this.gate = limiter(extractWidth);
  }

  /**
   * A recorder for one run, its coverage mapped against `view` (a change's, or `currentView`). At most
   * `extractWidth` extractions run at once, a quarter of the cores and at least two by default: a caller
   * whose own pool already bounds how many records wait, as `bun nv proofs` does, passes that pool's width.
   */
  static async open(store: SelectStore, view: Map<string, FileItems>, graph: Graph | null, label: string, opts: { root?: string; say?: (line: string) => void; extractWidth?: number } = {}): Promise<Recorder> {
    const root = opts.root ?? ROOT;
    const scope = graphScope(graph);
    const generated = generatedIncludes(view.values(), (f) => scope.pkgOf(f), root);
    const dir = join(root, REC_ROOT, `${label}-${process.pid}`);
    rmSync(dir, { recursive: true, force: true });
    await privateDir(join(dir, "tmp"));
    mkdirSync(join(dir, "_merge"), { recursive: true });
    const extractWidth = Math.max(1, opts.extractWidth ?? Math.max(2, Math.floor((cpus().length || 4) / 4)));
    return new Recorder(store, view, generated, dir, opts.say ?? (() => {}), extractWidth);
  }

  /** Maps what is recorded from here on against `view`: a formatter rewrote files, and their items'
   * lines moved. */
  remap(view: Map<string, FileItems>, graph: Graph | null, root: string = ROOT): void {
    const scope = graphScope(graph);
    this.index = new ItemIndex(view.values());
    this.generated = generatedIncludes(view.values(), (f) => scope.pkgOf(f), root);
  }

  /**
   * A copy of the binary `nvs` in this run's directory, for the run to start and map in its place, and
   * deleted with the directory. Another process can rebuild `nvs` while a long run records, from a
   * different profile or a newer tree, and the copy keeps every record of the run to the one build it
   * started with. It spends the binary's size on disk for the run's length. A copy is taken again when
   * `nvs` changed while it was copied.
   */
  pin(nvs: string): string {
    const copy = join(this.dir, basename(nvs));
    for (let tries = 1; ; tries++) {
      const before = statSync(nvs);
      copyFileSync(nvs, copy);
      const after = statSync(nvs);
      if (before.size === after.size && before.mtimeMs === after.mtimeMs && before.ino === after.ino) {
        this.covmap.pin(copy, { path: nvs, size: before.size, mtimeMs: before.mtimeMs });
        return copy;
      }
      if (tries === 3) throw new Error(`${nvs} changed each time it was copied`);
    }
  }

  /** The directory one atom's processes record into, made empty. */
  slot(name: string): string {
    const d = join(this.dir, "atoms", name);
    rmSync(d, { recursive: true, force: true });
    mkdirSync(d, { recursive: true });
    return d;
  }

  /** What every process of one run of the atom recorded as `name` is told: where its counters and its
   * log go, and the temporary directory. Counters go into a merge pool of a few files per binary, since
   * one atom can start hundreds of processes. */
  env(name: string): Record<string, string> {
    const d = join(this.dir, "atoms", name);
    mkdirSync(d, { recursive: true });
    return { ...this.tmpEnv, LLVM_PROFILE_FILE: join(d, `${name}-%4m.profraw`), NVS_FOOTPRINT_LOG: join(d, `${name}.log`) };
  }

  /** A compile cache of its own for the atom recorded as `name`, empty and private. */
  async cacheDir(name: string): Promise<Record<string, string>> {
    const cache = join(this.dir, "cache", name);
    rmSync(cache, { recursive: true, force: true });
    await privateDir(cache);
    return { LOCALAPPDATA: cache, XDG_CACHE_HOME: cache };
  }

  /** `extract`, with a record that cannot be read kept to its atom: the footprint becomes `*`, which
   * every change selects. */
  async extract(rec: Recorded, objects: string[]): Promise<Extracted> {
    return this.gate(async () => {
      try {
        return await extract(rec, objects, this.covmap, this.index, this.generated, this.merge);
      } catch (e) {
        this.say(`select: ${rec.name}: its record could not be read, so it depends on everything: ${(e as Error).message.split("\n")[0]}`);
        for (const f of rec.profraws) rmSync(f, { force: true });
        if (rec.log) rmSync(rec.log, { force: true });
        return { keys: new Map([[WILD, ""]]), executed: 0, unmapped: 0, processes: rec.profraws.length };
      }
    });
  }

  /** What the atom recorded as `name` left in its directory, as keys; null when it left nothing. The
   * directory is deleted. */
  async keysOf(name: string, objects: string[]): Promise<Extracted | null> {
    const d = join(this.dir, "atoms", name);
    const got = recordedIn(d).get(name) ?? null;
    const ext = got ? await this.extract(got, objects) : null;
    rmSync(d, { recursive: true, force: true });
    rmSync(join(this.dir, "cache", name), { recursive: true, force: true });
    return ext;
  }

  /** Records one run of `id` in the store. */
  record(id: string, def: string, verdict: Verdict, keys: Keyed): void {
    this.store.recordRun(id, { def, verdict, keys });
  }

  close(): void {
    rmSync(this.dir, { recursive: true, force: true });
  }
}

/** A test binary's last green run: its `test result:` lines and every test line it printed. A check that
 * names a test is judged on these when the binary does not run. */
export interface TestGreen {
  result?: string;
  tests?: string[];
}

export const testGreenSlot = (name: string) => `verify:test:${name}`;
const RESULT_LINE = /test result: \w+\. (\d+) passed; (\d+) failed/;
const TEST_LINE = /^test \S+ (?:- should panic )?\.\.\. \w+/;

/** The last green run of the binary `<package> <kind> <target>`, or null when none is kept. */
export function testGreen(store: SelectStore, name: string): TestGreen | null {
  const v = store.verdict(testGreenSlot(name));
  if (!v || !v.verdict) return null;
  try {
    return JSON.parse(v.verdict) as TestGreen;
  } catch {
    return null;
  }
}

/** Keeps what one run of a binary printed when it was green, and forgets it when it was not. */
export function putTestGreen(store: SelectStore, name: string, verdict: Verdict, text: string): void {
  const out = text.split(/\r?\n/);
  const green: TestGreen = { result: out.filter((l) => RESULT_LINE.test(l)).join("\n"), tests: out.filter((l) => TEST_LINE.test(l)).map((l) => l.trimEnd()) };
  store.putVerdict(testGreenSlot(name), "", verdict === "green" ? JSON.stringify(green) : "");
}

/** The slot a case's last skip is kept in: "skip" when `nvs test` skipped it on this platform. */
export const caseSkipSlot = (path: string) => `case-skip:${process.platform}:${path}`;

/** Whether the store holds a skip verdict for `path`: null when no run has said. */
export function caseSkipped(store: SelectStore, path: string): boolean | null {
  const v = store.verdict(caseSkipSlot(path));
  return v === null ? null : v.verdict === "skip";
}

/** What one `recordCases` call ran. */
export interface CaseRun {
  /** Everything `nvs test` printed, batch after batch. */
  out: string;
  passed: number;
  failed: number;
  skipped: number;
  /** Each case, with its verdict. */
  verdicts: Map<string, Verdict>;
  /** Each case that passed and left no recording, which makes it red. */
  unrecorded: string[];
  /** Each red case, with why it is red and what its batch's `nvs test` ended with (`redReason`). */
  reds: Map<string, string>;
  processes: number;
}

const NO_RECORDING = "it left no recording";

/** Why one case of a batch is red, or null when it is green: the verdict and its reason are one test. */
export function redReason(c: { failed: boolean; recorded: boolean; skipped: boolean; timedOut: boolean; counted: boolean }): string | null {
  if (c.failed) return "it failed";
  if (!c.recorded && !c.skipped) return NO_RECORDING;
  if (c.timedOut) return "its batch ran out of time";
  if (!c.counted) return "its batch printed no count line";
  return null;
}

/**
 * The lines that name each red case the output does not already explain: a red case is explained by its
 * `FAIL` line or by the list of passed cases that left no recording. Every other red case is listed with
 * its reason, grouped by reason, so a step that is red while every case passed says why.
 */
export function unexplainedReds(got: CaseRun): string {
  const failed = caseLabels(got.out, "FAIL");
  const lost = new Set(got.unrecorded);
  const byWhy = new Map<string, string[]>();
  for (const [path, v] of got.verdicts) {
    if (v !== "red" || failed.has(path) || lost.has(path)) continue;
    const why = got.reds.get(path) ?? "no reason was kept";
    (byWhy.get(why) ?? byWhy.set(why, []).get(why)!).push(path);
  }
  if (byWhy.size === 0) return "";
  const n = [...byWhy.values()].reduce((a, xs) => a + xs.length, 0);
  const out = [`${n} case(s) are red, and nothing above says why:`];
  for (const [why, paths] of byWhy) out.push(`  ${paths.length}: ${why}: ${few(paths, 3)}`);
  return out.join("\n") + "\n";
}

/** What one batch of `recordCases` lost: the passed cases that left no recording, and what the case
 * directory looked like when it was listed for extraction and again once the batch was extracted. */
export interface LostBatch {
  tree: string;
  /** The batch's place among its tree's batches, from 1. */
  index: number;
  batches: number;
  size: number;
  lost: string[];
  /** How many atoms the listing at extraction time held, and the error it threw, if it threw one. */
  listed: number;
  listError: string | null;
  /** The lost cases the listing at extraction time holds when each is looked up again: a case here was
   * listed, and the lookup that judged it missed it. */
  relooked: string[];
  /** Whether the `nvs test` parent left a recording. */
  parent: boolean;
  /** "exists", "missing", or what reading the directory's status threw. */
  dir: string;
  /** The directory's names once the batch was extracted, or what listing it threw. Extraction deletes
   * what it read, so a name here is one the first listing did not see or could not use. */
  fresh: string[] | string;
}

/**
 * The recording of the case at `path` in a batch's listing, by its record name. When the map's lookup
 * misses, the name is computed again and the listing's entries are compared with it one by one, which
 * does not use the map's hash. A recording found that way is the case's, and `missed` says which of the
 * two gave a wrong answer: the map's lookup, or `recordName`, which returned two different names. Under
 * load in a long `verify`, the lookup has missed names the same map held. That has not been reproduced
 * alone, so the scan keeps the verdict right and the line says which part failed.
 */
export function recordingOf(recorded: Map<string, Recorded>, path: string): { rec: Recorded | null; missed: string | null } {
  const name = recordName(path);
  const rec = recorded.get(name);
  if (rec !== undefined) return { rec, missed: null };
  const again = recordName(path);
  for (const [k, v] of recorded) {
    if (k !== again) continue;
    return { rec: v, missed: again === name ? "the map's lookup missed a name it holds" : `recordName returned ${JSON.stringify(name)} and then ${JSON.stringify(again)}` };
  }
  return { rec: null, missed: null };
}

const few = (xs: string[], n: number): string => xs.slice(0, n).join(", ") + (xs.length > n ? ` (+${xs.length - n} more)` : "");

/** The few lines a batch with lost recordings adds to the step's output. When the first listing holds a
 * lost case under a second lookup, it says so: the lookup missed, not the listing. When the second
 * listing finds a recording of a lost case, it says so: the first listing missed a file that was there. */
export function lostBlock(b: LostBatch): string {
  const out = [
    `coverage lost: batch ${b.index}/${b.batches} of ${b.tree} (${b.size} cases): ${b.lost.length} passed case(s) left no recording`,
    `  cases: ${few(b.lost, 3)}`,
    `  at extraction: ${b.listError ? `listing threw ${b.listError}` : `${b.listed} recorded atom(s) listed`}; parent recorded: ${b.parent ? "yes" : "no"}; directory: ${b.dir}`,
  ];
  if (b.relooked.length > 0) out.push(`  ${b.relooked.length} of the ${b.lost.length} are in that listing when looked up again, so the lookup missed names it held: ${few(b.relooked, 3)}`);
  if (typeof b.fresh === "string") out.push(`  listed again: threw ${b.fresh}`);
  else {
    const profraws = b.fresh.filter((f) => f.endsWith(".profraw")).length;
    const logs = b.fresh.filter((f) => f.endsWith(".log")).length;
    out.push(`  listed again: ${profraws} .profraw, ${logs} .log, ${b.fresh.length - profraws - logs} other${b.fresh.length ? `: ${few(b.fresh, 4)}` : ""}`);
    const now = recordsIn("", b.fresh);
    const found = b.lost.filter((p) => now.has(recordName(p)));
    out.push(
      found.length > 0
        ? `  ${found.length} of the ${b.lost.length} have a recording now, so the listing at extraction time missed files that were there: ${few(found, 3)}`
        : `  none of the ${b.lost.length} has a recording now`,
    );
  }
  return out.join("\n") + "\n";
}

/** What reading `dir`'s status gives: "exists", "missing", or the error it threw. */
function dirState(dir: string): string {
  try {
    return statSync(dir, { throwIfNoEntry: false }) ? "exists" : "missing";
  } catch (e) {
    return errText(e);
  }
}

const errText = (e: unknown): string => (e as NodeJS.ErrnoException).code ?? (e as Error).message.split("\n")[0]!;

/** The labels a `nvs test` report names as failed, or as skipped. */
export function caseLabels(out: string, word: "FAIL" | "SKIP"): Set<string> {
  const found = new Set<string>();
  const re = new RegExp(`^${word} (.+?)(?: [—-] .*)?\\r?$`);
  for (const line of out.split("\n")) {
    const m = re.exec(line);
    if (m) found.add(m[1]!.replace(/\\/g, "/").replace(/^\.\//, ""));
  }
  return found;
}

const CASES_RE = /(\d+) passed, (\d+) failed, (\d+) skipped/;

/**
 * Runs `cases` through `nvs test --cases <list> --record <dir>`, one tree and one batch at a time, and
 * records every case. The `nvs test` parent is recorded too, and what it ran is part of every case of
 * its batch; its listings of the tree are not, since what else a tree holds decides nothing about one
 * case's verdict. A case is red for the reason `redReason` gives: it failed, it left no record and was
 * not skipped, or its batch ran out of time or printed no count line. Each case's record is looked up
 * once per batch with `recordingOf`, and both its verdict and the batch's lost list are read from that
 * one lookup, so a batch with a passed case red for a missing record always adds `lostBlock`'s lines to
 * the output, and a recording only the scan found adds one `select:` line.
 * `stop` is asked before each batch starts, and once it says yes no other batch starts.
 */
export async function recordCases(
  r: Recorder,
  nvs: string,
  cases: string[],
  o: { jobs: number; batch?: number; onBatch?: (done: number, total: number) => void; timeoutMs?: number; root?: string; stop?: () => boolean },
): Promise<CaseRun> {
  const root = o.root ?? ROOT;
  const byTree = new Map<string, string[]>();
  for (const f of cases) (byTree.get(treeOf(f)) ?? byTree.set(treeOf(f), []).get(treeOf(f))!).push(f);
  const result: CaseRun = { out: "", passed: 0, failed: 0, skipped: 0, verdicts: new Map(), unrecorded: [], reds: new Map(), processes: 0 };
  let done = 0;
  for (const [tree, all] of byTree) {
    const batches = chunks(all, o.batch ?? 64);
    for (const [index, batch] of batches.entries()) {
      if (o.stop?.()) return result;
      const dir = join(r.dir, "cases");
      rmSync(dir, { recursive: true, force: true });
      mkdirSync(dir, { recursive: true });
      const list = join(r.dir, "cases.txt");
      writeFileSync(list, batch.join("\n") + "\n");
      const parent = join(r.dir, "parent");
      rmSync(parent, { recursive: true, force: true });
      mkdirSync(parent, { recursive: true });
      const p = await run([nvs, "test", "--cases", list, "--record", dir, "--jobs", String(o.jobs), `${tree}/`], {
        cwd: root,
        env: { ...r.tmpEnv, NO_COLOR: "1", NOVIS_NO_FILE_CACHE: "1", LLVM_PROFILE_FILE: join(parent, "p-%p.profraw"), NVS_FOOTPRINT_LOG: join(parent, "p.log") },
        timeoutMs: o.timeoutMs ?? 3_600_000,
        reap: true,
      });
      result.out += p.stdout + p.stderr + (p.timedOut ? `\nkilled after ${(o.timeoutMs ?? 3_600_000) / 60000} minutes\n` : "");
      const counts = CASES_RE.exec(p.stdout);
      if (counts) {
        result.passed += Number(counts[1]);
        result.failed += Number(counts[2]);
        result.skipped += Number(counts[3]);
      } else result.failed += batch.length;
      const failed = caseLabels(p.stdout, "FAIL");
      const skipped = caseLabels(p.stdout, "SKIP");
      const parentRec = recordedIn(parent).get("p");
      const parentKeys: Keyed = parentRec ? (await r.extract(parentRec, [nvs])).keys : new Map();
      for (const k of [...parentKeys.keys()]) if (/^(dir|exists|tree):/.test(k)) parentKeys.delete(k);
      // A listing that throws is kept as the error and an empty listing, so every case of the batch is
      // red and the error is named in the output.
      let recorded = new Map<string, Recorded>();
      let listError: string | null = null;
      try {
        recorded = recordedIn(dir);
      } catch (e) {
        listError = errText(e);
      }
      // One lookup per case, before any extraction starts: the verdict and the lost list are both read
      // from it, so a case red for a missing recording is always one the lost block names.
      const looked = batch.map((path) => [path, recordingOf(recorded, path)] as const);
      const found = new Map(looked.map(([path, l]) => [path, l.rec]));
      const scanned = looked.filter(([, l]) => l.missed !== null);
      if (scanned.length > 0) {
        result.out += `select: batch ${index + 1}/${batches.length} of ${tree}: ${scanned.length} recording(s) found by a scan of the listing: ${scanned[0]![1].missed}: ${few(scanned.map(([path]) => path), 3)}\n`;
      }
      const reasons = new Map(batch.map((path) => [path, redReason({ failed: failed.has(path), recorded: found.get(path) !== null, skipped: skipped.has(path), timedOut: p.timedOut, counted: !!counts })]));
      const lost = p.timedOut || !counts ? [] : batch.filter((path) => reasons.get(path) === NO_RECORDING);
      await pool(batch, o.jobs, async (path) => {
        const got = found.get(path) ?? null;
        const ext = got ? await r.extract(got, [nvs]) : null;
        const keys: Keyed = new Map(parentKeys);
        for (const [k, d] of ext?.keys ?? []) keys.set(k, d);
        const why = reasons.get(path) ?? null;
        const verdict: Verdict = why === null ? "green" : "red";
        if (why !== null) result.reds.set(path, `${why} (batch ${index + 1}/${batches.length} of ${tree}: exit ${p.code}, ${p.stderr.length} byte(s) on stderr, ${recorded.size} recorded)`);
        r.record(caseId(path), caseDef(path, root), verdict, keys);
        r.store.putVerdict(caseSkipSlot(path), "", skipped.has(path) ? "skip" : "");
        result.verdicts.set(path, verdict);
        result.processes += ext?.processes ?? 0;
      });
      if (lost.length > 0) {
        let fresh: string[] | string;
        try {
          fresh = readdirSync(dir);
        } catch (e) {
          fresh = errText(e);
        }
        result.unrecorded.push(...lost);
        const relooked = lost.filter((path) => recorded.has(recordName(path)));
        result.out += lostBlock({ tree, index: index + 1, batches: batches.length, size: batch.length, lost, listed: recorded.size, listError, relooked, parent: !!parentRec, dir: dirState(dir), fresh });
      }
      done += batch.length;
      o.onBatch?.(done, cases.length);
    }
  }
  return result;
}

/** The repo paths the dep-info files of a cargo build name: every file rustc read for the crates built
 * into `deps` (sources, and what `include_str!` and `include_bytes!` embed). */
export function depInfoPaths(depsDir: string, repo: (p: string) => string | null): Set<string> {
  const out = new Set<string>();
  let names: string[] = [];
  try {
    names = readdirSync(depsDir);
  } catch {
    return out;
  }
  for (const n of names) {
    if (!n.endsWith(".d")) continue;
    let text = "";
    try {
      text = readFileSync(join(depsDir, n), "utf8");
    } catch {
      continue;
    }
    for (const line of text.split("\n")) {
      const at = line.indexOf(": ");
      if (at < 0) continue;
      for (const raw of line.slice(at + 2).split(/(?<!\\) /)) {
        const p = raw.trim().replace(/\\ /g, " ");
        if (!p) continue;
        const r = repo(p);
        if (r !== null && r !== ".") out.add(r);
      }
    }
  }
  return out;
}

/** Set by a run that starts another `bun nv` process which records into the same store: the inner run
 * records what it ran and leaves the tree to the outer one, which moves it once for both. */
export const NO_ADVANCE_ENV = "NV_SELECT_NO_ADVANCE";

/**
 * Records the tree `change` was read at as the store's base, after a run, green or red, that ran the
 * atoms in `ran`. Each atom `sel` selected that this run did not run is marked owed first, and an atom
 * that ran red keeps its red verdict, so both stay selected. Each atom `sel` found gone is forgotten,
 * with its footprint. The changed Rust files' items replace the
 * stored ones, the items that are gone leave every footprint, and each generated file's digest is taken
 * from the `covws` build as it stands. Nothing moves in a process `NO_ADVANCE_ENV` names.
 */
export function advance(store: SelectStore, change: ChangeSet, sel: Selection | null, ran: Set<string>, graph: Graph | null, root: string = ROOT): { owed: number } {
  if (!change.tree || process.env[NO_ADVANCE_ENV]) return { owed: 0 };
  const owed = sel
    ? store.owe(
        [...sel.selected.keys()].filter((id) => !ran.has(id)),
        (id) => changedItems(sel.selected.get(id)!),
      )
    : 0;
  for (const id of sel?.gone ?? []) store.removeAtom(id);
  // A file that embeds a changed file was read again, so its stored items take the new digests too.
  const rust = new Set([...change.changes.filter((c) => c.path.endsWith(".rs")).map((c) => c.path), ...embedders(change.view.values(), change.changes.map((c) => c.path))]);
  if (change.full) store.replaceItems([...change.view.values()]);
  else {
    store.setItems([...rust].flatMap((f) => (change.view.has(f) ? [change.view.get(f)!] : [])));
    store.removeItems([...rust].filter((f) => !change.view.has(f)));
  }
  const gone = change.items.filter((c) => c.how === "removed").map((c) => fnKey(c.file, c.id));
  for (const f of rust) if (!change.view.has(f)) gone.push(fileWild(f));
  if (gone.length > 0) store.prune(gone);
  const scope = graphScope(graph);
  const pkgDirs = new Map([...(graph?.values() ?? [])].map((p) => [p.name, p.dir]));
  const scripts = buildScripts(abs(`${COVWS_TARGET}/${hostTriple()}/debug/build`, root), pkgDirs);
  for (const g of generatedIncludes(change.view.values(), (f) => scope.pkgOf(f), root)) store.setMeta(generatedMeta(store.platform, g), generatedDigest(scripts.get(g.pkg), g.name));
  store.setBase(change.tree.commit, change.tree.overlay);
  recordOverlayMarkers(store, change.tree.overlay, root);
  settle(store, treeSig(change.tree), sel, ran);
  return { owed };
}

/**
 * Clears each stamp the base, now the tree `sig` names, covers: a stamp of that tree; the stamp of an
 * atom `sel` selected that did not run, which is owed or red; and the stamp of an atom `sel` read against
 * its own tree and did not select, whose last run holds on this tree too. A stamp another process wrote
 * after `sel` was read is kept. Then every tree no atom is stamped with is deleted, and every judged
 * verdict taken on another tree.
 */
function settle(store: SelectStore, sig: string, sel: Selection | null, ran: Set<string>): void {
  const at = store.treeN(sig);
  const seen = sel?.stamps ?? new Map<string, number>();
  store.unstamp((id, n) => n === at || (sel !== null && (sel.selected.has(id) ? !ran.has(id) : seen.get(id) === n)));
  store.dropStale(sig);
}

/** A change that selects every atom, for a store with no recorded tree: every Rust file scanned as it is
 * now, and the tree taken to be recorded once the run is green. */
export async function fullChange(root: string = ROOT, why = "the store has no recorded tree"): Promise<ChangeSet> {
  const tree = await snapshot(root);
  const view = new Map(scanItems(await rustFiles(root), root).map((f) => [f.file, f]));
  return {
    since: "",
    changes: [],
    moved: new Map([[WILD, { path: "", how: "global" }]]),
    global: why,
    rustFiles: view.size,
    itemChanges: 0,
    wideFiles: [],
    items: [],
    view,
    tree,
    full: true,
    root,
  };
}
