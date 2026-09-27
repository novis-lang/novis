// The plan's checks as groups of atoms (`select/checks.ts`): which atoms each shape of check is made of,
// what a check's own atom is defined by, what a command that records nothing is keyed on, and the heavy
// set's named prediction.

import { describe, expect, test } from "bun:test";
import type { Check } from "../driver/accept.ts";
import type { Graph } from "../keys/graph.ts";
import { checkDef, commandKeys, grouped, harnessArgs, heavyCrates, heavyKeys, heavyTwin, legAtoms, type PlanContext, planContext, predicted } from "../select/checks.ts";
import { proofReadsSlot, SelectStore } from "../select/store.ts";
import { scratch } from "./scratch.ts";

const pkg = (name: string, dir: string, deps: [string, "normal" | "dev"][] = [], targets = [{ kind: "lib", name: name.replace(/-/g, "_"), src: `${dir}/src/lib.rs`, test: true }]) => [name, { name, dir, deps: new Map(deps), targets }] as const;
const graph: Graph = new Map([
  pkg("nvs-syntax", "crates/nvs-syntax"),
  pkg("nvs-host", "crates/nvs-host", [["nvs-syntax", "normal"]]),
  pkg("nvs-cli", "crates/nvs-cli", [["nvs-host", "normal"]], [
    { kind: "lib", name: "nvs_cli", src: "crates/nvs-cli/src/lib.rs", test: true },
    { kind: "bin", name: "nvs", src: "crates/nvs-cli/src/main.rs", test: true },
    { kind: "test", name: "live_config", src: "crates/nvs-cli/tests/live_config.rs", test: true },
  ]),
]);

const ctx: PlanContext = {
  graph,
  cases: ["tests/conformance/a.nvst", "tests/conformance/sub/b.nvst", "tests/differential/c.nvst"],
  nvTests: ["tools/nv/test/bg.test.ts", "tools/nv/test/guard.test.ts"],
  groupDirs: new Map([["Core\\Str", ["docs/examples/core/Str/length", "tests/hostile/core/Str/length"]]]),
  proofs: ["docs/examples/core/Str/length/01-basics.nvs", "tests/hostile/core/Str/length/01-x.nvs", "docs/examples/core/Arr/map/01-a.nvs"],
};

const check = (over: Partial<Check>): Check => ({ id: "c", kind: "command", stage: 1, ...over });

