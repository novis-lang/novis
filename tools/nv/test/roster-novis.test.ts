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
    expect(path("Core\\Str::length")).toBe("core/Str/length");
    expect(entries.find((e) => e.id === "Novis\\Image\\Image::Fit")?.path).toBe("types/Novis-Image-Image-Fit");
    expect(entries.filter((e) => e.kind === "member" && e.id.startsWith("Novis\\")).every((e) => e.path.startsWith("novis/"))).toBe(true);
  }, 120_000);

  test("a built-in component's engine class is no feature", async () => {
    const nvs = covwsNvs();
    if (!existsSync(nvs)) throw new Error(`${nvs} is built by \`bun nv verify\` before this test runs`);
    const meta = await metaJson(nvs);
    const engines = (meta.classes ?? []).filter((c) => c.internal).map((c) => c.name);
    expect(engines).toEqual(["Novis\\Image\\Codec", "Novis\\Intl\\Icu", "Novis\\Pdf\\Engine", "Novis\\Spreadsheet\\Engine"]);
    const ids = new Set((await roster(nvs, meta)).map((e) => e.id));
    expect([...ids].filter((id) => engines.some((name) => id.startsWith(`${name}::`)))).toEqual([]);
    expect(ids.has("Novis\\Image\\Image::Output")).toBe(false);
    expect(ids.has("Novis\\Image\\Image::open")).toBe(true);
  }, 120_000);
});
