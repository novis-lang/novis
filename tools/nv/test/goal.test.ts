import { afterEach, describe, expect, test } from "bun:test";
import { addModule } from "../cmd/goal.ts";
import { load, write } from "../lib/store.ts";
import { chain } from "../schema/chain.ts";
import { goal } from "../schema/goal.ts";
import { scratch, type Scratch } from "./scratch.ts";

let tmp: Scratch | undefined;
afterEach(() => {
  tmp?.cleanup();
  tmp = undefined;
});

/** A chain of two goals with `live` live. */
function seed(): Scratch {
  const s = scratch();
  write(chain, "chain", { live: "live", goals: ["walked", "live"] }, s.root);
  for (const slug of ["walked", "live"]) {
    write(goal, slug, { title: slug, milestone: null, files: [], context: { modules: ["tools/a.ts"] }, stages: [], checks: [], env: {} }, s.root);
  }
  s.put("tools/a.ts", "");
  s.put("tools/b.ts", "");
  return s;
}

const modulesOf = (root: string, slug: string) => load(goal, root).find((g) => g.id === slug)!.value.context.modules;

describe("nv goal context --add", () => {
  test("adds the path to the live goal's record and to no other", () => {
    tmp = seed();
    const added = addModule("tools\\b.ts", tmp.root);
    expect(added).toEqual({ path: "tools/b.ts", goal: "live", written: ["data/goals/live.json"] });
    expect(modulesOf(tmp.root, "live")).toEqual(["tools/a.ts", "tools/b.ts"]);
    expect(modulesOf(tmp.root, "walked")).toEqual(["tools/a.ts"]);
  });

  test("a path already listed is a no-op", () => {
    tmp = seed();
    expect(addModule("tools/b.ts", tmp.root).written).toEqual(["data/goals/live.json"]);
    expect(addModule("tools/b.ts", tmp.root).written).toEqual([]);
    expect(addModule("tools/a.ts", tmp.root).written).toEqual([]);
  });

  test("a path that is not on disk, or not in the repository, is refused and nothing is written", () => {
    tmp = seed();
    expect(() => addModule("tools/gone.ts", tmp!.root)).toThrow("is not on disk");
    expect(() => addModule("../outside.ts", tmp!.root)).toThrow("is not inside the repository");
    expect(modulesOf(tmp.root, "live")).toEqual(["tools/a.ts"]);
  });
});