describe("a check as atoms", () => {
  test("a fixture, a command and a bun nv check each have an atom of their own", () => {
    expect(grouped(check({ kind: "exact", file: "examples/a.nvs" }), ctx)).toMatchObject({ how: "fixture", atoms: ["check:c"] });
    expect(grouped(check({ argv: ["{nvs}", "ast", "--json", "examples/hello.nvs"] }), ctx)).toMatchObject({ how: "command", atoms: ["check:c"] });
    expect(grouped(check({ argv: ["bun", "nv", "rules", "--check"] }), ctx)).toMatchObject({ how: "nv", atoms: ["nv:c"] });
  });

  test("an nvs test suite is the cases under its tree, and another suite is a command", () => {
    const g = grouped(check({ kind: "nvs-suite", args: ["test", "tests/conformance/"] }), ctx);
    expect(g).toMatchObject({ how: "cases", atoms: ["case:tests/conformance/a.nvst", "case:tests/conformance/sub/b.nvst"] });
    expect(g.own).toBeUndefined();
    expect(grouped(check({ kind: "nvs-suite", args: ["lsp-test", "tests/lsp/"] }), ctx).how).toBe("command");
  });

  test("a cargo test check is the binaries it names, and --workspace is every one", () => {
    expect(grouped(check({ kind: "cargo-named", args: ["test", "-p", "nvs-cli", "--bin", "nvs", "units"] }), ctx)).toMatchObject({ how: "tests", tests: ["nvs-cli bin nvs"] });
    expect(grouped(check({ kind: "cargo-named", args: ["test", "-p", "nvs-cli"] }), ctx).tests).toEqual(["nvs-cli lib nvs_cli", "nvs-cli bin nvs", "nvs-cli test live_config"]);
    expect(grouped(check({ kind: "cargo-named", args: ["test", "--workspace"] }), ctx).atoms).toHaveLength(5);
    expect(grouped(check({ kind: "cargo-named", args: ["test", "-p", "nvs-cli", "--features", "x"] }), ctx).how).toBe("command");
  });

  test("a proofs check is its own atom and the programs of its groups, and a group never run is unknown", () => {
    const g = grouped(check({ argv: ["bun", "nv", "proofs", "--verify", "--group", "Core\\Str"] }), ctx);
    expect(g).toMatchObject({ how: "proofs", atoms: ["nv:c", "proof:docs/examples/core/Str/length/01-basics.nvs", "proof:tests/hostile/core/Str/length/01-x.nvs"], unknown: false });
    expect(grouped(check({ argv: ["bun", "nv", "proofs", "--verify", "--group", "Core\\Arr"] }), ctx)).toMatchObject({ how: "proofs", atoms: ["nv:c"], unknown: true });
  });

  test("a proofs check's programs are read from disk: a new one in a group's directory is its atom, and one in no group's directory makes every group unknown", () => {
    const t = scratch();
    const store = new SelectStore(":memory:", "test-os");
    try {
      t.put("docs/examples/core/Str/length/01-basics.nvs", "");
      t.put("docs/examples/core/Str/length/02-new.nvs", "");
      store.recordRun("proof:docs/examples/core/Str/length/01-basics.nvs", { def: "", verdict: "green", keys: new Map([["file:x", ""]]) });
      store.putVerdict(proofReadsSlot("Core\\Str"), "", JSON.stringify(["docs/examples/core/Str/length", "benches/members/core/Str/length.nvs"]));
      const c = check({ argv: ["bun", "nv", "proofs", "--verify", "--group", "Core\\Str"] });
      const known = planContext(store, null, t.root);
      expect(known.homeless).toBe(false);
      expect(grouped(c, known)).toMatchObject({ atoms: ["nv:c", "proof:docs/examples/core/Str/length/01-basics.nvs", "proof:docs/examples/core/Str/length/02-new.nvs"], unknown: false });
      t.put("docs/examples/core/Str/pad/01-new-feature.nvs", "");
      const stray = planContext(store, null, t.root);
      expect(stray.homeless).toBe(true);
      expect(grouped(c, stray).unknown).toBe(true);
    } finally {
      store.close();
      t.cleanup();
    }
  });

  test("the tools' gate is tsc and every test file, and one test file is its own atom", () => {
    expect(grouped(check({ argv: ["bun", "nv", "selftest"] }), ctx).atoms).toEqual(["step:nv", "nvtest:tools/nv/test/bg.test.ts", "nvtest:tools/nv/test/guard.test.ts"]);
    expect(grouped(check({ argv: ["bun", "test", "tools/nv/test/bg.test.ts"] }), ctx)).toMatchObject({ how: "nvtest", atoms: ["nvtest:tools/nv/test/bg.test.ts"] });
  });

  test("a heavy check is a heavy atom whatever its shape", () => {
    expect(grouped(check({ kind: "cargo-named", args: ["test", "--release", "-p", "nvs-cli"] }), ctx).atoms).toEqual(["heavy:c"]);
    expect(grouped(check({ argv: ["bun", "nv", "bench", "--guard"] }), ctx).atoms).toEqual(["heavy:c"]);
    expect(grouped(check({ kind: "exact", file: "examples/a.nvs", memoize: false }), ctx).atoms).toEqual(["heavy:c"]);
  });

  test("a heavy check's definition moves with the crates it is predicted to build, so a run records them afresh", () => {
    const c = check({ kind: "cargo-named", args: ["test", "--release", "-p", "nvs-cli"] });
    const narrower: Graph = new Map([pkg("nvs-cli", "crates/nvs-cli")]);
    expect(grouped(c, ctx).own!.def).toBe(grouped({ ...c, name: "renamed" }, ctx).own!.def);
    expect(grouped(c, { ...ctx, graph: narrower }).own!.def).not.toBe(grouped(c, ctx).own!.def);
  });

  test("a check's own atom is defined by what it runs and judges, never by its name or stage", () => {
    const a = check({ argv: ["bun", "nv", "rules", "--check"], want: ["ok"] });
    expect(checkDef({ ...a, name: "other", stage: 9 })).toBe(checkDef(a));
    expect(checkDef({ ...a, want: ["changed"] })).not.toBe(checkDef(a));
  });
});

