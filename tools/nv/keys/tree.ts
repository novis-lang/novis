// One reading of the working tree: every file git does not ignore, each file's raw digest, and each
// `.rs` file's `Analysis`. Both are read on first use, so a key over one package reads that package.
//
// `edited` returns a copy of the tree with some files' text replaced, added or deleted in memory. It
// shares every answer for a file it does not touch, so a what-if over one file costs that file.
//
// Analyses are memoized against a file's raw digest in `.agent-tmp/nv-key-tiers.json`, under
// `SCANNER`, so a run re-scans only the files that changed. What git ignores is never an input: every
// rule in `.gitignore` is build output, a cache or machine-local state.

import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { run } from "../lib/proc.ts";
import { NOT_INPUTS, ROOT, abs } from "../lib/paths.ts";
import { type Analysis, type Tier, SCANNER, analyseAll, digest } from "./scan.ts";

const MEMO = join(ROOT, ".agent-tmp", "nv-key-tiers.json");

/** `a/b/../c` as `a/c`; a path that climbs above the root keeps its leading `..`. */
export function normalize(path: string): string {
  const out: string[] = [];
  for (const part of path.replace(/\\/g, "/").split("/")) {
    if (part === "" || part === ".") continue;
    if (part === ".." && out.length > 0 && out[out.length - 1] !== "..") out.pop();
    else out.push(part);
  }
  return out.join("/");
}

/** Is `rel` the path `top` or under it? */
export function under(rel: string, top: string): boolean {
  return top === "" || rel === top || rel.startsWith(top + "/");
}

interface Shared {
  raw: Map<string, string>;
  memo: Record<string, Analysis>;
  memoDirty: boolean;
  /** The raw digests whose analysis this reading used, which is what a memo grown too big keeps. */
  used: Set<string>;
}

/** Past this many analyses, the memo keeps only the ones the last reading used. */
const MEMO_CAP = 20_000;

export class Tree {
  private constructor(
    /** Every input file, sorted, repo-relative with forward slashes. */
    readonly files: string[],
    /** `rustc -vV`, which every key built from Rust holds. */
    readonly toolchain: string,
    private readonly shared: Shared,
    private readonly overlay: Map<string, string | null>,
    private readonly own: Map<string, string>,
  ) {
    this.fileSet = new Set(files);
  }

  private readonly fileSet: Set<string>;

  /** The tree as it stands on disk. */
  static async read(): Promise<Tree> {
    const [ls, rustc] = await Promise.all([
      run(["git", "ls-files", "-z", "--cached", "--others", "--exclude-standard"], { timeoutMs: 60_000 }),
      run(["rustc", "-vV"], { timeoutMs: 60_000 }).catch(() => ({ stdout: "", stderr: "rustc could not be started" })),
    ]);
    const seen = new Set<string>();
    for (const rel of ls.stdout.split("\0")) {
      if (!rel || rel.split("/").some((p) => NOT_INPUTS.has(p)) || !existsSync(abs(rel))) continue;
      seen.add(rel);
    }
    let memo: Record<string, Analysis> = {};
    try {
      const got = JSON.parse(readFileSync(MEMO, "utf8"));
      if (got && typeof got === "object" && got[SCANNER] && typeof got[SCANNER] === "object") memo = got[SCANNER];
    } catch {
      // No memo yet, or an unreadable one: every file is scanned again.
    }
    return new Tree([...seen].sort(), rustc.stdout + rustc.stderr, { raw: new Map(), memo, memoDirty: false, used: new Set() }, new Map(), new Map());
  }

  /** This tree with `changes` applied in memory: a path to its new text, or to `null` to delete it. */
  edited(changes: Record<string, string | null>): Tree {
    const overlay = new Map(this.overlay);
    const files = new Set(this.files);
    for (const [rel, text] of Object.entries(changes)) {
      overlay.set(rel, text);
      if (text === null) files.delete(rel);
      else files.add(rel);
    }
    return new Tree([...files].sort(), this.toolchain, this.shared, overlay, new Map());
  }

  has(rel: string): boolean {
    return this.overlay.has(rel) ? this.overlay.get(rel) !== null : this.fileSet.has(rel);
  }

