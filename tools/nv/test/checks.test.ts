import { describe, expect, test } from "bun:test";
import { type Check, checkName, isWide, proofGroups, recordId, roleOf, spawnParts, suitesIn, testTargets, units } from "../keys/checks.ts";
import type { Graph, Package } from "../keys/graph.ts";
import { keyOf, testBuild } from "../keys/key.ts";
import { Tree } from "../keys/tree.ts";

function pkg(name: string, targets: [string, string][]): Package {
  return { name, dir: `crates/${name}`, deps: new Map(), targets: targets.map(([kind, t]) => ({ kind, name: t, src: `crates/${name}/src/${t}.rs`, test: true })) };
}

const graph: Graph = new Map([
  ["nvs-cli", pkg("nvs-cli", [["lib", "nvs-cli"], ["bin", "nvs"], ["test", "agent"]])],
  ["nvs-host", pkg("nvs-host", [["lib", "nvs-host"]])],
]);

function sources(checks: Check[], wide: string[] = []): Record<string, string> {
  const us = units({ graph, checks, reads: new Map(), wide: new Set(wide), proofReads: new Map(), nvReads: new Map() });
  return Object.fromEntries(us.map((u) => [u.name, u.source]));
}

describe("the units a key answers for", () => {
  test("a test check names the binaries its flags select", () => {
    expect(testTargets(graph, ["test", "-p", "nvs-cli"])?.names).toEqual(["nvs-cli lib nvs_cli", "nvs-cli bin nvs", "nvs-cli test agent"]);
    expect(testTargets(graph, ["test", "-p", "nvs-cli", "--bin", "nvs", "filter"])?.names).toEqual(["nvs-cli bin nvs"]);
    expect(testTargets(graph, ["test", "--release", "-p", "nvs-cli", "--test", "agent"])?.names).toEqual(["nvs-cli test agent"]);
    expect(testTargets(graph, ["test", "-p", "nvs-cli", "--lib"])?.names).toEqual(["nvs-cli lib nvs_cli"]);
    expect(testTargets(graph, ["test", "--workspace"])).toBeUndefined();
  });

  test("each shape of check keys where the spec puts it", () => {
    const got = sources(
      [
        { kind: "cargo-named", name: "bin", args: ["test", "-p", "nvs-cli", "--bin", "nvs", "x"] },
        { kind: "cargo-named", name: "workspace", args: ["test", "--workspace"] },
        { kind: "nvs-suite", name: "conformance", args: ["test", "tests/conformance/"] },
        { kind: "nvs-suite", name: "hostile", args: ["test", "tests/hostile/"] },
        { kind: "exact", file: "examples/hello.nvs" },
        { kind: "command", name: "meta", argv: ["{nvs}", "meta"] },
        { kind: "command", name: "vsix", cwd: "editors/vscode", argv: ["npm", "run", "package"] },
        { kind: "command", name: "fuzz", argv: ["wsl.exe", "--", "bash", "-lc", "cargo +nightly fuzz run prefix"] },
        { kind: "command", name: "tsan", argv: ["wsl.exe", "--", "bash", "-lc", "bash tools/tsan.sh"] },
        { kind: "command", name: "grep", argv: ["git", "grep", "-n", "x", "--", "crates/a.rs"] },
        { kind: "command", name: "glob", argv: ["git", "grep", "-n", "x", "--", "crates/*.rs"] },
        { kind: "command", name: "bun", argv: ["bun", "nv", "selftest"] },
      ],
      ["nvs-cli bin nvs"],
    );
    expect(got).toMatchObject({
      "nvs-cli bin nvs": "everything",
      "nvs-host lib nvs_host": "binary key",
      "valgrind sweep": "package key",
      bin: "verify record",
      workspace: "everything",
      conformance: "verify record",
      hostile: "package key",
      "examples/hello.nvs": "package key",
      meta: "package key",
      vsix: "partitions",
      fuzz: "package key",
      tsan: "package key",
      grep: "partitions",
      glob: "everything",
      bun: "everything",
    });
  });

  test("each group a proofs run names is a unit, and one narrowed to a feature is not", () => {
    const got = sources([
      { kind: "command", name: "two groups", argv: ["bun", "nv", "proofs", "--verify", "--group", "lang:types", "--group=types:enum"] },
      { kind: "command", name: "one feature", argv: ["bun", "nv", "proofs", "--run", "--group", "Core\\Arr", "--id", "Core\\Arr::map"] },
      { kind: "command", name: "audit", argv: ["bun", "nv", "proofs", "--group", "Core\\Str"] },
    ]);
    expect(got).toMatchObject({ "two groups": "observed", "proofs: lang:types": "observed", "proofs: types:enum": "observed", "one feature": "everything", audit: "everything" });
    expect(Object.keys(got).filter((n) => n.startsWith("proofs: ")).sort()).toEqual(["proofs: lang:types", "proofs: types:enum"]);
    expect(proofGroups({ kind: "command", argv: ["bun", "nv", "proofs", "--verify"] })).toBeUndefined();
  });

  test("a proofs group keys on its own recorded paths, and on everything before it has run", async () => {
    const check: Check = { kind: "command", name: "g", argv: ["bun", "nv", "proofs", "--verify", "--group", "lang:types"] };
    const records = (proofReads: Map<string, string[]>) => ({ graph, checks: [check], reads: new Map(), wide: new Set<string>(), proofReads, nvReads: new Map() });
    const tree = await Tree.read();
    const unit = (proofReads: Map<string, string[]>) => units(records(proofReads)).find((u) => u.name === "proofs: lang:types")!;
    expect(isWide(unit(new Map()).parts(tree))).toBe(true);
    const labels = unit(new Map([["lang:types", ["docs/examples/lang/types", "tests/hostile/lang/types"]]])).parts(tree).map((p) => p.label);
    expect(labels).toContain("docs/examples/lang/types/");
    expect(labels).toContain("<conformance>");
    expect(labels).not.toContain("<hostile>");
  });

  test("a `bun nv` check keys on what it last read, and on everything until it has run", async () => {
    const check: Check = { kind: "command", name: "plan", argv: ["bun", "nv", "plan", "--check"] };
    const rec = (spawns: string[][]) => ({ files: ["data/chain.json"], exists: ["docs/plan/m0.md"], dirs: ["data/goals"], spawns });
    const unit = (nvReads: Map<string, ReturnType<typeof rec>>) =>
      units({ graph, checks: [check], reads: new Map(), wide: new Set<string>(), proofReads: new Map(), nvReads }).find((u) => u.name === "plan")!;
    const tree = await Tree.read();
    expect(unit(new Map()).source).toBe("everything");
    const seen = unit(new Map([[recordId(".", check.argv as string[]), rec([["git", "ls-files", "-z"]])]]));
    expect(seen.source).toBe("observed");
    const key = (t: Tree) => keyOf("plan", seen.parts(t));
    expect(isWide(seen.parts(tree))).toBe(false);
    expect(seen.parts(tree).map((p) => p.label)).toContain("tools/nv/cmd/plan.ts");
    // What it read moves the key, and what it did not read does not.
    expect(key(tree.edited({ "data/chain.json": "{}" }))).not.toBe(key(tree));
    expect(key(tree.edited({ "docs/agent/playbook.md": "x" }))).toBe(key(tree));
    expect(key(tree.edited({ "tools/nv/cmd/webcrypto-vectors.ts": "x" }))).toBe(key(tree));
    expect(key(tree.edited({ "tools/nv/cmd/plan.ts": "x" }))).not.toBe(key(tree));
    // A listed directory moves it with a new name, not with new text in a file it never opened.
    expect(key(tree.edited({ "data/goals/zz-new.json": "{}" }))).not.toBe(key(tree));
    // A program it cannot key widens it.
    expect(isWide(unit(new Map([[recordId(".", check.argv as string[]), rec([["git", "log", "-1"]])]])).parts(tree))).toBe(true);
  });

  test("a key on everything leaves out the handoff a wrap rewrites", async () => {
    const tree = await Tree.read();
    const wide = units({ graph, checks: [{ kind: "command", name: "w", argv: ["bun", "nv", "selftest"] }], reads: new Map(), wide: new Set<string>(), proofReads: new Map(), nvReads: new Map() }).find((u) => u.name === "w")!;
    const key = (t: Tree) => keyOf("w", wide.parts(t));
    expect(key(tree.edited({ "data/goals/zz-probe.handoff.json": "{}" }))).toBe(key(tree));
    expect(key(tree.edited({ "docs/agent/playbook.md": "x" }))).not.toBe(key(tree));
  });

  test("a wide test binary's key leaves out what a wrap writes, and a check keyed on everything does not", async () => {
    const tree = await Tree.read();
    const us = units({ graph, checks: [{ kind: "command", name: "w", argv: ["bun", "nv", "selftest"] }], reads: new Map(), wide: new Set(["nvs-cli bin nvs"]), proofReads: new Map(), nvReads: new Map() });
    const binary = us.find((u) => u.name === "nvs-cli bin nvs")!;
    const check = us.find((u) => u.name === "w")!;
    expect(isWide(binary.parts(tree))).toBe(true);
    for (const rel of ["docs/agent/playbook/zz-probe.md", "data/playbook/zz-probe.json", "docs/implementation-plan.md", "docs/plan/zz-probe.md"]) {
      const edited = tree.edited({ [rel]: "x" });
      expect(keyOf("b", binary.parts(edited))).toBe(keyOf("b", binary.parts(tree)));
      expect(keyOf("w", check.parts(edited))).not.toBe(keyOf("w", check.parts(tree)));
    }
    expect(keyOf("b", binary.parts(tree.edited({ "docs/examples/zz-probe.nvs": "x" })))).not.toBe(keyOf("b", binary.parts(tree)));
  });

  test("each program a `bun nv` process starts keys on what that program reads", async () => {
    const tree = await Tree.read();
    const labels = (argv: string[]) => spawnParts(tree, graph, argv)?.map((p) => p.label) ?? null;
    expect(labels(["bun", "nv", "peek", "x"])).toEqual([]);
    expect(labels([process.execPath, `${process.cwd()}/tools/nv/main.ts`, "orient"])).toEqual([]);
    expect(labels(["git", "ls-files", "-z"])).toEqual(["<names>./"]);
    expect(labels(["git", "rev-parse", "--show-toplevel"])).toEqual([]);
    expect(labels(["git", "grep", "-n", "x", "--", "AGENTS.md"])).toEqual(["AGENTS.md"]);
    expect(labels(["cargo", "metadata", "--format-version", "1"])).toEqual(["<every Cargo.toml>"]);
    expect(labels(["git", "log", "-1"])).toBeNull();
    expect(labels(["docker", "ps"])).toBeNull();
  });

  test("an unnamed check is named by its file, else by what it runs", () => {
    expect(checkName({ kind: "exact", file: "examples/a.nvs" })).toBe("examples/a.nvs");
    expect(checkName({ kind: "command", argv: ["git", "grep", "x"] })).toBe("command git grep x");
  });

  test("a check's role is what it runs", () => {
    const role = (c: Check) => roleOf(c);
    expect(role({ kind: "min-bytes", file: "a.nvs" })).toBe("program");
    expect(role({ kind: "nvs-suite", args: ["test", "tests/hostile/"] })).toBe("suite");
    expect(role({ kind: "cargo-named", args: ["test", "-p", "nvs-cli"] })).toBe("test");
    expect(role({ kind: "command", argv: ["{nvs}", "agent", "find", "x"] })).toBe("nvs");
    expect(role({ kind: "command", cwd: "editors/vscode", argv: ["npm", "run", "package"] })).toBe("vsix");
    expect(role({ kind: "command", cwd: "editors/vscode", argv: ["npm", "test"] })).toBe("editor");
    expect(role({ kind: "command", argv: ["bun", "nv", "db-matrix", "--all"] })).toBe("db-matrix");
    expect(role({ kind: "command", argv: ["git", "grep", "-e", "tools/tsan.sh", "--", "ci.yml"] })).toBe("grep");
    expect(role({ kind: "command", argv: ["bun", "nv", "check"] })).toBe("nv");
  });
});

