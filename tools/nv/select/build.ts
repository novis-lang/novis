// What the `covws` build's build scripts read and write, as cargo recorded it.
//
// A build script's inputs are the `cargo:rerun-if-changed` paths in its `output` file under
// `target/covws/<triple>/debug/build/<package>-<hash>/`, never a list kept here. What it produces reaches
// the code two ways: a generated file under `out/` that an item includes with
// `include!(concat!(env!("OUT_DIR"), "/<name>"))`, and a `cargo:rustc-env` variable an item reads with
// `env!`. An included file's bytes are recorded per platform in the store's `meta` when a run is
// recorded, so a later change to an input moves exactly the including item when the file it generated
// changed, and nothing when it did not. A build that has not run since an input changed cannot say, and
// then the including item moves.
//
// A cargo package can hold several `<package>-<hash>` directories from older builds; the one whose
// `output` was written last is the build's.

import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";
import type { FileItems } from "../keys/scan.ts";
import { digest } from "../keys/scan.ts";
import { abs } from "../lib/paths.ts";
import { repoPath } from "./keys.ts";

export interface BuildScript {
  pkg: string;
  /** The `<package>-<hash>` directory, absolute. */
  dir: string;
  /** Repo-relative paths the script said it reads: files, and directories standing for what is beneath. */
  inputs: string[];
  /** The variables it set with `cargo:rustc-env`. */
  envs: string[];
  /** When `output` was last written, in milliseconds. */
  ranAt: number;
}

/** Each package's build script, from the build directory `dir` (`target/covws/<triple>/debug/build`).
 * `pkgDirs` names each package's repo-relative directory, which a relative input is written against;
 * the script's own `build.rs` is always an input. */
export function buildScripts(dir: string, pkgDirs: Map<string, string> = new Map()): Map<string, BuildScript> {
  const out = new Map<string, BuildScript>();
  if (!existsSync(dir)) return out;
  for (const name of readdirSync(dir)) {
    const m = /^(.*)-[0-9a-f]{16}$/.exec(name);
    const output = join(dir, name, "output");
    if (!m || !existsSync(output)) continue;
    const pkg = m[1]!;
    const ranAt = statSync(output).mtimeMs;
    if ((out.get(pkg)?.ranAt ?? -1) >= ranAt) continue;
    const pkgDir = pkgDirs.get(pkg) ?? "";
    const parsed = parseOutput(readFileSync(output, "utf8"), pkgDir);
    if (pkgDir) parsed.inputs = [...new Set([...parsed.inputs, `${pkgDir}/build.rs`])].sort();
    out.set(pkg, { pkg, dir: join(dir, name), ...parsed, ranAt });
  }
  return out;
}

/** The inputs and variables in one `output` file. A relative input is relative to the package's own
 * directory `pkgDir`, which is where cargo runs the script; a path outside the repository is no input. */
export function parseOutput(text: string, pkgDir = ""): { inputs: string[]; envs: string[] } {
  const inputs = new Set<string>();
  const envs = new Set<string>();
  for (const raw of text.split("\n")) {
    const line = raw.replace(/\r$/, "").replace(/^cargo::/, "cargo:");
    if (line.startsWith("cargo:rerun-if-changed=")) {
      const value = line.slice("cargo:rerun-if-changed=".length);
      const absolute = /^[A-Za-z]:[\\/]|^[\\/]/.test(value);
      const p = absolute ? repoPath(value) : pkgDir ? repoPath(`${pkgDir}/${value}`) : null;
      if (p !== null && p !== ".") inputs.add(p);
    } else if (line.startsWith("cargo:rustc-env=")) {
      const kv = line.slice("cargo:rustc-env=".length);
      envs.add(kv.slice(0, kv.indexOf("=")));
    }
  }
  return { inputs: [...inputs].sort(), envs: [...envs].sort() };
}

/** Whether `path` is one of `inputs` or beneath one. */
export function isInput(path: string, inputs: string[]): boolean {
  return inputs.some((i) => path === i || path.startsWith(`${i}/`));
}

/** An item that includes a generated file. */
export interface Generated {
  pkg: string;
  /** The generated file's name under `out/`. */
  name: string;
  /** The including item. */
  file: string;
  id: string;
}

const INCLUDE = /include!\s*\(\s*concat!\s*\(\s*env!\s*\(\s*"OUT_DIR"\s*\)\s*,\s*"\/?([^"]+)"/;

/** Every item of `files` that includes a generated file, found from the item's own lines. `pkgOf`
 * names a file's package. */
export function generatedIncludes(files: Iterable<FileItems>, pkgOf: (file: string) => string | null, root = abs(".")): Generated[] {
  const out: Generated[] = [];
  for (const f of files) {
    const candidates = f.items.filter((i) => i.kind === "macro" && i.refs.includes("include") && i.refs.includes("env"));
    if (candidates.length === 0) continue;
    let lines: string[];
    try {
      lines = readFileSync(join(root, f.file), "utf8").split("\n");
    } catch {
      continue;
    }
    for (const item of candidates) {
      const m = INCLUDE.exec(lines.slice(item.start - 1, item.end).join("\n"));
      const pkg = pkgOf(f.file);
      if (m && pkg) out.push({ pkg, name: m[1]!, file: f.file, id: item.id });
    }
  }
  return out;
}

/** The items of `files` whose own lines read one of `envs` with `env!`. */
export function envReaders(files: Iterable<FileItems>, envs: string[], root = abs(".")): { file: string; id: string }[] {
  if (envs.length === 0) return [];
  const pattern = new RegExp(`env!\\s*\\(\\s*"(${envs.map((e) => e.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")).join("|")})"`);
  const out: { file: string; id: string }[] = [];
  for (const f of files) {
    const candidates = f.items.filter((i) => i.refs.includes("env"));
    if (candidates.length === 0) continue;
    let lines: string[];
    try {
      lines = readFileSync(join(root, f.file), "utf8").split("\n");
    } catch {
      continue;
    }
    for (const item of candidates) if (pattern.test(lines.slice(item.start - 1, item.end).join("\n"))) out.push({ file: f.file, id: item.id });
  }
  return out;
}

/** The digest of a generated file's bytes, or `""` when the build has none. */
export function generatedDigest(script: BuildScript | undefined, name: string): string {
  if (!script) return "";
  try {
    return digest(readFileSync(join(script.dir, "out", name)));
  } catch {
    return "";
  }
}

/** The meta key a generated file's recorded digest is kept under. */
export const generatedMeta = (platform: string, g: { pkg: string; name: string }) => `gen:${platform}:${g.pkg}/${g.name}`;
