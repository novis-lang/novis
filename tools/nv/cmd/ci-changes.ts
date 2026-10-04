// `bun nv ci-changes`: which CI lanes a push or a pull request needs, from the paths its diff touched.
//
//     bun nv ci-changes --base HEAD~1    what CI would run for the last commit
//     bun nv ci-changes --base <sha>     ... for any range ending at HEAD
//     bun nv ci-changes --full           every lane, as the nightly and the release run it
//     bun nv ci-changes --files -        lanes for a path list on stdin, touching no git
//     bun nv ci-changes --lanes          each lane and the prefixes it matches
//
// `.github/workflows/ci.yml`'s `changes` job runs this, and every other job in that file gates on one
// of the booleans it emits. `LANES` below is that policy's only home -- `rule:testing/ci-lanes` is the
// reasoning, and the workflow holds no `paths:` filter of its own.
//
// Inside Actions it takes `BASE` and `FULL` from the environment and appends `name=value` lines to
// `$GITHUB_OUTPUT`; run by hand it prints the same lines to stdout, so the two cannot disagree.
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
import { closure, type DepKind, type Graph } from "../keys/graph.ts";
import { ROOT } from "../lib/paths.ts";
import { run as runProc } from "../lib/proc.ts";
import { ArgError, parseArgs } from "../lib/py.ts";

export const summary = "which CI lanes a diff needs: nv ci-changes --base <sha> | --full | --files <file> | --lanes";

const USAGE = "usage: nv ci-changes [-h] [--base BASE] [--full] [--files FILE] [--lanes]";

// One entry per gate a job in ci.yml spells as `needs.changes.outputs.<name> == 'true'`. A prefix
// ending in `/` matches everything beneath it; anything else matches that one path exactly.
//
// `.github/workflows/` is in every lane on purpose: a run that edits the workflows is the one run
// where the lanes themselves are what needs proving.
const LANES: Record<string, string[]> = {
  // Any input to a cargo build. Gates the Linux build/test/conformance leg and `lint`.
  code: [
    "crates/", "benches/", "tests/", "examples/", "fuzz/", "tools/",
    "Cargo.toml", "Cargo.lock", "rustfmt.toml", "rust-toolchain.toml", "deny.toml",
    ".github/workflows/",
  ],
  // The crates that emit or execute machine code, plus the probes that measure them. These are where
  // a use-after-free lives, and they are the only inputs a cost baseline has, so this gates ASAN and
  // the release-profile guard run. It never narrows the platform matrix: every platform runs whenever
  // any code changes, because which host a contributor pushes from is unknowable.
  native: [
    "crates/nvs-runtime/", "crates/nvs-codegen/", "crates/nvs-stdlib/", "crates/nvs-host/",
    "benches/abi-probe/", "Cargo.lock", "rust-toolchain.toml",
    ".github/workflows/",
  ],
  // Where `wasmtime` enters the tree, and so the only thing `extension-sandbox` can observe.
  probe: ["benches/abi-probe/", ".github/workflows/"],
  // The VS Code client, and every crate with it: the protocol suite spawns the real `nvs lsp` and the
  // host suite points a throwaway profile at the binary the same run built
  // (`rule:ide/headless-gates-the-loop-the-host-run-gates-the-milestone`), so a change anywhere in the
  // workspace can change what either of them answers. Gates `extension` and `extension-host`.
  editor: ["editors/", "crates/", "Cargo.toml", "Cargo.lock", ".github/workflows/"],
  // What `cargo deny` and the attribution diff read. Their answer cannot differ in a run that changed
  // none of these (ADR 0068 § Verification).
  deps: [
    "Cargo.toml", "Cargo.lock", "deny.toml", "THIRD-PARTY-LICENSES.txt",
    "tools/nv/cmd/gen-attribution.ts", ".github/workflows/",
  ],
  // docs/novis.md and the website's `Core` pages are generated from the binary's registry and the
  // chapters, so either side moving can make a committed file stale. Gates `reference`.
  refdoc: [
    "crates/", "docs/reference/", "docs/novis.md", "tools/nv/cmd/reference.ts",
    "docs/spec/", "data/spec/", "website/", "tools/nv/renderers/", ".github/workflows/",
  ],
  // What `bun nv proofs --verify` reads: the binary's inputs, every feature's examples, attacks, benches
  // and chapters, the perf ledger, the proof policy and the gap records a `proof: gap` marker names,
  // and what `bun nv site --check` reads beside them: the website's pages, snippets and lock.
  // Gates `proofs`.
  proofs: [
    "crates/", "benches/", "tests/", "tools/", "Cargo.toml", "Cargo.lock", "rust-toolchain.toml",
    "docs/examples/", "docs/reference/", "docs/perf/", "data/proofs/", "data/gaps/",
    "website/", ".github/workflows/",
  ],
  // The five-driver matrix against real servers: the harness that points each driver at a container,
  // the compose file those containers come from, and -- added by `dbLane` below -- every package
  // `bun nv db-matrix` runs the tests of and every package those are compiled against. Gates
  // `database`, the one job that needs a daemon.
  db: ["tests/db/", "tools/nv/cmd/db-matrix.ts", "Cargo.toml", "Cargo.lock", ".github/workflows/"],
  // The slotted response's polyfill and trigger, the Chromium test that runs them, and the lockfile
  // that pins the Playwright release and so the browser it downloads. Gates `browser`.
  browser: [
    "crates/nvs-server/src/slotted/", "tools/nv/test/html-later-polyfill.test.ts",
    "package.json", "bun.lock", ".github/workflows/",
  ],
};

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
  for (const n of names) for (const d of closure(graph, n)) built.add(d);
  return [...base, ...[...built].map((n) => `${graph.get(n)!.dir}/`).sort()];
}

const ZERO = "0".repeat(40);

/** The paths `base..HEAD` touched, or `null` if `base` is not a commit that can be reached. */
async function changedPaths(base: string): Promise<string[] | null> {
  if (!base || base === ZERO) return null;
  const git = async (...args: string[]) => {
    try {
      const r = await runProc(["git", ...args]);
      return r.code === 0 ? r.stdout : null;
    } catch {
      return null;
    }
  };
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
function lanesFor(lanes: Record<string, string[]>, paths: string[] | null): [string, boolean][] {
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
    "  --full        the deep lane: every gate true, every platform in the matrix",
    "  --files FILE  read the changed paths from a file (`-` for stdin) instead of from git",
    "  --lanes       print each lane and the prefixes it matches, and nothing else",
  ].join("\n");
}

export async function run(args: string[]): Promise<number> {
  let flags: Set<string>;
  let values: Map<string, string>;
  try {
    ({ flags, values } = parseArgs(args, { flags: ["--full", "--lanes"], valued: ["--base", "--files"], order: ["--base", "--full", "--files", "--lanes"] }));
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
    return 0;
  }
  const full = flags.has("--full") || truthy(process.env.FULL);
  let paths: string[] | null;
  if (full) paths = null;
  else if (values.has("--files")) {
    const file = values.get("--files")!;
    paths = nonEmpty(file === "-" ? await Bun.stdin.text() : readFileSync(file, "utf8"));
  } else paths = await changedPaths(values.get("--base") ?? process.env.BASE ?? "");

  const lines = [...lanesFor(lanes, paths), ["deep", full] as [string, boolean]].map(([n, v]) => `${n}=${v}`);
  const sink = process.env.GITHUB_OUTPUT;
  if (sink) appendFileSync(sink, lines.join("\n") + "\n");
  for (const line of lines) console.log(line);
  return 0;
}
