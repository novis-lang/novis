import { afterEach, describe, expect, test } from "bun:test";
import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { NUMBER_CITE } from "../cmd/chain.ts";
import { chainGoals, liveGoal } from "../lib/chain.ts";
import { ROOT } from "../lib/paths.ts";
import { GOAL_PLAN, renderGoalPlan } from "../renderers/goal-plan.ts";
import { apply, fill, lf, markdown, MARKER, orphans, removeOrphans } from "../lib/render.ts";
import { recordFiles, SCHEMA_DIR } from "../lib/store.ts";
import { recordSchemas, renderRecordSchemas } from "../renderers/record-schemas.ts";
import { RECORDS } from "../schema/index.ts";
import { parseSignature } from "../renderers/website-core.ts";
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


  test("the goal plan lists every goal in chain order, marks the live one, and names none by its number", () => {
    const goals = chainGoals();
    const live = liveGoal(goals);
    const plan = renderGoalPlan(ROOT);
    expect(plan.path).toBe(GOAL_PLAN);
    expect(plan.text.split("\n", 1)[0]).toBe(`<!-- ${MARKER} -->`);
    const rows = [...plan.text.matchAll(/^\| (\d+) \| \[?`([^`]+)`/gm)].map((m) => [Number(m[1]), m[2]]);
    expect(rows).toEqual(goals.map((g) => [g.num, g.slug]));
    if (live) expect(plan.text).toContain(`\`${live.slug}\` is live at ${live.num} of ${goals.length}`);
    expect(plan.text.match(NUMBER_CITE)).toBeNull();
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
});
