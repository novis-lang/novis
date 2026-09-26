// A per-file result memoized against the file's text, for a tool that reads every file of a tree to
// look at its comments or docs. A re-run still reads every file, since what a check reads is what the
// selection keys it on, and computes the result again only for a file whose text changed.
//
// The memo is `.cache/filememo/<name>.json`, which nothing commits. It is keyed on the digest of the
// module that computes the results as well, so an edit to that module computes everything again.
// Only the files a run asked about are written back, so a deleted file's entry goes with the next run.
//
// What it spends: one JSON file per tool, holding each file's digest and result; most results are empty.

import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { CACHE } from "./paths.ts";

const DIR = join(CACHE, "filememo");

/** A short digest of `text`. */
const digestOf = (text: string) => Bun.hash(text).toString(36);

export class FileMemo<T> {
  private readonly file: string;
  private readonly version: string;
  private held: Record<string, [string, T]> = {};
  private readonly kept: Record<string, [string, T]> = {};
  private dirty = false;

  /** `module` is the path of the module whose code computes the results (`import.meta.path`). */
  constructor(name: string, module: string) {
    this.file = join(DIR, `${name}.json`);
    let source = "";
    try {
      source = readFileSync(module, "utf8");
    } catch {
      // A module that cannot be read gives a version no memo holds, and everything is computed.
    }
    this.version = digestOf(source || String(Math.random()));
    try {
      const got = JSON.parse(readFileSync(this.file, "utf8")) as { version?: string; entries?: Record<string, [string, T]> };
      if (got.version === this.version && got.entries && typeof got.entries === "object") this.held = got.entries;
    } catch {
      // No memo yet, or one that cannot be read: every file is computed.
    }
  }

  /** `path`'s result over `text`: the memo's when the text is the one it was computed over, else
   * `compute`'s. */
  get(path: string, text: string, compute: () => T): T {
    const d = digestOf(text);
    const was = this.held[path];
    if (was && was[0] === d) {
      this.kept[path] = was;
      return was[1];
    }
    const value = compute();
    this.kept[path] = [d, value];
    this.dirty = true;
    return value;
  }

  /** Writes what this run asked about back. A memo that cannot be written costs the next run the
   * computing, never a wrong answer. */
  save(): void {
    if (!this.dirty && Object.keys(this.kept).length === Object.keys(this.held).length) return;
    try {
      mkdirSync(dirname(this.file), { recursive: true });
      writeFileSync(this.file, JSON.stringify({ version: this.version, entries: this.kept }));
    } catch {
      // See above.
    }
  }
}
