// `bun nv select --mutate <file>`: proof that the selection loses no coverage (ADR 0226, § Verification).
//
// The file is JSON under `tools/data/`: `{ "about": "...", "batches": [[edit, ...], ...] }`, where an edit
// is `{ "file", "find", "replace", "note" }`. `find` must occur exactly once in `file`; an empty `find`
// creates `file`, which must not exist, with `replace` as its bytes. Each edit is chosen to break
// something, so some atom goes red.
//
// One batch at a time: every edit of the batch is applied, the selection since the store's tree is taken,
// and every atom that is not heavy runs (`full.ts` `fullRun`) against a copy of the store, so nothing a
// mutated tree does is recorded in the real one. `STORE_ENV` names the copy for this process and every
// process it starts. Every atom that went red must be one the selection picked; a red atom it did not
// pick is a miss, printed with what `--explain` says about it. Every edit is reverted in a `finally`, and
// the batch fails when `git status` afterwards differs from before.

import { existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { ROOT } from "../lib/paths.ts";
import { selectionMisses } from "./full.ts";
import type { Selection } from "./select.ts";
import { type AtomKind, kindOfAtom, SelectStore, STORE_ENV, storeFile, type Verdict } from "./store.ts";

export interface Edit {
  file: string;
  find: string;
  replace: string;
  note: string;
}

export interface MutationFile {
  about?: string;
  batches: Edit[][];
}

/** A file as it was before a batch touched it: its bytes, or null for a file the batch created. */
export interface Applied {
  file: string;
  original: Buffer | null;
}

export interface BatchReport {
  /** The batch's number, from 1. */
  index: number;
  edits: Edit[];
  /** How many atoms of each kind the selection picked. */
  selected: Partial<Record<AtomKind, number>>;
  red: string[];
  /** Each red atom the selection did not pick, with what `--explain` says about it. */
  misses: { id: string; explain: string[] }[];
  /** Why the batch proves nothing: an edit that did not apply, a build that failed, a tree left changed. */
  failed?: string;
}

export interface RunResult {
  selection: Selection;
  ran: Map<string, Verdict>;
  buildFailed?: string;
}

export interface BatchOptions {
  root?: string;
  /** The real store's file, which is copied and never written. */
  storeFile?: string;
  /** Where the copy goes; deleted afterwards. */
  scratch?: string;
  /** Runs every atom on the copy. */
  run: (store: SelectStore) => Promise<RunResult>;
  /** What `--explain` says about an atom. */
  explain?: (store: SelectStore, sel: Selection, id: string) => string[];
  /** The tree's state, compared before and after; `git status` by default. */
  status?: (root: string) => string;
}

/** Reads and checks a mutation file. */
export function readMutations(path: string): MutationFile {
  const doc = JSON.parse(readFileSync(path, "utf8")) as MutationFile;
  if (!doc || !Array.isArray(doc.batches) || doc.batches.length === 0) throw new Error(`${path}: no "batches" list`);
  doc.batches.forEach((b, i) => {
    if (!Array.isArray(b) || b.length === 0) throw new Error(`${path}: batch ${i + 1} is not a list of edits`);
    for (const e of b) {
      for (const k of ["file", "find", "replace", "note"] as const) if (typeof e[k] !== "string") throw new Error(`${path}: batch ${i + 1}: an edit has no string "${k}"`);
      if (e.find === e.replace) throw new Error(`${path}: batch ${i + 1}: the edit of ${e.file} changes nothing`);
    }
  });
  return doc;
}

/** How often `needle` occurs in `hay`, overlaps not counted. */
function occurrences(hay: string, needle: string): number {
  let n = 0;
  for (let i = hay.indexOf(needle); i !== -1; i = hay.indexOf(needle, i + needle.length)) n++;
  return n;
}

/** A file a batch writes: what it was, and what it becomes. */
export interface Planned extends Applied {
  next: string;
}

/**
 * Works out every edit, in order, in memory, and writes nothing: each touched file as it was and as it
 * becomes. An edit whose `find` does not occur exactly once in the file as the edits before it left it
 * throws. A file with CRLF line ends is matched with the edit's line ends turned to CRLF.
 */
export function planEdits(edits: Edit[], root: string = ROOT): Planned[] {
  const files = new Map<string, Planned>();
  for (const e of edits) {
    const path = join(root, e.file);
    if (e.find === "") {
      if (files.has(e.file) || existsSync(path)) throw new Error(`${e.file}: an edit with an empty "find" creates the file, and it exists`);
      files.set(e.file, { file: e.file, original: null, next: e.replace });
      continue;
    }
    let p = files.get(e.file);
    if (p === undefined) {
      if (!existsSync(path)) throw new Error(`${e.file}: no such file`);
      const bytes = readFileSync(path);
      p = { file: e.file, original: bytes, next: bytes.toString("utf8") };
      files.set(e.file, p);
    }
    const crlf = p.next.includes("\r\n");
    const find = crlf ? e.find.replace(/\r?\n/g, "\r\n") : e.find;
    const replace = crlf ? e.replace.replace(/\r?\n/g, "\r\n") : e.replace;
    const n = occurrences(p.next, find);
    if (n !== 1) throw new Error(`${e.file}: "find" occurs ${n} times, and must occur once: ${JSON.stringify(e.find.slice(0, 80))}`);
    p.next = p.next.replace(find, () => replace);
  }
  return [...files.values()];
}

/**
 * Applies every edit and returns what each touched file was before. Every edit is worked out before any
 * file is written (`planEdits`), so an edit that does not match leaves the tree untouched; a write that
 * fails reverts the files already written.
 */
export function applyEdits(edits: Edit[], root: string = ROOT): Applied[] {
  const planned = planEdits(edits, root);
  const applied: Applied[] = [];
  try {
    for (const p of planned) {
      const path = join(root, p.file);
      applied.push({ file: p.file, original: p.original });
      mkdirSync(dirname(path), { recursive: true });
      writeFileSync(path, p.next);
    }
  } catch (err) {
    revertEdits(applied, root);
    throw err;
  }
  return applied;
}

/** Puts every file back as it was: its bytes written again, or the file deleted when a batch made it.
 * Every file is tried; the first failure is thrown after the rest. */
export function revertEdits(applied: Applied[], root: string = ROOT): void {
  let first: unknown = null;
  for (const a of [...applied].reverse()) {
    try {
      const path = join(root, a.file);
      if (a.original === null) rmSync(path, { force: true });
      else writeFileSync(path, a.original);
    } catch (e) {
      first ??= e;
    }
  }
  if (first !== null) throw first;
}

/** `git status --porcelain` over every file, ignored ones left out. */
export function gitStatus(root: string = ROOT): string {
  const p = Bun.spawnSync(["git", "status", "--porcelain", "--untracked-files=all"], { cwd: root, stdout: "pipe", stderr: "pipe" });
  if (p.exitCode !== 0) throw new Error(`git status failed: ${p.stderr.toString()}`);
  return p.stdout.toString();
}

/** The red atoms of `ran` that `selected` does not hold, sorted: the selection misses `--full` reports
 * (`full.ts` `selectionMisses`). */
export function unselectedReds(ran: Map<string, Verdict>, selected: Set<string>): string[] {
  return selectionMisses(new Map(), selected, ran).map((m) => m.id);
}

/** The errors Windows gives while another handle still has a file of the directory open. */
const BUSY = new Set(["EBUSY", "EPERM", "ENOTEMPTY"]);

/**
 * Deletes `dir` and everything in it. On Windows a file another handle has open cannot be deleted, and
 * the store copy is still open for a moment after a long run (a scanner, or a process the run started
 * that is still exiting), so a busy error is tried again every `delayMs` for up to `tries` attempts before
 * it is thrown.
 */
export function removeScratch(dir: string, o: { tries?: number; delayMs?: number; rm?: (dir: string) => void } = {}): void {
  const rm = o.rm ?? ((d: string) => rmSync(d, { recursive: true, force: true }));
  const tries = o.tries ?? 60;
  for (let i = 1; ; i++) {
    try {
      rm(dir);
      return;
    } catch (e) {
      if (i >= tries || !BUSY.has((e as NodeJS.ErrnoException).code ?? "")) throw e;
      Bun.sleepSync(o.delayMs ?? 500);
    }
  }
}

/**
 * Runs `body` on a copy of the store in `from`, with `STORE_ENV` naming the copy for this process and every
 * process it starts. The copy and its directory are deleted afterwards (`removeScratch`), and `STORE_ENV`
 * is put back.
 */
export async function withStoreCopy<T>(from: string, scratch: string, body: (store: SelectStore) => Promise<T>): Promise<T> {
  rmSync(scratch, { recursive: true, force: true });
  mkdirSync(scratch, { recursive: true });
  const copy = join(scratch, "select.sqlite");
  const was = process.env[STORE_ENV];
  let store: SelectStore | null = null;
  try {
    const real = new SelectStore(from);
    try {
      real.copyTo(copy);
    } finally {
      real.close();
    }
    process.env[STORE_ENV] = copy;
    store = new SelectStore(copy);
    return await body(store);
  } finally {
    store?.close();
    if (was === undefined) delete process.env[STORE_ENV];
    else process.env[STORE_ENV] = was;
    removeScratch(scratch);
  }
}

/** Applies batch `index` (from 1), runs every atom on a copy of the store, and checks that every atom that
 * went red was selected. The edits are always reverted. */
export async function mutateBatch(index: number, edits: Edit[], o: BatchOptions): Promise<BatchReport> {
  const root = o.root ?? ROOT;
  const status = o.status ?? gitStatus;
  const report: BatchReport = { index, edits, selected: {}, red: [], misses: [] };
  const before = status(root);
  let applied: Applied[] = [];
  try {
    applied = applyEdits(edits, root);
    await withStoreCopy(o.storeFile ?? storeFile(), o.scratch ?? join(root, ".agent-tmp", `select-mutate-${process.pid}`), async (store) => {
      const res = await o.run(store);
      if (res.buildFailed) {
        report.failed = `the build failed with the batch applied, so no atom's verdict says anything: ${res.buildFailed.split("\n")[0]}`;
        return;
      }
      for (const s of res.selection.selected.values()) report.selected[s.kind] = (report.selected[s.kind] ?? 0) + 1;
      report.red = [...res.ran].filter(([, v]) => v === "red").map(([id]) => id).sort();
      const picked = new Set(res.selection.selected.keys());
      report.misses = unselectedReds(res.ran, picked).map((id) => ({ id, explain: o.explain ? o.explain(store, res.selection, id) : [] }));
      if (report.red.length === 0) report.failed = "no atom went red, so the batch proves nothing: an edit of it should break more";
    });
  } catch (e) {
    report.failed = (e as Error).message;
  } finally {
    revertEdits(applied, root);
  }
  const after = status(root);
  if (after !== before) report.failed = `${report.failed ? `${report.failed}; ` : ""}the tree is not as it was before the batch:\n${after}`;
  return report;
}

/** The lines `bun nv select --mutate` prints for one batch. */
export function describeBatch(b: BatchReport): string[] {
  const lines = [`batch ${b.index}: ${b.edits.length} edit(s)`];
  for (const e of b.edits) lines.push(`  edit  ${e.file}: ${e.note}`);
  const kinds = Object.entries(b.selected).map(([k, n]) => `${n} ${k}`).join(", ");
  lines.push(`  selected: ${kinds || "nothing"}`);
  const redKinds = new Map<string, number>();
  for (const id of b.red) redKinds.set(kindOfAtom(id), (redKinds.get(kindOfAtom(id)) ?? 0) + 1);
  const byKind = [...redKinds].map(([k, n]) => `${n} ${k}`).join(", ");
  lines.push(`  red: ${b.red.length}${b.red.length > 0 ? ` (${byKind}): ${b.red.slice(0, 8).join(", ")}${b.red.length > 8 ? ", ..." : ""}` : ""}`);
  if (b.failed) lines.push(`  FAILED: ${b.failed}`);
  else if (b.misses.length === 0) lines.push("  every red atom was selected");
  for (const m of b.misses) {
    lines.push(`  MISS  ${m.id} went red and was not selected`);
    for (const l of m.explain) lines.push(`        ${l}`);
  }
  return lines;
}
