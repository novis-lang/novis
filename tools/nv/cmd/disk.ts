// `bun nv disk`: what this tree costs on disk, and the one command that reclaims it.
//
//     bun nv disk               what is on disk, what is reclaimable, what is free
//     bun nv disk --clean       sweep it
//     bun nv disk --clean -n    say what --clean would delete; delete nothing
//     bun nv disk --deep        also size the places outside this repository
//
// Nothing here runs on the session path. The loop driver runs `clean`, the whole of `--clean`, after
// every session's acceptance check, which is the one moment it knows nothing is building. A person runs
// the same `--clean` by hand, and it refuses while a driver holds `.loop/running`, because a person
// cannot see whether a session is mid-build.
//
// Cargo never garbage-collects `target/`. A crate's artifacts are named `<name>-<metadata-hash>`, and
// the hash covers the dependency graph, so every `Cargo.toml` or `Cargo.lock` edit mints a fresh
// generation for every crate downstream of it and orphans the previous one. `.loop/logs` and
// `.agent-tmp` only grow unless something deletes from them. An unattended run that fills the disk dies
// with the tree half-edited, so the driver checks free space before it starts (`MIN_FREE_GB`).
//
// Age alone never condemns anything in `deps/`. Cargo never rewrites an artifact it has decided is
// fresh, so a superseded generation and a live one can carry the same date. The live set is asked for
// instead: the cargo commands in `LIVE_QUERIES` over `target/debug`, which a person builds by hand, and
// in `COVWS_QUERIES` over `target/covws`, which the pipeline builds, each warm and with
// `--message-format=json`, name every file those graphs use. Age is only ever a reason to keep: nothing
// written in the last `GRACE_HOURS` is swept whatever the list says, which covers a build still under
// way and a `--test <name>` run that resolves a dev-dependency its own way. `incremental/` has no
// artifact list to ask, so there age and the newest `KEEP_INCREMENTAL` per crate are the whole rule,
// under `target/` and under `target/covws` alike. `release/deps` is never swept: its live set can only
// be asked for with a release build.
//
// `--clean` also deletes what nothing reads any more: a `default_*.profraw` an instrumented binary left
// at the root or in a package directory (`strayProfiles`), and the memo files the selection store
// replaced (`RETIRED_MEMOS`).
//
// Those cargo runs are nearly all of a sweep's time, so their answer is remembered in
// `.cache/nv-disk-live.json` under `liveKey`, a hash of everything that decides an artifact's name, and
// cargo is asked again only when that key changes. Editing source never changes it; a manifest, the
// lock file, a new build target, the toolchain or the build environment does. Deleting the file makes
// the next sweep ask.
//
// Nothing here can produce a wrong build. Cargo re-checks every fingerprint against the files on disk,
// so the worst a mistake costs is rebuilding something that was still wanted. Everything outside this
// repository is reported and never touched: it is other tools' state.

import { createHash } from "node:crypto";
import {
  existsSync,
  mkdirSync,
  readdirSync,
  readFileSync,
  renameSync,
  rmSync,
  statfsSync,
  statSync,
  unlinkSync,
  writeFileSync,
} from "node:fs";
import type { Dirent } from "node:fs";
import { readdir, stat as statAsync } from "node:fs/promises";
import { homedir } from "node:os";
import { dirname, join, resolve, sep } from "node:path";
import { covwsCargo } from "../lib/covws.ts";
import { CACHE, ROOT } from "../lib/paths.ts";
import { run as runProgram } from "../lib/proc.ts";
import { pyInt } from "../lib/py.ts";

export const summary = "what the tree costs on disk, and the sweep: nv disk [--clean [-n]] [--deep]";

const TARGET = join(ROOT, "target");
/** The cargo profile the feature proofs run on, built into `target/covws` (`tools/nv/proofs/run.ts`). */
const PROOF_PROFILE = "proof";
const LOGDIR = join(ROOT, ".loop", "logs");
const SCRATCH = join(ROOT, ".agent-tmp");
const RUNNING = join(ROOT, ".loop", "running");
/** The live set the last sweep asked cargo for, and the `liveKey` it was asked under. */
const LIVE_CACHE = join(CACHE, "nv-disk-live.json");

// The retention policy has exactly one home, and this is it.
/** `.loop/logs`: how many loop runs keep their session logs. */
export const KEEP_RUNS = 5;
/** `.agent-tmp`: older than this belongs to no session that is still running. */
export const SCRATCH_DAYS = 2;
/** `.agent-tmp/worktrees`: git worktrees, never swept (`pruneScratch`). */
export const WORKTREES = "worktrees";
/** `target/*\/incremental`: cache generations kept per crate. */
export const KEEP_INCREMENTAL = 2;
/** Below this, the loop driver will not start a run. */
export const MIN_FREE_GB = 10;
/** `target/`: nothing written more recently than this is swept, whatever cargo says. */
export const GRACE_HOURS = 2;

