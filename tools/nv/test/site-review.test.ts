// `bun nv site --review` and `--check review` (`lib/texts.ts`): every text an end user reads has a key
// and a hash, a changed one owes a review stamp, the baseline only excuses an unchanged text, and the
// Reviewed proof is owed while a feature has a text with no review.

import { afterEach, beforeEach, describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { reviewCheck, reviewList, type World } from "../cmd/site.ts";
import { owed, type Policy, type Proofs } from "../proofs/collect.ts";
import type { Entry } from "../proofs/roster.ts";
import {
  commentText,
  featureTexts,
  isReviewed,
  readReview,
  REVIEW_BASELINE,
  REVIEW_QUESTIONS,
  reviewBrief,
  stampReview,
  userTexts,
  writeReviewBaseline,
} from "../lib/texts.ts";
import { scratch, type Scratch } from "./scratch.ts";

const LENGTH: Entry = {
  id: "Core\\Str::length",
  kind: "member",
  group: "Core\\Str",
  path: "core/Str/length",
  anchor: "",
  summary: "",
  help: "",
  card: '{"name":"length","doc":{"short":"Returns the number of characters."}}',
};
const world: World = { entries: [LENGTH], meta: {} };

let s: Scratch;

beforeEach(() => {
  s = scratch();
  s.put("docs/examples/core/Str/length/about.md", "`Core\\Str::length` returns the number of characters.\n");
  s.put("docs/examples/core/Str/length/details.md", "An empty string has length `0`.\n");
  s.put("docs/examples/core/Str/length/basic.nvs", "// covers: Core\\Str::length\n// This prints `5`.\necho Core\\Str::length('hello');\n");
  s.put("tests/hostile/core/Str/length/huge.nvs", "// Attack: a huge string.\n// hostile: comment-payload\n// AAAA\n$x = 1;\n");
  s.put("benches/members/core/Str/length.nvs", "// bench: complexity linear\n// Measures `length` on a long string.\n");
  s.put("benches/members/core/Str/length.scale.nvs", "$x = 1;\n");
  s.put("website/snippets/hello.nvs", "// Prints a greeting.\necho 'hi';\n");
  s.put("website/src/content/docs/guide.mdx", "---\ntitle: Guide\n---\nRead this.\n");
  s.put("website/src/content/docs/impressum.mdx", "Legal text.\n");
  s.put("website/src/content/docs/reference/core/Str.mdx", "Generated.\n");
  s.put("website/src/data/changes.json", JSON.stringify({ entries: [{ kind: "removed", features: ["Core\\Str::pad"], text: "`pad` is gone.", instead: "Use `padStart`." }], unseen: {} }));
});

afterEach(() => s.cleanup());

const keys = () => userTexts(s.root, world).map((t) => t.key);

describe("the user texts", () => {
  test("every about.md, details.md, help card, handwritten page, published program comment and the changes file is a user text", () => {
    expect(keys()).toEqual([
      "benches/members/core/Str/length.nvs",
      "docs/examples/core/Str/length/about.md",
      "docs/examples/core/Str/length/basic.nvs",
      "docs/examples/core/Str/length/details.md",
      "help:Core\\Str::length",
      "tests/hostile/core/Str/length/huge.nvs",
      "website/snippets/hello.nvs",
      "website/src/content/docs/guide.mdx",
      "website/src/data/changes.json",
    ]);
    const byKey = new Map(userTexts(s.root, world).map((t) => [t.key, t]));
    expect(byKey.get("docs/examples/core/Str/length/basic.nvs")!.feature).toBe("Core\\Str::length");
    expect(byKey.get("benches/members/core/Str/length.nvs")!.feature).toBe("Core\\Str::length");
    expect(byKey.get("website/snippets/hello.nvs")!.feature).toBe("");
    expect(byKey.get("website/src/data/changes.json")!.text).toBe("`pad` is gone.\nUse `padStart`.");
    expect(featureTexts(s.root, LENGTH).map((t) => t.key)).toEqual([
      "docs/examples/core/Str/length/about.md",
      "docs/examples/core/Str/length/details.md",
      "help:Core\\Str::length",
      "docs/examples/core/Str/length/basic.nvs",
      "tests/hostile/core/Str/length/huge.nvs",
      "benches/members/core/Str/length.nvs",
    ]);
  });

  test("a program's text is its comments without directive lines, so a code edit owes no review", () => {
    expect(commentText("// covers: X\n// Prints `5`.\n// hostile: comment-payload\n// AAAA\n$x = 1; // trailing\n/// A doc line.\n")).toBe("// Prints `5`.\n/// A doc line.");
    const before = userTexts(s.root, world).find((t) => t.key.endsWith("basic.nvs"))!.hash;
    s.put("docs/examples/core/Str/length/basic.nvs", "// covers: Core\\Str::slice\n// This prints `5`.\necho Core\\Str::length('world');\n");
    expect(userTexts(s.root, world).find((t) => t.key.endsWith("basic.nvs"))!.hash).toBe(before);
    s.put("docs/examples/core/Str/length/basic.nvs", "// This prints `6`.\necho Core\\Str::length('world');\n");
    expect(userTexts(s.root, world).find((t) => t.key.endsWith("basic.nvs"))!.hash).not.toBe(before);
  });
});

describe("site --check review", () => {
  test("a changed text with no review stamp is named, and a stamp clears it", () => {
    expect(writeReviewBaseline(s.root, world).problems).toEqual([]);
    expect(reviewCheck(world, s.root)).toEqual([]);
    s.put("docs/examples/core/Str/length/about.md", "`Core\\Str::length` counts the characters of a string.\n");
    expect(reviewCheck(world, s.root)).toEqual([
      "docs/examples/core/Str/length/about.md (Core\\Str::length): changed with no review; review it, fix every finding, then `bun nv site --review --stamp docs/examples/core/Str/length/about.md`",
    ]);
    expect(stampReview(s.root, world, ["docs/examples/core/Str/length/about.md"])).toEqual([]);
    expect(reviewCheck(world, s.root)).toEqual([]);
    expect(stampReview(s.root, world, ["docs/nothing.md"])).toEqual(["docs/nothing.md: no user text has this key"]);
  });

  test("a text on the baseline is excused at its baseline hash and owes a review once it changes", () => {
    expect(reviewCheck(world, s.root)).toEqual([`${REVIEW_BASELINE} is missing: write it once with \`bun nv site --review --baseline\``]);
    expect(writeReviewBaseline(s.root, world)).toEqual({ problems: [], texts: 9 });
    const guide = () => userTexts(s.root, world).find((t) => t.key.endsWith("guide.mdx"))!;
    expect(isReviewed(readReview(s.root), guide())).toBe(true);
    s.put("website/src/content/docs/guide.mdx", "---\ntitle: Guide\n---\nRead this page.\n");
    expect(isReviewed(readReview(s.root), guide())).toBe(false);
    expect(reviewList(userTexts(s.root, world).filter((t) => !isReviewed(readReview(s.root), t)))).toEqual(["no feature:", "  website/src/content/docs/guide.mdx"]);
  });

  test("a review stamp takes the text off the baseline, and the baseline is never written twice", () => {
    writeReviewBaseline(s.root, world);
    stampReview(s.root, world, ["website/snippets/hello.nvs"]);
    const baseline = JSON.parse(readFileSync(join(s.root, REVIEW_BASELINE), "utf8")) as Record<string, string>;
    expect(Object.keys(baseline)).toHaveLength(8);
    expect("website/snippets/hello.nvs" in baseline).toBe(false);
    expect(readReview(s.root).reviewed["website/snippets/hello.nvs"]).toBeDefined();
    expect(writeReviewBaseline(s.root, world)).toEqual({ problems: [`${REVIEW_BASELINE} exists; the baseline is written once and only shrinks`], texts: 0 });
  });

  test("the brief prints each text with the three questions and the prose rules", () => {
    const texts = userTexts(s.root, world).filter((t) => t.key === "help:Core\\Str::length" || t.key.endsWith("about.md"));
    const brief = reviewBrief(texts, "- **A sentence has at most 25 words.**", (t) => `shown ${t.feature}`);
    expect(brief).toContain("- **A sentence has at most 25 words.**");
    for (const [i, q] of REVIEW_QUESTIONS.entries()) expect(brief).toContain(`${i + 1}. ${q}`);
    expect(brief).toContain("=== docs/examples/core/Str/length/about.md (Core\\Str::length)\n\n`Core\\Str::length` returns the number of characters.");
    expect(brief).toContain("=== help:Core\\Str::length (Core\\Str::length)\n\nshown Core\\Str::length");
  });
});

describe("the Reviewed proof", () => {
  test("a feature with an unreviewed text owes Reviewed", () => {
    const want = { tests: 0, rust: 0, examples: 0, perf: false, hostile: 0, about: false, help: false, reviewed: true };
    const policy = { member: want } as unknown as Policy;
    const proofs: Proofs = { nvst: [], rust: [], examples: [], hostile: [], about: "", aboutProblem: "", details: "", intro: "", introNames: false, bench: "", perf: null, perfAny: null, inferred: 0, gaps: [] };
    const first = featureTexts(s.root, LENGTH).find((t) => !isReviewed(readReview(s.root), t))!.key;
    expect(first).toBe("docs/examples/core/Str/length/about.md");
    expect(owed(LENGTH, { ...proofs, unreviewed: first }, policy, new Map()).reviewed).toContain("about.md changed with no review");
    // No skip file excuses it.
    expect(owed(LENGTH, { ...proofs, unreviewed: first }, policy, new Map([[LENGTH.id, { reviewed: "no" }]])).reviewed).toBeDefined();
    expect(owed(LENGTH, { ...proofs, unreviewed: "" }, policy, new Map()).reviewed).toBeUndefined();
  });
});