  /** The files under `top`, sorted. */
  under(top: string): string[] {
    let got = this.underCache.get(top);
    if (got !== undefined) return got;
    got = [];
    if (this.fileSet.has(top)) got.push(top);
    // `files` is sorted, so the paths below `top/` are one run of it.
    const prefix = top + "/";
    let lo = 0;
    let hi = this.files.length;
    while (lo < hi) {
      const mid = (lo + hi) >> 1;
      if (this.files[mid]! < prefix) lo = mid + 1;
      else hi = mid;
    }
    for (let i = lo; i < this.files.length && this.files[i]!.startsWith(prefix); i++) got.push(this.files[i]!);
    this.underCache.set(top, got);
    return got;
  }

  private readonly underCache = new Map<string, string[]>();

  /** A file's text, from the overlay or the disk. */
  text(rel: string): string {
    const o = this.overlay.get(rel);
    if (o !== undefined) {
      if (o === null) throw new Error(`${rel}: deleted in this tree`);
      return o;
    }
    return readFileSync(abs(rel), "utf8");
  }

  /** A file's raw digest; `absent` for one that is not in the tree. */
  raw(rel: string): string {
    if (this.overlay.has(rel)) {
      const o = this.overlay.get(rel);
      if (o === null || o === undefined) return "absent";
      let got = this.own.get(rel);
      if (got === undefined) this.own.set(rel, (got = digest(o)));
      return got;
    }
    let got = this.shared.raw.get(rel);
    if (got === undefined) {
      try {
        got = digest(readFileSync(abs(rel)));
      } catch {
        got = "absent";
      }
      this.shared.raw.set(rel, got);
    }
    return got;
  }

  /** A `.rs` file's analysis, from the memo when its raw digest has one. */
  analysis(rel: string): Analysis {
    const raw = this.raw(rel);
    this.shared.used.add(raw);
    if (!this.shared.memo[raw]) this.scanMissing(rel);
    return this.shared.memo[raw]!;
  }

  /** Scans `rel` and every other `.rs` file of this tree the memo has no analysis for, in one run of
   * the scanner, since starting it costs more than scanning a file. */
  private scanMissing(rel: string): void {
    const todo = new Map<string, string>();
    for (const f of [rel, ...this.files]) {
      if (!f.endsWith(".rs") || !this.has(f)) continue;
      const raw = this.raw(f);
      if (!this.shared.memo[raw] && !todo.has(raw)) todo.set(raw, f);
    }
    const got = analyseAll([...todo.values()].map((f) => this.text(f)));
    [...todo.keys()].forEach((raw, i) => (this.shared.memo[raw] = got[i]!));
    this.shared.memoDirty = true;
  }

  /** A file's digest at `tier`. Anything but a `.rs` file is its bytes at every tier. */
  digest(rel: string, tier: Tier): string {
    if (tier === "raw" || !rel.endsWith(".rs") || !this.has(rel)) return this.raw(rel);
    return this.analysis(rel)[tier];
  }

  /** The repo-relative file each include site in `rel` names, with whether the site is a test one.
   * A path that climbs out of the repository is left out. */
  includes(rel: string): { path: string; inTest: boolean }[] {
    if (!rel.endsWith(".rs") || !this.has(rel)) return [];
    return this.analysis(rel)
      .includes.map((s) => ({ path: normalize(`${dirname(rel)}/${s.path}`), inTest: s.inTest }))
      .filter((s) => !s.path.startsWith(".."));
  }

  /** Writes the analyses this reading made, for the next one. */
  save(): void {
    if (!this.shared.memoDirty) return;
    try {
      mkdirSync(dirname(MEMO), { recursive: true });
      let memo = this.shared.memo;
      if (Object.keys(memo).length > MEMO_CAP) memo = Object.fromEntries([...this.shared.used].map((d) => [d, memo[d]!]));
      writeFileSync(MEMO, JSON.stringify({ [SCANNER]: memo }));
      this.shared.memoDirty = false;
    } catch {
      // A memo that cannot be written costs the next run a scan, and nothing else.
    }
  }
}
