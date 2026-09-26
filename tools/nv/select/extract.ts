// From what a recorded run left behind to a footprint.
//
// A recorded atom leaves, in its record directory, one `<name>-<pid>.profraw` per process it started and
// one `<name>.log` they all appended to (`nvs test --record`, `NV_PROOF_RECORD`, and the test binaries
// the seed runs). Extraction turns them into keys and then deletes them:
//
// 1. `llvm-profdata merge -sparse` over the atom's raw profiles, then `llvm-profdata show
//    --all-functions`: a sparse profile keeps only the functions with a counter above zero, so the
//    names it lists are the ones that ran. Two launches per atom.
// 2. Each name is looked up in the coverage map of the objects the atom ran: `llvm-cov export -format=text
//    -skip-expansions` once per object and build, cached under `.cache/select/covmap/`, gives each
//    function's file and first line. A function in a file `nv-scan` read maps to the innermost item whose
//    lines hold that first line; one in a file a build script generated maps to the item that includes
//    that file. A function in a file of the repository no item holds gives `fn:<file>#*`, and a name no
//    map knows gives `*`: what cannot be attributed widens.
// 3. The log's lines become `class:`, `card:`, `file:`, `dir:`, `exists:`, `tree:` and `named:` keys
//    (`keys.ts`).
//
// On Windows the tools print CRLF lines and a name can start with a drive letter
// (`D:\...\a.rs;_RNv...` for a function with internal linkage), so a name is the line less its last
// colon, never the text before the first one.

