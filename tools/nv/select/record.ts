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

import { mkdirSync, readdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { cpus } from "node:os";
import { join } from "node:path";
import type { FileItems } from "../keys/scan.ts";
import { scanItems } from "../keys/scan.ts";
import type { Graph } from "../keys/graph.ts";
import { COVWS_TARGET, hostTriple } from "../lib/covws.ts";
import { abs, ROOT } from "../lib/paths.ts";
import { run } from "../lib/proc.ts";
import { recordName } from "../proofs/run.ts";
import { caseDef, caseId, treeOf } from "./atoms.ts";
import { buildScripts, type Generated, generatedDigest, generatedIncludes, generatedMeta } from "./build.ts";
import { snapshot } from "./change.ts";
import { CovMap, type Extracted, extract, ItemIndex, type Recorded, recordedIn } from "./extract.ts";
import { fileWild, fnKey, WILD } from "./keys.ts";
import { type ChangeSet, graphScope, rustFiles, type Selection } from "./select.ts";
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
  private readonly gate = limiter(Math.max(2, Math.floor((cpus().length || 4) / 4)));

  private constructor(
    readonly store: SelectStore,
    view: Map<string, FileItems>,
    generated: Generated[],
    dir: string,
    readonly say: (line: string) => void,
  ) {
    this.index = new ItemIndex(view.values());
    this.generated = generated;
    this.dir = dir;
    this.tmpEnv = { TMP: join(dir, "tmp"), TEMP: join(dir, "tmp"), TMPDIR: join(dir, "tmp") };
    this.merge = join(dir, "_merge");
  }

  /** A recorder for one run, its coverage mapped against `view` (a change's, or `currentView`). */
  static async open(store: SelectStore, view: Map<string, FileItems>, graph: Graph | null, label: string, opts: { root?: string; say?: (line: string) => void } = {}): Promise<Recorder> {
    const root = opts.root ?? ROOT;
    const scope = graphScope(graph);
    const generated = generatedIncludes(view.values(), (f) => scope.pkgOf(f), root);
    const dir = join(root, REC_ROOT, `${label}-${process.pid}`);
    rmSync(dir, { recursive: true, force: true });
    await privateDir(join(dir, "tmp"));
    mkdirSync(join(dir, "_merge"), { recursive: true });
    return new Recorder(store, view, generated, dir, opts.say ?? (() => {}));
  }

  /** Maps what is recorded from here on against `view`: a formatter rewrote files, and their items'
   * lines moved. */
  remap(view: Map<string, FileItems>, graph: Graph | null, root: string = ROOT): void {
    const scope = graphScope(graph);
    this.index = new ItemIndex(view.values());
    this.generated = generatedIncludes(view.values(), (f) => scope.pkgOf(f), root);
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

/** What one `recordCases` call ran. */
export interface CaseRun {
  /** Everything `nvs test` printed, batch after batch. */
  out: string;
  passed: number;
  failed: number;
  skipped: number;
  /** Each case, with its verdict. */
  verdicts: Map<string, Verdict>;
  processes: number;
}

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
 * case's verdict. A case that failed, that left no record and was not skipped, or whose batch ran out
 * of time is red.
 */
export async function recordCases(
  r: Recorder,
  nvs: string,
  cases: string[],
  o: { jobs: number; batch?: number; onBatch?: (done: number, total: number) => void; timeoutMs?: number; root?: string },
): Promise<CaseRun> {
  const root = o.root ?? ROOT;
  const byTree = new Map<string, string[]>();
  for (const f of cases) (byTree.get(treeOf(f)) ?? byTree.set(treeOf(f), []).get(treeOf(f))!).push(f);
  const result: CaseRun = { out: "", passed: 0, failed: 0, skipped: 0, verdicts: new Map(), processes: 0 };
  let done = 0;
  for (const [tree, all] of byTree) {
    for (const batch of chunks(all, o.batch ?? 64)) {
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
      const recorded = recordedIn(dir);
      await pool(batch, o.jobs, async (path) => {
        const got = recorded.get(recordName(path));
        const ext = got ? await r.extract(got, [nvs]) : null;
        const keys: Keyed = new Map(parentKeys);
        for (const [k, d] of ext?.keys ?? []) keys.set(k, d);
        const verdict: Verdict = failed.has(path) || (!got && !skipped.has(path)) || p.timedOut || !counts ? "red" : "green";
        r.record(caseId(path), caseDef(path, root), verdict, keys);
        result.verdicts.set(path, verdict);
        result.processes += ext?.processes ?? 0;
      });
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
 * that ran red keeps its red verdict, so both stay selected. The changed Rust files' items replace the
 * stored ones, the items that are gone leave every footprint, and each generated file's digest is taken
 * from the `covws` build as it stands. Nothing moves in a process `NO_ADVANCE_ENV` names.
 */
export function advance(store: SelectStore, change: ChangeSet, sel: Selection | null, ran: Set<string>, graph: Graph | null, root: string = ROOT): { owed: number } {
  if (!change.tree || process.env[NO_ADVANCE_ENV]) return { owed: 0 };
  const owed = sel ? store.owe([...sel.selected.keys()].filter((id) => !ran.has(id))) : 0;
  const rust = new Set(change.changes.filter((c) => c.path.endsWith(".rs")).map((c) => c.path));
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
  return { owed };
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
  };
}
