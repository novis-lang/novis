import { afterEach, describe, expect, test } from "bun:test";
import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { isGenerated } from "../cmd/links.ts";
import { ROOT } from "../lib/paths.ts";
import { apply, fill, lf, markdown, MARKER, orphans, removeOrphans } from "../lib/render.ts";
import { isDraft, PAGES_DIR as CORE_DIR, parseSignature, websiteCore } from "../renderers/website-core.ts";
import { PAGES_DIR, renderWebsiteRules, websiteRules } from "../renderers/website-rules.ts";
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

  test("a file in an owned directory that no output writes is an orphan, unless it is kept", () => {
    tmp = scratch();
    tmp.put("site/index.mdx", "hub\n");
    tmp.put("site/a/index.md", "a\n");
    tmp.put("site/gone/old.md", "old\n");
    const owned = [{ dir: "site", keep: ["index.mdx"] }];
    const found = orphans(owned, [{ path: "site/a/index.md", text: "a" }], tmp.root);
    expect(found).toEqual(["site/gone/old.md"]);
    removeOrphans(found, ["site"], tmp.root);
    expect(existsSync(join(tmp.root, "site/gone"))).toBe(false);
    expect(existsSync(join(tmp.root, "site/index.mdx"))).toBe(true);
  });

  test("a Core page is an orphan only while it is still a draft, since a person owns the rest", () => {
    tmp = scratch();
    tmp.put(`${CORE_DIR}/index.mdx`, "hub\n");
    tmp.put(`${CORE_DIR}/str/gone.mdx`, "---\nnovis:\n  draft: true\n---\n");
    tmp.put(`${CORE_DIR}/str/kept.mdx`, "---\nnovis:\n  kind: method\n---\n");
    expect(orphans([websiteCore.owns!], [], tmp.root)).toEqual([`${CORE_DIR}/str/gone.mdx`]);
    expect(isDraft("---\ndraft: true\n---\n")).toBe(true);
    expect(isDraft("draft: true\n")).toBe(false);
  });

  test("a spec signature reads its receiver, its parameters and its options bag", () => {
    const sig = parseSignature("`$d->plus(Duration $by, ?int ...$rest = null, {utc?: bool}): DateTime — note`")!;
    expect(sig.receiver).toBe("d");
    expect(sig.params.map((p) => [p.name, p.type, p.variadic, p.optional])).toEqual([
      ["by", "Duration", false, false],
      ["rest", "?int", true, true],
    ]);
    expect(sig.options).toEqual([{ name: "utc", type: "bool" }]);
    expect(sig.returnType).toBe("DateTime");
    expect(parseSignature("not a signature")).toBeNull();
  });

  test("the website's rule pages are what the records render, and every one carries both markers", async () => {
    const outputs = await renderWebsiteRules(ROOT);
    const pages = outputs.filter((o) => o.path.startsWith(`${PAGES_DIR}/`));
    expect(pages.length).toBeGreaterThan(0);
    for (const page of pages) {
      const head = page.text.split("\n", 4).join("\n");
      expect(head).toContain(MARKER);
      expect(isGenerated(page.text)).toBe(true);
    }
    expect(orphans([websiteRules.owns!], outputs)).toEqual([]);
  });

  test("two outputs for one path are an error", () => {
    expect(() => apply([{ path: "x", text: "" }, { path: "x", text: "" }], { check: true })).toThrow("two renderers");
  });
});
