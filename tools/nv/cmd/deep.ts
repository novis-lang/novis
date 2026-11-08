// `bun nv deep`: every `ci.yml` job this machine can run, in one call, one after another.
//
//     bun nv deep                  # every step; about one to two hours
//     bun nv deep --failed         # only the steps the last report names red
//     bun nv deep --only <job>     # one job's steps, such as `--only miri` or `--only test`
//     bun nv deep --list           # the steps, and the jobs and legs only CI runs
//
// **Each `ci.yml` job maps to the steps that run its commands here** (`STEPS`). A step runs the job's
// own `run:` lines, as the job runs them, on Windows or in WSL's copy of the tree (`accept/mirror.ts`
// `syncMirror`) where the job runs on Linux. A job, or a leg of one, that this machine cannot run is in
// `CI_ONLY` with its reason: macOS and aarch64. `test/deep.test.ts` fails when `ci.yml` has a job that
// neither names, so a new CI job is never skipped unseen. Where a step departs from its job, its `note`
// says how: a debug build here never takes `-p` (AGENTS.md), so a step builds the whole tree and runs
// the built binary; Miri runs each crate whole, where CI splits it into shards; the fuzzers run under
// `bun nv fuzz --all`, which goes on after a crash.
//
// **It runs every step and stops at none.** A step runs its commands in order and stops at its first
// red one, then runs its `always` commands, which stop the services it started. Before it runs, a step
// checks the programs it needs (`TOOLS`), and a missing one makes it red with the line that installs
// it. One step runs at a time, so a timing guard never shares the machine with a fuzzer or Miri. It
// prints one line per step as it ends, and at the end each red step with its command and the tail of
// its log. Each step's whole log is `.agent-tmp/deep/<step>.log`, and `REPORT` keeps every step's
// verdict, which `--failed` reads. It exits 1 when a step is red, and 2 on a usage error.
//
// Windows only, since the Linux legs run in WSL. What it spends: about one to two hours of this machine
// per full run, three target directories in WSL beside the debug one (the sanitizers' and the fuzz
// workspace's), and Docker volumes for the image job's Linux build.

