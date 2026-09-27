// What `tools/nv-scan` reads off a `.rs` file, in two modes.
//
// `analyse` digests a file at five tiers, each over less of the file than the one before it. The perf
// ledger's `implHash` (`proofs/collect.ts`) takes the `card` tier, so a perf figure stays current over
// an edit to a comment, a test or a reference card:
//
// - `raw`: the bytes.
// - `docs`: the tokens, doc comments among them.
// - `code`: the tokens, with each run of doc comments as one placeholder. Layout and plain comments
//   are not tokens. A file with a bidirectional-text character is its text in every tier, because
//   `rustc` then denies a comment.
// - `shipped`: the code without any item, impl or trait member or statement that its `#[cfg(...)]`
//   shuts off when `test` is not set. A binary built without `cfg(test)` cannot reach a token in one.
// - `card`: the shipped code without the registry's reference cards. Every `const` or `static` of a
//   card type is left out, a field `doc: Some(&CARD)` naming one of them reads as `doc: None`, and a
//   card type is left out of a `use` list.
//
// Every tier but `raw` is read off the syntax tree by `tools/nv-scan`, a Rust program that parses the
// file with `syn`, which this module builds on first use and runs once for a batch of files. A file
// `syn` cannot parse is its text in every tier. No tier sees a line number. `includes` lists every
// `include_str!`/`include_bytes!` site with a literal path, and whether the site is in code a build
// without `cfg(test)` leaves out.
//
// `scanItems` runs the scanner's other mode, `--items`: every item of each file, with a stable id,
// its line span, a digest without doc comments, the names it refers to and binds, and which `Core`
// class a registry row or card belongs to. Observed selection (`tools/nv/select/`) maps coverage and
// changes onto these.

import { existsSync, readFileSync, statSync } from "node:fs";
import { abs } from "../lib/paths.ts";

const HELPER = "tools/nv-scan";
const SOURCES = ["Cargo.toml", "Cargo.lock", "src/main.rs", "src/items.rs"].map((f) => `${HELPER}/${f}`);

/** Folded into every tiered digest, so a change to the scanner changes every digest it gives. */
export const SCANNER = digest("nv-scan", ...SOURCES.map((f) => readFileSync(abs(f))));

export type Tier = "raw" | "docs" | "code" | "shipped" | "card";

/** The tiers from widest to narrowest. */
export const TIERS: readonly Tier[] = ["raw", "docs", "code", "shipped", "card"];

export interface IncludeSite {
  /** The literal path, as written. */
  path: string;
  /** Is the site in code a build without `cfg(test)` leaves out? */
  inTest: boolean;
}

export interface Analysis {
  docs: string;
  code: string;
  shipped: string;
  card: string;
  includes: IncludeSite[];
}

let built = "";

/** The scanner's executable, built first when it is missing or older than its sources. */
function helper(): string {
  if (built) return built;
  const target = abs(`${HELPER}/target`);
  const exe = `${target}/release/nv-scan${process.platform === "win32" ? ".exe" : ""}`;
  const age = (p: string) => statSync(abs(p)).mtimeMs;
  if (!existsSync(exe) || SOURCES.some((f) => age(f) > statSync(exe).mtimeMs)) {
    const r = Bun.spawnSync(["cargo", "build", "--release", "--quiet", "--manifest-path", abs(`${HELPER}/Cargo.toml`)], {
      env: { ...process.env, CARGO_TARGET_DIR: target },
      stdout: "pipe",
      stderr: "pipe",
    });
    if (r.exitCode !== 0) throw new Error(`nv-scan did not build:\n${r.stderr.toString()}`);
  }
  return (built = exe);
}

/** Each text's analysis, in order, from one run of the scanner. */
export function analyseAll(texts: string[]): Analysis[] {
  if (texts.length === 0) return [];
  const input: Buffer[] = [];
  for (const t of texts) {
    const bytes = Buffer.from(t, "utf8");
    input.push(Buffer.from(`${bytes.length}\n`), bytes);
  }
  const r = Bun.spawnSync([helper()], { stdin: Buffer.concat(input), stdout: "pipe", stderr: "pipe" });
  if (r.exitCode !== 0) throw new Error(`nv-scan failed:\n${r.stderr.toString()}`);
  const lines = r.stdout.toString().split("\n").filter((l) => l !== "");
  if (lines.length !== texts.length) throw new Error(`nv-scan answered ${lines.length} of ${texts.length} file(s)`);
  return lines.map((l) => JSON.parse(l) as Analysis);
}

/** One text's analysis. A caller with several texts uses `analyseAll`, which starts one process. */
export function analyse(text: string): Analysis {
  return analyseAll([text])[0]!;
}

/** One array element of a class table: the classes it names and its own digest. */
export interface ItemRow {
  classes: string[];
  digest: string;
}

/** One item of a Rust file, as `nv-scan --items` reads it. `tools/nv-scan/src/items.rs` says what
 * each field holds. */
export interface Item {
  id: string;
  kind: string;
  start: number;
  end: number;
  test: boolean;
  digest: string;
  refs: string[];
  defines: string[];
  parent?: string;
  /** Who may name the item when that is narrower than `pub`; missing means `pub`. */
  scope?: "private" | "crate";
  includes?: string[];
  class?: string[];
  rows?: ItemRow[];
  cards?: string[];
}

/** One file's items. A file that is missing or does not parse has `parsed: false` and no items. */
export interface FileItems {
  file: string;
  parsed: boolean;
  /** The digest of the file's bytes, empty when the file is missing. */
  raw: string;
  items: Item[];
}

/** Each file's items, in order, from one run of the scanner. Paths are relative to `root`, the repository
 * root unless another tree is named; class attribution reads every file of the batch, so a caller passes
 * every file at once. */
export function scanItems(files: string[], root: string = abs(".")): FileItems[] {
  if (files.length === 0) return [];
  const r = Bun.spawnSync([helper(), "--items", root], { stdin: Buffer.from(files.join("\n") + "\n"), stdout: "pipe", stderr: "pipe" });
  if (r.exitCode !== 0) throw new Error(`nv-scan --items failed:\n${r.stderr.toString()}`);
  const lines = r.stdout.toString().split("\n").filter((l) => l !== "");
  if (lines.length !== files.length) throw new Error(`nv-scan --items answered ${lines.length} of ${files.length} file(s)`);
  return lines.map((l) => JSON.parse(l) as FileItems);
}

/** A hex blake2b digest of `chunks`, each followed by a separator so no two splits collide. */
export function digest(...chunks: (string | Uint8Array)[]): string {
  const h = new Bun.CryptoHasher("blake2b256");
  for (const c of chunks) {
    h.update(c);
    h.update("\0");
  }
  return h.digest("hex").slice(0, 32);
}
