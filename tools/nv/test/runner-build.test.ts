import { describe, expect, test } from "bun:test";
import { sweepTestFilters } from "../driver/runner.ts";
import type { Graph, Target } from "../keys/graph.ts";

const target = (kind: string, name: string): Target => ({ kind, name, src: `${name}.rs`, test: true });
const graph: Graph = new Map([
  ["nvs-ir", { name: "nvs-ir", dir: "crates/nvs-ir", deps: new Map(), targets: [target("lib", "nvs-ir"), target("test", "refusals")] }],
  ["nvs-cli", { name: "nvs-cli", dir: "crates/nvs-cli", deps: new Map(), targets: [target("lib", "nvs-cli"), target("bin", "nvs"), target("test", "agent"), target("bench", "startup")] }],
]);
const EVERY = ["nvs-cli bin nvs", "nvs-cli lib nvs_cli", "nvs-cli test agent", "nvs-ir lib nvs_ir", "nvs-ir test refusals"];

describe("the sweep's test build", () => {
  test("an ordinary sweep builds only the binaries it runs, with target filters and never a `-p`", () => {
    expect(sweepTestFilters(["nvs-ir test refusals"], graph, false)).toEqual(["--test", "refusals"]);
    expect(sweepTestFilters(["nvs-ir lib nvs_ir", "nvs-cli bin nvs"], graph, false)).toEqual(["--bin", "nvs", "--lib"]);
  });

  test("a sweep over every atom, one with no graph, and one that runs every binary build every one", () => {
    expect(sweepTestFilters(["nvs-ir test refusals"], graph, true)).toBeNull();
    expect(sweepTestFilters(["nvs-ir test refusals"], null, false)).toBeNull();
    expect(sweepTestFilters(EVERY, graph, false)).toBeNull();
  });

  test("a binary the graph does not name is left out, and nothing to build is no filter at all", () => {
    expect(sweepTestFilters(["nvs-ir test refusals", "gone test old"], graph, false)).toEqual(["--test", "refusals"]);
    expect(sweepTestFilters(["gone test old"], graph, false)).toEqual([]);
    expect(sweepTestFilters([], graph, false)).toEqual([]);
  });
});