describe("what a command that records nothing reads", () => {
  test("git grep over literal paths reads those paths, git ls-files tests its paths, and any other git the tree", () => {
    expect([...commandKeys(check({ argv: ["git", "grep", "-n", "x", "--", "crates/a.rs", "docs/"] })).keys()]).toEqual(["tree:crates/a.rs", "tree:docs"]);
    expect([...commandKeys(check({ argv: ["git", "ls-files", "data/chain.json"] })).keys()]).toEqual(["exists:data/chain.json"]);
    expect([...commandKeys(check({ argv: ["git", "grep", "-n", "x", "--", "crates/*.rs"] })).keys()]).toEqual(["tree:."]);
  });

  test("the extension's tests read the extension, nvs records its own reads, and anything else reads everything", () => {
    expect([...commandKeys(check({ argv: ["npm", "run", "test:headless"], cwd: "editors/vscode" })).keys()]).toEqual(["tree:editors/vscode"]);
    expect(commandKeys(check({ argv: ["{nvs}", "ast", "x"] })).size).toBe(0);
    expect([...commandKeys(check({ argv: ["bun", "-e", "1"] })).keys()]).toEqual(["*"]);
    expect([...commandKeys(check({ argv: ["cargo", "metadata"] })).keys()]).toEqual(["*"]);
  });
});

describe("the heavy set's twins", () => {
  test("a test binary is handed the check's test-name filters and what follows `--`, and no option's value", () => {
    expect(harnessArgs(["test", "--release", "-p", "nvs-cli", "--bin", "nvs", "by_the_margin", "--", "--test-threads=1"])).toEqual(["by_the_margin", "--test-threads=1"]);
    expect(harnessArgs(["test", "-p", "nvs-host", "--lib"])).toEqual([]);
    expect(harnessArgs(["--bin", "nvs", "worker::"])).toEqual(["worker::"]);
  });

  test("a release test's twin is the same binary and filters in debug, with the tests only a release build runs", () => {
    const twin = heavyTwin(check({ kind: "cargo-named", args: ["test", "--release", "-p", "nvs-cli", "--bin", "nvs", "by_the_margin"] }), graph);
    expect(twin?.tests).toEqual([{ name: "nvs-cli bin nvs", args: ["by_the_margin", "--include-ignored"] }]);
    expect(twin?.held).toEqual(["profile:optimized"]);
  });

  test("a bench's twin runs once on the covws nvs, with no budget to fail and nothing written", () => {
    const serve = heavyTwin(check({ argv: ["bun", "nv", "bench", "--serve-vs-fpm", "--record", "benches/results/serve.json"] }), graph);
    expect(serve?.argv).toEqual(["bun", "nv", "bench", "--serve-vs-fpm", "--nvs", "{nvs}", "--allow-debug", "--reps", "1", "--requests", "200"]);
    const warm = heavyTwin(check({ argv: ["bun", "nv", "bench", "--warm-start", "--max-work-ms", "6"] }), graph);
    expect(warm?.argv).toEqual(["bun", "nv", "bench", "--warm-start", "--nvs", "{nvs}", "--allow-debug", "--reps", "1"]);
  });

  test("the database matrix's twin is its SQLite leg, and it holds every file of nvs-db and the code only Linux compiles", () => {
    const test = (name: string) => ({ kind: "test", name, src: `crates/nvs-stdlib/tests/${name}.rs`, test: true });
    const dbGraph: Graph = new Map([
      ...graph,
      pkg("nvs-db", "crates/nvs-db"),
      pkg("nvs-stdlib", "crates/nvs-stdlib", [], [{ kind: "lib", name: "nvs_stdlib", src: "crates/nvs-stdlib/src/lib.rs", test: true }, test("queue"), test("db_stream"), test("queue_sqlite")]),
    ]);
    const twin = heavyTwin(check({ argv: ["bun", "nv", "db-matrix", "--all"] }), dbGraph);
    expect(twin?.tests).toEqual([
      { name: "nvs-db lib nvs_db", args: [] },
      { name: "nvs-stdlib test queue", args: [] },
      { name: "nvs-stdlib test db_stream", args: [] },
      { name: "nvs-cli bin nvs", args: ["worker::"] },
      { name: "nvs-stdlib test queue_sqlite", args: [] },
    ]);
    expect(twin?.env.NVS_DB_MATRIX_DRIVER).toBe("sqlite");
    expect(twin?.crates).toEqual(["nvs-db"]);
    expect(twin?.held).toContain("platform:elsewhere");
  });

  test("fuzz and TSan have no twin and keep the prediction, and a check never memoized has neither", () => {
    const fuzz = check({ argv: ["wsl.exe", "--", "bash", "-lc", "cargo +nightly fuzz run lex fuzz/corpus/lex"] });
    expect(predicted(fuzz)).toBe(true);
    expect(heavyTwin(fuzz, graph)).toBeNull();
    const host = check({ argv: ["npm", "run", "test:host"], memoize: false });
    expect(predicted(host)).toBe(false);
    expect(heavyTwin(host, graph)).toBeNull();
  });

  test("a leg runs every fixture of the plan and every case of its suites again, and nothing else", () => {
    const plan = [check({ id: "f", kind: "exact", file: "examples/a.nvs" }), check({ id: "s", kind: "nvs-suite", args: ["test", "tests/conformance/"] }), check({ id: "n", argv: ["bun", "nv", "rules", "--check"] })];
    const groups = new Map(plan.map((c) => [c.id, grouped(c, ctx)]));
    expect(legAtoms(plan, groups)).toEqual(["check:f", "case:tests/conformance/a.nvst", "case:tests/conformance/sub/b.nvst"]);
  });
});

