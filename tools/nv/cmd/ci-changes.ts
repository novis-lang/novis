// `bun nv ci-changes`: which CI lanes a push or a pull request needs, from the paths its diff touched.
//
//     bun nv ci-changes --base HEAD~1    what CI would run for the last commit
//     bun nv ci-changes --base <sha>     ... for any range ending at HEAD
//     bun nv ci-changes --since-green    ... since the newest commit on main whose CI went green
//     bun nv ci-changes --full           every lane, as the nightly and the release run it
//     bun nv ci-changes --files -        lanes for a path list on stdin, touching no git
//     bun nv ci-changes --lanes          each lane and the prefixes it matches
//
// `.github/workflows/ci.yml`'s `changes` job runs this, and every other job in that file gates on one
// of the booleans it emits. `LANES` below is that policy's only home -- `rule:testing/ci-lanes` is the
// reasoning, and the workflow holds no `paths:` filter of its own. `dev-build.yml` runs it too, and
// reads only `release`: whether the dev build has anything new to build (`UNBUILT` below).
//
// Inside Actions it takes `BASE`, `FULL` and `SINCE_GREEN` from the environment and appends
// `name=value` lines to `$GITHUB_OUTPUT`; run by hand it prints the same lines to stdout, so the two
// cannot disagree.
//
// **A push to `main` diffs against the newest green CI run, not against the push's `before`.**
// `ci.yml` sets `SINCE_GREEN` for a push to `main`, and `greenBase` below picks the base: the commit of
// the newest successful `ci.yml` run on `main` that is an ancestor of HEAD and not HEAD itself. That is
// what lets `ci.yml`'s concurrency group cancel an older push's run when a newer push arrives: the
// newer run's diff starts at the last commit CI proved, so it covers every commit whose run was
// cancelled. A dry run that `bun nv ci --dispatch` started cancels the push runs of its own commit in
// the same way. The base is sound by induction. A full run (the nightly or a dispatch) checked every
// lane at its own commit. A push run that went green checked every lane its diff from a sound base
// touched, and each lane it did not touch has the same files as at that base, where it was checked
// already. So every lane at a green run's commit was checked on the same files, and a newer push that
// diffs from it runs every lane whose files changed since. A pull request's run is never a base: it ran
// on a merge commit `main` never had. When the lookup fails, or none of the newest
// `GREEN_RUNS_LISTED` green runs is an ancestor of HEAD, no base is known and every lane runs. The
// push's `before` is no fallback: its run may have been cancelled, and a run green from it would then
// be a base for the next push without having proved the commits in between. What it spends: one `gh api` call per push run on `main`, and a commit whose
// run a newer push cancelled has no verdict of its own: the next run verifies it.
//
// **An unknown base turns every lane on.** A first push to a branch, a force push and a shallow clone
// all leave this command without a commit it can diff against, and the failure mode of guessing the
// other way is a commit that reached `main` with nothing having checked it.
//
// The `changes` job runs before any toolchain is installed, so the package graph the database lane
// needs is read off the workspace's manifests here rather than asked of `cargo metadata`, which
// `tools/nv/keys/graph.ts` does for every caller that has cargo.