// `libnvs_stdlib-2a3f7eaa9f84477e.rlib` -> `libnvs_stdlib`. Cargo's metadata hash is 16 hex digits and
// always the last dash-separated component of the stem.
const HASHED = /^(.+)-[0-9a-f]{16}$/;

// Reported, never deleted. `{home}` is the user's home directory and `{tmp}` the Linux temp directory.
const ELSEWHERE: [string, string, string, string][] = [
  [
    "WSL Linux target",
    "{tmp}/nvs-linux",
    "the valgrind leg's own target dir, inside the ext4 vhdx",
    "wsl.exe -- rm -rf /var/tmp/nvs-linux    # frees ext4 space; see commands.md for the vhdx",
  ],
  [
    "WSL loop target",
    "{tmp}/nvs-target-wsl",
    "the loop's WSL leg target dir, beside it and just as large",
    "wsl.exe -- rm -rf /var/tmp/nvs-target-wsl    # the next leg pays a 32s cold build",
  ],
  [
    "WSL leg tree copies",
    "{tmp}/nvs-target-wsl-src",
    "the WSL leg's copy of each checkout, which it builds and runs from",
    "wsl.exe -- rm -rf /var/tmp/nvs-target-wsl-src    # the next leg copies the tree again and rebuilds the workspace crates",
  ],
  [
    "harness transcripts",
    "{home}/.claude/projects",
    "one JSONL per session, kept forever by the harness",
    "delete the oldest project directories by hand",
  ],
  [
    "cargo registry",
    "{home}/.cargo/registry",
    "downloaded crate sources and their extracted copies",
    "rm -rf ~/.cargo/registry/src    # cargo re-extracts on the next build",
  ],
];

const WINDOWS = process.platform === "win32";

// ------------------------------------------------------------------------------------------ measuring

/** Python's `format(x, ".Nf")`: a tie in the exact binary value goes to the even digit. */
function fixed(x: number, digits: number): string {
  const scaled = x * 10 ** digits;
  if (Number.isFinite(scaled) && scaled % 1 === 0.5) {
    const down = Math.floor(scaled);
    return ((down % 2 === 0 ? down : down + 1) / 10 ** digits).toFixed(digits);
  }
  return x.toFixed(digits);
}

export function human(n: number): string {
  for (const unit of ["B", "K", "M", "G", "T"]) {
    if (n < 1024 || unit === "T") return unit === "B" ? `${fixed(n, 0)}${unit}` : `${fixed(n, 1)}${unit}`;
    n /= 1024;
  }
  return `${fixed(n, 1)}T`;
}

/** `stat` that follows a link, or null when it cannot be asked. */
function stat(path: string): ReturnType<typeof statSync> | null {
  try {
    return statSync(path);
  } catch {
    return null;
  }
}

function isDir(path: string): boolean {
  return stat(path)?.isDirectory() ?? false;
}

function listDir(path: string): string[] {
  try {
    return readdirSync(path);
  } catch {
    return [];
  }
}

interface FileAge {
  path: string;
  size: number;
  mtime: number;
}

/** How many directories `walk` lists at the same time. */
const WALK_DIRS_AT_ONCE = 32;

/**
 * The total bytes under `root`, and every file's size and age, in no fixed order. A missing directory
 * is zeroes. A link to a directory is not followed, and a link to a file counts as the file, the way
 * `os.walk` reads a tree. A directory listing carries no sizes, so a walk costs one `stat` per file;
 * all of a directory's files are stat'ed together and `WALK_DIRS_AT_ONCE` directories are listed at a
 * time, because waiting for each `stat` in turn is nearly all of a walk's time.
 */
async function walk(root: string): Promise<{ total: number; files: FileAge[] }> {
  const files: FileAge[] = [];
  let total = 0;
  if (!isDir(root)) return { total, files };
  const pending = [root];
  const visit = async (dir: string): Promise<void> => {
    let entries: Dirent[];
    try {
      entries = await readdir(dir, { withFileTypes: true });
    } catch {
      return;
    }
    await Promise.all(
      entries.map(async (entry) => {
        const path = join(dir, entry.name);
        if (entry.isDirectory()) {
          pending.push(path);
          return;
        }
        let st;
        try {
          st = await statAsync(path);
        } catch {
          return;
        }
        if (st.isDirectory()) return;
        total += st.size;
        files.push({ path, size: st.size, mtime: st.mtimeMs / 1000 });
      }),
    );
  };
  await new Promise<void>((done) => {
    let busy = 0;
    const pump = (): void => {
      while (busy < WALK_DIRS_AT_ONCE && pending.length > 0) {
        busy += 1;
        void visit(pending.pop()!).then(() => {
          busy -= 1;
          pump();
        });
      }
      if (busy === 0) done();
    };
    pump();
  });
  return { total, files };
}

/** Free space, in GiB, on the volume holding `path`. 0 if it cannot be asked. */
export function freeGb(path: string = ROOT): number {
  try {
    const fs = statfsSync(path);
    return (Number(fs.bavail) * Number(fs.bsize)) / 1024 ** 3;
  } catch {
    return 0;
  }
}

