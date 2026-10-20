import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { ROOT } from "../lib/paths.ts";
import { endlessLoops, foldedTimelines, styleProblems } from "../lib/site-css.ts";

describe("site css", () => {
  test("a rule with the animation shorthand and a timeline is named, and longhands are not", () => {
    const css = [
      ".a {",
      "  animation: grow linear both;",
      "  animation-timeline: view();",
      "}",
      ".b { animation-name: grow; animation-fill-mode: both; animation-timeline: view(); }",
      ".c { animation: spin 1s infinite; }",
      "@media (x) { .d { animation-timeline: --t; animation: run linear both; } }",
    ].join("\n");
    expect(foldedTimelines(css)).toEqual([1, 7]);
  });

  test("an endless animation is named unless it runs only in a state the reader started", () => {
    const css = [
      ".a { animation: spin 1s linear infinite; }",
      ".b:hover .c { animation: spin 1s infinite; }",
      ".d:focus-visible { animation-iteration-count: infinite; }",
      ".e[data-replay-state='playing'] .f { animation: blink 1s step-end infinite; }",
      ".g { animation: once 1s 2 alternate; }",
      "@media (x) {",
      "  .h {",
      "    animation-iteration-count: infinite;",
      "  }",
      "}",
    ].join("\n");
    expect(endlessLoops(css)).toEqual([1, 7]);
    expect(endlessLoops(`<div data-motion></div>\n${css}`)).toEqual([]);
  });

  test("custom.css pauses every animation in a region that is not on screen", () => {
    const css = readFileSync(join(ROOT, "website/src/styles/custom.css"), "utf8");
    expect(css).toContain("[data-motion]:not([data-on-screen]) *::before");
    expect(css).toMatch(/\[data-motion\]:not\(\[data-on-screen\]\) \*::after \{\s*animation-play-state: paused !important;/);
  });

  test("the website's styles keep every timeline and loop nothing forever", () => {
    expect(styleProblems(ROOT)).toEqual([]);
  });
});
