import { afterEach, describe, expect, test } from "bun:test";
import { CORE_CLASSES, rankedClasses, renderCoreClasses } from "../renderers/core-classes.ts";
import { scratch, type Scratch } from "./scratch.ts";

let tmp: Scratch;
afterEach(() => tmp?.cleanup());

const SRC = "crates/nvs-stdlib/src";

describe("core-classes", () => {
  test("a class is listed by rank, then by path, under the path the crate writes", () => {
    tmp = scratch();
    tmp.put(`${SRC}/lib.rs`, "mod db;\nmod math;\n");
    tmp.put(`${SRC}/math.rs`, "/// Numbers.\n// rank: 20\npub const CLASS: CoreClass = CoreClass {\n};\n");
    tmp.put(`${SRC}/db/mod.rs`, "mod registry;\npub(crate) use self::registry::*;\nmod hooks;\n");
    tmp.put(
      `${SRC}/db/registry.rs`,
      "// rank: 10\n#[allow(dead_code)]\npub(crate) const CLASS: CoreClass = CoreClass {\n};\n" +
        "// rank: 20\npub(crate) const ROW: CoreClass = CoreClass {\n};\n",
    );
    tmp.put(`${SRC}/db/hooks.rs`, "// rank: 5\npub const HOOK: CoreClass = CoreClass {\n};\n");
    tmp.put(`${SRC}/cursor.rs`, "pub const CLASS: CoreClass = CoreClass {\n};\n");
    expect(rankedClasses(tmp.root).map((c) => c.path)).toEqual([
      "crate::db::hooks::HOOK",
      "crate::db::CLASS",
      "crate::db::ROW",
      "crate::math::CLASS",
    ]);
    const [out] = renderCoreClasses(tmp.root);
    expect(out!.path).toBe(CORE_CLASSES);
    expect(out!.text).toContain("pub const CLASSES: &[CoreClass] = &[\n    crate::db::hooks::HOOK,\n");
  });

  test("a CoreClass const with no rank fails the render, naming its file and line", () => {
    tmp = scratch();
    tmp.put(`${SRC}/lib.rs`, "mod uri;\n");
    tmp.put(`${SRC}/uri.rs`, "\n/// Links.\npub const CLASS: CoreClass = CoreClass {\n};\n");
    expect(() => rankedClasses(tmp.root)).toThrow(`crate::uri::CLASS (${SRC}/uri.rs:3)`);
  });
});