function now(): number {
  return Date.now() / 1000;
}

function mtime(path: string): number {
  const st = stat(path);
  return st ? Number(st.mtimeMs) / 1000 : 0;
}

/**
 * The latest mtime of `path` and of every file under it; 0 if it cannot be read. A directory's own
 * mtime moves only when an entry directly inside it is added or removed, so a directory whose files
 * are still being rewritten reads as old as the day it was made.
 */
async function newest(path: string): Promise<number> {
  let latest = mtime(path);
  if (latest && isDir(path)) for (const f of (await walk(path)).files) latest = Math.max(latest, f.mtime);
  return latest;
}

/** Python's `Path.stem` and `Path.suffix` of a file name. */
function splitSuffix(name: string): [string, string] {
  const i = name.lastIndexOf(".");
  return i > 0 && i < name.length - 1 ? [name.slice(0, i), name.slice(i)] : [name, ""];
}

// ------------------------------------------------------------------------------------------- deleting

/**
 * Delete a file or a directory tree, and return the bytes freed, 0 on any failure. It never throws: a
 * sweep that aborts halfway because one file was locked has deleted some of what it meant to and
 * reported nothing.
 */
async function rm(path: string, dryRun: boolean): Promise<number> {
  try {
    if (isDir(path)) {
      const size = (await walk(path)).total;
      if (!dryRun) {
        try {
          rmSync(path, { recursive: true, force: true });
        } catch {
          // A locked file stays; what was freed around it still counts.
        }
      }
      return size;
    }
    const size = Number(statSync(path).size);
    if (!dryRun) unlinkSync(path);
    return size;
  } catch {
    return 0;
  }
}

/**
 * `20260825-114204-0001.log` -> `20260825-114204`, and anything else -> ''. The run stamp rather than
 * the session index, because the index restarts at 1 every run.
 */
function runIdOf(name: string): string {
  const parts = name.split("-");
  if (parts.length >= 3 && /^\d+$/.test(parts[0]!) && /^\d+$/.test(parts[1]!)) return `${parts[0]}-${parts[1]}`;
  return "";
}

/**
 * Keep the newest `keep` runs' logs under `.loop/logs` and delete the rest; returns the bytes freed.
 * Whole runs rather than the newest files, because a run is read as a unit and half a run measures
 * nothing.
 */
async function pruneLogs(keep: number = KEEP_RUNS, dryRun = false): Promise<number> {
  if (!isDir(LOGDIR)) return 0;
  const entries = listDir(LOGDIR).sort();
  const runs = [...new Set(entries.map(runIdOf))].filter((r) => r !== "").sort();
  const doomed = new Set(keep > 0 ? runs.slice(0, Math.max(0, runs.length - keep)) : runs);
  let freed = 0;
  for (const name of entries) if (doomed.has(runIdOf(name))) freed += await rm(join(LOGDIR, name), dryRun);
  return freed;
}

/**
 * Delete the `.agent-tmp` entries nothing has written for `days`; returns the bytes freed. By age
 * rather than wholesale, because a person may still be reading the log a session that just exited
 * wrote. `worktrees/` is never swept and never walked: a git worktree is somebody's branch, removed
 * with `git worktree remove` by whoever made it.
 */
async function pruneScratch(days: number = SCRATCH_DAYS, dryRun = false): Promise<number> {
  if (!isDir(SCRATCH)) return 0;
  const cutoff = now() - days * 86400;
  let freed = 0;
  for (const name of listDir(SCRATCH)) {
    if (name === WORKTREES) continue;
    const path = join(SCRATCH, name);
    const seen = await newest(path);
    if (seen && seen < cutoff) freed += await rm(path, dryRun);
  }
  return freed;
}

/** The directories an instrumented binary may have been started in: the root and each package's. */
const PROFRAW_DIRS = ["", "crates", "benches", "tools", "editors"];
const STRAY_PROFRAW = /^default_[0-9a-z_]*\.profraw$/;

/**
 * Every `default_*.profraw` at the root or directly in a package directory: the counters an
 * instrumented binary writes when nothing named `LLVM_PROFILE_FILE`. No run reads them, and every
 * process the tools start is told where to write (`lib/proc.ts` `childEnv`), so each one is left over.
 */
export function strayProfiles(root: string = ROOT): string[] {
  const dirs = new Set<string>([root]);
  for (const top of PROFRAW_DIRS.slice(1)) for (const name of listDir(join(root, top))) if (isDir(join(root, top, name))) dirs.add(join(root, top, name));
  return [...dirs].flatMap((d) => listDir(d).filter((n) => STRAY_PROFRAW.test(n)).map((n) => join(d, n))).sort();
}

/**
 * The memo files the selection store replaced, and the Python driver's before them. Nothing reads them
 * now; `clean` deletes the ones still on disk.
 */