describe("the heavy set's named prediction", () => {
  test("a release test builds its package and what it is compiled against, a bench and a leg nvs-cli", () => {
    expect(heavyCrates(check({ kind: "cargo-named", args: ["test", "--release", "-p", "nvs-host"] }), graph).crates).toEqual(["nvs-host", "nvs-syntax"]);
    expect(heavyCrates(check({ argv: ["bun", "nv", "bench"] }), graph).crates).toEqual(["nvs-cli", "nvs-host", "nvs-syntax"]);
    expect(heavyCrates(null, graph).crates).toEqual(["nvs-cli", "nvs-host", "nvs-syntax"]);
    expect(heavyCrates(check({ argv: ["wsl.exe", "--", "bash", "-lc", "odd"] }), graph).crates).toEqual(["nvs-cli", "nvs-host", "nvs-syntax"]);
  });

  test("a fuzz target builds the crates its own source uses with their ordinary dependencies, never one only its comments name, and the manifest's when the source is missing", () => {
    const t = scratch();
    const fuzzGraph: Graph = new Map([
      pkg("nvs-diagnostics", "crates/nvs-diagnostics"),
      pkg("nvs-repo", "crates/nvs-repo"),
      pkg("nvs-syntax", "crates/nvs-syntax", [["nvs-diagnostics", "normal"], ["nvs-repo", "dev"]]),
      pkg("nvs-host", "crates/nvs-host", [["nvs-syntax", "normal"]]),
      pkg("nvs-cli", "crates/nvs-cli", [["nvs-host", "normal"]]),
    ]);
    try {
      t.put("fuzz/Cargo.toml", '[dependencies]\nnvs-host = { path = "../crates/nvs-host" }\nnvs-syntax = { path = "../crates/nvs-syntax" }\n');
      t.put("fuzz/fuzz_targets/lex.rs", "//! Compare with `nvs_host::run` and `nvs-cli`.\n/* nvs_cli */\nuse nvs_syntax::tokenize;\n");
      const fuzz = (target: string) => check({ argv: ["wsl.exe", "--", "bash", "-lc", `mkdir -p fuzz/corpus/${target} && cargo +nightly fuzz run ${target} fuzz/corpus/${target} -- -max_total_time=300`] });
      expect(heavyCrates(fuzz("lex"), fuzzGraph, t.root)).toEqual({ crates: ["nvs-diagnostics", "nvs-syntax"], extra: ["fuzz"] });
      expect(heavyCrates(fuzz("gone"), fuzzGraph, t.root).crates).toEqual(["nvs-diagnostics", "nvs-host", "nvs-syntax"]);
    } finally {
      t.cleanup();
    }
  });

  test("its keys are every Rust file of those crates, each directory that holds one, and what it reads besides", () => {
    const files = ["crates/nvs-host/src/lib.rs", "crates/nvs-host/src/io/mod.rs", "crates/nvs-syntax/src/lib.rs", "crates/nvs-cli/src/main.rs"];
    const keys = [...heavyKeys(check({ kind: "cargo-named", args: ["test", "--release", "-p", "nvs-host"] }), graph, files).keys()].sort();
    expect(keys).toEqual([
      "dir:crates",
      "dir:crates/nvs-host",
      "dir:crates/nvs-host/src",
      "dir:crates/nvs-host/src/io",
      "dir:crates/nvs-syntax",
      "dir:crates/nvs-syntax/src",
      "fn:crates/nvs-host/src/io/mod.rs#*",
      "fn:crates/nvs-host/src/lib.rs#*",
      "fn:crates/nvs-syntax/src/lib.rs#*",
    ]);
  });
});
