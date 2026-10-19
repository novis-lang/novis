// The cargo observer: the test binaries one `cargo test --no-run --message-format=json` built, and the
// footprint of each, read from the dep-info files rustc wrote beside what it built. It ports
// `crates/looper-select/src/cargo.rs`.
//
// - rustc's dep-info names the sources of the one crate it compiled, never its dependencies. So a test
//   binary rests on its own dep-info's sources and on the library dep-info's sources of every local
//   package it depends on, transitively, as `cargo metadata` resolves them.
// - It rests on its package's directory too, which catches a fixture it reads at run time, and on the
//   directory of every local dependency with a build script, because a build script reads files that
//   no dep-info names.
// - It rests on the directory of each bin target's `src_path` in every local dependency, as `cargo
//   metadata` names it. A test may build and run such a binary at run time, and a dependent's build
//   never compiles it, so no dep-info names its sources. The directory is `src/bin/` or `src/`, never
//   the whole package, so a dependency's own tests do not reach its dependents.
// - A dep-info path is relative to the workspace root cargo ran in, or absolute. A source outside the
//   project's root, such as a registry crate, is dropped, because the lock file is a global file. So is
//   a build output under an ignored `target/`.
// - A binary whose dep-info, or a local dependency's, cannot be read, or whose package lies outside the
//   root, rests on the whole tree.

import { readFileSync } from "node:fs";
import { basename, dirname, extname, join, resolve } from "node:path";
import { type Footprint, observed, type Tree, wholeTree } from "./select.ts";

/** One test binary cargo built. */
export interface CargoTest {
  /** The package's id as cargo names it, which `cargo metadata` names it by too. */
  packageId: string;
  /** The package's name. */
  package: string;
  /** The target's kind: `lib`, `bin`, `test`, `bench` or `example`. */
  kind: string;
  /** The target's name. */
  name: string;
  executable: string;
  /** The dep-info file rustc wrote beside the executable. */
  depInfo: string;
  /** The directory of the package's `Cargo.toml`. */
  packageDir: string;
}

/** The unit's id of a test binary, such as `cargo:looper-select/test/select`. */
export function testId(t: CargoTest): string {
  return `cargo:${t.package}/${t.kind}/${t.name}`;
}

/** A local package as a dependency. */
interface Package {
  dir: string;
  /** The dep-info of its library, of every kind cargo built it as. */
  depInfos: string[];
  buildScript: boolean;
  /** The directory of each of its bin targets' root source file. */
  binDirs: string[];
}

interface Message {
  reason?: string;
  package_id?: string;
  manifest_path?: string;
  target?: { kind?: string[]; name?: string };
  profile?: { test?: boolean };
  filenames?: string[];
  executable?: string | null;
}

interface Metadata {
  packages?: { id: string; source?: string | null; targets?: { kind: string[]; src_path: string }[] }[];
  resolve?: { nodes?: { id: string; dependencies?: string[] }[] } | null;
}

const LIB_KINDS = new Set(["lib", "rlib", "dylib", "proc-macro", "custom-build"]);

/** What one build reported: its test binaries, and for every local package what a dependent rests on. */
export class Build {
  readonly tests: CargoTest[] = [];
  private readonly packages = new Map<string, Package>();
  /** Each package's direct dependencies of every kind, from `cargo metadata`. */
  private readonly graph = new Map<string, string[]>();

  private pkg(id: string): Package {
    let p = this.packages.get(id);
    if (!p) {
      p = { dir: "", depInfos: [], buildScript: false, binDirs: [] };
      this.packages.set(id, p);
    }
    return p;
  }

  /**
   * Reads cargo's JSON `messages` from the build and the output of `cargo metadata --format-version 1`
   * in the same workspace. A message line that does not parse is skipped, and so is an artifact that
   * is neither a test binary nor a local library or build script.
   */
  static parse(messages: string, metadata: string): Build {
    let meta: Metadata;
    try {
      meta = JSON.parse(metadata) as Metadata;
    } catch (e) {
      throw new Error(`\`cargo metadata\` printed what the kit cannot read: ${(e as Error).message}`);
    }
    const build = new Build();
    for (const node of meta.resolve?.nodes ?? []) build.graph.set(node.id, node.dependencies ?? []);
    const local = new Set<string>();
    for (const p of meta.packages ?? []) {
      if (p.source !== null && p.source !== undefined) continue;
      for (const t of p.targets ?? []) {
        if (!t.kind.includes("bin")) continue;
        const dir = dirname(t.src_path);
        const binDirs = build.pkg(p.id).binDirs;
        if (!binDirs.includes(dir)) binDirs.push(dir);
      }
      local.add(p.id);
    }
    for (const line of messages.split(/\r?\n/)) {
      let m: Message;
      try {
        m = JSON.parse(line) as Message;
      } catch {
        continue;
      }
      const { package_id: id, manifest_path: manifest, target } = m;
      if (!id || !manifest || !target || m.reason !== "compiler-artifact") continue;
      const dir = dirname(manifest);
      const kind = target.kind?.[0] ?? "";
      const name = target.name ?? "";
      if (m.profile?.test) {
        if (!m.executable) continue;
        build.tests.push({
          package: packageName(id),
          packageId: id,
          depInfo: depInfoOf(name, m.executable),
          kind,
          name,
          executable: m.executable,
          packageDir: dir,
        });
      } else if (local.has(id) && LIB_KINDS.has(kind)) {
        const p = build.pkg(id);
        p.dir = dir;
        const file = m.filenames?.[0];
        if (kind === "custom-build") p.buildScript = true;
        else if (file) p.depInfos.push(depInfoOf(name, file));
      }
    }
    return build;
  }

