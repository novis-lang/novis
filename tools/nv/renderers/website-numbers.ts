// The counts the website prints: `website/src/data/numbers.json`, which the front page's numbers and
// the page about how Novis is written read. Each count is taken from the tree, and this file is the
// one home of what each one counts:
//
// - `attacks`: the cases under `tests/hostile/`, each a file named `NN-slug.nvs`. A file in a
//   case's own directory under another name is a script the case runs, and is not counted.
// - `languageTests`: the `.nvst` cases under `tests/conformance/`, one case per file.
// - `rustTests`: the lines in `crates/` that open with a `#[test]` or `#[tokio::test]` attribute.
//   `cargo test -- --list` lists more, because it adds doc-tests and the tests a macro writes.
// - `examples`: the examples under `docs/examples/`, each a file named `NN-slug.nvs`.
// - `benchmarks`: the `.nvs` programs under `benches/members/`, the calibration pair aside.
// - `editorTests`: the `.lspt` cases under `tests/lsp/`.
// - `crates` and `cratesWithoutUnsafe`: the crates under `crates/`, and those not on `bun nv lints`'s
//   `UNSAFE_CRATES`, where the workspace's `unsafe_code = "forbid"` applies.
//
// A count is written rounded down to its first two digits (1,422 is written 1,400), so the site
// says "at least" and stays true while tests are added. That is also why the file has its own
// `--check`: a file on disk passes while every figure in it is still at most what the tree has
// and the two crate counts are exact, so a new test does not make it stale and a deleted one can.
// `bun nv render` without `--check` always writes the current figures.

import { existsSync, readdirSync, readFileSync } from "node:fs";
import { join, relative } from "node:path";
import { UNSAFE_CRATES } from "../cmd/lints.ts";
import { MARKER, type Output, type Renderer } from "../lib/render.ts";

export const WEBSITE_NUMBERS = "website/src/data/numbers.json";

/** The counts written rounded down. */
const AT_LEAST = ["attacks", "languageTests", "rustTests", "examples", "benchmarks", "editorTests"] as const;
/** The counts written as they are. */
const EXACT = ["crates", "cratesWithoutUnsafe"] as const;

export type Counts = Record<(typeof AT_LEAST)[number] | (typeof EXACT)[number], number>;

const CASE_RE = /(?:^|\/)\d{2}-[^/]+\.nvs$/;
const RUST_TEST_RE = /^\s*#\[(?:tokio::)?test\b/gm;

/** Every file under `dir` of `root`, repo-relative with forward slashes, or none when it is absent. */
function filesUnder(root: string, dir: string): string[] {
  const full = join(root, dir);
  if (!existsSync(full)) return [];
  return readdirSync(full, { recursive: true, withFileTypes: true })
    .filter((e) => e.isFile())
    .map((e) => relative(root, join(e.parentPath, e.name)).replaceAll("\\", "/"));
}

/** The tree's counts, exactly. */
export function countTree(root: string): Counts {
  const crates = readdirSync(join(root, "crates"), { withFileTypes: true })
    .filter((e) => e.isDirectory() && existsSync(join(root, "crates", e.name, "Cargo.toml")))
    .map((e) => `crates/${e.name}`);
  const rust = filesUnder(root, "crates").filter((p) => p.endsWith(".rs"));
  return {
    attacks: filesUnder(root, "tests/hostile").filter((p) => CASE_RE.test(p)).length,
    languageTests: filesUnder(root, "tests/conformance").filter((p) => p.endsWith(".nvst")).length,
    rustTests: rust.reduce((n, p) => n + (readFileSync(join(root, p), "utf8").match(RUST_TEST_RE)?.length ?? 0), 0),
    examples: filesUnder(root, "docs/examples").filter((p) => CASE_RE.test(p)).length,
    benchmarks: filesUnder(root, "benches/members").filter((p) => p.endsWith(".nvs") && !p.startsWith("benches/members/_calibration/")).length,
    editorTests: filesUnder(root, "tests/lsp").filter((p) => p.endsWith(".lspt")).length,
    crates: crates.length,
    cratesWithoutUnsafe: crates.filter((c) => !(c in UNSAFE_CRATES)).length,
  };
}

/** `n` rounded down to its first two digits: 1422 is 1400, 384 is 380, and 75 stays 75. */
export function roundDown(n: number): number {
  const step = 10 ** Math.max(0, String(Math.floor(n)).length - 2);
  return Math.floor(n / step) * step;
}

/** Whether `onDisk` is a numbers file that is still true of `exact`: the same fields, every rounded
 * one at most its count, and the exact ones equal. */
export function stillTrue(onDisk: string, exact: Counts): boolean {
  let old: Record<string, unknown>;
  try {
    old = JSON.parse(onDisk) as Record<string, unknown>;
  } catch {
    return false;
  }
  const keys = Object.keys(old).filter((k) => k !== "comment");
  if (old.comment !== MARKER || keys.length !== AT_LEAST.length + EXACT.length) return false;
  return (
    AT_LEAST.every((k) => typeof old[k] === "number" && (old[k] as number) <= exact[k]) &&
    EXACT.every((k) => old[k] === exact[k])
  );
}

/** The numbers file for `exact`. */
export function renderWebsiteNumbers(exact: Counts): Output {
  const shown = { ...exact };
  for (const k of AT_LEAST) shown[k] = roundDown(exact[k]);
  return {
    path: WEBSITE_NUMBERS,
    text: `${JSON.stringify({ comment: MARKER, ...shown }, null, 2)}\n`,
    current: (onDisk) => stillTrue(onDisk, exact),
  };
}

export const websiteNumbers: Renderer = {
  name: "website-numbers",
  render: (root) => [renderWebsiteNumbers(countTree(root))],
};
