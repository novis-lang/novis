import { afterEach, describe, expect, test } from "bun:test";
import { adoptLegacyPointer, cutOver, legacyPointer, pointerSlug, readPointer, writePointer } from "../lib/state.ts";
import { scratch, type Scratch } from "./scratch.ts";

let tmp: Scratch;
afterEach(() => tmp?.cleanup());

describe("the chain pointer", () => {
  test("a tree the driver has never run in has no pointer", () => {
    tmp = scratch();
    expect(readPointer(tmp.root)).toBeNull();
    expect(legacyPointer(tmp.root)).toBeNull();
    expect(pointerSlug(tmp.root)).toBeNull();
  });

  test("the state database names the goal by slug, and a second write moves it", () => {
    tmp = scratch();
    writePointer("first-goal", tmp.root);
    expect(readPointer(tmp.root)).toBe("first-goal");
    writePointer("second-goal", tmp.root);
    expect(readPointer(tmp.root)).toBe("second-goal");
  });

  test("the Python driver's number resolves to the slug of the file that carries it", () => {
    tmp = scratch();
    tmp.put(".loop/chain.json", '{\n  "goal": 2\n}\n');
    tmp.put("docs/agent/goals/1-first-goal.md", "# First\n");
    tmp.put("docs/agent/goals/2-second-goal.md", "# Second\n");
    tmp.put("docs/agent/goals/2-second-goal.handoff.md", "# Handoff\n");
    expect(legacyPointer(tmp.root)).toBe("second-goal");
    expect(pointerSlug(tmp.root)).toBe("second-goal");
  });

  test("the state database wins over the Python driver's pointer", () => {
    tmp = scratch();
    tmp.put(".loop/chain.json", '{"goal": 1}');
    tmp.put("docs/agent/goals/1-first-goal.md", "# First\n");
    writePointer("second-goal", tmp.root);
    expect(pointerSlug(tmp.root)).toBe("second-goal");
  });

  test("the cutover moves the Python driver's pointer into the state database, and only then is it cut over", () => {
    tmp = scratch();
    tmp.put(".loop/chain.json", '{"goal": 2}');
    tmp.put("docs/agent/goals/1-first-goal.md", "# First\n");
    tmp.put("docs/agent/goals/2-second-goal.md", "# Second\n");
    expect(cutOver(tmp.root)).toBe(false);
    expect(adoptLegacyPointer(tmp.root)).toBe("second-goal");
    expect(readPointer(tmp.root)).toBe("second-goal");
    expect(cutOver(tmp.root)).toBe(true);
  });

  test("a legacy pointer that names no goal is not adopted, and the tree is not cut over", () => {
    tmp = scratch();
    tmp.put(".loop/chain.json", '{"goal": 7}');
    expect(adoptLegacyPointer(tmp.root)).toBeNull();
    expect(cutOver(tmp.root)).toBe(false);
  });

  test("a number no goal file carries, or a pointer that is not a number, names nothing", () => {
    tmp = scratch();
    tmp.put("docs/agent/goals/1-first-goal.md", "# First\n");
    tmp.put(".loop/chain.json", '{"goal": 7}');
    expect(legacyPointer(tmp.root)).toBeNull();
    tmp.put(".loop/chain.json", '{"goal": "first-goal"}');
    expect(legacyPointer(tmp.root)).toBeNull();
    tmp.put(".loop/chain.json", "not json");
    expect(legacyPointer(tmp.root)).toBeNull();
  });
});
