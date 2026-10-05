// The atoms: the smallest things that run on their own, and what each one's definition is.
//
// | kind | one atom is | its definition (`def:`) |
// |---|---|---|
// | `case` | one `.nvst` file of a case tree under `tests/` | the file's bytes |
// | `proof` | one example or attack program of the feature roster | the program, its `.out`, `.in`, `.nvsr`, `.nvsp` and `.nvse`, and the `nvs.toml` beside it |
// | `test` | one Rust test executable of the `covws` build, `<package> <kind> <target>` | nothing: its sources are items |
// | `nv` | one `bun nv` command check of the live plan | its argument list and working directory |
// | `nvtest` | one `bun test` file of the tools, `tools/nv/**/*.test.ts` | the file's bytes |
// | `bench` | one bench program under `benches/members/` (`benchFiles`) | a proof program's files, and its `.scale.nvs` and `.twin.nvs` siblings |
// | `check`, `heavy` | another plan check, and one of the heavy set | the check as the plan writes it (`select/checks.ts`) |
//
// A definition's digest is computed, never observed. When it changes, the atom runs, and its footprint
// starts again from that run. A bench's footprint is taken at its smallest batch (`recordBench` in
// `proofs/select.ts`), because which code a bench reaches does not depend on how many times it runs.

import { existsSync, readdirSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import type { TestExe } from "../driver/accept.ts";
import { digest } from "../keys/scan.ts";
import { abs, ROOT } from "../lib/paths.ts";
import { EXAMPLES, examplesDir, HOSTILE, hostileDir, namesIn, roster } from "../proofs/roster.ts";

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
export const benchId = (path: string) => `bench:${path}`;

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

/** A proof program's definition: what `bun nv proofs` remembers a green verdict against. The `.nvsr`
 * request file, the `.nvsp` peer file and the `.nvse` events file join only where they exist, so a
 * proof without one keeps the definition it had before those files were read. */
export function proofDef(path: string, root: string = ROOT): string {
  const chunks: (string | Uint8Array)[] = [];
  for (const p of [path, sibling(path, ".out"), sibling(path, ".in"), `${dirname(path)}/nvs.toml`]) {
    const full = join(root, p);
    chunks.push(existsSync(full) ? readFileSync(full) : "");
  }
  const request = join(root, sibling(path, ".nvsr"));
  if (existsSync(request)) chunks.push(readFileSync(request));
  const peer = join(root, sibling(path, ".nvsp"));
  if (existsSync(peer)) chunks.push("peer", readFileSync(peer));
  const events = join(root, sibling(path, ".nvse"));
  if (existsSync(events)) chunks.push("events", readFileSync(events));
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

/**
 * Every proof program on disk, sorted, found by listing directories and without the roster: each `.nvs`
 * file under the example and attack trees whose directory has no ancestor, from the tree's area
 * (`docs/examples/<area>`, `tests/hostile/<area>`) down, that holds a `.nvs` file itself. A feature's programs
 * sit directly in its directory, and a file in a directory below them is a helper one of them loads, so
 * this is the set `proofPrograms` reads from the roster, a feature the roster does not know yet included.
 */
export function proofFiles(root: string = ROOT): string[] {
  const out: string[] = [];
  const walk = (dir: string, depth: number, under: boolean) => {
    let entries: import("node:fs").Dirent[];
    try {
      entries = readdirSync(join(root, dir), { withFileTypes: true });
    } catch {
      return;
    }
    const here = entries.filter((e) => e.isFile() && e.name.endsWith(".nvs")).map((e) => `${dir}/${e.name}`);
    if (!under) out.push(...here);
    // `docs/examples` and `tests/hostile` are depth 2. From an area (depth 3) down, a directory that holds
    // a program makes every `.nvs` file below it a helper.
    const holds = depth >= 3 && here.length > 0;
    for (const e of entries) if (e.isDirectory()) walk(`${dir}/${e.name}`, depth + 1, under || holds);
  };
  for (const tree of [EXAMPLES, HOSTILE]) walk(tree, 2, false);
  return out.sort();
}

/** The bench tree: one program per measured feature. */
export const BENCH_ROOT = "benches/members";
/** The folder of the calibration programs, which every record is measured against and which are no bench. */
const CALIBRATION = "_calibration";
/** The siblings a bench program keeps beside it that are no bench of their own: the input ramp and the
 * same operation written another way. */
const BENCH_SIBLINGS = [".scale.nvs", ".twin.nvs"];

/**
 * Every bench program under `BENCH_ROOT`, repo-relative and sorted. A `.scale.nvs` or `.twin.nvs`
 * sibling belongs to its bench, the calibration folder holds none, and a folder `<name>/` beside a
 * program `<name>.nvs` holds the files that program loads.
 */
export function benchFiles(root: string = ROOT): string[] {
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
        if (e.name !== CALIBRATION && !existsSync(join(root, `${p}.nvs`))) walk(p);
      } else if (e.name.endsWith(".nvs") && !BENCH_SIBLINGS.some((s) => e.name.endsWith(s))) out.push(p);
    }
  };
  walk(BENCH_ROOT);
  return out.sort();
}

/** A bench's definition: its program's files as a proof program's (`proofDef`), and the siblings its
 * ramp and its twin run. */
export function benchDef(path: string, root: string = ROOT): string {
  const chunks: (string | Uint8Array)[] = [proofDef(path, root)];
  for (const suffix of BENCH_SIBLINGS) {
    const full = join(root, sibling(path, suffix));
    if (existsSync(full)) chunks.push(suffix, readFileSync(full));
  }
  return digest(...chunks);
}

/** The bench whose definition `path` is part of, or null for a path that is none. */
export function benchOf(path: string): string | null {
  if (!path.startsWith(`${BENCH_ROOT}/`)) return null;
  const sib = BENCH_SIBLINGS.find((s) => path.endsWith(s));
  if (sib) return path.slice(0, -sib.length) + ".nvs";
  if (/\.(nvs|out|in|nvsr|nvsp|nvse)$/.test(path)) return path.replace(/\.[^./]+$/, ".nvs");
  return null;
}

/** The kinds whose atom is named by a file of the tree. */
const FILE_KINDS = ["case:", "proof:", "nvtest:", "bench:"];

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
  if (id.startsWith("bench:")) return benchDef(id.slice(6));
  return null;
}