export const RETIRED_MEMOS = [
  ".loop/proofs-green.json",
  ".loop/proof-reads.json",
  ".agent-tmp/impact-reads.json",
  ".agent-tmp/verify-green.json",
  ".agent-tmp/verify-test-green.json",
  ".agent-tmp/nv-key-tiers.json",
  ".loop/goal-green.json",
  ".loop/dossier-green.json",
  ".loop/check-reads.json",
  ".loop/check-times.ndjson",
  ".loop/floor-gate.json",
  ".loop/last-fail.json",
  ".loop/verify-norms.json",
];

/** A path as a set member: Windows compares paths case-insensitively, so the set must too. */
function key(path: string): string {
  const full = resolve(ROOT, path);
  return WINDOWS ? full.toLowerCase() : full;
}

/**
 * What `verify` builds, as the cargo commands its `build`, `test` and `clippy` steps run, plus `build
 * --workspace --all-targets` for every target kind. A step whose arguments change there changes here,
 * or what it builds stops counting as live and is kept only by `GRACE_HOURS`.
 */
const LIVE_QUERIES = [
  ["build"],
  ["test", "--no-run"],
  ["clippy", "--all-targets", "--", "-D", "warnings"],
  ["build", "--workspace", "--all-targets"],
];

/**
 * What the pipeline builds in `target/covws` (`lib/covws.ts`): `verify`'s build, test and clippy steps,
 * which go to `<triple>/debug`, with the build scripts and proc-macros in `debug`, and the proof
 * binary's profile. `wrapped` is whether the query goes through `covwrap`, as it does in the pipeline:
 * `clippy` shares the directory without it.
 */
const COVWS_QUERIES: { args: string[]; wrapped: boolean }[] = [
  { args: ["build"], wrapped: true },
  { args: ["test", "--no-run"], wrapped: true },
  { args: ["build", "--profile", "proof", "--bin", "nvs"], wrapped: true },
  { args: ["clippy", "--all-targets", "--", "-D", "warnings"], wrapped: false },
];

/** The environment and the arguments of one query: `LIVE_QUERIES` as they are, a covws one on covws. */
function queryArgv(query: string[], covws: { wrapped: boolean } | null): { argv: string[]; env?: Record<string, string> } {
  // `--message-format` is cargo's, so it goes ahead of the `--` that hands the rest to clippy.
  const cut = query.indexOf("--");
  const head = cut < 0 ? query : query.slice(0, cut);
  const tail = cut < 0 ? [] : query.slice(cut);
  if (covws === null) return { argv: ["cargo", head[0]!, "--message-format=json", ...head.slice(1), ...tail] };
  const { env, args } = covwsCargo();
  const own = covws.wrapped ? env : { CARGO_TARGET_DIR: env.CARGO_TARGET_DIR! };
  return { argv: ["cargo", head[0]!, "--message-format=json", ...head.slice(1), ...args, ...tail], env: own };
}

/**
 * Every file the graph `LIVE_QUERIES` and `COVWS_QUERIES` build uses, as cargo reports it. A warm call
 * is a no-op build that still emits a `compiler-artifact` line per unit. Null if any query fails, and
 * the caller then leaves `deps/` untouched rather than deleting on a partial answer.
 */
async function liveArtifacts(): Promise<Set<string> | null> {
  const live = new Set<string>();
  const queries = [...LIVE_QUERIES.map((q) => queryArgv(q, null)), ...COVWS_QUERIES.map((q) => queryArgv(q.args, q))];
  for (const { argv, env } of queries) {
    let out;
    try {
      out = await runProgram(argv, { timeoutMs: 4 * 60 * 60 * 1000, ...(env ? { env } : {}) });
    } catch {
      return null;
    }
    if (out.code !== 0) return null;
    for (const line of out.stdout.split(/\r?\n/)) {
      let msg;
      try {
        msg = JSON.parse(line);
      } catch {
        continue;
      }
      if (msg?.reason !== "compiler-artifact") continue;
      for (const name of [...(msg.filenames ?? []), msg.executable]) {
        if (!name) continue;
        live.add(key(name));
        // On windows-msvc the debug info, the import library and the dep-info file sit beside the
        // artifact under the same stem, and cargo lists none of them.
        const dir = name.slice(0, Math.max(name.lastIndexOf("/"), name.lastIndexOf("\\")) + 1);
        const [stem] = splitSuffix(name.slice(dir.length));
        for (const ext of [".pdb", ".d", ".exp", ".lib"]) live.add(key(dir + stem + ext));
      }
    }
  }
  return live;
}

/** The environment variables that change what cargo builds or what it names the result. */
const BUILD_ENV =
  /^(RUSTFLAGS|RUSTDOCFLAGS|RUSTC|RUSTC_WRAPPER|RUSTC_WORKSPACE_WRAPPER|CARGO_ENCODED_RUSTFLAGS|CARGO_INCREMENTAL|CARGO_TARGET_DIR|CARGO_BUILD_\w+|CARGO_PROFILE_\w+|CARGO_TARGET_\w+)$/;