import { appendFileSync, existsSync, mkdirSync, readFileSync, readdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { mirrorPath, q, syncMirror, wslPath } from "../accept/mirror.ts";
import { ROOT } from "../lib/paths.ts";
import { run as runProc } from "../lib/proc.ts";

export const summary = "runs every CI job this machine can run, one after another: nv deep [--failed | --only <job> | --list]";

const USAGE = "usage: bun nv deep [--failed | --only <job> | --list]";

/** Where each step's log and the report go, under the root. */
export const OUT = ".agent-tmp/deep";
/** The verdict of every step the last run ran, which `--failed` reads. */
export const REPORT = `${OUT}/report.json`;
/** The WSL leg's debug target directory, which `linux-test` builds into too. */
const WSL_TARGET = "/var/tmp/nvs-target-wsl";
/** How many lines of a red step's log the summary prints. */
const TAIL_LINES = 30;
/** The longest one command of a step may run: a cold build and Miri over a whole crate. */
const CMD_TIMEOUT_MS = 4 * 60 * 60 * 1000;

/** A program a step needs, how it is found and how it is installed. */
export interface Tool {
  /** Where it must be: on Windows, or inside WSL. */
  where: "windows" | "wsl";
  /** The `bash` test that passes when it is there, run on that side. */
  probe: string;
  /** The line that installs it, which the step's error prints. */
  install: string;
}

/** Every program a step may need that a clone does not bring. `commands.md` § *Without looper* lists the same lines. */
export const TOOLS: Record<string, Tool> = {
  "wsl bun": { where: "wsl", probe: "command -v bun", install: "in WSL: curl -fsSL https://bun.sh/install | bash" },
  "wsl node": { where: "wsl", probe: "command -v node", install: "in WSL: sudo apt-get install -y nodejs npm (Node 22 or newer)" },
  "wsl nightly miri": {
    where: "wsl",
    probe: "cargo +nightly miri --version",
    install: "in WSL: rustup toolchain install nightly --component miri rust-src && cargo +nightly miri setup",
  },
  "wsl cargo-nextest": { where: "wsl", probe: "command -v cargo-nextest", install: "in WSL: cargo install cargo-nextest@0.9.148 --locked" },
  "wsl cargo-fuzz": { where: "wsl", probe: "command -v cargo-fuzz", install: "in WSL: cargo install cargo-fuzz --version 0.13.2 --locked" },
  "wsl valgrind": { where: "wsl", probe: "command -v valgrind", install: "in WSL: sudo apt-get install -y --no-install-recommends valgrind" },
  "wsl docker": {
    where: "wsl",
    probe: "docker info >/dev/null 2>&1",
    install: "turn on Docker Desktop's WSL integration for the distro (Settings > Resources > WSL integration)",
  },
  "cargo-nextest": { where: "windows", probe: "command -v cargo-nextest", install: "cargo install cargo-nextest@0.9.148 --locked" },
  "cargo-deny": { where: "windows", probe: "command -v cargo-deny", install: "cargo install cargo-deny --locked" },
  docker: { where: "windows", probe: "docker info >/dev/null 2>&1", install: "start Docker Desktop, with its Linux engine" },
  node: { where: "windows", probe: "command -v node && command -v npm", install: "install Node 22 or newer from https://nodejs.org" },
  "wit-bindgen": { where: "windows", probe: "command -v wit-bindgen", install: "cargo install wit-bindgen-cli --version 0.63.0 --locked" },
  "wasi-sdk": {
    where: "windows",
    probe: "[ -n \"$WASI_SDK_PATH\" ] && [ -d \"$WASI_SDK_PATH\" ]",
    install: "unpack wasi-sdk 34 for x86_64-windows from https://github.com/WebAssembly/wasi-sdk/releases and set WASI_SDK_PATH to it",
  },
  redis: {
    where: "windows",
    probe: "(exec 3<>/dev/tcp/127.0.0.1/16379) 2>/dev/null",
    install: "start Redis on port 16379, as CI's test job does: redis-server --port 16379",
  },
};

/** One command of a step: a program on Windows, or a `bash` line in WSL's copy of the tree. */
export type Cmd =
  | { argv: string[]; env?: Record<string, string>; cwd?: string; glob?: string }
  | { wsl: string; env?: Record<string, string> };

/** One local step: a job, or one leg of it, as this machine runs it. */
export interface Step {
  /** Unique, and what `--only` and the report name. */
  id: string;
  /** The `ci.yml` job it runs. */
  job: string;
  tools?: (keyof typeof TOOLS)[];
  cmds: Cmd[];
  /** Run after `cmds` however they ended, such as stopping the services the step started. */
  always?: Cmd[];
  /** How it departs from the job's own `run:` lines, when it does. */
  note?: string;
}

/** `cargo test` under a sanitizer in WSL, in a target directory of its own so the debug one is not rebuilt. */
const ASAN_ENV = { RUSTFLAGS: "-Zsanitizer=address", CARGO_TARGET_DIR: `${WSL_TARGET}-asan` };
const DEBUG_NVS = join(ROOT, "target", "debug", "nvs.exe");
const PROOF_NVS = "target/proof/nvs.exe";
const SITE_CHECKS = ["structure", "snippets", "stale", "questions", "syntax", "guides", "tools", "in-depth", "roadmap", "prose", "apps", "code"];
/** The Linux debug binary in WSL's copy. */
const WSL_NVS = `${WSL_TARGET}/debug/nvs`;
const BUN_INSTALL = "bun install --frozen-lockfile";
const MIRI_UNITS = "-Zmiri-disable-isolation -Zmiri-ignore-leaks -Zmiri-deterministic-floats";
/** The separate workspaces `supply-chain` checks for advisories, as `ci.yml` lists them. */
const ADVISORY_MANIFESTS = [
  "extensions/image", "extensions/intl", "extensions/pdf", "extensions/spreadsheet",
  "fuzz", "tools/covwrap", "tools/nv-scan", "website/guides/highlight-extension",
];

/** The `--test <name>` list of `benches/abi-probe/tests/`, as `cost-guards` builds it. */
export function abiTests(root: string = ROOT): string[] {
  return readdirSync(join(root, "benches", "abi-probe", "tests"))
    .filter((f) => f.endsWith(".rs"))
    .sort()
    .flatMap((f) => ["--test", f.slice(0, -3)]);
}

/** The conformance runs `test` splits into two halves, with the binary at `nvs`. */
function conformance(nvs: string): string[][] {
  return [
    [nvs, "test", "--shared-worker", "tests/conformance/"],
    [nvs, "test", "--opt-level", "none", "tests/conformance/"],
    [nvs, "test", "--probes", "tests/conformance/"],
  ];
}

/** Every local step, in the order a run takes them: the quick file checks first, the long runs last. */
export function steps(root: string = ROOT): Step[] {
  const abi = abiTests(root);
  const guards = (where: "windows" | "wsl"): Cmd[] => {
    const lines = [
      ["cargo", "build", "--release", "-p", "nvs-abi-probe", "-p", "nvs-cli", "--bin", "nvs", ...abi],
      ["cargo", "test", "--release", "-p", "nvs-abi-probe", "-p", "nvs-cli", "--lib", ...abi],
      ["cargo", "test", "--release", "-p", "nvs-abi-probe", "-p", "nvs-cli", "--bin", "nvs", "--", "by_the_margin_this_test_names"],
      ["cargo", "test", "--release", "-p", "nvs-abi-probe", "-p", "nvs-cli", "--bin", "nvs", "--", "--ignored",
        "live_bytes_stay_flat_while_one_core_serves_thousands_of_requests"],
    ];
    return where === "windows" ? lines.map((argv) => ({ argv })) : lines.map((argv) => ({ wsl: argv.map(q).join(" ") }));
  };
  return [
    {
      id: "docs",
      job: "docs",
      cmds: [
        ["check"], ["render", "--check"], ["rules", "--check"], ["rules", "--render", "--check"], ["links"],
        ["layout", "--check"], ["release", "--check"],
      ].map((a) => ({ argv: ["bun", "nv", ...a] })).concat([{ argv: ["bun", "test", "tools/nv/test/no-echo-loop-child.test.ts"] }]),
    },
    {
      id: "lint",
      job: "lint",
      cmds: [
        { argv: ["cargo", "fmt", "--all", "--check"] },
        { argv: ["bun", "nv", "lints", "--check"] },
        { wsl: "cargo clippy --workspace --all-targets -- -D warnings" },
        { wsl: "cargo doc --no-deps --workspace", env: { RUSTDOCFLAGS: "-A rustdoc::private_intra_doc_links -D warnings" } },
      ],
      note: "clippy and rustdoc run in WSL, on Linux as the job does, so code under cfg(unix) is linted",
    },
    {
      id: "attribution",
      job: "attribution",
      tools: ["node"],
      cmds: [
        { argv: ["cargo", "fetch", "--locked"] },
        { argv: ["npm", "ci", "--ignore-scripts"], cwd: "editors/vscode" },
        { argv: ["bun", "nv", "gen-attribution", "--check"] },
        { argv: ["bun", "test", "tools/nv/test/attribution-vscode.test.ts"] },
        { argv: ["bun", "nv", "gen-attribution", "--check-c-deps"] },
      ],
    },
    {
      id: "supply-chain",
      job: "supply-chain",
      tools: ["cargo-deny"],
      cmds: [
        { argv: ["cargo", "deny", "check"] },
        ...ADVISORY_MANIFESTS.map((d) => ({
          argv: ["cargo", "deny", "--all-features", "--manifest-path", `${d}/Cargo.toml`, "--config", "deny.toml", "check", "advisories"],
        })),
      ],
      note: "the job pins cargo-deny by the action's commit; here it is the installed cargo-deny",
    },
    {
      id: "reference",
      job: "reference",
      cmds: [
        { argv: ["cargo", "build"] },
        { argv: ["bun", "nv", "reference"], env: { NVS_BIN: DEBUG_NVS } },
        { argv: ["bun", "nv", "render", "--website", "--check"], env: { NVS_BIN: DEBUG_NVS } },
      ],
    },
    { id: "webp-lib", job: "webp-lib", cmds: [{ argv: ["bun", "nv", "webp-lib", "--check"] }, { argv: ["bun", "nv", "webp-lib", "--verify"] }] },
    {
      id: "extension-sandbox",
      job: "extension-sandbox",
      cmds: [
        { argv: ["cargo", "test", "--release", "-p", "nvs-abi-probe", "--features", "wasm-probe"] },
        { argv: ["cargo", "clippy", "--workspace", "--all-targets", "--features", "nvs-abi-probe/wasm-probe", "--", "-D", "warnings"] },
      ],
    },
    {
      id: "ext-templates",
      job: "ext-templates",
      tools: ["wit-bindgen", "wasi-sdk"],
      cmds: [{ argv: ["cargo", "build"] }, { argv: ["bun", "nv", "ext-templates"], env: { NVS_BIN: DEBUG_NVS } }],
      note: "the Windows leg; `bun nv ext-templates` runs the job's three runs in one",
    },
    {
      id: "extension",
      job: "extension",
      tools: ["node"],
      cmds: [
        { argv: ["cargo", "build"] },
        ...[["ci"], ["run", "lint"], ["run", "test:headless"], ["run", "package"]].map((a) => ({
          argv: ["npm", ...a], cwd: "editors/vscode", env: { NVS_BIN: DEBUG_NVS },
        })),
      ],
      note: "the Windows leg",
    },
    {
      id: "extension-host",
      job: "extension-host",
      tools: ["node"],
      cmds: [
        { argv: ["cargo", "build"] },
        { argv: ["npm", "ci"], cwd: "editors/vscode" },
        { argv: ["npm", "run", "test:host", "--", "--nvs", DEBUG_NVS], cwd: "editors/vscode" },
      ],
      note: "on Windows, where VS Code opens a window of its own; the job runs it under xvfb on Linux",
    },
    {
      id: "browser",
      job: "browser",
      cmds: [
        { argv: ["node", "node_modules/playwright-core/cli.js", "install", "--only-shell", "chromium"], env: { PLAYWRIGHT_BROWSERS_PATH: "0" } },
        { argv: ["bun", "test", "tools/nv/test/html-later-polyfill.test.ts"], env: { NV_BROWSER: "1" } },
        { argv: ["bun", "test", "tools/nv/test/replay-browser.test.ts"], env: { NV_BROWSER: "1" } },
        { argv: ["bun", "nv", "viewer", "--verify"], env: { NV_BROWSER: "1" } },
        { argv: ["bun", "test"], glob: "viewer/test/*.browser.test.ts", env: { NV_BROWSER: "1" } },
        { argv: ["cargo", "build"] },
        { argv: ["bun", "test", "tools/nv/test/registry-browser.test.ts"], env: { NV_BROWSER: "1", NVS_BIN: DEBUG_NVS } },
        { argv: ["bun", "test", "tools/nv/test/realtime-browser.test.ts"], env: { NV_BROWSER: "1", NVS_BIN: DEBUG_NVS } },
      ],
      tools: ["node"],
    },
    {
      id: "database",
      job: "database",
      tools: ["docker"],
      cmds: [
        { argv: ["bun", "nv", "db-matrix", "--all"] },
        { argv: ["docker", "compose", "-f", "tests/db/compose.yaml", "up", "-d", "--wait", "samba-ad"] },
        { argv: ["docker", "compose", "-f", "tests/db/compose.yaml", "cp", "samba-ad:/certs/ca.crt", "tests/db/ca.crt"] },
        { argv: ["cargo", "test", "--test", "samba"], env: { NVS_LDAP_SAMBA: "1" } },
      ],
      always: [{ argv: ["bun", "nv", "db-matrix", "--down"] }],
    },
    {
      id: "docker",
      job: "docker",
      tools: ["docker"],
      cmds: [
        {
          argv: ["docker", "run", "--rm", "-v", `${ROOT}:/src:ro`, "-v", "nvs-deep-target:/target", "-v", "nvs-deep-cargo:/usr/local/cargo/registry",
            "-v", "nvs-deep-rustup:/usr/local/rustup", "-v", `${join(ROOT, "docker", "bin")}:/out`, "-w", "/src", "-e", "CARGO_TARGET_DIR=/target",
            "rust:1.99.1-bookworm", "bash", "-c",
            "cargo build --locked --bin nvs && mkdir -p /out/amd64 /out/notices && cp /target/debug/nvs /out/amd64/nvs && " +
            "cp LICENSE THIRD-PARTY-LICENSES.txt README.md CHANGELOG.md /out/notices/"],
        },
        ...["distroless", "debian"].flatMap((v): Cmd[] => [
          { argv: ["docker", "build", "docker", "--target", v, "--platform", "linux/amd64", "-t", `novis-smoke:${v}`] },
          { argv: ["bash", "tests/docker/smoke.sh", `novis-smoke:${v}`] },
        ]),
      ],
      note: "the binary is built in a Debian 12 container, as old a glibc as the job's ubuntu-22.04 runner gives, since WSL's is newer than the images'",
    },
    {
      id: "cost-guards-windows",
      job: "cost-guards",
      cmds: [...guards("windows"), { argv: ["bun", "nv", "bench", "--warm-start", "--max-work-ms", "9"] }],
    },
    { id: "cost-guards-linux", job: "cost-guards", cmds: guards("wsl"), note: "the bench line runs in the Windows step" },
    {
      id: "proofs-windows",
      job: "proofs",
      cmds: [
        { argv: ["cargo", "build", "--profile", "proof", "--bin", "nvs"] },
        { argv: ["cargo", "test", "--profile", "proof", "--test", "forced_unwind"] },
        { argv: ["bun", "nv", "proofs", "--verify", "--nvs", PROOF_NVS] },
        { argv: ["bun", "nv", "site", "--check", ...SITE_CHECKS, "--nvs", PROOF_NVS] },
      ],
    },
    {
      id: "proofs-linux",
      job: "proofs",
      tools: ["wsl bun", "wsl node", "wsl docker"],
      cmds: [
        { wsl: "cargo build --profile proof --bin nvs" },
        { wsl: "cargo test --profile proof --test forced_unwind" },
        { wsl: BUN_INSTALL },
        { wsl: `bun nv proofs --verify --nvs ${WSL_TARGET}/proof/nvs` },
        { wsl: `bun nv pdf-validate --bin ${WSL_TARGET}/proof/nvs` },
        { wsl: `bun nv site --check ${SITE_CHECKS.join(" ")} --nvs ${WSL_TARGET}/proof/nvs` },
        { wsl: "node node_modules/playwright-core/cli.js install --with-deps --only-shell chromium", env: { PLAYWRIGHT_BROWSERS_PATH: "0" } },
        { wsl: "cd editors/vscode && npm ci --ignore-scripts" },
        { wsl: `bun nv site --check shots --nvs ${WSL_TARGET}/proof/nvs`, env: { NV_BROWSER: "1", PLAYWRIGHT_BROWSERS_PATH: "0" } },
      ],
    },
    {
      id: "test-windows",
      job: "test",
      tools: ["cargo-nextest", "redis"],
      cmds: [
        { argv: ["bun", "nv", "checkout-acl"] },
        { argv: ["cargo", "build", "--workspace", "--all-targets"] },
        { argv: ["cargo", "nextest", "run", "--profile", "ci", "--workspace", "--lib", "--bins", "--tests", "--examples"] },
        { argv: ["cargo", "test", "--workspace", "--benches"] },
        { argv: ["cargo", "test", "--workspace", "--doc"] },
        ...conformance(DEBUG_NVS).map((argv) => ({ argv })),
        { argv: [DEBUG_NVS, "test", "tests/conformance/"], env: { NOVIS_CHECK_TYPES: "1" } },
      ],
      note: "the three nextest shards run as one, and the conformance runs use the built binary, not `cargo run -p`",
    },
    {
      id: "test-linux",
      job: "test",
      tools: ["wsl cargo-nextest", "wsl docker"],
      cmds: [
        { wsl: "docker compose -f tests/db/compose.yaml up -d --wait redis samba-ad" },
        { wsl: "docker compose -f tests/db/compose.yaml cp samba-ad:/certs/ca.crt tests/db/ca.crt" },
        { wsl: "cargo build --workspace --all-targets" },
        { wsl: "cargo nextest run --profile ci --workspace --lib --bins --tests --examples", env: { NVS_LDAP_SAMBA: "1" } },
        { wsl: "cargo test --workspace --benches" },
        { wsl: "cargo test --workspace --doc" },
        ...conformance(WSL_NVS).map((argv) => ({ wsl: argv.map(q).join(" ") })),
        { wsl: `${WSL_NVS} test tests/conformance/`, env: { NOVIS_CHECK_TYPES: "1" } },
      ],
      always: [{ wsl: "docker compose -f tests/db/compose.yaml down --volumes" }],
      note: "the conformance runs use the built binary, not `cargo run -p`",
    },
    { id: "flood", job: "flood", tools: ["docker"], cmds: [{ argv: ["bun", "nv", "bench-compare", "--scenarios", "--smoke", "--row", "flood"] }] },
    {
      id: "valgrind",
      job: "valgrind",
      tools: ["wsl valgrind", "wsl bun"],
      cmds: [{ wsl: "cargo build --bin nvs" }, { wsl: BUN_INSTALL }, { wsl: `bun nv valgrind --nvs ${WSL_NVS}` }],
    },
    {
      id: "asan",
      job: "asan",
      cmds: [
        {
          wsl: "cargo +nightly test --target x86_64-unknown-linux-gnu --features nvs-runtime/sanitizer,nvs-host/asan --tests -p nvs-runtime -p nvs-stdlib",
          env: { ...ASAN_ENV, ASAN_OPTIONS: "detect_leaks=0:detect_stack_use_after_return=1:detect_odr_violation=1" },
        },
        {
          wsl: "cargo +nightly test --target x86_64-unknown-linux-gnu --features nvs-runtime/sanitizer --tests -p nvs-codegen",
          env: { ...ASAN_ENV, ASAN_OPTIONS: "detect_leaks=0:detect_stack_use_after_return=0:detect_odr_violation=1" },
        },
      ],
    },
    { id: "tsan", job: "tsan", cmds: [{ wsl: "bash tools/tsan.sh", env: { CARGO_TARGET_DIR: `${WSL_TARGET}-tsan` } }] },
    {
      id: "miri",
      job: "miri",
      tools: ["wsl nightly miri"],
      cmds: [
        { wsl: "cargo +nightly miri test -p nvs-diagnostics -p nvs-syntax -p nvs-hir", env: { MIRIFLAGS: "-Zmiri-disable-isolation" } },
        {
          wsl: "cargo +nightly miri test -p nvs-types --lib -- ty::tests:: layout::tests:: generics::tests:: lower::tests:: | tee /tmp/nvs-deep-miri.log && " +
            "grep -q '^test result: ok\\. [1-9]' /tmp/nvs-deep-miri.log && " +
            "cargo +nightly miri test -p nvs-ir --lib -- --exact " +
            "lower::tests::a_generator_lowers_to_a_factory_a_state_class_and_a_resumption_switch " +
            "lower::tests::an_anon_fn_lowers_to_a_captured_environment_object_and_an_invoke_method | tee /tmp/nvs-deep-miri.log && " +
            "grep -q '^test result: ok\\. [1-9]' /tmp/nvs-deep-miri.log",
          env: { MIRIFLAGS: "-Zmiri-disable-isolation" },
        },
      ],
    },
    {
      id: "miri-units",
      job: "miri-units",
      tools: ["wsl nightly miri", "wsl cargo-nextest"],
      cmds: [
        { wsl: "cargo +nightly miri nextest run --profile miri -p nvs-runtime --lib --features nvs-runtime/sanitizer", env: { MIRIFLAGS: MIRI_UNITS } },
        { wsl: "cargo +nightly miri nextest run --profile miri -p nvs-stdlib --lib", env: { MIRIFLAGS: MIRI_UNITS } },
      ],
      note: "each crate runs whole; the job splits nvs-runtime into 2 shards and nvs-stdlib into 12",
    },
    { id: "fuzz-smoke", job: "fuzz-smoke", cmds: [{ argv: ["bun", "nv", "fuzz", "--all"] }], note: "`bun nv fuzz --all` goes on after a crash, and runs each target longer" },
  ];
}

/** The jobs, and the legs of jobs, this machine does not run, each with its reason. */
export const CI_ONLY: Record<string, string> = {
  changes: "it picks the jobs a push runs from its diff; a local run runs every job",
  "unsafe-audit": "it writes cargo-geiger's counts to the run summary and is never red",
  "test (macos-aarch64, linux-aarch64)": "no such machine here",
  "cost-guards (macos-aarch64)": "no such machine here",
  "proofs (macos-aarch64)": "no such machine here",
  "ext-templates (linux-x86_64, macos-aarch64)": "the Windows leg builds the same templates; no macOS machine here",
  "extension (linux-x86_64, macos-aarch64)": "the Windows leg runs the same suite; no macOS machine here",
};

/** The `ci.yml` job a `CI_ONLY` entry names: its key up to the legs in brackets. */
export const jobOf = (key: string): string => key.replace(/ \(.*$/, "");

/** One step's verdict in the report. */
export interface Verdict {
  id: string;
  job: string;
  ok: boolean;
  seconds: number;
  /** The command that went red, as a line, or "" when the step is green. */
  failed: string;
  log: string;
}

/** The steps a run takes: every one, the ones `last` names red, or the ones of `only`. Returns the reason
 * when `only` names no step. */
export function selected(all: Step[], opts: { failed: boolean; only: string | null }, last: Verdict[]): Step[] | string {
  if (opts.only !== null) {
    const picked = all.filter((s) => s.job === opts.only || s.id === opts.only);
    return picked.length > 0 ? picked : `no step runs the job \`${opts.only}\`; \`bun nv deep --list\` lists them`;
  }
  if (opts.failed) {
    const red = new Set(last.filter((v) => !v.ok).map((v) => v.id));
    return all.filter((s) => red.has(s.id));
  }
  return all;
}

/** A command as one line, for the report. */
export function cmdLine(c: Cmd): string {
  const env = Object.entries(c.env ?? {}).map(([k, v]) => `${k}=${q(v)} `).join("");
  if ("wsl" in c) return `wsl: ${env}${c.wsl}`;
  const where = c.cwd ? `(in ${c.cwd}) ` : "";
  return `${where}${env}${c.argv.map(q).join(" ")}${c.glob ? ` ${c.glob}` : ""}`;
}

/** The `bash` line that runs `c` in WSL's copy at `mirror`, into its target directory unless `c` names another. */
export function wslLine(mirror: string, c: { wsl: string; env?: Record<string, string> }): string {
  const env = { CARGO_TARGET_DIR: WSL_TARGET, INSTA_WORKSPACE_ROOT: mirror, ...c.env };
  const exports = Object.entries(env).map(([k, v]) => `export ${k}=${q(v)}; `).join("");
  return `set -o pipefail; cd ${q(mirror)} && ${exports}${c.wsl}`;
}

/** The report's text, merged over `last` so `--failed` keeps the verdicts of the steps it did not run. */
export function merged(last: Verdict[], now: Verdict[]): Verdict[] {
  const ids = new Set(now.map((v) => v.id));
  return [...last.filter((v) => !ids.has(v.id)), ...now];
}

/** The last `n` lines of `text`. */
export function tail(text: string, n: number): string {
  return text.trimEnd().split("\n").slice(-n).join("\n");
}

function readReport(): Verdict[] {
  try {
    return (JSON.parse(readFileSync(join(ROOT, REPORT), "utf8")) as { steps: Verdict[] }).steps;
  } catch {
    return [];
  }
}

const minutes = (s: number): string => `${Math.floor(s / 60)}m${String(Math.round(s % 60)).padStart(2, "0")}s`;

/** Why each tool `step` needs is missing, with its install line, or [] when all are there. */
async function missing(step: Step): Promise<string[]> {
  const out: string[] = [];
  for (const name of step.tools ?? []) {
    const t = TOOLS[name]!;
    const argv = t.where === "wsl" ? ["wsl.exe", "--exec", "bash", "-lc", t.probe] : ["bash", "-c", t.probe];
    const r = await runProc(argv, { timeoutMs: 120_000, hang: false });
    if (r.code !== 0) out.push(`${name} is missing; install it: ${t.install}`);
  }
  return out;
}

/** Runs one command of a step, appending its output to `log`, and returns its exit status. */
async function runCmd(c: Cmd, mirror: string, log: string): Promise<number> {
  appendFileSync(log, `\n$ ${cmdLine(c)}\n`);
  const onLine = (line: string): void => appendFileSync(log, `${line}\n`);
  if ("wsl" in c) {
    const r = await runProc(["wsl.exe", "--exec", "bash", "-lc", wslLine(mirror, c)], { timeoutMs: CMD_TIMEOUT_MS, hang: false, onLine });
    return r.timedOut ? 124 : r.code;
  }
  const files = c.glob ? globbed(c.glob) : [];
  const r = await runProc([...c.argv, ...files], { cwd: c.cwd ? join(ROOT, c.cwd) : ROOT, env: c.env ?? {}, timeoutMs: CMD_TIMEOUT_MS, onLine });
  if (r.hung) appendFileSync(log, `HANG: ${r.hung}\n`);
  return r.timedOut ? 124 : r.code;
}

function globbed(pattern: string): string[] {
  return [...new Bun.Glob(pattern).scanSync({ cwd: ROOT })].sort().map((f) => `./${f.replaceAll("\\", "/")}`);
}

/** Runs one step whole and returns its verdict. */
async function runStep(step: Step, mirror: string): Promise<Verdict> {
  const log = join(ROOT, OUT, `${step.id}.log`);
  writeFileSync(log, `deep: ${step.id} (ci.yml job ${step.job})${step.note ? `\nnote: ${step.note}` : ""}\n`);
  const start = Date.now();
  const verdict = (failed: string): Verdict => ({
    id: step.id, job: step.job, ok: failed === "", seconds: (Date.now() - start) / 1000, failed, log: `${OUT}/${step.id}.log`,
  });
  const gone = await missing(step);
  if (gone.length > 0) {
    appendFileSync(log, `${gone.join("\n")}\n`);
    return verdict(gone.join("; "));
  }
  if (step.cmds.some((c) => "wsl" in c) || (step.always ?? []).some((c) => "wsl" in c)) {
    const synced = await syncMirror(mirror);
    if (synced !== "") {
      appendFileSync(log, `the WSL copy of the tree could not be synced: ${synced}\n`);
      return verdict(`sync the WSL copy: ${synced}`);
    }
  }
  let failed = "";
  for (const c of step.cmds) {
    const code = await runCmd(c, mirror, log);
    if (code !== 0) {
      failed = `${cmdLine(c)} (exit ${code})`;
      break;
    }
  }
  for (const c of step.always ?? []) await runCmd(c, mirror, log);
  return verdict(failed);
}

function list(all: Step[]): void {
  for (const s of all) {
    console.log(`${s.id.padEnd(22)} job ${s.job}${s.note ? `: ${s.note}` : ""}`);
    for (const c of s.cmds) console.log(`    ${cmdLine(c)}`);
  }
  console.log("\nCI only:");
  for (const [k, why] of Object.entries(CI_ONLY)) console.log(`  ${k}: ${why}`);
}

export async function run(args: string[]): Promise<number> {
  if (args[0] === "--help" || args[0] === "-h") {
    console.log(`${USAGE}\n\n${summary}`);
    return 0;
  }
  const failedOnly = args.length === 1 && args[0] === "--failed";
  const only = args.length === 2 && args[0] === "--only" ? args[1]! : null;
  const listing = args.length === 1 && args[0] === "--list";
  if (args.length > 0 && !failedOnly && only === null && !listing) {
    console.error(`${USAGE}\nnv deep: error: unknown arguments ${args.join(" ")}`);
    return 2;
  }
  const all = steps();
  if (listing) {
    list(all);
    return 0;
  }
  if (process.platform !== "win32") {
    console.error("nv deep: this host is not Windows; it runs the Linux legs in WSL");
    return 2;
  }
  const last = readReport();
  const picked = selected(all, { failed: failedOnly, only }, last);
  if (typeof picked === "string") {
    console.error(`nv deep: error: ${picked}`);
    return 2;
  }
  if (picked.length === 0) {
    console.log("nv deep: the last report names no red step");
    return 0;
  }
  mkdirSync(join(ROOT, OUT), { recursive: true });
  const mirror = mirrorPath(WSL_TARGET, wslPath(ROOT));
  const start = Date.now();
  const now: Verdict[] = [];
  for (const step of picked) {
    console.log(`deep: ${step.id} ...`);
    const v = await runStep(step, mirror);
    now.push(v);
    console.log(`deep: ${step.id} ${v.ok ? "green" : "RED"} in ${minutes(v.seconds)}`);
    writeFileSync(join(ROOT, REPORT), `${JSON.stringify({ steps: merged(last, now) }, null, 2)}\n`);
  }
  const red = now.filter((v) => !v.ok);
  console.log(`\ndeep: ${now.length - red.length} green, ${red.length} red, in ${minutes((Date.now() - start) / 1000)}`);
  for (const v of red) {
    const logText = existsSync(join(ROOT, v.log)) ? readFileSync(join(ROOT, v.log), "utf8") : "";
    console.log(`\nRED ${v.id} (job ${v.job}): ${v.failed}\nlog: ${v.log}\n${tail(logText, TAIL_LINES)}`);
  }
  return red.length === 0 ? 0 : 1;
}
