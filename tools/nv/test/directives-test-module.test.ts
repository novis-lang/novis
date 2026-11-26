import { describe, expect, test } from "bun:test";
import { TEST_MODULE } from "../cmd/directives.ts";

describe("where `nv directives` stops reading a source file", () => {
  test("a `#[cfg(test)]` on a `use` is not the test module", () => {
    const text = [
      "#[cfg(test)]",
      "pub(crate) use smtp::tests::localhost;",
      "",
      'fn read() { configured(ctx, endpoint, "return_path"); }',
      "",
      "#[cfg(test)]",
      "mod tests;",
    ].join("\n");
    const cut = TEST_MODULE.exec(text);
    expect(cut).not.toBeNull();
    expect(text.slice(0, cut!.index)).toContain("return_path");
  });

  test("a test module with attributes or a visibility is the cut", () => {
    for (const module of [
      "#[cfg(test)]\nmod tests {",
      "#[cfg(test)]\n#[allow(clippy::unwrap_used)]\nmod tests;",
      "#[cfg(test)]\npub(crate) mod fixtures;",
    ]) {
      const text = `fn live() {}\n${module}`;
      expect(TEST_MODULE.exec(text)?.index).toBe("fn live() {}\n".length);
    }
  });
});