/**
 * A hash of everything an artifact's name depends on: `LIVE_QUERIES`, `Cargo.lock`, every manifest in
 * the workspace, the build targets cargo discovers, the compiler, the cargo configuration and the build
 * environment. Source contents are not in it, because cargo's metadata hash does not cover them. Null
 * when cargo or rustc cannot be asked.
 */
export async function liveKey(root: string = ROOT): Promise<string | null> {
  const [meta, rustc] = await Promise.all([
    runProgram(["cargo", "metadata", "--no-deps", "--format-version", "1", "--offline"], { cwd: root, timeoutMs: 60_000 }),
    runProgram(["rustc", "-vV"], { cwd: root, timeoutMs: 60_000 }),
  ]).catch(() => [null, null]);
  if (!meta || !rustc || meta.code !== 0 || rustc.code !== 0) return null;
  let workspace: string;
  let manifests: string[];
  try {
    const parsed = JSON.parse(meta.stdout);
    workspace = parsed.workspace_root;
    manifests = parsed.packages.map((p: { manifest_path: string }) => p.manifest_path);
  } catch {
    return null;
  }
  const hash = createHash("sha256");
  const part = (label: string, text: string) => hash.update(`${label}\0${text.length}\0${text}\0`, "utf8");
  const file = (path: string) => {
    try {
      part(path, readFileSync(path, "utf8"));
    } catch {
      part(path, "\0missing");
    }
  };
  part("queries", JSON.stringify(LIVE_QUERIES));
  part("covws queries", JSON.stringify(COVWS_QUERIES));
  part("metadata", meta.stdout);
  part("rustc", rustc.stdout);
  const cargoHome = process.env.CARGO_HOME ?? join(homedir(), ".cargo");
  const configs = [".cargo/config.toml", ".cargo/config", "rust-toolchain.toml", "rust-toolchain"].map((p) => join(workspace, p));
  for (const path of [join(workspace, "Cargo.toml"), join(workspace, "Cargo.lock"), ...manifests.sort(), ...configs])
    file(path);
  file(join(cargoHome, "config.toml"));
  for (const name of Object.keys(process.env).filter((n) => BUILD_ENV.test(n)).sort()) part(`env ${name}`, process.env[name]!);
  return hash.digest("hex");
}

/** The live set remembered in `file` under `key`, or null when none is. */
function recall(key: string, file: string): Set<string> | null {
  try {
    const saved = JSON.parse(readFileSync(file, "utf8"));
    if (saved?.key === key && Array.isArray(saved.live)) return new Set<string>(saved.live);
  } catch {
    // No file, or one this code did not write: ask cargo.
  }
  return null;
}

/** Remembers `live` under `key`. A failure only means the next sweep asks cargo again. */
function remember(key: string, live: Set<string>, file: string): void {
  try {
    mkdirSync(dirname(file), { recursive: true });
    writeFileSync(`${file}.tmp`, `${JSON.stringify({ key, live: [...live].sort() })}\n`);
    renameSync(`${file}.tmp`, file);
  } catch {
    // Nothing to do: the sweep itself does not depend on the file.
  }
}

export interface LiveSetOptions {
  root?: string;
  file?: string;
  ask?: () => Promise<Set<string> | null>;
}

/**
 * The live set, from `file` when `liveKey` is what it was the last time cargo was asked, and from
 * `liveArtifacts` otherwise. A new answer is remembered only when the key is the same after the asking
 * as before it, so a manifest edited during a sweep is never stored under the key from before the edit.
 * An answer cargo could not give is never remembered.
 */
export async function liveSet(opts: LiveSetOptions = {}): Promise<Set<string> | null> {
  const root = opts.root ?? ROOT;
  const file = opts.file ?? LIVE_CACHE;
  const before = await liveKey(root);
  if (before !== null) {
    const saved = recall(before, file);
    if (saved) return saved;
  }
  const live = await (opts.ask ?? liveArtifacts)();
  if (live !== null && before !== null && (await liveKey(root)) === before) remember(before, live, file);
  return live;
}

/** `target/covws/<triple>`, or null when there is no covws build. */
function covwsTriple(): string | null {
  const dir = join(TARGET, "covws");
  return listDir(dir).map((t) => join(dir, t)).find((d) => isDir(join(d, "debug", "deps"))) ?? null;
}

/**
 * The files in `target/debug/{deps,examples}`, and in the covws build's debug and proof directories and
 * its host `debug/deps`, that no live unit claims and nothing has written for `graceHours`. Only the
 * profiles the live set is asked of: under any other profile every file would read as dead.
 */
function deadDeps(live: Set<string>, graceHours: number = GRACE_HOURS): string[] {
  const cutoff = now() - graceHours * 3600;
  const doomed: string[] = [];
  const cov = covwsTriple();
  const dirs = [join(TARGET, "debug", "deps"), join(TARGET, "debug", "examples"), join(TARGET, "covws", "debug", "deps")];
  if (cov !== null) dirs.push(join(cov, "debug", "deps"), join(cov, "debug", "examples"), join(cov, PROOF_PROFILE, "deps"));
  for (const dir of dirs) {
    if (!isDir(dir)) continue;
    for (const name of listDir(dir)) {
      const path = join(dir, name);
      const st = stat(path);
      if (!st || !st.isFile() || live.has(key(path))) continue;
      if (Number(st.mtimeMs) / 1000 < cutoff) doomed.push(path);
    }
  }
  return doomed;
}

