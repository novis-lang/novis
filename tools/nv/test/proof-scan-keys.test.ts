import { describe, expect, test } from "bun:test";
import { markerKeys, scannedFor } from "../proofs/markers.ts";
import { anchorScan, isAnchorFile } from "../proofs/roster.ts";

describe("the keys a proof scan reads a file by", () => {
  test("a case names the features its markers cover and the calls it makes", () => {
    const text = "// covers: Core\\Str::length, `lang:x`\n--FILE--\nCore\\Str::upper(\"a\");\n";
    expect(markerKeys("tests/conformance/core/a.nvst", text).sort()).toEqual(["calls:#static:Str::upper", "covers:#Core\\Str::length", "covers:#lang:x"]);
    expect(markerKeys("docs/examples/core/a.nvs", text)).toEqual([]);
    expect(scannedFor("crates/nvs-stdlib/src/a.rs")).toEqual({ markers: true, calls: false });
  });

  test("a stdlib file names the members its class declares, and its name consts sign it", () => {
    const text = 'const NAME: &str = r"Core\\Demo";\nconst CLASS: CoreClass = CoreClass { name: NAME, methods: &[CoreMethod { name: "run" }] };\n';
    const got = anchorScan("crates/nvs-stdlib/src/demo.rs", text);
    expect(got.keys).toEqual(["anchor:#Core\\Demo::run"]);
    expect(anchorScan("crates/nvs-stdlib/src/demo.rs", text.replace("Demo", "Other")).consts).not.toBe(got.consts);
    expect(isAnchorFile("crates/nvs-runtime/src/x.rs")).toBe(true);
    expect(isAnchorFile("crates/nvs-ir/src/x.rs")).toBe(false);
  });
});
