import { afterEach, describe, expect, test } from "bun:test";
import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { ROOT } from "../lib/paths.ts";
import { apply, fill, lf, markdown, MARKER, orphans, removeOrphans, type Output } from "../lib/render.ts";
import { coreMembers, specTables } from "../renderers/spec-tables.ts";
import { recordFiles, SCHEMA_DIR } from "../lib/store.ts";
import { recordSchemas, renderRecordSchemas } from "../renderers/record-schemas.ts";
import { RECORDS } from "../schema/index.ts";
import { addRegistryMembers, parseSignature } from "../renderers/website-core.ts";
import { referenceData } from "../renderers/website-reference.ts";
import type { Entry } from "../proofs/roster.ts";
import { renderWebsiteCompare } from "../renderers/website-compare.ts";
import { EDITOR_GRAMMAR, renderWebsiteGrammar } from "../renderers/website-grammar.ts";
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

  test("the website grammar is the editors' grammar opened in code mode, with no comments", () => {
    const editor = JSON.parse(readFileSync(join(ROOT, EDITOR_GRAMMAR), "utf8"));
    const site = JSON.parse(renderWebsiteGrammar(readFileSync(join(ROOT, EDITOR_GRAMMAR), "utf8")).text);
    expect(site.scopeName).toBe(editor.scopeName);
    expect(site.aliases).toEqual(["novis", "nvs"]);
    expect(site.patterns).toContainEqual({ include: "#code" });
    expect(Object.keys(site.repository)).toEqual(Object.keys(editor.repository));
    expect(site.comment).toBe(MARKER);
    expect(JSON.stringify(site.repository)).not.toContain('"comment"');
  });

  test("the website's comparison data is the record, marked, or says that none is recorded", () => {
    expect(JSON.parse(renderWebsiteCompare(null).text)).toEqual({ comment: MARKER, recorded: false });
    const record = { date: "2026-10-10T08:00:00", cases: { hello: {} }, scenarios: null };
    const copy = JSON.parse(renderWebsiteCompare(JSON.stringify(record)).text);
    expect(copy).toEqual({ comment: MARKER, recorded: true, ...record });
    expect(Object.keys(copy).slice(0, 3)).toEqual(["comment", "recorded", "date"]);
    expect(() => renderWebsiteCompare("[]")).toThrow("no cases");
    expect(() => renderWebsiteCompare('{"date":"x"}')).toThrow("no cases");
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

  test("a class the spec does not name takes its members, shape types and owned enums from the registry", () => {
    const reg = {
      classes: [
        {
          name: "Novis\\Pdf\\Document",
          short: "A PDF document.",
          members: [{ name: "standard", kind: "instance", signature: "standard(Standard $standard): Document" }],
          types: [{ name: "Asset", type: "{path: string, bytes: bytes}" }],
        },
      ],
      enums: [
        { name: "Novis\\Pdf\\Document::Standard", cases: ["PdfA2b", "PdfUa1"] },
        { name: "Novis\\Image\\Format", cases: ["Png"] },
      ],
    };
    const [cls] = addRegistryMembers([], reg, {});
    expect(cls!.members.map((m) => m.signature)).toEqual(["standard(Standard $standard): Document"]);
    expect(cls!.types).toEqual([{ name: "Asset", type: "{path: string, bytes: bytes}" }]);
    // A namespace's own enum has no class to sit on, so it is a Types page alone.
    expect(cls!.enums).toEqual([{ name: "Standard", cases: ["PdfA2b", "PdfUa1"], raw: "Standard { PdfA2b, PdfUa1 }" }]);
  });

  test("every enum, exception and interface in the roster is a Types page, grouped by kind", () => {
    const entry = (id: string, kind: Entry["kind"], path: string): Entry => ({ id, kind, group: `types:${kind}`, path, anchor: "", summary: "", help: "" });
    const pages = referenceData(
      [
        entry("Throwable", "exception", "types/Throwable"),
        entry("Novis\\Pdf\\Document::Standard", "enum", "types/Novis-Pdf-Document-Standard"),
        entry("Core\\Arr::Order", "enum", "types/Arr-Order"),
        entry("Core\\Str::length", "member", "core/Str/length"),
      ],
      [],
    );
    expect(pages.map((p) => [p.area, p.group, p.url, p.intro?.examples])).toEqual([
      ["types", "Enums", "/reference/types/arr-order/", "types/Arr-Order"],
      ["types", "Enums", "/reference/types/novis-pdf-document-standard/", "types/Novis-Pdf-Document-Standard"],
      ["types", "Exceptions", "/reference/types/throwable/", "types/Throwable"],
    ]);
  });


  test("every record type has a JSON Schema under data/schema/, which no record walk reads", () => {
    const outputs = renderRecordSchemas();
    expect(outputs.map((o) => o.path)).toEqual(RECORDS.map((t) => `${SCHEMA_DIR}/${t.name}.schema.json`));
    for (const o of outputs) {
      const schema = JSON.parse(o.text);
      expect(schema.$comment).toBe(MARKER);
      expect(schema.$schema).toContain("json-schema.org");
    }
    expect(orphans([recordSchemas.owns!], outputs)).toEqual([]);
    expect(recordFiles().filter((p) => p.startsWith(`${SCHEMA_DIR}/`))).toEqual([]);
  });

  test("two outputs for one path are an error", () => {
    expect(() => apply([{ path: "x", text: "" }, { path: "x", text: "" }], { check: true })).toThrow("two renderers");
  });

  test("a core table keeps its heading path and escaped cells, and a fenced table is not read", () => {
    tmp = scratch();
    tmp.put(
      "docs/spec/01-core-library.md",
      [
        "# Spec",
        "",
        "| Mark | Meaning |",
        "|---|---|",
        "| Q | quick |",
        "",
        "# Part I",
        "## 1. `Core\\Str`",
        "### Inspection",
        "",
        "| Member | Signature |",
        "|---|---|",
        "| `length` | `(string $s): int\\|null` |",
        "",
        "```",
        "| not | a table |",
        "|---|---|",
        "```",
      ].join("\n"),
    );
    expect(coreMembers(tmp.root)).toEqual({
      tables: [
        { section: ["Spec"], columns: ["Mark", "Meaning"], rows: [["Q", "quick"]] },
        { section: ["Part I", "1. `Core\\Str`", "Inspection"], columns: ["Member", "Signature"], rows: [["`length`", "`(string $s): int\\|null`"]] },
      ],
    });
    const outputs = specTables.render(tmp.root) as Output[];
    expect(outputs.map((o) => o.path)).toEqual(["data/spec/core-members.json"]);
  });

  test("a spec row with the wrong cell count fails the render naming the line", () => {
    tmp = scratch();
    tmp.put("docs/spec/01-core-library.md", "| A | B |\n|---|---|\n| `at` | `(a)` | extra |\n");
    expect(() => coreMembers(tmp.root)).toThrow("docs/spec/01-core-library.md:3 has 3 cell(s) under 2 column(s)");
  });
});
