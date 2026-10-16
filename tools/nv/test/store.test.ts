import { afterEach, describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { defineRecord, s } from "../lib/schema.ts";
import { idAt, load, pathOf, remove, serialize, write } from "../lib/store.ts";
import { scratch, type Scratch } from "./scratch.ts";

const rule = defineRecord({
  name: "rule",
  dir: "rules",
  schema: s.object({ title: s.string(), status: s.enum("live", "retired"), seeAlso: s.array(s.string()), env: s.map(s.int()) }),
});
const chain = defineRecord({ name: "chain", dir: "chain", single: true, schema: s.array(s.slug()) });

let tmp: Scratch;
afterEach(() => tmp?.cleanup());

describe("store", () => {
  test("a written record loads back equal, and writing it again changes nothing", () => {
    tmp = scratch();
    const value = { env: { b: 2, a: 1 }, seeAlso: ["x/y"], status: "live" as const, title: "T" };
    const first = write(rule, "types/conversion", value, tmp.root);
    expect(first).toEqual({ path: "data/rules/types/conversion.json", changed: true });
    const [loaded] = load(rule, tmp.root);
    expect(loaded?.id).toBe("types/conversion");
    expect(loaded?.issues).toEqual([]);
    expect(loaded?.value).toEqual(value);
    expect(write(rule, "types/conversion", loaded!.value, tmp.root).changed).toBe(false);
  });

  test("the text is schema key order, sorted map keys, two-space indent, LF and one final newline", () => {
    tmp = scratch();
    write(rule, "a/b", { env: { z: 1, a: 2 }, seeAlso: [], status: "retired", title: "T" }, tmp.root);
    const text = readFileSync(join(tmp.root, "data/rules/a/b.json"), "utf8");
    expect(text).toBe('{\n  "title": "T",\n  "status": "retired",\n  "seeAlso": [],\n  "env": {\n    "a": 2,\n    "z": 1\n  }\n}\n');
  });

  test("an invalid value is never written", () => {
    tmp = scratch();
    expect(() => write(rule, "a/b", { title: 1 } as never, tmp.root)).toThrow();
    expect(load(rule, tmp.root)).toEqual([]);
  });

  test("a file that is not JSON, or not in the writer's layout, is an issue on load", () => {
    tmp = scratch();
    tmp.put("data/rules/a/broken.json", "{");
    tmp.put("data/rules/a/loose.json", '{"title":"T","status":"live","seeAlso":[],"env":{}}');
    const [broken, loose] = load(rule, tmp.root);
    expect(broken?.issues[0]?.message).toStartWith("is not JSON");
    expect(loose?.issues.map((i) => i.message)).toEqual([
      "is not in the writer's layout; `bun nv` rewrites it when it next writes this record",
    ]);
    expect(write(rule, "a/loose", loose!.value, tmp.root).changed).toBe(true);
    expect(readFileSync(join(tmp.root, "data/rules/a/loose.json"), "utf8")).toBe(serialize(rule, loose!.value));
    expect(load(rule, tmp.root)[1]?.issues).toEqual([]);
  });

  test("a single-file type has one path and its name as its id", () => {
    tmp = scratch();
    expect(pathOf(chain, "anything")).toBe("data/chain.json");
    write(chain, "chain", ["one", "two"], tmp.root);
    expect(load(chain, tmp.root).map((r) => [r.id, r.value])).toEqual([["chain", ["one", "two"]]]);
    expect(idAt(rule, "data/chain.json")).toBeNull();
  });

  test("a file with a second dot is not a plain record, and remove deletes one", () => {
    expect(idAt(rule, "data/rules/a/b.handoff.json")).toBeNull();
    tmp = scratch();
    write(rule, "a/b", { title: "T", status: "live", seeAlso: [], env: {} }, tmp.root);
    expect(remove(rule, "a/b", tmp.root)).toBe(true);
    expect(remove(rule, "a/b", tmp.root)).toBe(false);
  });
});
