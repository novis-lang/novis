import { describe, expect, test } from "bun:test";
import { existsSync } from "node:fs";
import { covwsNvs } from "../lib/covws.ts";
import { memberPath, metaJson, roster } from "../proofs/roster.ts";

describe("the roster's paths for a built-in component's class", () => {
  test("a Novis class member has its proofs under novis/", async () => {
    expect(memberPath("Novis\\Image\\Image", "open")).toBe("novis/Image-Image/open");
    expect(memberPath("Core\\Db\\Row", "get")).toBe("core/Db-Row/get");

    const nvs = covwsNvs();
    if (!existsSync(nvs)) throw new Error(`${nvs} is built by \`bun nv verify\` before this test runs`);
    const entries = await roster(nvs, await metaJson(nvs));
    const path = (id: string) => entries.find((e) => e.id === id)?.path;
    expect(path("Novis\\Image\\Image::open")).toBe("novis/Image-Image/open");
    expect(path("Novis\\Image\\Color::hex")).toBe("novis/Image-Color/hex");
    expect(path("Novis\\Image\\Codec::info")).toBe("novis/Image-Codec/info");
    expect(path("Core\\Str::length")).toBe("core/Str/length");
    expect(entries.find((e) => e.id === "Novis\\Image\\Fit")?.path).toBe("types/Novis-Image-Fit");
    expect(entries.filter((e) => e.kind === "member" && e.id.startsWith("Novis\\")).every((e) => e.path.startsWith("novis/"))).toBe(true);
  }, 120_000);
});
