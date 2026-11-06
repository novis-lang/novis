// Every website walkthrough has a brief and exactly one moment (`rule:website/a-walkthrough-has-one-moment`),
// and every step's caption sits where its step happens (`rule:website/a-caption-sits-where-it-happens`).
// A scene is a module under `website/src/components/walkthrough/scenes/` that exports `start`. Its
// brief is its `brief` export, and its moment is the one step written with `moment: true`.

import { expect, test } from "bun:test";
import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { ROOT } from "../lib/paths.ts";

const SCENES = join(ROOT, "website", "src", "components", "walkthrough", "scenes");
const scenes = readdirSync(SCENES).filter((f) => f.endsWith(".ts"));

test("there is at least one walkthrough scene", () => {
  expect(scenes.length).toBeGreaterThan(0);
});

for (const file of scenes) {
  test(`${file} has a full brief and exactly one moment`, async () => {
    const mod = await import(join(SCENES, file));
    expect(typeof mod.start).toBe("function");
    const brief = mod.brief;
    expect(brief).toBeDefined();
    for (const key of ["moment", "expectation", "proof", "then"]) {
      expect(typeof brief[key]).toBe("string");
      expect(brief[key].trim().length).toBeGreaterThan(0);
    }
    expect(Array.isArray(brief.leavesOut)).toBe(true);
    expect(brief.leavesOut.length).toBeGreaterThan(0);
    const moments = readFileSync(join(SCENES, file), "utf8").match(/\bmoment: true\b/g) ?? [];
    expect(moments.length).toBe(1);
  });

  // `rule:website/a-caption-sits-where-it-happens`: every step places its caption with `stage.say`.
  test(`${file} places every step's caption on the stage`, () => {
    const src = readFileSync(join(SCENES, file), "utf8");
    const steps = src.match(/\btl\.beat\(/g) ?? [];
    const says = src.match(/\bstage\.say\(/g) ?? [];
    expect(steps.length).toBeGreaterThan(0);
    expect(says.length).toBe(steps.length);
  });
}
