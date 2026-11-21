import { describe, expect, test } from "bun:test";
import { AFTER_BUILD, BEFORE_BUILD, BUILD, phases, prePush } from "../cmd/close.ts";
import { PRE_PUSH } from "../cmd/ci.ts";

describe("the goal-end check", () => {
  test("every check it takes from ci --check is still in PRE_PUSH under that name", () => {
    for (const name of [...BEFORE_BUILD, BUILD, ...AFTER_BUILD]) expect(PRE_PUSH.some((c) => c.name === name)).toBe(true);
    expect(() => prePush("no such check")).toThrow("has no check named");
  });

  test("it builds once, between the checks that read files and the ones that read the binary", () => {
    const [before, build, after] = phases("some-goal");
    expect(build!.map((c) => c.name)).toEqual([BUILD]);
    expect(before!.map((c) => c.name)).toContain("rustfmt");
    expect(after!.map((c) => c.name)).toContain("website stale guard");
  });

  test("it gates the goal's own owners and the proofs of what its work reaches", () => {
    const all = phases("some-goal").flat();
    expect(all.find((c) => c.name === "owners")!.argv).toEqual(["bun", "nv", "owners", "--closes", "some-goal"]);
    expect(all.find((c) => c.name === "goal proofs")!.argv.slice(0, 6)).toEqual(["bun", "nv", "proofs", "--gate", "--goal", "some-goal"]);
  });
});
