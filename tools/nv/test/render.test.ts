import { afterEach, describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { apply, fill, lf, markdown, MARKER } from "../lib/render.ts";
import { scratch, type Scratch } from "./scratch.ts";

let tmp: Scratch;
afterEach(() => tmp?.cleanup());

describe("render", () => {
  test("lf normalises line ends, trailing blanks and the final newline", () => {
    expect(lf("a \r\nb\r\n\r\n\n")).toBe("a\nb\n");
    expect(lf("x")).toBe("x\n");
  });

  test("fill replaces every name and refuses a missing one", () => {
    expect(fill("{{a}} and {{ b }}", { a: "1", b: "2" })).toBe("1 and 2");
    expect(() => fill("{{gone}}", {})).toThrow("{{gone}}");
  });

  test("the marker goes first, or right after front matter", () => {
    expect(markdown("# T")).toBe(`<!-- ${MARKER} -->\n# T\n`);
    expect(markdown("---\nid: x\n---\n# T\n")).toBe(`---\nid: x\n---\n<!-- ${MARKER} -->\n# T\n`);
  });

  test("apply --check reports without writing, and a write makes the next check clean", () => {
    tmp = scratch();
    tmp.put("out/a.md", "old\n");
    tmp.put("out/b.md", "same\n");
    const outputs = [
      { path: "out/a.md", text: "new\r\n" },
      { path: "out/b.md", text: "same" },
      { path: "out/c.md", text: "created" },
    ];
    expect(apply(outputs, { check: true, root: tmp.root })).toEqual({ stale: ["out/a.md", "out/c.md"], unchanged: 1 });
    expect(readFileSync(join(tmp.root, "out/a.md"), "utf8")).toBe("old\n");
    apply(outputs, { check: false, root: tmp.root });
    expect(readFileSync(join(tmp.root, "out/a.md"), "utf8")).toBe("new\n");
    expect(apply(outputs, { check: true, root: tmp.root }).stale).toEqual([]);
  });

  test("two outputs for one path are an error", () => {
    expect(() => apply([{ path: "x", text: "" }, { path: "x", text: "" }], { check: true })).toThrow("two renderers");
  });
});
