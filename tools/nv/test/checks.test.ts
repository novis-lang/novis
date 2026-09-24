import { describe, expect, test } from "bun:test";
import { type Check, checkName, testTargets, units } from "../keys/checks.ts";
import type { Graph, Package } from "../keys/graph.ts";

function pkg(name: string, targets: [string, string][]): Package {
  return { name, dir: `crates/${name}`, deps: new Map(), targets: targets.map(([kind, t]) => ({ kind, name: t, src: `crates/${name}/src/${t}.rs`, test: true })) };
}

const graph: Graph = new Map([
  ["nvs-cli", pkg("nvs-cli", [["lib", "nvs-cli"], ["bin", "nvs"], ["test", "agent"]])],
  ["nvs-host", pkg("nvs-host", [["lib", "nvs-host"]])],
]);

function sources(checks: Check[], wide: string[] = []): Record<string, string> {
  const us = units({ graph, checks, reads: new Map(), wide: new Set(wide), observed: new Map() });
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
        { kind: "command", name: "gate", argv: ["python", "tools/never-observed.py"] },
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
      gate: "everything",
      grep: "partitions",
      glob: "everything",
      bun: "everything",
    });
  });

  test("an unnamed check is named by its file, else by what it runs", () => {
    expect(checkName({ kind: "exact", file: "examples/a.nvs" })).toBe("examples/a.nvs");
    expect(checkName({ kind: "command", argv: ["git", "grep", "x"] })).toBe("command git grep x");
  });
});
