// What `bun nv <command>` loads: `main.ts`, its static imports, and the command's module with every
// module it imports, statically or through `import("...")` with a literal path, followed to the end.
// `main.ts` loads a command's module only when it runs, so its dynamic imports are not followed. A bare
// specifier such as `node:fs` or a package is Bun's or `node_modules`', and the manifests
// `NV_MANIFESTS` names stand for those.

import { dirname } from "node:path";
import { normalize, type Tree } from "./tree.ts";

/** What every `bun nv` run is loaded against besides its modules. */
export const NV_MANIFESTS = ["package.json", "bun.lock", "tsconfig.json"];
export const MAIN = "tools/nv/main.ts";

const transpiler = new Bun.Transpiler({ loader: "ts" });
const memo = new WeakMap<Tree, Map<string, string[]>>();

/** The relative imports of `rel`, resolved to repo-relative paths; `statics` leaves out `import()`. */
function importsOf(tree: Tree, rel: string, statics = false): string[] {
  let found: { path: string; kind: string }[];
  try {
    found = transpiler.scanImports(tree.text(rel));
  } catch {
    return [];
  }
  return found.filter((i) => i.path.startsWith(".") && (!statics || i.kind !== "dynamic-import")).map((i) => normalize(`${dirname(rel)}/${i.path}`));
}

/** `entry` and every module it reaches, sorted. A path the tree lacks is kept, so its arrival moves a key. */
export function closure(tree: Tree, entry: string): string[] {
  let m = memo.get(tree);
  if (!m) memo.set(tree, (m = new Map()));
  const got = m.get(entry);
  if (got) return got;
  const seen = new Set<string>();
  const todo = [entry];
  while (todo.length > 0) {
    const rel = todo.pop()!;
    if (seen.has(rel)) continue;
    seen.add(rel);
    if (tree.has(rel)) todo.push(...importsOf(tree, rel));
  }
  const out = [...seen].sort();
  m.set(entry, out);
  return out;
}

/** The modules `bun nv <name>` loads, or null when no command module is named `name`. */
export function commandModules(tree: Tree, name: string): string[] | null {
  const mod = `tools/nv/cmd/${name}.ts`;
  if (!/^[a-z][a-z0-9-]*$/.test(name) || !tree.has(mod)) return null;
  const out = new Set([MAIN, ...closure(tree, mod)]);
  for (const dep of importsOf(tree, MAIN, true)) for (const rel of closure(tree, dep)) out.add(rel);
  return [...out].sort();
}