import { appendFileSync, existsSync, readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { parse as parseToml } from "smol-toml";
import { dependencySet, type DepKind, type Graph } from "../keys/graph.ts";
import { ROOT } from "../lib/paths.ts";
import { run as runProc } from "../lib/proc.ts";
import { ArgError, parseArgs } from "../lib/py.ts";

export const summary = "which CI lanes a diff needs: nv ci-changes --base <sha> [--since-green] | --full | --files <file> | --lanes";

const USAGE = "usage: nv ci-changes [-h] [--base BASE] [--since-green] [--full] [--files FILE] [--lanes]";

/** Each directory with a lockfile of its own beside the root's. `ci.yml`'s `supply-chain` job and `bun nv
 * ci --check` run `cargo deny check advisories` on each, under the root's `deny.toml`
 * (`rule:packaging/a-dependency-move-proves-two-things`). */
export const LOCKFILE_DIRS = [
  "extensions/image", "extensions/intl", "extensions/pdf", "extensions/spreadsheet",
  "fuzz", "tools/covwrap", "tools/nv-scan", "website/guides/highlight-extension",
];

// The code the binary is compiled from outside `crates/`: the patched crates under `vendor/`, and the
// extension components `nvs-ext`'s build script compiles with the WIT they implement.
// `ci-changes.test.ts` fails when `Cargo.toml` names a path crate that `code` does not cover.
const COMPILED_INPUTS = ["vendor/", "wit/", "extensions/"];

// The files the crates embed as text. A change to one changes the binary's bytes and not its code, so
// it runs no platform matrix: the dev build rebuilds for it, the lanes that read it run (`refdoc`,
// `proofs`, `browser`, `deps`), and the few unit tests that read one run on the next push that changes
// code, and nightly. `ci-changes.test.ts` fails when a crate embeds a file that neither this list nor
// `code` covers.
export const EMBEDDED_TEXT = [
  "docs/reference/", "viewer/dist/", "apps/telemetry/",
  "LICENSE", "README.md", "CHANGELOG.md", "THIRD-PARTY-LICENSES.txt",
];

// `ci.yml` is in every lane: a run that edits it is the one run where the lanes themselves are what
// needs proving. The other workflows are in none. No job of `ci.yml` runs them, and each one proves
// itself when it runs: `dev-build.yml` with `build.yml` and `image.yml` at every push that changes
// them, `pages.yml` at its own push, and `release.yml` and `release-promote.yml` in `bun nv ci
// --dispatch dry-run`. A full suite on four platforms for an edit to one of them proves nothing
// about it.
export const CI_WORKFLOW = ".github/workflows/ci.yml";

// The `bun nv` commands the jobs `code` gates run: `test` runs `checkout-acl`, and through
// `nvs-ir`'s refusal roster `holes`; `lint` runs `lints`. `ci-changes.test.ts` walks each one's
// imports from `main.ts` and fails when a module it loads is missing from `CODE_TOOLS`.
export const CODE_COMMANDS = ["checkout-acl", "holes", "lints"];

// The files under `tools/` that the jobs `code` gates read: what cargo compiles or embeds from
// `tools/build/` and `tools/data/`, `gaps.ts`, which `nvs-stdlib`'s conformance coverage test reads
// as text, and the modules `CODE_COMMANDS` load. The rest of `tools/` runs in no job this lane
// gates, so a change there alone runs no platform matrix. `package.json` and `bun.lock` are here
// because both jobs install the tools' dependencies and run them.
export const CODE_TOOLS = [
  "tools/build/", "tools/data/", "tools/nv/cmd/gaps.ts",
  "tools/nv/main.ts", "tools/nv/cmd/checkout-acl.ts", "tools/nv/cmd/holes.ts", "tools/nv/cmd/lints.ts",
  "tools/nv/lib/chain.ts", "tools/nv/lib/hang.ts", "tools/nv/lib/paths.ts", "tools/nv/lib/proc.ts",
  "tools/nv/lib/proctree.ts", "tools/nv/lib/progress.ts", "tools/nv/lib/py.ts", "tools/nv/lib/reads.ts",
  "tools/nv/lib/tty.ts",
  "package.json", "bun.lock",
];

// One entry per gate a job in ci.yml spells as `needs.changes.outputs.<name> == 'true'`. A prefix
// ending in `/` matches everything beneath it; anything else matches that one path exactly.
export const LANES: Record<string, string[]> = {
  // Any input to a cargo build. Gates the Linux build/test/conformance leg and `lint`.
  code: [
    "crates/", "benches/", "tests/", "examples/", "fuzz/",
    "Cargo.toml", "Cargo.lock", "rustfmt.toml", "rust-toolchain.toml", "deny.toml",
    ...COMPILED_INPUTS, ...CODE_TOOLS, CI_WORKFLOW,
  ],
  // The crates that emit or execute machine code, plus the probes that measure them and the script
  // the sanitizer jobs run. These are where a use-after-free lives, and they are the only inputs a
  // cost baseline has, so this gates ASAN, TSAN and the `cost-guards` job. It never narrows the platform matrix: every platform runs whenever
  // any code changes, because which host a contributor pushes from is unknowable.
  native: [
    "crates/nvs-runtime/", "crates/nvs-codegen/", "crates/nvs-stdlib/", "crates/nvs-host/",
    "benches/abi-probe/", "Cargo.lock", "rust-toolchain.toml", "tools/tsan.sh",
    CI_WORKFLOW,
  ],
  // Where `wasmtime` enters the tree, and so the only thing `extension-sandbox` can observe.
  probe: ["benches/abi-probe/", CI_WORKFLOW],
  // What the `nvs ext new` templates and the extension guide's example are built from and loaded
  // by: the extension host, the `nvs ext` command and the templates it writes, the world's WIT, the
  // example's project, the tool that drives them and the toolchain that lists `wasm32-wasip2`; and
  // libwebp's prebuilt library with the tool that rebuilds it. Gates `ext-templates` and `webp-lib`.
  ext: [
    "crates/nvs-ext/", "crates/nvs-cli/src/ext.rs", "crates/nvs-cli/src/ext/", "crates/nvs-cli/templates/ext/",
    "wit/nvs-ext/", "website/guides/highlight-extension/", "tools/nv/cmd/ext-templates.ts", "rust-toolchain.toml",
    CI_WORKFLOW, "extensions/image/libwebp/", "tools/nv/cmd/webp-lib.ts",
  ],
  // The VS Code client, and every crate with it: the protocol suite spawns the real `nvs lsp` and the
  // host suite points a throwaway profile at the binary the same run built
  // (`rule:ide/headless-gates-the-loop-the-host-run-gates-the-milestone`), so a change anywhere in the
  // workspace can change what either of them answers. Gates `extension` and `extension-host`.
  editor: ["editors/", "crates/", "Cargo.toml", "Cargo.lock", ...COMPILED_INPUTS, CI_WORKFLOW],
  // What `cargo deny` and the attribution diff read, the other lockfiles' manifests among them. Their
  // answer cannot differ in a run that changed none of these
  // (`rule:packaging/a-dependency-move-proves-two-things`).
  deps: [
    "Cargo.toml", "Cargo.lock", "deny.toml", "THIRD-PARTY-LICENSES.txt",
    "tools/nv/cmd/gen-attribution.ts", "viewer/package.json", "bun.lock", CI_WORKFLOW,
    "editors/vscode/package.json", "editors/vscode/package-lock.json", "editors/vscode/THIRD-PARTY-NOTICES.txt",
    "editors/vscode/.vscodeignore", "editors/vscode/scripts/viewer.mjs", "editors/vscode/src/",
    "tools/nv/test/attribution-vscode.test.ts", "wit/nvs-ext/deps/README.md",
    ...LOCKFILE_DIRS.flatMap((d) => [`${d}/Cargo.toml`, `${d}/Cargo.lock`]),
  ],
  // The website's `Core` pages and `/llms-full.txt` are rendered from the binary's registry and the
  // chapters, so either side moving can make a committed file stale. Gates `reference`.
  refdoc: [
    "crates/", "docs/reference/", "tools/nv/cmd/reference.ts",
    "website/", "tools/nv/renderers/", CI_WORKFLOW,
  ],
  // What `bun nv proofs --verify` reads: the binary's inputs, every feature's examples, attacks, benches
  // and chapters, the perf ledger, the proof policy and the gap records a `proof: gap` marker names,
  // and what `bun nv site --check` reads beside them: the website's pages, snippets and lock, and the
  // example apps its `apps` part tests and serves. Gates `proofs`.
  proofs: [
    "crates/", "benches/", "tests/", "tools/", "Cargo.toml", "Cargo.lock", "rust-toolchain.toml",
    "docs/examples/", "docs/reference/", "data/perf/", "data/proofs/", "data/gaps/",
    "website/", "apps/", ...COMPILED_INPUTS, CI_WORKFLOW,
  ],
  // The five-driver matrix against real servers: the harness that points each driver at a container,
  // the compose file those containers come from, and -- added by `dbLane` below -- every package
  // `bun nv db-matrix` runs the tests of and every package those are compiled against. Gates
  // `database`, the one job that needs a daemon. `crates/nvs-ldap/` is listed by hand: the same job
  // runs its tests against the compose file's `samba-ad`, outside the harness, and every crate it is
  // compiled against is one `nvs-db` is compiled against too.
  db: [
    "tests/db/", "tools/nv/cmd/db-matrix.ts", "crates/nvs-ldap/", "Cargo.toml", "Cargo.lock",
    CI_WORKFLOW,
  ],
  // The slotted response's polyfill and trigger, a website replay's modules and component, the debug
  // viewer and the command that builds and checks it, the Chromium tests that run them and their
  // shared driver, the registry's app and its Rust commands that its pages are tested over, and the
  // lockfile that pins the Playwright release and so the browser it downloads. Gates `browser`.
  browser: [
    "crates/nvs-server/src/slotted/", "tools/nv/test/html-later-polyfill.test.ts",
    "apps/registry/", "crates/nvs-cli/src/registry.rs", "crates/nvs-cli/src/package/",
    "tools/nv/test/registry-browser.test.ts",
    "website/config/replay-frames.mjs", "website/config/replay-narrow.mjs", "website/config/replay-player.mjs",
    "website/src/components/Replay.astro", "viewer/", "tools/nv/cmd/viewer.ts",
    "tools/nv/test/replay-browser.test.ts", "tools/nv/test/chromium.ts",
    "package.json", "bun.lock", "bunfig.toml", CI_WORKFLOW,
  ],
  // The release images' Dockerfile and the smoke test that runs them. A change in the binary
  // itself reaches the images through the nightly's full run, and through the release, which
  // smoke-tests every variant before it pushes one. Gates `docker`.
  docker: ["docker/", "tests/docker/", CI_WORKFLOW],
};

// The paths no part of the dev build reads: not the release binaries, the VS Code extension or the
// container images. `release` is true when a diff touched any other path. `dev-build.yml` gates on it
// at each push and diffs against the commit of the newest green dev build that still has its
// artifacts, so a push that changed only these keeps that build's binaries and its `dev` image. Unlike a lane, this list names what
// the build skips. A path added to the tree tomorrow is then built, which is the safe direction.
// `docs/reference/` is missing from the list on purpose: `nvs-stdlib`'s build script embeds it.
export const UNBUILT = [
  "docs/agent/", "docs/rules/", "docs/goals/", "docs/examples/", "docs/release-notes/",
  "docs/ground-rules.md", "docs/release.md", "docs/setup.md", "website/", "data/", ".claude/",
  "AGENTS.md", "CODE_OF_CONDUCT.md", "CONTRIBUTING.md", "SECURITY.md", "looper.toml",
  ".config/", "clippy.toml", "rustfmt.toml", "deny.toml",
];

/** Whether a diff reaches what the dev build publishes. `paths` of `null` means "not known": it does. */
export function releaseNeeded(paths: string[] | null): boolean {
  return paths === null || paths.some((p) => !matches(p, UNBUILT));
}

const DB_HARNESS = "tools/nv/cmd/db-matrix.ts";

const DEP_TABLES: [string, DepKind][] = [
  ["dependencies", "normal"],
  ["dev-dependencies", "dev"],
  ["build-dependencies", "build"],
];

type Toml = Record<string, unknown>;

/** The workspace's `crates/*` and `benches/*` members that have a `[package]`, and what each depends on
 * in every dependency table, the `[target.*]` ones included. `null` when a manifest cannot be read. */
export function manifestGraph(root: string = ROOT): Graph | null {
  const found = new Map<string, { dir: string; doc: Toml }>();
  try {
    for (const top of ["crates", "benches"]) {
      for (const d of readdirSync(join(root, top)).sort()) {
        const manifest = join(root, top, d, "Cargo.toml");
        if (!existsSync(manifest)) continue;
        const doc = parseToml(readFileSync(manifest, "utf8")) as Toml;
        const pkg = doc.package as Toml | undefined;
        if (pkg) found.set(String(pkg.name), { dir: `${top}/${d}`, doc });
      }
    }
  } catch {
    return null;
  }
  const out: Graph = new Map();
  for (const [name, { dir, doc }] of found) {
    const deps = new Map<string, DepKind>();
    const tables = [doc, ...Object.values((doc.target as Record<string, Toml> | undefined) ?? {})];
    for (const table of tables) {
      for (const [key, kind] of DEP_TABLES) {
        for (const [dep, spec] of Object.entries((table[key] as Toml | undefined) ?? {})) {
          const real = spec && typeof spec === "object" && "package" in spec ? String((spec as Toml).package) : dep;
          if (found.has(real) && deps.get(real) !== "normal") deps.set(real, kind);
        }
      }
    }
    out.set(name, { name, dir, deps, targets: [] });
  }
  return out;
}

/** The workspace packages a file names anywhere in its text, comments included: more names is the
 * wide direction. */
function namedIn(graph: Graph, path: string): string[] {
  let text: string;
  try {
    text = readFileSync(join(ROOT, path), "utf8");
  } catch {
    return [];
  }
  const escape = (s: string) => s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  return [...graph.keys()].filter((n) => new RegExp(`(?<![\\w-])${escape(n)}(?![\\w-])`).test(text)).sort();
}

/** `LANES.db` with the packages the matrix builds. They are read rather than listed, so an edit to a
 * crate `nvs-db` is compiled against runs the database leg. With no readable graph the lane is every
 * crate, which is the wide direction. */
function dbLane(base: string[]): string[] {
  const graph = manifestGraph();
  const names = graph ? namedIn(graph, DB_HARNESS) : [];
  if (!graph || names.length === 0) return [...base, "crates/", "benches/"];
  const built = new Set(names);
  for (const n of names) for (const d of dependencySet(graph, n)) built.add(d);
  return [...base, ...[...built].map((n) => `${graph.get(n)!.dir}/`).sort()];
}

const ZERO = "0".repeat(40);

/** `git`'s stdout, or `null` when it exits non-zero or does not start. */
async function git(...args: string[]): Promise<string | null> {
  try {
    const r = await runProc(["git", ...args]);
    return r.code === 0 ? r.stdout : null;
  } catch {
    return null;
  }
}

/**
 * The events whose green `ci.yml` run on `main` is a base. `schedule` and `workflow_dispatch` run every
 * lane, and a `push` run ran every lane its diff from a base touched. A `pull_request` run is not one:
 * it tested a merge commit, and a fork's pull request may name its branch `main`.
 */
export const GREEN_EVENTS = ["push", "schedule", "workflow_dispatch"];

/** How many of the newest green `ci.yml` runs on `main` the lookup reads, in one `gh api` call. */
export const GREEN_RUNS_LISTED = 50;

/** A `ci.yml` run, as the runs API lists it. */
export interface CiRun {
  event: string;
  conclusion: string | null;
  head_sha: string;
  head_branch: string | null;
}

/**
 * The commit of the first run in `runs`, newest first, that went green on `main` under one of
 * `GREEN_EVENTS` and whose commit `isAncestor` of HEAD, or `null` when there is none. A run on `head`
 * itself is passed over, so a run started again on the same commit runs its lanes again. Any green
 * ancestor is a sound base, and an older one only widens the diff.
 */
export async function greenBase(runs: CiRun[], head: string, isAncestor: (sha: string) => Promise<boolean>): Promise<string | null> {
  const tried = new Set<string>([head]);
  for (const r of runs) {
    if (r.conclusion !== "success" || r.head_branch !== "main" || !GREEN_EVENTS.includes(r.event)) continue;
    if (tried.has(r.head_sha)) continue;
    tried.add(r.head_sha);
    if (await isAncestor(r.head_sha)) return r.head_sha;
  }
  return null;
}

/**
 * The base for a push to `main`: `greenBase` over the newest green `ci.yml` runs GitHub lists, or ""
 * when `gh` cannot list them or none is an ancestor of HEAD, and "" turns every lane on. It prints the
 * base it chose and why on stderr, which the run's log shows.
 */
async function sinceGreen(): Promise<string> {
  const say = (line: string) => console.error(`nv ci-changes: ${line}`);
  const back = (why: string) => {
    say(`${why}: no base is known, so every lane runs`);
    return "";
  };
  const slug = process.env.GITHUB_REPOSITORY || "{owner}/{repo}";
  const query = `repos/${slug}/actions/workflows/ci.yml/runs?branch=main&status=success&per_page=${GREEN_RUNS_LISTED}`;
  let listed: string;
  try {
    const r = await runProc(["gh", "api", query, "--jq", "[.workflow_runs[] | {event, conclusion, head_sha, head_branch}]"], { timeoutMs: 60_000 });
    if (r.code !== 0) return back(`\`gh api\` exited ${r.code}: ${r.stderr.trim().split("\n")[0] ?? ""}`);
    listed = r.stdout;
  } catch (e) {
    return back(`gh did not start: ${e instanceof Error ? e.message : String(e)}`);
  }
  let runs: CiRun[];
  try {
    runs = JSON.parse(listed) as CiRun[];
    if (!Array.isArray(runs)) return back("`gh api` printed no list of runs");
  } catch {
    return back("`gh api` printed no JSON");
  }
  const head = (await git("rev-parse", "HEAD"))?.trim();
  if (!head) return back("HEAD has no commit");
  const base = await greenBase(runs, head, async (sha) => (await git("merge-base", "--is-ancestor", sha, "HEAD")) !== null);
  if (base === null) return back(`none of the ${runs.length} newest green CI runs on main is on an ancestor of HEAD`);
  say(`diffing against ${base}, the newest commit on main whose CI run went green`);
  return base;
}

/** The paths `base..HEAD` touched, or `null` if `base` is not a commit that can be reached. */
async function changedPaths(base: string): Promise<string[] | null> {
  if (!base || base === ZERO) return null;
  if ((await git("cat-file", "-e", `${base}^{commit}`)) === null) return null;
  const out = await git("diff", "--name-only", base, "HEAD");
  if (out === null) return null;
  return nonEmpty(out);
}

function nonEmpty(text: string): string[] {
  return text
    .split(/\r?\n/)
    .map((l) => l.trim())
    .filter(Boolean);
}

const matches = (path: string, prefixes: string[]) => prefixes.some((p) => (p.endsWith("/") ? path.startsWith(p) : path === p));

/** One boolean per lane. `paths` of `null` means "not known", which means all of them. */
export function lanesFor(lanes: Record<string, string[]>, paths: string[] | null): [string, boolean][] {
  return Object.entries(lanes).map(([name, pre]) => [name, paths === null || paths.some((p) => matches(p, pre))]);
}

const truthy = (value: string | undefined) => ["1", "true", "yes", "on"].includes((value ?? "").trim().toLowerCase());

function help(): string {
  return [
    USAGE,
    "",
    "which CI lanes a push or a pull request needs, from the paths its diff touched",
    "",
    "options:",
    "  -h, --help    show this help message and exit",
    "  --base BASE   the commit HEAD is compared against; empty means every lane",
    "  --since-green compare against the newest commit on main whose CI run went green; when `gh`",
    "                finds none, every lane",
    "  --full        the deep lane: every gate true, every platform in the matrix",
    "  --files FILE  read the changed paths from a file (`-` for stdin) instead of from git",
    "  --lanes       print each lane and the prefixes it matches, and nothing else",
  ].join("\n");
}

export async function run(args: string[]): Promise<number> {
  let flags: Set<string>;
  let values: Map<string, string>;
  try {
    ({ flags, values } = parseArgs(args, { flags: ["--full", "--lanes", "--since-green"], valued: ["--base", "--files"], order: ["--base", "--since-green", "--full", "--files", "--lanes"] }));
  } catch (e) {
    if (!(e instanceof ArgError)) throw e;
    console.error(`${USAGE}\nnv ci-changes: error: ${e.message}`);
    return 2;
  }
  if (flags.has("--help")) {
    console.log(help());
    return 0;
  }
  const lanes = { ...LANES, db: dbLane(LANES.db!) };
  if (flags.has("--lanes")) {
    for (const [name, pre] of Object.entries(lanes)) console.log(`"${name}": (${pre.map((p) => `"${p}"`).join(", ")})`);
    console.log(`"release": every path but (${UNBUILT.map((p) => `"${p}"`).join(", ")})`);
    return 0;
  }
  const full = flags.has("--full") || truthy(process.env.FULL);
  let paths: string[] | null;
  if (full) paths = null;
  else if (values.has("--files")) {
    const file = values.get("--files")!;
    paths = nonEmpty(file === "-" ? await Bun.stdin.text() : readFileSync(file, "utf8"));
  } else {
    const given = values.get("--base") ?? process.env.BASE ?? "";
    const green = flags.has("--since-green") || truthy(process.env.SINCE_GREEN);
    paths = await changedPaths(green ? await sinceGreen() : given);
  }

  const lines = [
    ...lanesFor(lanes, paths),
    ["release", releaseNeeded(paths)] as [string, boolean],
    ["deep", full] as [string, boolean],
  ].map(([n, v]) => `${n}=${v}`);
  const sink = process.env.GITHUB_OUTPUT;
  if (sink) appendFileSync(sink, lines.join("\n") + "\n");
  for (const line of lines) console.log(line);
  return 0;
}