import { existsSync, mkdirSync, readdirSync, readFileSync, rmSync, statSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import type { FileItems, Item } from "../keys/scan.ts";
import { digest } from "../keys/scan.ts";
import { llvmTool } from "../lib/covws.ts";
import { CACHE } from "../lib/paths.ts";
import { run } from "../lib/proc.ts";
import type { Generated } from "./build.ts";
import { fileWild, fnKey, logKeys, repoPath, WILD } from "./keys.ts";
import type { Keyed } from "./store.ts";

/** Where a function's code is: a repo file, a generated file (`gen:<package>/<name>`), or null for code
 * of no file of ours. */
export type CovLoc = [file: string | null, start: number, end: number];

const COVMAP_DIR = join(CACHE, "select", "covmap");

/** The files of one recorded atom in a record directory. */
export interface Recorded {
  name: string;
  profraws: string[];
  log: string | null;
}

/** Every recorded atom in `dir`, by record name: `<name>-<pid>.profraw`, or `<name>-<signature>_<n>.profraw`
 * from a merge pool (`%Nm`), and `<name>.log`. */
export function recordedIn(dir: string): Map<string, Recorded> {
  const out = new Map<string, Recorded>();
  if (!existsSync(dir)) return out;
  const get = (name: string) => out.get(name) ?? out.set(name, { name, profraws: [], log: null }).get(name)!;
  for (const f of readdirSync(dir)) {
    const raw = /^(.*)-\d+(?:_\d+)?\.profraw$/.exec(f);
    if (raw) get(raw[1]!).profraws.push(join(dir, f));
    else if (f.endsWith(".log")) get(f.slice(0, -4)).log = join(dir, f);
  }
  for (const r of out.values()) r.profraws.sort();
  return out;
}

/** The names `llvm-profdata show --all-functions` lists: its lines indented by exactly two spaces that end
 * in a colon, each less that colon. */
export function namesFromShow(text: string): string[] {
  const out: string[] = [];
  for (const raw of text.split("\n")) {
    const line = raw.replace(/\r$/, "");
    if (!line.startsWith("  ") || line.startsWith("   ") || !line.endsWith(":")) continue;
    out.push(line.slice(2, -1));
  }
  return out;
}

/** Merges `profraws` into `out` and returns the names of the functions that ran. The list of inputs goes
 * through a file, since a test binary can leave more of them than a command line holds. */
export async function executedNames(profraws: string[], out: string): Promise<string[]> {
  if (profraws.length === 0) return [];
  const list = `${out}.inputs`;
  writeFileSync(list, profraws.join("\n") + "\n");
  try {
    const merged = await run([llvmTool("llvm-profdata"), "merge", "-sparse", `--input-files=${list}`, "-o", out], { timeoutMs: 600_000 });
    if (merged.code !== 0) throw new Error(`llvm-profdata merge failed:\n${merged.stderr}`);
    const shown = await run([llvmTool("llvm-profdata"), "show", "--all-functions", out], { timeoutMs: 600_000 });
    if (shown.code !== 0) throw new Error(`llvm-profdata show failed:\n${shown.stderr}`);
    return namesFromShow(shown.stdout);
  } finally {
    rmSync(list, { force: true });
  }
}

/** `path` as a coverage location spells it: repo-relative, `gen:<package>/<name>` for a file under a
 * build script's `out/`, or null. */
export function covFile(path: string): string | null {
  const p = path.replace(/\\/g, "/");
  const gen = /\/build\/(.+)-[0-9a-f]{16}\/out\/(.+)$/.exec(p);
  if (gen) return `gen:${gen[1]}/${gen[2]}`;
  const r = repoPath(p);
  return r === null || r === "." ? null : r;
}

/** One `llvm-cov export -format=text` document's functions, by name: the file of the function's own
 * regions and the first and last line they cover. */
export function parseExport(json: string): Record<string, CovLoc> {
  const doc = JSON.parse(json) as { data: { functions: { name: string; regions: number[][]; filenames: string[] }[] }[] };
  const out: Record<string, CovLoc> = {};
  for (const d of doc.data) {
    for (const f of d.functions) {
      const own = f.regions.filter((r) => r[5] === 0);
      if (own.length === 0 || !f.filenames[0]) continue;
      out[f.name] = [covFile(f.filenames[0]), Math.min(...own.map((r) => r[0]!)), Math.max(...own.map((r) => r[2]!))];
    }
  }
  return out;
}

/** Name to location for the objects an atom ran, each object's map cached on disk by its path, size and
 * modification time, and the last few in memory. */
export class CovMap {
  private maps = new Map<string, Map<string, CovLoc>>();
  private pending = new Map<string, Promise<Map<string, CovLoc>>>();
  private objects: string[] = [];

  constructor(
    private readonly dir: string = COVMAP_DIR,
    private readonly keep = 6,
  ) {}

  private cacheFile(object: string): string {
    const st = statSync(object);
    return join(this.dir, `${digest(object, String(st.size), String(st.mtimeMs)).slice(0, 24)}.json`);
  }

  private async load(object: string, profdata: string): Promise<Map<string, CovLoc>> {
    const cache = this.cacheFile(object);
    let map: Record<string, CovLoc>;
    if (existsSync(cache)) map = JSON.parse(readFileSync(cache, "utf8"));
    else {
      const r = await run([llvmTool("llvm-cov"), "export", "-format=text", "-skip-expansions", "-instr-profile", profdata, object], { timeoutMs: 1_800_000 });
      // An object built from no instrumented crate holds no coverage map; a name only it could place is
      // then unmapped, which widens.
      if (r.code !== 0 && r.stderr.includes("no coverage data found")) map = {};
      else if (r.code !== 0) throw new Error(`llvm-cov export ${object} failed:\n${r.stderr.slice(0, 2000)}`);
      else map = parseExport(r.stdout);
      mkdirSync(this.dir, { recursive: true });
      writeFileSync(cache, JSON.stringify(map));
    }
    return new Map(Object.entries(map));
  }

  /** Makes `objects` the ones names are looked up in, in that order, exporting any not cached with
   * `profdata` as the profile `llvm-cov` asks for (any profile will do: every instrumented function of
   * the object is listed, counted or not). */
  async ensure(objects: string[], profdata: string): Promise<void> {
    for (const object of objects) {
      const held = this.maps.get(object);
      if (held) {
        // Taken out and put back, so the oldest in memory is always the first.
        this.maps.delete(object);
        this.maps.set(object, held);
        continue;
      }
      let p = this.pending.get(object);
      if (!p) {
        p = this.load(object, profdata);
        this.pending.set(object, p);
      }
      try {
        this.maps.set(object, await p);
      } finally {
        this.pending.delete(object);
      }
    }
    this.objects = objects;
    while (this.maps.size > Math.max(this.keep, objects.length)) {
      const oldest = [...this.maps.keys()].find((o) => !objects.includes(o));
      if (oldest === undefined) break;
      this.maps.delete(oldest);
    }
  }

  /** Where `name` is, in the objects `ensure` named last, or in `objects` when given. */
  get(name: string, objects: string[] = this.objects): CovLoc | undefined {
    for (const o of objects) {
      const loc = this.maps.get(o)?.get(name);
      if (loc) return loc;
    }
    return undefined;
  }
}

/** Each file's items, innermost first, for mapping a line to the item that holds it. */
export class ItemIndex {
  private byFile = new Map<string, Item[]>();

  constructor(files: Iterable<FileItems>) {
    for (const f of files) {
      if (!f.parsed) continue;
      this.byFile.set(
        f.file,
        f.items.filter((i) => i.kind !== "impl-rest" && i.kind !== "trait-rest").sort((a, b) => a.end - a.start - (b.end - b.start)),
      );
    }
  }

  has(file: string): boolean {
    return this.byFile.has(file);
  }

  /** The innermost item of `file` whose lines hold `line`, or null. */
  enclosing(file: string, line: number): Item | null {
    for (const item of this.byFile.get(file) ?? []) if (item.start <= line && line <= item.end) return item;
    return null;
  }
}

export interface Extracted {
  keys: Keyed;
  /** Functions that ran, and how many of them map to nothing. */
  executed: number;
  unmapped: number;
  processes: number;
}

/** The keys the functions `names` give. */
export function namesToKeys(names: string[], covmap: { get(name: string): CovLoc | undefined }, index: ItemIndex, generated: Generated[]): { keys: Keyed; unmapped: number } {
  const keys: Keyed = new Map();
  const gen = new Map(generated.map((g) => [`gen:${g.pkg}/${g.name}`, g]));
  let unmapped = 0;
  for (const name of names) {
    const loc = covmap.get(name);
    if (!loc) {
      unmapped++;
      keys.set(WILD, "");
      continue;
    }
    const [file, start] = loc;
    if (file === null) continue;
    if (file.startsWith("gen:")) {
      const g = gen.get(file);
      if (g) keys.set(fnKey(g.file, g.id), "");
      else keys.set(WILD, "");
      continue;
    }
    const item = index.enclosing(file, start);
    if (item) keys.set(fnKey(file, item.id), item.digest);
    else keys.set(fileWild(file), "");
  }
  return { keys, unmapped };
}

/**
 * One recorded atom's footprint, from its raw profiles and its log, which are deleted once read. `objects`
 * are the binaries its processes ran, for the coverage map; `scratch` is where the merged profile goes.
 */
export async function extract(rec: Recorded, objects: string[], covmap: CovMap, index: ItemIndex, generated: Generated[], scratch: string): Promise<Extracted> {
  const keys: Keyed = new Map();
  let executed = 0;
  let unmapped = 0;
  if (rec.profraws.length > 0) {
    const profdata = join(scratch, `${rec.name}.profdata`);
    try {
      const names = await executedNames(rec.profraws, profdata);
      executed = names.length;
      await covmap.ensure(objects, profdata);
      const got = namesToKeys(names, { get: (n) => covmap.get(n, objects) }, index, generated);
      unmapped = got.unmapped;
      for (const [k, d] of got.keys) keys.set(k, d);
    } finally {
      rmSync(profdata, { force: true });
    }
  }
  if (rec.log) {
    for (const k of logKeys(readFileSync(rec.log, "utf8"))) if (!keys.has(k)) keys.set(k, "");
  }
  for (const f of rec.profraws) rmSync(f, { force: true });
  if (rec.log) rmSync(rec.log, { force: true });
  return { keys, executed, unmapped, processes: rec.profraws.length };
}
