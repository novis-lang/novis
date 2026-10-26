import { expect, test } from "bun:test";
import { build } from "../../../viewer/build.ts";
import { ROOT } from "../lib/paths.ts";
import { stale } from "../cmd/viewer.ts";
import { scratch } from "./scratch.ts";

test("two builds of the viewer give the same bytes, and name no path of this machine", async () => {
  const [a, b] = [await build(), await build()];
  expect(Buffer.from(a["viewer.js"]).equals(Buffer.from(b["viewer.js"]))).toBe(true);
  expect(Buffer.from(a["viewer.css"]).equals(Buffer.from(b["viewer.css"]))).toBe(true);
  const js = new TextDecoder().decode(a["viewer.js"]);
  for (const root of [ROOT, ROOT.replace(/\\/g, "/")]) expect(js.includes(root)).toBe(false);
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
