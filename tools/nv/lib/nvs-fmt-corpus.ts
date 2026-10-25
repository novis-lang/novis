// The `.nvs` files `nvs fmt` holds to its layout, read from `tools/data/nvs-fmt-corpus.txt`, which says
// what the corpus is and who else reads it. `crates/nvs-fmt/tests/common/mod.rs` reads the same file.

import { readFileSync, statSync } from "node:fs";
import { join } from "node:path";
import { ROOT } from "./paths.ts";
import { run } from "./proc.ts";

export const CORPUS_FILE = "tools/data/nvs-fmt-corpus.txt";

export interface Corpus {
  /** Each `tree` directory, repo-relative, with no trailing `/`. */
  trees: string[];
  /** Each `skip` directory or file, repo-relative, with no trailing `/`. */
  skips: string[];
}

/** The corpus `text` declares, in the format `CORPUS_FILE` documents. Throws on a line it cannot read. */
export function parseCorpus(text: string): Corpus {
  const corpus: Corpus = { trees: [], skips: [] };
  for (const [i, raw] of text.split(/\r?\n/).entries()) {
    const line = raw.trim();
    if (line === "" || line.startsWith("#")) continue;
    const m = /^(tree|skip)\s+(\S+?)\/?(?:\s+--\s+\S.*)?$/.exec(line);
    if (!m) throw new Error(`${CORPUS_FILE}:${i + 1}: expected \`tree <dir>\` or \`skip <dir>  --  <why>\`, got \`${line}\``);
    if (m[1] === "skip" && !line.includes(" -- ")) throw new Error(`${CORPUS_FILE}:${i + 1}: a skip says why after \` -- \``);
    (m[1] === "tree" ? corpus.trees : corpus.skips).push(m[2]!);
  }
  return corpus;
}

/** The corpus as the tree's `CORPUS_FILE` declares it. */
export function readCorpus(root: string = ROOT): Corpus {
  return parseCorpus(readFileSync(join(root, CORPUS_FILE), "utf8"));
}

const under = (rel: string, dir: string) => rel.startsWith(`${dir}/`);

/** Whether the repo-relative `rel` is a file the formatter holds: under a tree, and neither a skip nor
 * under one. */
export function held(corpus: Corpus, rel: string): boolean {
  return corpus.trees.some((t) => under(rel, t)) && !corpus.skips.some((s) => rel === s || under(rel, s));
}

/** Whether the repo-relative `rel` is under no tree at all, which `bun nv nvs-fmt --check` fails on. */
export function outside(corpus: Corpus, rel: string): boolean {
  return !corpus.trees.some((t) => under(rel, t));
}

/** Every `.nvs` file git tracks or would track, repo-relative and sorted, with the ones that exist on disk. */
export async function nvsFiles(root: string = ROOT): Promise<string[]> {
  const p = await run(["git", "ls-files", "-z", "--cached", "--others", "--exclude-standard", "--", "*.nvs"], { cwd: root });
  if (p.code !== 0) throw new Error(`git ls-files exited ${p.code}: ${p.stderr.trim()}`);
  const seen = new Set<string>();
  for (const rel of p.stdout.split("\0")) {
    if (!rel.endsWith(".nvs") || seen.has(rel)) continue;
    try {
      if (statSync(join(root, rel)).isFile()) seen.add(rel);
    } catch {
      // A tracked file deleted from the working tree has nothing to format.
    }
  }
  return [...seen].sort();
}
