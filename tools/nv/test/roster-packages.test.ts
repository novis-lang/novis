import { describe, expect, test } from "bun:test";
import { markersIn } from "../proofs/markers.ts";
import { packageFeatures } from "../proofs/roster.ts";

describe("the roster's entries for a package's class", () => {
  test("each type under packages/<dir>/src is one entry with its proofs under package/", () => {
    const entries = packageFeatures();
    const client = entries.find((e) => e.id === "Novis\\Rest\\Client");
    expect(client?.kind).toBe("package");
    expect(client?.group).toBe("package:Novis\\Rest");
    expect(client?.path).toBe("package/Novis-Rest-Client");
    expect(client?.anchor).toMatch(/^packages\/rest\/src\/Client\.nvs:\d+$/);
    // The class has a comment above it, so it owes no help.
    expect(client?.help).toBe("");
    expect(client?.summary).toStartWith("An OAuth 2 client");
    expect(entries.every((e) => e.kind === "package" && e.path.startsWith("package/"))).toBe(true);
  });

  test("a covers: marker in a .nvs test credits the #[Test] method under it", () => {
    const text = "    // covers: Novis\\Rest\\Token, Novis\\Rest\\Client\n    #[Test]\n    public function aTokenIsRead(): void\n";
    expect(markersIn("packages/rest/tests/TokenTest.nvs", text)).toEqual([
      ["Novis\\Rest\\Token", "packages/rest/tests/TokenTest.nvs::aTokenIsRead"],
      ["Novis\\Rest\\Client", "packages/rest/tests/TokenTest.nvs::aTokenIsRead"],
    ]);
  });
});
