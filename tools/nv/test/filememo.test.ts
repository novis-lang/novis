import { describe, expect, test } from "bun:test";
import { rmSync } from "node:fs";
import { join } from "node:path";
import { FileMemo } from "../lib/filememo.ts";
import { linksIn } from "../cmd/links.ts";
import { CACHE } from "../lib/paths.ts";
import { scratch } from "./scratch.ts";

describe("a per-file memo", () => {
  test("a file's result is computed once for its text, again when the text changes, and a file not asked about is dropped", () => {
    const s = scratch();
    const name = `test-${crypto.randomUUID()}`;
    try {
      s.put("tool.ts", "export const version = 1;");
      const module = join(s.root, "tool.ts");
      let computed = 0;
      const count = () => ++computed;
      const first = new FileMemo<number>(name, module);
      first.get("a.rs", "one", count);
      first.get("b.rs", "two", count);
      first.save();
      expect(computed).toBe(2);
      const second = new FileMemo<number>(name, module);
      expect(second.get("a.rs", "one", count)).toBe(1);
      expect(second.get("b.rs", "changed", count)).toBe(3);
      second.save();
      expect(computed).toBe(3);
      // `a.rs` was not asked about in the third run, so the fourth computes it again.
      const third = new FileMemo<number>(name, module);
      third.get("b.rs", "changed", count);
      third.save();
      expect(new FileMemo<number>(name, module).get("a.rs", "one", count)).toBe(4);
      // An edit to the module that computes the results computes everything again.
      s.put("tool.ts", "export const version = 2;");
      expect(new FileMemo<number>(name, module).get("b.rs", "changed", count)).toBe(5);
    } finally {
      rmSync(join(CACHE, "filememo", `${name}.json`), { force: true });
      s.cleanup();
    }
  });

  test("the links of a text are read without looking a target up", () => {
    const md = "see [a](other.md) and [b](/abs.md)\n```\n[c](inside.md)\n```\n";
    expect(linksIn(md, false)).toEqual([
      { line: 1, raw: "other.md", from: "base", target: "other.md" },
      { line: 1, raw: "/abs.md", kind: "absolute" },
    ]);
    expect(linksIn("//! [x](/docs/a.md) [y](docs/b.md) [z](c.md)\n", true)).toEqual([
      { line: 1, raw: "/docs/a.md", from: "root", target: "docs/a.md" },
      { line: 1, raw: "docs/b.md", kind: "relative" },
    ]);
  });
});
