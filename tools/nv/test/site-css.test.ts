import { describe, expect, test } from "bun:test";
import { ROOT } from "../lib/paths.ts";
import { foldedTimelineProblems, foldedTimelines } from "../lib/site-css.ts";

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

  test("the website's styles keep every timeline", () => {
    expect(foldedTimelineProblems(ROOT)).toEqual([]);
  });
});