/** Every `target/<profile>/<sub>` directory, and each of the covws build's: `target/covws/<profile>/<sub>`
 * for what it builds for the host, `target/covws/<triple>/<profile>/<sub>` for the rest. */
function profileDirs(sub: string): string[] {
  const roots = listDir(TARGET).map((profile) => join(TARGET, profile));
  const cov = join(TARGET, "covws");
  for (const name of listDir(cov)) {
    roots.push(join(cov, name));
    for (const profile of listDir(join(cov, name))) roots.push(join(cov, name, profile));
  }
  return roots.map((r) => join(r, sub)).filter(isDir);
}

/** The files and the distinct artifacts across `target/*\/deps`: an estimate that runs no build. */
function generations(): { files: number; names: number } {
  let files = 0;
  let names = 0;
  for (const dir of profileDirs("deps")) {
    const stems = new Set<string>();
    for (const name of listDir(dir)) {
      const [stem, suffix] = splitSuffix(name);
      const m = HASHED.exec(stem);
      if (!m) continue;
      files += 1;
      stems.add(m[1] + suffix);
    }
    names += stems.size;
  }
  return { files, names };
}

/**
 * The cache directories in `target/*\/incremental` past the newest `keep` for their crate that no build
 * has written for `graceHours`. Decided by name and age, because the hash here is an incremental
 * session id no artifact list mentions. Age carries it because a crate is built several ways at once,
 * each with its own entry, so the newest `keep` alone would condemn caches still in use. Deleting one
 * costs its crate one non-incremental compile, since nothing under this directory is an output.
 */
function staleIncremental(keep: number = KEEP_INCREMENTAL, graceHours: number = GRACE_HOURS): string[] {
  const cutoff = now() - graceHours * 3600;
  const doomed: string[] = [];
  for (const dir of profileDirs("incremental")) {
    const groups = new Map<string, string[]>();
    for (const name of listDir(dir)) {
      const cut = name.lastIndexOf("-");
      const crate = cut >= 0 ? name.slice(0, cut) : name;
      if (!groups.has(crate)) groups.set(crate, []);
      groups.get(crate)!.push(join(dir, name));
    }
    for (const entries of groups.values()) {
      const byAge = entries.map((path) => ({ path, mtime: mtime(path) })).sort((a, b) => b.mtime - a.mtime);
      for (const e of byAge.slice(keep)) if (e.mtime < cutoff) doomed.push(e.path);
    }
  }
  return doomed;
}

export interface CleanOptions {
  keepRuns?: number;
  scratchDays?: number;
  keepIncremental?: number;
  graceHours?: number;
  dryRun?: boolean;
}

/**
 * The whole of `--clean`, as the bytes freed per part. `target/deps` is null when cargo could not name
 * the live set, and `deps/` was then left alone. The command a person types and the sweep the driver
 * runs are this one function under the same policy.
 */
export async function clean(opts: CleanOptions = {}): Promise<Record<string, number | null>> {
  const dryRun = opts.dryRun ?? false;
  const grace = opts.graceHours ?? GRACE_HOURS;
  const live = await liveSet();
  const logs = await pruneLogs(opts.keepRuns ?? KEEP_RUNS, dryRun);
  const scratch = await pruneScratch(opts.scratchDays ?? SCRATCH_DAYS, dryRun);
  let deps: number | null = null;
  if (live !== null) {
    deps = 0;
    for (const p of deadDeps(live, grace)) deps += await rm(p, dryRun);
  }
  let incremental = 0;
  for (const p of staleIncremental(opts.keepIncremental ?? KEEP_INCREMENTAL, grace)) incremental += await rm(p, dryRun);
  let leftovers = 0;
  for (const p of strayProfiles()) leftovers += await rm(p, dryRun);
  for (const p of RETIRED_MEMOS) if (existsSync(join(ROOT, p))) leftovers += await rm(join(ROOT, p), dryRun);
  return { ".loop/logs": logs, ".agent-tmp": scratch, "target/deps": deps, "target/incremental": incremental, "leftovers": leftovers };
}

/** The bytes a `clean` result freed, a part it left alone counting as none. */
export function total(freed: Record<string, number | null>): number {
  return Object.values(freed).reduce<number>((sum, n) => sum + (n ?? 0), 0);
}

/** One line per part of a `clean` result, printed alike by `--clean` and by the driver. */
export function freedLines(freed: Record<string, number | null>, verb = "freed"): string[] {
  return Object.entries(freed).map(([name, n]) =>
    n === null
      ? `${name.padEnd(19)} left alone -- cargo could not name the live set; does the tree build?`
      : `${name.padEnd(19)} ${verb} ${human(n)}`,
  );
}

