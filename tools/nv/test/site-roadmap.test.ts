import { afterEach, describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { README, ROADMAP, ROADMAP_END, ROADMAP_START, renderRoadmap, roadmapProblems, writeRoadmap, type Roadmap } from "../lib/site-roadmap.ts";
import { scratch, type Scratch } from "./scratch.ts";

let tmp: Scratch;
afterEach(() => tmp?.cleanup());

const CHAIN = ["live", "fmt", "cli", "editor", "pkg"];

const GOOD: Roadmap = {
  internal: ["live"],
  themes: [
    { title: "Command line", text: "`nvs fmt` formats code.", goals: ["fmt", "cli"] },
    { title: "Editor", text: "The editor renames.", goals: ["editor"] },
    { title: "Packages", text: "A registry.", goals: ["pkg"] },
  ],
};

/** A scratch tree with `chain`, `map`, and a README whose list is `map`'s unless `readme` is given. */
function tree(map: Roadmap, chain: string[] = CHAIN, readme?: string): string {
  tmp = scratch();
  tmp.put("docs/goals/README.md", `---\nlooper: 1\nchain: [${chain.join(", ")}]\n---\n# Goals\n`);
  tmp.put(ROADMAP, JSON.stringify(map));
  tmp.put(README, `# Novis\n\n## Roadmap\n\n${ROADMAP_START}\n${readme ?? renderRoadmap(map)}\n${ROADMAP_END}\n\n## Licence\n`);
  return tmp.root;
}

describe("site roadmap", () => {
  test("a roadmap in step with the chain and the README has no problem", () => {
    expect(roadmapProblems(tree(GOOD))).toEqual([]);
  });

  test("the README's list numbers the themes and labels the first", () => {
    expect(renderRoadmap(GOOD).split("\n")[0]).toBe("1. **Command line.** *Working on it now.* `nvs fmt` formats code.");
  });

  test("a goal on the chain in no theme is named", () => {
    const problems = roadmapProblems(tree(GOOD, [...CHAIN, "new"]));
    expect(problems).toEqual([`${ROADMAP}: new is on the chain and in no theme; add it to a theme, or to \`internal\``]);
  });

  test("a theme that names a goal off the chain is named", () => {
    const map = { ...GOOD, themes: [...GOOD.themes.slice(0, 2), { ...GOOD.themes[2]!, goals: ["pkg", "gone"] }] };
    expect(roadmapProblems(tree(map))).toEqual([`${ROADMAP}: "Packages" names gone, which is not on the chain`]);
  });

  test("an internal goal off the chain is named", () => {
    const problems = roadmapProblems(tree(GOOD, CHAIN.slice(1)));
    expect(problems).toEqual([`${ROADMAP}: \`internal\` names live, which is not on the chain`]);
  });

  test("a goal named twice is named", () => {
    const map = { ...GOOD, themes: [GOOD.themes[0]!, { ...GOOD.themes[1]!, goals: ["editor", "cli"] }, GOOD.themes[2]!] };
    expect(roadmapProblems(tree(map))).toEqual([`${ROADMAP}: cli is in "Command line" and in "Editor"; a goal is in one place`]);
  });

  test("a theme with no goal left on the chain is named", () => {
    const problems = roadmapProblems(tree(GOOD, ["live", "fmt", "cli", "pkg"]));
    expect(problems).toContain(`${ROADMAP}: "Editor" has no goal left on the chain; remove the theme`);
  });

  test("themes out of the chain order of their first goal are named", () => {
    const map = { ...GOOD, themes: [GOOD.themes[1]!, GOOD.themes[0]!, GOOD.themes[2]!] };
    expect(roadmapProblems(tree(map))).toEqual([`${ROADMAP}: "Command line" comes after "Editor", but its first goal is earlier on the chain`]);
  });

  test("a README whose copy differs is named, and --roadmap writes it", () => {
    const root = tree(GOOD, CHAIN, "1. **Old.** Stale.");
    expect(roadmapProblems(root)).toEqual([`${README}: the roadmap differs from ${ROADMAP}; run \`bun nv site --roadmap\``]);
    expect(writeRoadmap(root)).toBeNull();
    expect(roadmapProblems(root)).toEqual([]);
    expect(readFileSync(join(root, README), "utf8")).toContain("## Licence");
  });

  test("a README with no markers is named", () => {
    const root = tree(GOOD);
    tmp.put(README, "# Novis\n");
    expect(roadmapProblems(root)).toEqual([`${README}: no roadmap markers; put ${ROADMAP_START} and ${ROADMAP_END} where the list goes`]);
    expect(writeRoadmap(root)).not.toBeNull();
  });
});
