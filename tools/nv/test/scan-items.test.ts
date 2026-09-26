import { describe, expect, test } from "bun:test";
import { scanItems } from "../keys/scan.ts";

describe("scanItems", () => {
  test("a helper spelled inside a macro invocation maps to that invocation, and a class const to its class", () => {
    const [math] = scanItems(["crates/nvs-stdlib/src/math.rs"]);
    expect(math!.parsed).toBe(true);
    const sqrt = math!.items.find((i) => i.defines.includes("nvs_core_math_sqrt"));
    expect(sqrt?.id).toBe("unary_float!{nvs_core_math_sqrt}");
    expect(sqrt?.kind).toBe("macro");
    expect(math!.items.find((i) => i.id === "CLASS")?.class).toEqual(["Core\\Math"]);
    expect(math!.items.find((i) => i.id === "CARD")?.cards).toEqual(["Core\\Math"]);
    // The first call may build the scanner.
  }, 180_000);

  test("every row of the registry's class list names its class when the batch holds the files it points at", () => {
    const files = ["crates/nvs-stdlib/src/registry.rs", "crates/nvs-stdlib/src/math.rs", "crates/nvs-stdlib/src/str.rs"];
    const [registry] = scanItems(files);
    const rows = registry!.items.find((i) => i.id === "CLASSES")!.rows!;
    expect(rows[0]!.classes).toEqual(["Core\\Str"]);
    expect(rows.some((r) => r.classes.includes("Core\\Math"))).toBe(true);
  });

  test("a missing file answers unparsed with no items", () => {
    const [gone] = scanItems(["crates/no-such-crate/src/lib.rs"]);
    expect(gone).toEqual({ file: "crates/no-such-crate/src/lib.rs", parsed: false, raw: "", items: [] });
  });
});
