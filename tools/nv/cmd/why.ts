// `bun nv why "<check name>"`: what one unit's key is read from. It prints `source: <source>`, which
// says where the verdict is kept and how narrow the key is (`tools/nv/keys/checks.ts` defines each),
// then the key's inputs grouped by partition. A package's files at one tier are one line, and any
// other input is a line of its own. The name is a check's, as the driver files its verdict, or a test
// binary's `<package> <kind> <target>`. A name several checks share prints each of them.
//
// Exits 0 when the name is a unit's, and 2 when it is not.

import { type Unit, loadRecords, units } from "../keys/checks.ts";
import { metadata } from "../keys/graph.ts";
import type { Part } from "../keys/key.ts";
import { Tree } from "../keys/tree.ts";

export const summary = "what a check's key is read from, grouped by partition: nv why <check name>";

/** One line per package directory and tier, and one per other input, under each partition. */
export function describe(parts: Part[]): string[] {
  const groups = new Map<string, Map<string, number>>();
  for (const p of parts) {
    const pkg = /^(crates\/[^/]+\/)(?!tests\/|benches\/|examples\/)./.exec(p.label)?.[1];
    const key = pkg !== undefined && !p.label.endsWith("/Cargo.toml") ? `${pkg}\0${p.tier}` : `${p.label}\0${p.tier}\0one`;
    let g = groups.get(p.partition);
    if (!g) groups.set(p.partition, (g = new Map()));
    g.set(key, (g.get(key) ?? 0) + 1);
  }
  const out: string[] = [];
  for (const partition of [...groups.keys()].sort()) {
    const g = groups.get(partition)!;
    const n = [...g.values()].reduce((a, b) => a + b, 0);
    out.push(`${partition}: ${n} input(s)`);
    for (const [key, count] of [...g].sort(([x], [y]) => (x < y ? -1 : x > y ? 1 : 0))) {
      const [label, tier, one] = key.split("\0");
      out.push(one ? `  ${label}  ${tier}` : `  ${label}  ${count} file(s) at ${tier}`);
    }
  }
  return out;
}

export async function run(args: string[]): Promise<number> {
  if (args.length !== 1 || args[0]!.startsWith("-")) {
    console.error("nv why: takes one check name, quoted");
    return 2;
  }
  const graph = await metadata();
  if (!graph) {
    console.error("nv why: `cargo metadata` failed, and every key needs the graph");
    return 2;
  }
  const found: Unit[] = units(loadRecords(graph)).filter((u) => u.name === args[0]);
  if (found.length === 0) {
    console.error(`nv why: no check or test binary is named ${JSON.stringify(args[0])}`);
    return 2;
  }
  const tree = await Tree.read();
  found.forEach((u, i) => {
    if (i > 0) console.log("");
    if (found.length > 1) console.log(`${u.name} (${i + 1} of ${found.length})`);
    console.log(`source: ${u.source}`);
    for (const line of describe(u.parts(tree))) console.log(line);
  });
  tree.save();
  return 0;
}
