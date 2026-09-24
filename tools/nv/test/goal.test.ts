import { afterEach, describe, expect, test } from "bun:test";
import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { parse as parseToml } from "smol-toml";
import { addModule, addToManifest, MANIFEST } from "../cmd/goal.ts";
import { writePointer } from "../lib/state.ts";
import { load, write } from "../lib/store.ts";
import { chain } from "../schema/chain.ts";
import { goal } from "../schema/goal.ts";
import { scratch, type Scratch } from "./scratch.ts";

let tmp: Scratch | undefined;
afterEach(() => {
  tmp?.cleanup();
  tmp = undefined;
});

/** A chain of two goals with `live` installed, and `manifest` as the driver's toml when given. */
function seed(manifest?: string): Scratch {
  const s = scratch();
  write(chain, "chain", ["walked", "live"], s.root);
  for (const slug of ["walked", "live"]) {
    write(goal, slug, { title: slug, milestone: null, files: [], context: { modules: ["tools/a.ts"] }, stages: [], checks: [], env: {} }, s.root);
  }
  writePointer("live", s.root);
  s.put("tools/a.ts", "");
  s.put("tools/b.ts", "");
  if (manifest !== undefined) s.put(MANIFEST, manifest);
  return s;
}

const modulesOf = (root: string, slug: string) => load(goal, root).find((g) => g.id === slug)!.value.context.modules;

const MULTI = [
  "[goal]",
  'title = "x"',
  "",
  "[context]",
  "",
  "modules = [",
  "  # The first module.",
  '  "tools/a.ts"',
  "]",
  "",
  'rules = ["a/b"]',
  "",
  "[context.stage.4]",
  'shapes = ["A shape"]',
  "",
].join("\n");

describe("nv goal context --add", () => {
  test("adds the path to the live goal's record and to the toml's array, and leaves every other line alone", () => {
    tmp = seed(MULTI);
    const added = addModule("tools\\b.ts", tmp.root);
    expect(added).toEqual({ path: "tools/b.ts", goal: "live", written: ["data/goals/live.json", MANIFEST] });
    expect(modulesOf(tmp.root, "live")).toEqual(["tools/a.ts", "tools/b.ts"]);
    expect(modulesOf(tmp.root, "walked")).toEqual(["tools/a.ts"]);
    const text = readFileSync(join(tmp.root, MANIFEST), "utf8");
    expect(text).toBe(MULTI.replace('  "tools/a.ts"\n', '  "tools/a.ts",\n  "tools/b.ts",\n'));
  });

  test("a path already listed is a no-op in each home separately", () => {
    tmp = seed(MULTI.replace('  "tools/a.ts"\n', '  "tools/a.ts",\n  "tools/b.ts",\n'));
    expect(addModule("tools/b.ts", tmp.root).written).toEqual(["data/goals/live.json"]);
    expect(addModule("tools/b.ts", tmp.root).written).toEqual([]);
    expect(addModule("tools/a.ts", tmp.root).written).toEqual([]);
  });

  test("a path that is not on disk, or not in the repository, is refused and nothing is written", () => {
    tmp = seed(MULTI);
    expect(() => addModule("tools/gone.ts", tmp!.root)).toThrow("is not on disk");
    expect(() => addModule("../outside.ts", tmp!.root)).toThrow("is not inside the repository");
    expect(modulesOf(tmp.root, "live")).toEqual(["tools/a.ts"]);
    expect(readFileSync(join(tmp.root, MANIFEST), "utf8")).toBe(MULTI);
  });

  test("with no toml on disk only the record is written", () => {
    tmp = seed();
    expect(addModule("tools/b.ts", tmp.root).written).toEqual(["data/goals/live.json"]);
    expect(existsSync(join(tmp.root, MANIFEST))).toBe(false);
  });
});

describe("addToManifest", () => {
  const modules = (text: string) => (parseToml(text).context as { modules: string[] }).modules;

  test("a CRLF file stays CRLF, and a trailing comment keeps its place", () => {
    const text = '[context]\r\nmodules = [\r\n  "a", # one\r\n]\r\n';
    expect(addToManifest(text, "b")).toBe('[context]\r\nmodules = [\r\n  "a", # one\r\n  "b",\r\n]\r\n');
  });

  test("a one-line array grows in place, empty or not", () => {
    expect(addToManifest('[context]\nmodules = []\n', "b")).toBe('[context]\nmodules = ["b"]\n');
    expect(addToManifest('[context]\nmodules = ["a"] # kept\n', "b")).toBe('[context]\nmodules = ["a", "b"] # kept\n');
  });

  test("a missing key or table is created, and a listed path is null", () => {
    expect(modules(addToManifest('[context]\nrules = []\n', "b")!)).toEqual(["b"]);
    expect(modules(addToManifest('[goal]\ntitle = "x"\n', "b")!)).toEqual(["b"]);
    expect(addToManifest('[context]\nmodules = ["b"]\n', "b")).toBeNull();
  });
});
