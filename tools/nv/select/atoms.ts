// The atoms: the smallest things that run on their own, and what each one's definition is.
//
// | kind | one atom is | its definition (`def:`) |
// |---|---|---|
// | `case` | one `.nvst` file of a case tree under `tests/` | the file's bytes |
// | `proof` | one example or attack program of the feature roster | the program, its `.out` and `.in`, and the `nvs.toml` beside it |
// | `test` | one Rust test executable of the `covws` build, `<package> <kind> <target>` | nothing: its sources are items |
// | `nv` | one `bun nv` command check of the live plan | its argument list and working directory |
// | `nvtest` | one `bun test` file of the tools, `tools/nv/**/*.test.ts` | the file's bytes |
// | `check`, `heavy` | another plan check, and one of the heavy set | the check as the plan writes it (`select/checks.ts`) |
//
// A definition's digest is computed, never observed. When it changes, the atom runs, and its footprint
// starts again from that run.

import { existsSync, readdirSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import type { TestExe } from "../driver/accept.ts";
import { digest } from "../keys/scan.ts";
import { abs, ROOT } from "../lib/paths.ts";
import { examplesDir, hostileDir, namesIn, roster } from "../proofs/roster.ts";

/** The case trees: every directory under `tests/` whose `.nvst` files `nvs test` runs. */
export const CASE_ROOT = "tests";

export interface Atom {
  id: string;
  def: string;
}

/** Every `.nvst` file under `tests/`, repo-relative and sorted. */
export function caseFiles(root: string = ROOT): string[] {
  const out: string[] = [];
  const walk = (dir: string) => {
    let entries: import("node:fs").Dirent[];
    try {
      entries = readdirSync(join(root, dir), { withFileTypes: true });
    } catch {
      return;
    }
    for (const e of entries) {
      const p = `${dir}/${e.name}`;
      if (e.isDirectory()) walk(p);
      else if (e.name.endsWith(".nvst")) out.push(p);
    }
  };
  walk(CASE_ROOT);
  return out.sort();
}

/** The tree a case belongs to: `tests/<tree>`, which `nvs test` is pointed at. */
export const treeOf = (path: string) => path.split("/").slice(0, 2).join("/");

export const caseId = (path: string) => `case:${path}`;
export const proofId = (path: string) => `proof:${path}`;
export const testId = (pkg: string, t: TestExe) => `test:${pkg} ${t.kind} ${t.target}`;
export const nvId = (checkId: string) => `nv:${checkId}`;
export const nvTestId = (path: string) => `nvtest:${path}`;

/** A file's bytes as a definition: "" when it cannot be read. */
export function fileDef(path: string, root: string = ROOT): string {
  try {
    return digest(readFileSync(join(root, path)));
  } catch {
    return "";
  }
}

/** A case's definition. */
export const caseDef = fileDef;

/** The tools' `bun test` files: every `*.test.ts` under `tools/nv`, repo-relative and sorted. */
export function nvTestFiles(root: string = ROOT): string[] {
  const out: string[] = [];
  const walk = (dir: string) => {
    let entries: import("node:fs").Dirent[];
    try {
      entries = readdirSync(join(root, dir), { withFileTypes: true });
    } catch {
      return;
    }
    for (const e of entries) {
      const p = `${dir}/${e.name}`;
      if (e.isDirectory()) {
        if (e.name !== "node_modules") walk(p);
      } else if (e.name.endsWith(".test.ts")) out.push(p);
    }
  };
  walk("tools/nv");
  return out.sort();
}

const sibling = (proof: string, suffix: string) => proof.replace(/\.nvs$/, suffix);

/** A proof program's definition: what `bun nv proofs` remembers a green verdict against. */
export function proofDef(path: string, root: string = ROOT): string {
  const chunks: (string | Uint8Array)[] = [];
  for (const p of [path, sibling(path, ".out"), sibling(path, ".in"), `${dirname(path)}/nvs.toml`]) {
    const full = join(root, p);
    chunks.push(existsSync(full) ? readFileSync(full) : "");
  }
  return digest(...chunks);
}

/** A `bun nv` check's definition. */
export function nvDef(argv: string[], cwd: string): string {
  return digest(JSON.stringify(argv), cwd);
}

/** Every proof program of the roster `nvs` answers, sorted: the `.nvs` files directly in each feature's
 * example and attack directories. */
export async function proofPrograms(nvs: string): Promise<{ what: "examples" | "hostile"; path: string }[]> {
  const entries = await roster(nvs);
  const out = new Map<string, "examples" | "hostile">();
  for (const e of entries) {
    for (const [what, dir] of [
      ["examples", examplesDir(e)],
      ["hostile", hostileDir(e)],
    ] as const) {
      for (const n of namesIn(dir, ".nvs")) out.set(`${dir}/${n}`, what);
    }
  }
  return [...out].map(([path, what]) => ({ what, path })).sort((a, b) => (a.path < b.path ? -1 : 1));
}

/** The kinds whose atom is named by a file of the tree. */
const FILE_KINDS = ["case:", "proof:", "nvtest:"];

/** Whether a known atom's definition is still on disk: a case, proof or test file that is gone is no atom. */
export function stillThere(id: string): boolean {
  if (FILE_KINDS.some((k) => id.startsWith(k))) return existsSync(abs(id.slice(id.indexOf(":") + 1)));
  return true;
}

/** The current definition of an atom named by a file; null for a kind whose definition is not a file. */
export function currentDef(id: string): string | null {
  if (id.startsWith("case:")) return caseDef(id.slice(5));
  if (id.startsWith("proof:")) return proofDef(id.slice(6));
  if (id.startsWith("nvtest:")) return fileDef(id.slice(7));
  return null;
}
