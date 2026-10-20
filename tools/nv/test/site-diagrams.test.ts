import { afterEach, describe, expect, test } from "bun:test";
import { proseProblems } from "../cmd/site.ts";
import { ROOT } from "../lib/paths.ts";
import { diagramAnchors, diagramProblems, diagramStrings } from "../lib/site-diagrams.ts";
import { scratch, type Scratch } from "./scratch.ts";

let tmp: Scratch;
afterEach(() => tmp?.cleanup());

const DOCS = "website/src/content/docs";

describe("site diagrams", () => {
  test("every string is prose but a tone, an icon or a link", () => {
    const data = { root: "Why?", branches: [{ label: "A", tone: "violet", icon: "moon", leaves: [{ label: "B", href: "#b" }] }] };
    expect(diagramStrings(data)).toEqual([
      { at: "root", text: "Why?" },
      { at: "branches[0].label", text: "A" },
      { at: "branches[0].leaves[0].label", text: "B" },
    ]);
    expect(diagramAnchors(data)).toEqual(["b"]);
    expect(diagramAnchors({ href: "/in-depth/" })).toEqual([]);
  });

  test("a long label and a dead anchor are named, and a page that does not draw the diagram is not", () => {
    tmp = scratch();
    const long = Array.from({ length: 30 }, () => "word").join(" ");
    tmp.put("website/src/data/diagrams/map.json", JSON.stringify({ root: long, leaves: [{ label: "x", href: "#here" }, { label: "y", href: "#gone" }] }));
    tmp.put(`${DOCS}/a.mdx`, "import map from '@data/diagrams/map.json'\n\n<div id=\"here\" />\n");
    tmp.put(`${DOCS}/b.mdx`, "No diagram here.\n");
    const problems = diagramProblems(tmp.root, DOCS, proseProblems);
    expect(problems).toHaveLength(2);
    expect(problems[0]).toContain("website/src/data/diagrams/map.json: root: a 30-word sentence");
    expect(problems[1]).toBe(`website/src/data/diagrams/map.json: links to #gone, and ${DOCS}/a.mdx has no element with that id`);
  });

  test("the diagrams on disk are clean", () => {
    expect(diagramProblems(ROOT, DOCS, proseProblems)).toEqual([]);
  });
});