// ------------------------------------------------------------------------------------------- report

async function report(deep: boolean): Promise<void> {
  const drive = WINDOWS ? (/^[A-Za-z]:/.exec(ROOT)?.[0] ?? "/") : "/";
  console.log(`free on ${drive}  ${fixed(freeGb(), 1)}G   (\`bun nv loop\` refuses to start a run below ${MIN_FREE_GB}G)`);
  console.log();

  const [target, logs, scratchDir] = await Promise.all([walk(TARGET), walk(LOGDIR), walk(SCRATCH)]);
  const { files, names } = generations();

  // Both incremental figures come from the one walk of `target/`: under it, a cache file's path is
  // `<profile>/incremental/<cache directory>/...`. Every path `walk` returns starts with its root and a
  // separator, so cutting that off is the relative path; `path.relative` costs far more per file.
  // The proof profile's total comes from the same walk: under the covws build its path is
  // `covws/<triple>/proof/...`, with what it builds for the host under `covws/proof/...`.
  const stale = new Set(staleIncremental());
  let inc = 0;
  let staleInc = 0;
  let proof = 0;
  for (const f of target.files) {
    const parts = f.path.slice(TARGET.length + 1).split(sep);
    if (parts[0] === "covws" && (parts[1] === PROOF_PROFILE || parts[2] === PROOF_PROFILE)) proof += f.size;
    const at = parts.indexOf("incremental");
    if (at < 1 || at > 3 || parts.length < at + 2) continue;
    inc += f.size;
    if (parts.length > at + 2 && stale.has(join(TARGET, ...parts.slice(0, at + 2)))) staleInc += f.size;
  }

  const blank = `  ${"".padEnd(12)} ${"".padStart(8)}   `;
  console.log("in this repository");
  console.log(
    `  target/       ${human(target.total).padStart(8)}   deps/: ${files} files for ${names} artifacts` +
      (names ? ` -- about ${fixed(files / names, 1)} generations` : ""),
  );
  console.log(
    `${blank}incremental/: ${human(inc)}, of which ${human(staleInc)} ` +
      `is past the newest ${KEEP_INCREMENTAL} per crate and idle ${GRACE_HOURS}h`,
  );
  console.log(
    `${blank}\`--clean\` keeps what verify and the proof binary build and anything written in the ` +
      `last ${GRACE_HOURS}h; release/deps is never swept`,
  );
  console.log(
    `  covws/${PROOF_PROFILE}/ ${human(proof).padStart(8)}   ` +
      `the proof binary's build in target/covws, which nv proofs runs -- counted in target/ above`,
  );
  console.log(`  .loop/logs    ${human(logs.total).padStart(8)}   kept: newest ${KEEP_RUNS} runs -- swept after every loop session`);
  console.log(
    `  .agent-tmp    ${human(scratchDir.total).padStart(8)}   kept: written within ${SCRATCH_DAYS}d -- swept after every loop session`,
  );
  console.log();

  console.log("outside this repository -- reported, never touched by this script");
  const home = homedir().replaceAll("\\", "/");
  const tmp = WINDOWS ? "(WSL) /var/tmp" : "/var/tmp";
  const indent = `  ${"".padEnd(20)} ${"".padStart(8)}  `;
  for (const [name, path, what, how] of ELSEWHERE) {
    const shown = path.replace("{home}", home).replace("{tmp}", tmp);
    const size = deep && isDir(shown) ? human((await walk(shown)).total).padStart(8) : "       ?";
    console.log(`  ${name.padEnd(20)} ${size}  ${shown}`);
    console.log(`${indent}${what}`);
    console.log(`${indent}$ ${how}`);
  }
  if (!deep) console.log(`  (sizes: --deep, which walks all ${ELSEWHERE.length} and is slow)`);
}

// --------------------------------------------------------------------------------------------- args

const USAGE = [
  "usage: nv disk [-h] [--clean] [-n] [--deep] [--keep-runs KEEP_RUNS]",
  "               [--scratch-days SCRATCH_DAYS]",
  "               [--keep-incremental KEEP_INCREMENTAL]",
  "               [--grace-hours GRACE_HOURS]",
].join("\n");

const FLAGS = ["--help", "--clean", "--dry-run", "--deep"];
const VALUED = ["--keep-runs", "--scratch-days", "--keep-incremental", "--grace-hours"];

class ArgError extends Error {}

/** `'x'`, the way Python's `repr` prints a string with no quote in it. */
function repr(s: string): string {
  return s.includes("'") && !s.includes('"') ? `"${s}"` : `'${s}'`;
}

