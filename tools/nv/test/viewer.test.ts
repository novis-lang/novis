import { expect, test } from "bun:test";
import { build } from "../../../viewer/build.ts";
import { ROOT } from "../lib/paths.ts";
import { readFileSync } from "node:fs";
import { hasFontFace, palette, paletteDrift, stale, TOKENS_CSS, WEBSITE_CSS } from "../cmd/viewer.ts";
import { scratch } from "./scratch.ts";

test("two builds of the viewer give the same bytes, and name no path of this machine", async () => {
  const [a, b] = [await build(), await build()];
  expect(Buffer.from(a["viewer.js"]).equals(Buffer.from(b["viewer.js"]))).toBe(true);
  expect(Buffer.from(a["viewer.css"]).equals(Buffer.from(b["viewer.css"]))).toBe(true);
  const js = new TextDecoder().decode(a["viewer.js"]);
  for (const root of [ROOT, ROOT.replace(/\\/g, "/")]) expect(js.includes(root)).toBe(false);
});

test("the viewer's palette is the website's, and a changed or missing colour is drift", () => {
  const website = readFileSync(WEBSITE_CSS, "utf8");
  const viewer = readFileSync(TOKENS_CSS, "utf8");
  expect(Object.keys(palette(website))).toHaveLength(7);
  expect(paletteDrift(website, viewer)).toEqual([]);
  expect(paletteDrift(website, viewer.replace(/--nv-blue:\s*#[0-9a-f]+;/i, "--nv-blue: #0000ff;"))).toEqual([
    `--nv-blue: the website has ${palette(website).blue}, the viewer has #0000ff`,
  ]);
  expect(paletteDrift(website, viewer.replace(/--nv-gold:[^;]+;/g, ""))[0]).toContain("the viewer has nothing");
});

test("a built stylesheet with a font face is caught, and the viewer's own has none", async () => {
  expect(hasFontFace("@font-face{font-family:x;src:url(x.woff2)}")).toBe(true);
  expect(hasFontFace(new TextDecoder().decode((await build())["viewer.css"]))).toBe(false);
});

test("a committed file that is missing or differs is stale, and one with the build's bytes is not", () => {
  const tmp = scratch();
  try {
    const bundle = { "viewer.js": new TextEncoder().encode("js"), "viewer.css": new TextEncoder().encode("css") };
    expect(stale(bundle, tmp.root)).toEqual(["viewer.js", "viewer.css"]);
    tmp.put("viewer.js", "js");
    tmp.put("viewer.css", "css changed");
    expect(stale(bundle, tmp.root)).toEqual(["viewer.css"]);
    tmp.put("viewer.css", "css");
    expect(stale(bundle, tmp.root)).toEqual([]);
  } finally {
    tmp.cleanup();
  }
});