  /** What `test` rests on, with relative dep-info paths taken from `workspace`. */
  footprint(test: CargoTest, tree: Tree, workspace: string): Footprint {
    if (tree.relative(test.packageDir) === undefined) return wholeTree();
    const depInfos = [test.depInfo];
    const dirs = [test.packageDir];
    const seen = new Set([test.packageId]);
    const queue = [test.packageId];
    for (let id = queue.pop(); id !== undefined; id = queue.pop()) {
      const p = this.packages.get(id);
      if (p) {
        depInfos.push(...p.depInfos);
        if (p.buildScript) dirs.push(p.dir);
        dirs.push(...p.binDirs);
      }
      for (const dep of this.graph.get(id) ?? []) {
        if (!seen.has(dep)) {
          seen.add(dep);
          queue.push(dep);
        }
      }
    }
    const sources: string[] = [];
    for (const path of depInfos) {
      let rules: string;
      try {
        rules = readFileSync(path, "utf8");
      } catch {
        return wholeTree();
      }
      for (const s of depInfoSources(rules)) sources.push(resolve(workspace, s));
    }
    return observed(tree, sources, dirs);
  }
}

/** The diagnostics in cargo's JSON `messages`, each as rustc rendered it for a terminal, in order. */
export function diagnostics(messages: string): string[] {
  const out: string[] = [];
  for (const line of messages.split(/\r?\n/)) {
    try {
      const m = JSON.parse(line) as { reason?: string; message?: { rendered?: string | null } };
      if (m.reason === "compiler-message" && typeof m.message?.rendered === "string") out.push(m.message.rendered);
    } catch {}
  }
  return out;
}

/**
 * The dep-info file rustc wrote for the target `name` beside `file`, one of the files it built. Every
 * file of one compile shares a `-<hash>` suffix, and the dep-info is `<crate>-<hash>.d`.
 */
export function depInfoOf(name: string, file: string): string {
  const stem = basename(file, extname(file));
  const at = stem.lastIndexOf("-");
  const hash = at < 0 ? "" : stem.slice(at + 1);
  return join(dirname(file), `${name.replaceAll("-", "_")}-${hash}.d`);
}

/** The package's name in a package id: `path+file:///…/name#1.0.0`, `…#name@1.0.0`, or the older `name 1.0.0 (source)`. */
export function packageName(id: string): string {
  const hash = id.indexOf("#");
  if (hash < 0) return id.split(/\s+/)[0] || id;
  const source = id.slice(0, hash);
  const fragment = id.slice(hash + 1);
  const at = fragment.indexOf("@");
  if (at >= 0) return fragment.slice(0, at);
  return source.slice(source.lastIndexOf("/") + 1);
}

/**
 * The prerequisites of every rule in a dep-info file, as written, sorted and each once. A `\ ` is an
 * escaped space, and any other backslash is a path separator. A `#` line, such as `# env-dep:`, is
 * skipped.
 */
export function depInfoSources(text: string): string[] {
  const out = new Set<string>();
  for (const line of text.split(/\r?\n/)) {
    if (line.startsWith("#")) continue;
    const at = ruleColon(line);
    if (at === undefined) continue;
    let path = "";
    const rest = line.slice(at + 1);
    for (let i = 0; i < rest.length; i++) {
      const c = rest[i] as string;
      if (c === "\\" && rest[i + 1] === " ") {
        path += " ";
        i++;
      } else if (c === " " || c === "\t") {
        if (path !== "") out.add(path);
        path = "";
      } else {
        path += c;
      }
    }
    if (path !== "") out.add(path);
  }
  return [...out].sort();
}

/** Where a rule's target ends: the first `:` followed by a space or the line's end, so a drive letter's colon is not taken for it. */
function ruleColon(line: string): number | undefined {
  for (let i = 0; i < line.length; i++) {
    if (line[i] === ":" && (i + 1 === line.length || line[i + 1] === " ")) return i;
  }
  return undefined;
}
