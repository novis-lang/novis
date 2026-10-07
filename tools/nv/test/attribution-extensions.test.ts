// `bun nv gen-attribution` over the built-in components: the crates each component links and the C
// library it carries prebuilt ship inside `nvs`, so the committed notice names every one of them.

import { expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { readRecord } from "../cmd/webp-lib.ts";
import { ROOT } from "../lib/paths.ts";

const NOTICE = readFileSync(join(ROOT, "THIRD-PARTY-LICENSES.txt"), "utf8").replaceAll("\r\n", "\n");

/** The notice's component table, as `name version` pairs. */
function listed(): Set<string> {
  const table = NOTICE.slice(NOTICE.indexOf("COMPONENTS ("), NOTICE.indexOf("LICENSE TEXTS ("));
  const rows = [...table.matchAll(/^ {2}(\S+) +(\S+) +\S/gm)];
  return new Set(rows.map((m) => `${m[1]} ${m[2]}`));
}

test("the notice carries every crate the built-in components link", () => {
  const r = Bun.spawnSync(
    [
      "cargo",
      "metadata",
      "--format-version",
      "1",
      "--locked",
      "--manifest-path",
      join(ROOT, "extensions", "image", "Cargo.toml"),
      "--filter-platform",
      "wasm32-wasip2",
    ],
    { cwd: ROOT },
  );
  expect(r.exitCode).toBe(0);
  const meta = JSON.parse(r.stdout.toString()) as {
    packages: { id: string; name: string; version: string }[];
    workspace_members: string[];
    resolve: { nodes: { id: string; deps: { pkg: string; dep_kinds: { kind: string | null }[] }[] }[] };
  };
  const nodes = new Map(meta.resolve.nodes.map((n) => [n.id, n]));
  const seen = new Set<string>();
  const stack = [...meta.workspace_members];
  while (stack.length > 0) {
    const id = stack.pop()!;
    if (seen.has(id)) continue;
    seen.add(id);
    for (const dep of nodes.get(id)!.deps) {
      if (dep.dep_kinds.some((k) => k.kind === null || k.kind === "build")) stack.push(dep.pkg);
    }
  }
  const own = new Set(meta.workspace_members);
  const crates = meta.packages.filter((p) => seen.has(p.id) && !own.has(p.id)).map((p) => `${p.name} ${p.version}`);
  expect(crates).toContain(`image ${meta.packages.find((p) => p.name === "image")!.version}`);
  const table = listed();
  expect(crates.filter((c) => !table.has(c))).toEqual([]);
});

test("the notice carries libwebp's licence", () => {
  const dir = join(ROOT, "extensions", "image", "libwebp");
  const version = readRecord(dir).source.version;
  expect(listed()).toContain(`libwebp ${version}`);
  for (const file of ["COPYING", "PATENTS"]) {
    const first = readFileSync(join(dir, file), "utf8").replaceAll("\r\n", "\n").trim().split("\n")[0]!;
    expect(NOTICE).toContain(first);
  }
  expect(NOTICE).toMatch(new RegExp(`\\[\\d+/\\d+\\]  BSD-3-Clause\\n=+\\n\\nApplies to:\\n(?:  .+\\n)*  libwebp ${version.replaceAll(".", "\\.")}\\n`));
});
