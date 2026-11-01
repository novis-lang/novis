// The VS Code extension's notice, `editors/vscode/THIRD-PARTY-NOTICES.txt`: it names every npm package
// esbuild bundles into `out/extension.js` and the package `scripts/viewer.mjs` copies, and the `.vsix`
// packs it. `bun nv gen-attribution --check` holds the file equal to what the generator writes; this
// holds the generator's package set against what the bundle really carries.
//
// It needs the extension's packages installed: `npm ci` in `editors/vscode`.

import { Glob } from "bun";
import { expect, test } from "bun:test";
import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";
import { ROOT } from "../lib/paths.ts";

const DIR = join(ROOT, "editors", "vscode");
const NOTICE_NAME = "THIRD-PARTY-NOTICES.txt";

/** The first column of the notice's component table. */
function listed(): Set<string> {
  const notice = readFileSync(join(DIR, NOTICE_NAME), "utf8").replaceAll("\r\n", "\n");
  const table = notice.slice(notice.indexOf("COMPONENTS ("), notice.indexOf("LICENSE TEXTS ("));
  return new Set([...table.matchAll(/^ {2}(\S+) +\S/gm)].map((m) => m[1]!));
}

/** The package a path under `node_modules` belongs to, the innermost one where packages nest. */
function packageOf(path: string): string | null {
  const parts = path.replaceAll("\\", "/").split("/");
  const at = parts.lastIndexOf("node_modules");
  if (at < 0 || at + 1 >= parts.length) return null;
  const first = parts[at + 1]!;
  return first.startsWith("@") ? `${first}/${parts[at + 2]}` : first;
}

function sources(dir: string): string[] {
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    if (statSync(path).isDirectory()) return sources(path);
    return name.endsWith(".ts") ? [path] : [];
  });
}

test("the notice names every npm package esbuild bundles into out/extension.js", async () => {
  expect(existsSync(join(DIR, "node_modules", "esbuild")), "run `npm ci` in editors/vscode").toBe(true);
  // The flags of `package.json`'s `bundle` script, with nothing written.
  // The root's packages have no esbuild, so its one call is typed here.
  const esbuild = (await import(join(DIR, "node_modules", "esbuild", "lib", "main.js"))) as {
    build(options: Record<string, unknown>): Promise<{ metafile: { inputs: Record<string, unknown> } }>;
  };
  const result = await esbuild.build({
    absWorkingDir: DIR,
    entryPoints: ["src/extension.ts"],
    bundle: true,
    outfile: "out/extension.js",
    external: ["vscode"],
    format: "cjs",
    platform: "node",
    target: "es2022",
    write: false,
    metafile: true,
    logLevel: "silent",
  });
  const bundled = new Set(Object.keys(result.metafile.inputs).map(packageOf).filter((p) => p !== null));
  expect(bundled.size).toBeGreaterThan(0);
  const names = listed();
  expect([...bundled].filter((p) => !names.has(p)).sort()).toEqual([]);
});

test("the notice names the package scripts/viewer.mjs copies into the package", () => {
  const script = readFileSync(join(DIR, "scripts", "viewer.mjs"), "utf8");
  const copied = [...script.matchAll(/join\(ROOT, "node_modules", "([^"]+)"\)/g)].map((m) => m[1]!);
  expect(copied.length).toBeGreaterThan(0);
  const names = listed();
  expect(copied.filter((p) => !names.has(p))).toEqual([]);
});

test("src/ imports only vscode, node: modules and the manifest's dependencies", () => {
  const manifest = JSON.parse(readFileSync(join(DIR, "package.json"), "utf8")) as { dependencies?: Record<string, string> };
  const allowed = new Set(Object.keys(manifest.dependencies ?? {}));
  const stray: string[] = [];
  for (const file of sources(join(DIR, "src"))) {
    const text = readFileSync(file, "utf8");
    for (const m of text.matchAll(/(?:\bfrom\s+|\bimport\s*\(\s*|\brequire\(\s*|^import\s+)["']([^"'.][^"']*)["']/gm)) {
      const spec = m[1]!;
      if (spec === "vscode" || spec.startsWith("node:")) continue;
      const name = spec.startsWith("@") ? spec.split("/").slice(0, 2).join("/") : spec.split("/")[0]!;
      if (!allowed.has(name)) stray.push(`${file.slice(DIR.length + 1)}: ${spec}`);
    }
  }
  expect(stray).toEqual([]);
});

test("the .vsix packs the notice", () => {
  expect(existsSync(join(DIR, NOTICE_NAME))).toBe(true);
  const patterns = readFileSync(join(DIR, ".vscodeignore"), "utf8")
    .split(/\r?\n/)
    .map((l) => l.trim())
    .filter((l) => l !== "" && !l.startsWith("#") && !l.startsWith("!"));
  expect(patterns.length).toBeGreaterThan(0);
  expect(patterns.filter((p) => new Glob(p).match(NOTICE_NAME))).toEqual([]);
});
