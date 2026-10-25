// `bun nv machine`: what this box is, and the parallel widths it implies. `tools/nv/lib/machine.ts` owns
// the policy, the probes and the cache in `.loop/machine.json`; this prints them.
//
//     bun nv machine             what this box is, and the widths it implies
//     bun nv machine --refresh   probe again, forgetting what was cached
//     bun nv machine --json      the cache as it stands, for a tool to read

import { FLOOR, FRACTION, dump, load, profile, save, stale, width, type Doc } from "../lib/machine.ts";
import { fixed } from "../lib/py.ts";

export const summary = "what this box is and how wide work may run on it, cached in .loop/machine.json: nv machine [--refresh] [--json]";

const HELP = `usage: bun nv machine [--refresh] [--json]

  --refresh   re-probe every cached context now, instead of when it goes stale
  --json      print the cache as JSON`;

function show(doc: Doc, asJson: boolean): number {
  if (asJson) {
    console.log(dump(doc));
    return 0;
  }
  const contexts = doc.contexts ?? {};
  if (Object.keys(contexts).length === 0) {
    console.log("machine: nothing probed yet. The first `bun nv try` or `bun nv valgrind` run fills this in,");
    console.log("         or `bun nv machine --refresh` does it now.");
    return 0;
  }
  console.log(`${"context".padEnd(9)} ${"cores".padStart(5)} ${"free".padStart(8)}  ${"width".padStart(5)}  probed`);
  for (const name of Object.keys(contexts).sort()) {
    const e = contexts[name]!;
    const mem = e.mem_kb ? `${fixed(e.mem_kb / 1048576, 1)}G` : "--";
    const w = width(e.cores);
    const stamp = (e.probed ?? "?") + (stale(e) ? " (stale)" : "");
    console.log(`${name.padEnd(9)} ${String(e.cores ?? 0).padStart(5)} ${mem.padStart(8)}  ${String(w).padStart(5)}  ${stamp}`);
  }
  console.log(
    `\n  ${Math.trunc(FRACTION * 100)}% of the cores the work sees, floor ${FLOOR}, capped by the item count.\n` +
      "  NVS_JOBS overrides every width for one run; NVS_VALGRIND_JOBS and NVS_TRY_JOBS one command's.",
  );
  return 0;
}

export async function run(args: string[]): Promise<number> {
  if (args.some((a) => a === "-h" || a === "--help")) {
    console.log(HELP);
    return 0;
  }
  const unknown = args.filter((a) => a !== "--refresh" && a !== "--json");
  if (unknown.length > 0) {
    console.error(`${HELP}\nnv machine: error: unrecognized arguments: ${unknown.join(" ")}`);
    return 2;
  }
  if (args.includes("--refresh")) {
    // `local` is probed from here. A cache may still hold another context an older tool probed, and
    // refreshing drops it.
    const doc = load();
    const contexts = doc.contexts ?? {};
    const dropped = Object.keys(contexts).filter((n) => n !== "local").sort();
    if (dropped.length > 0) {
      for (const name of dropped) delete contexts[name];
      save(doc);
      console.log(`machine: dropped ${dropped.join(", ")} -- the next run that uses one re-probes it`);
    }
    profile("local", { refresh: true });
  } else {
    profile("local");
  }
  return show(load(), args.includes("--json"));
}