/** Python's `float` over a command-line word, or null where it would raise. */
function pyFloat(word: string): number | null {
  const w = word.trim();
  if (/^[+-]?(inf|infinity)$/i.test(w)) return w.startsWith("-") ? -Infinity : Infinity;
  if (/^[+-]?nan$/i.test(w)) return NaN;
  if (!/^[+-]?(\d+(_\d+)*(\.(\d+(_\d+)*)?)?|\.\d+(_\d+)*)([eE][+-]?\d+(_\d+)*)?$/.test(w)) return null;
  return Number(w.replaceAll("_", ""));
}

/** An option word, resolved the way argparse resolves one: exactly, or by a unique prefix. */
function resolveOption(word: string): string | null {
  const all = [...FLAGS, ...VALUED];
  if (all.includes(word)) return word;
  const hits = all.filter((o) => o.startsWith(word));
  if (hits.length > 1) throw new ArgError(`ambiguous option: ${word} could match ${hits.join(", ")}`);
  return hits[0] ?? null;
}

interface Opts {
  help: boolean;
  clean: boolean;
  dryRun: boolean;
  deep: boolean;
  keepRuns: number;
  scratchDays: number;
  keepIncremental: number;
  graceHours: number;
}

/** A word argparse would take as a value rather than as an option. */
function looksLikeValue(word: string): boolean {
  return !word.startsWith("-") || /^-\d+$|^-\d*\.\d+$/.test(word);
}

function parse(args: string[]): Opts {
  const opts: Opts = {
    help: false,
    clean: false,
    dryRun: false,
    deep: false,
    keepRuns: KEEP_RUNS,
    scratchDays: SCRATCH_DAYS,
    keepIncremental: KEEP_INCREMENTAL,
    graceHours: GRACE_HOURS,
  };
  // Every option word is classified before any value is read, so an ambiguous prefix is reported
  // ahead of a bad value, and an unrecognized word only after both.
  const classified = args.map((word) => {
    if (word === "-h") return "--help";
    if (word === "-n") return "--dry-run";
    if (!word.startsWith("--") || word === "--") return null;
    const eq = word.indexOf("=");
    return resolveOption(eq >= 0 ? word.slice(0, eq) : word);
  });
  const unknown: string[] = [];
  for (let i = 0; i < args.length; i++) {
    const option = classified[i] ?? null;
    if (option === null) {
      unknown.push(args[i]!);
      continue;
    }
    if (FLAGS.includes(option)) {
      if (option === "--help") opts.help = true;
      else if (option === "--clean") opts.clean = true;
      else if (option === "--dry-run") opts.dryRun = true;
      else opts.deep = true;
      continue;
    }
    const word = args[i]!;
    const eq = word.indexOf("=");
    let value: string;
    if (eq >= 0) value = word.slice(eq + 1);
    else if (i + 1 < args.length && looksLikeValue(args[i + 1]!)) value = args[++i]!;
    else throw new ArgError(`argument ${option}: expected one argument`);
    if (option === "--grace-hours") {
      const n = pyFloat(value);
      if (n === null) throw new ArgError(`argument ${option}: invalid float value: ${repr(value)}`);
      opts.graceHours = n;
    } else {
      const n = pyInt(value);
      if (n === null) throw new ArgError(`argument ${option}: invalid int value: ${repr(value)}`);
      if (option === "--keep-runs") opts.keepRuns = n;
      else if (option === "--scratch-days") opts.scratchDays = n;
      else opts.keepIncremental = n;
    }
  }
  if (unknown.length > 0) throw new ArgError(`unrecognized arguments: ${unknown.join(" ")}`);
  return opts;
}

// --------------------------------------------------------------------------------------------- main

export async function run(args: string[]): Promise<number> {
  let opts: Opts;
  try {
    opts = parse(args);
  } catch (e) {
    if (!(e instanceof ArgError)) throw e;
    console.error(`${USAGE}\nnv disk: error: ${e.message}`);
    return 2;
  }
  if (opts.help) {
    console.log(`${USAGE}\n\n${summary}`);
    return 0;
  }

  if (!opts.clean) {
    await report(opts.deep);
    return 0;
  }

  // A driver is mid-run, so cargo may be writing target/ right now. Sweeping under a live build is the
  // one way this could break something rather than merely cost a rebuild; the driver runs this same
  // `clean` itself after every session, where it knows none is.
  if (existsSync(RUNNING)) {
    console.log("a loop driver holds .loop/running -- stop it first.");
    console.log("Sweeping target/ under a running build is the one thing here that is not safe;");
    console.log("the driver sweeps on its own after every session's acceptance check.");
    return 2;
  }

  const before = freeGb();
  const freed = await clean(opts);
  const verb = opts.dryRun ? "would free" : "freed";
  for (const line of freedLines(freed, verb)) console.log(`  ${line}`);
  console.log(
    `  ${"total".padEnd(19)} ${verb} ${human(total(freed))}` +
      (opts.dryRun ? "" : `; ${fixed(before, 1)}G -> ${fixed(freeGb(), 1)}G free`),
  );
  if (!opts.dryRun && (freed["target/deps"] || freed["target/incremental"])) {
    console.log("\nThe next build is slower by whatever it has to make again. That is the whole cost.");
  }
  return 0;
}