describe("what a script's `cargo test` lists build", () => {
  test("an integration test builds its package's library without its test modules", async () => {
    const script = `SUITES = (\n    ["-p", "nvs-host"],\n    ["-p", "nvs-cli", "--test", "agent"],\n    ["--bin", "nvs", "worker::"],\n)\nOTHER = ["not", "cargo"]\n`;
    const tree = (await Tree.read()).edited({ "tools/probe-script.py": script });
    expect(suitesIn(tree, graph, "tools/probe-script.py")).toEqual([
      { own: ["nvs-host"], ownTier: "raw", depTier: "card", test: true },
      { own: ["nvs-cli"], ownTier: "card", depTier: "card", test: true, srcTest: false },
      { own: ["nvs-cli"], ownTier: "raw", depTier: "card", test: true },
    ]);
    expect(suitesIn(tree.edited({ "tools/probe-script.py": `S = (["-p", "nvs-gone"],)\n` }), graph, "tools/probe-script.py")).toEqual([]);
  });

  test("a test binary's own source is raw, and an integration test's is shipped", () => {
    expect(testBuild("nvs-cli", "bin")).toMatchObject({ ownTier: "raw", test: true });
    expect(testBuild("nvs-cli", "test")).toMatchObject({ ownTier: "shipped", test: true, srcTest: false });
  });
});
