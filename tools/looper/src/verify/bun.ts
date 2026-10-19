// The bun observer: what one `bun test <file>` run imported and read, as `bun-preload.ts` records it,
// and the footprint of the test file read back from that record. It ports
// `crates/looper-select/src/bun.rs`.
//
// - The caller runs each test file in its own `bun test --preload <PRELOAD> <file>`, with `READS_ENV`
//   naming a record path no earlier run wrote. The preload is a file of the kit itself, so nothing is
//   written to put it in place.
// - The record is a `looper-reads 1` line, then one line per distinct path the run touched. Nothing in
//   it grows across runs.
// - A test file rests on what its record names, as `observed` attributes it. A run that wrote no
//   record, or one the kit cannot read, rests on the test file's package directory: the nearest one
//   above it holding a `package.json`, else the whole tree.

import { readFileSync } from "node:fs";
import { join } from "node:path";
import { type Footprint, footprintOf, observed, rel, type Tree, wholeTree } from "./select.ts";

/** The preload's path. */
export const PRELOAD = join(import.meta.dir, "bun-preload.ts");

/** The environment variable that names the record a run writes. */
export const READS_ENV = "LOOPER_READS";

/** The record's first line. */
const HEADER = "looper-reads 1";

/** What the test file at the root-relative `test` rests on, from the record its run wrote at `record`. */
export function footprint(tree: Tree, test: string, record: string): Footprint {
  let text: string;
  try {
    text = readFileSync(record, "utf8");
  } catch {
    return packageDir(tree, test);
  }
  if (!text.startsWith(HEADER)) return packageDir(tree, test);
  const files = [join(tree.root, test)];
  const dirs: string[] = [];
  for (const line of text.slice(HEADER.length).split(/\r?\n/)) {
    const kind = line.slice(0, 2);
    const path = line.slice(2);
    if (kind === "m " || kind === "r ") files.push(path);
    else if (kind === "d ") dirs.push(path);
  }
  return observed(tree, files, dirs);
}

/** The footprint of a test file whose reads were not recorded: its package's directory. */
function packageDir(tree: Tree, test: string): Footprint {
  let dir = rel(test);
  for (let at = dir.lastIndexOf("/"); at >= 0; at = dir.lastIndexOf("/")) {
    dir = dir.slice(0, at);
    if (tree.contains(`${dir}/package.json`)) return footprintOf([], [dir]);
  }
  return wholeTree();
}
