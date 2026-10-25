import { describe, expect, test } from "bun:test";
import { existsSync } from "node:fs";
import { join } from "node:path";
import { blocks, replaced, unwrapped, wrapped } from "../lib/nvs-fences.ts";
import { held, heldPage, outside, parseCorpus, readCorpus } from "../lib/nvs-fmt-corpus.ts";
import { ROOT } from "../lib/paths.ts";

describe("parseCorpus", () => {
  test("reads trees, pages and skips, and leaves comments and blank lines out", () => {
    const c = parseCorpus("# a comment\n\ntree docs/examples\ntree tests/\npages README.md\nskip tests/fmt/input  --  wrong on purpose\n");
    expect(c).toEqual({ trees: ["docs/examples", "tests"], skips: ["tests/fmt/input"], pages: ["README.md"] });
  });

  test("a skip without its reason, and a line of another kind, are errors", () => {
    expect(() => parseCorpus("skip tests/fmt/input\n")).toThrow(/a skip says why/);
    expect(() => parseCorpus("take tests\n")).toThrow(/expected `tree <dir>`/);
  });
});

describe("held and outside", () => {
  const c = parseCorpus("tree tests\ntree docs/examples\nskip tests/fmt/input  --  why\n");

  test("a file under a tree is held, and one under its skip is not", () => {
    expect(held(c, "tests/hostile/a.nvs")).toBe(true);
    expect(held(c, "tests/fmt/input/a.nvs")).toBe(false);
    expect(outside(c, "tests/fmt/input/a.nvs")).toBe(false);
  });

  test("a skip can name one file", () => {
    const one = parseCorpus("tree tests\nskip tests/check/one-line.nvs  --  why\n");
    expect(held(one, "tests/check/one-line.nvs")).toBe(false);
    expect(held(one, "tests/check/one-line.nvs.bak/a.nvs")).toBe(true);
  });

  test("a directory that only starts with a tree's name is outside it", () => {
    expect(held(c, "docs/examples-old/a.nvs")).toBe(false);
    expect(outside(c, "docs/examples-old/a.nvs")).toBe(true);
  });
});

describe("heldPage", () => {
  const c = parseCorpus("pages docs/reference\npages README.md\nskip docs/reference/old  --  why\n");

  test("a .md or .mdx file under a pages entry is held, and a file of another kind is not", () => {
    expect(heldPage(c, "docs/reference/core/Str.md")).toBe(true);
    expect(heldPage(c, "docs/reference/a.mdx")).toBe(true);
    expect(heldPage(c, "README.md")).toBe(true);
    expect(heldPage(c, "docs/reference/a.nvs")).toBe(false);
    expect(heldPage(c, "docs/reference/old/a.md")).toBe(false);
    expect(heldPage(c, "docs/rules/a.md")).toBe(false);
  });
});

describe("blocks", () => {
  const page = "# Title\n\n```nvs\necho \"a\";\n```\n\n- item\n\n  ```novis\n  echo \"b\";\n\n  echo \"c\";\n  ```\n\n```toml\na = 1\n```\n";

  test("finds each nvs block with its line, its indent and its body without the indent", () => {
    const found = blocks(page);
    expect(found.map((b) => [b.line, b.indent, b.body])).toEqual([
      [3, "", 'echo "a";\n'],
      [9, "  ", 'echo "b";\n\necho "c";\n'],
    ]);
  });

  test("a block without <?nvs is formatted behind one, and one with it as it is", () => {
    expect(wrapped("echo 1;\n")).toBe("<?nvs\necho 1;\n");
    expect(unwrapped("echo 1;\n", "<?nvs\necho 1;\n")).toBe("echo 1;\n");
    expect(wrapped("<?nvs\necho 1;\n")).toBe("<?nvs\necho 1;\n");
    expect(unwrapped("<p><?= 1 ?></p>\n", "<p><?= 1 ?></p>\n")).toBe("<p><?= 1 ?></p>\n");
  });

  test("replaced puts each new body back at its indent and keeps the rest of the page", () => {
    const found = blocks(page);
    const out = replaced(page, found, ["echo 'a';\n", "echo 'b';\n\necho 'c';\n"]);
    expect(out).toBe(page.replace('"a"', "'a'").replace('"b"', "'b'").replace('"c"', "'c'"));
    const crlf = page.replaceAll("\n", "\r\n");
    expect(replaced(crlf, blocks(crlf), ["x;\n", "y;\n"])).toContain("```nvs\r\nx;\r\n```");
  });
});

test("every tree, page and skip of the tree's own corpus file exists", () => {
  const c = readCorpus();
  for (const dir of [...c.trees, ...c.pages, ...c.skips.filter((s) => !s.endsWith("/generated"))]) expect(existsSync(join(ROOT, dir))).toBe(true);
});
