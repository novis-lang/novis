import { describe, expect, test } from "bun:test";
import { chapterFile, chapterSources, parseChapterSource } from "../proofs/chapters.ts";

const FRONT = "---\norder: 20\ntitle: Types\nsummary: every type\nkeywords: int, string\n---\n\n";

describe("a language chapter's source", () => {
  test("each heading is a feature, in order, with the markers placed on it", () => {
    const text = `${FRONT}# Comments\n<!-- generated: interfaces -->\n<!-- primer -->\n# Doc comments: \`///\`\n`;
    const s = parseChapterSource("programs", "docs/examples/lang/programs/chapter.md", text);
    expect(s.problems).toEqual([]);
    expect(chapterFile(s)).toBe("docs/reference/lang/20-programs.md");
    expect(s.features.map((f) => [f.slug, f.line, f.primer, f.generated])).toEqual([
      ["comments", 8, false, ["interfaces"]],
      ["doc-comments", 11, true, []],
    ]);
  });

  test("a sentence, a missing key and a heading with no directory are problems", () => {
    const text = "---\norder: x\ntitle: Types\nsummary: s\n---\n\nA chapter preamble.\n# No such feature\n<!-- primer -->\n";
    const problems = parseChapterSource("types", "docs/examples/lang/types/chapter.md", text).problems.join("\n");
    expect(problems).toContain("no `keywords`");
    expect(problems).toContain("`order` is not a positive whole number");
    expect(problems).toContain(":7: only a `# ` heading or a marker belongs here");
    expect(problems).toContain("`# No such feature` has no directory docs/examples/lang/types/no-such-feature");
    expect(problems).toContain("`<!-- primer -->` with no feature below it");
  });

  test("every chapter has a source with no problem, and each source names its chapter's file", () => {
    const sources = chapterSources();
    expect(sources.flatMap((s) => s.problems)).toEqual([]);
    expect(sources.map(chapterFile)).toEqual([
      "docs/reference/lang/10-programs.md",
      "docs/reference/lang/20-types.md",
      "docs/reference/lang/30-expressions.md",
      "docs/reference/lang/40-statements.md",
      "docs/reference/lang/50-classes.md",
      "docs/reference/lang/55-enums.md",
      "docs/reference/lang/60-iteration.md",
      "docs/reference/lang/70-errors.md",
      "docs/reference/lang/80-concurrency.md",
      "docs/reference/lang/90-attributes.md",
      "docs/reference/lang/95-testing.md",
    ]);
  });
});
