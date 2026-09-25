// `bun nv relink`: frees `target/release/nvs` for the linker by hand, when a running copy of it holds the
// path. `tools/nv/lib/relink.ts` owns why a held binary fails a Windows link and why moving it aside is
// the answer; the release builds the loop and `bun nv proofs` run already go through it.
//
//     bun nv relink            delete every moved-aside copy nobody runs any longer
//     bun nv relink --free     move the release binary aside first, so the next build can link

import { existsSync } from "node:fs";
import { free, releaseCli, sweep } from "../lib/relink.ts";
import { rel } from "../lib/paths.ts";

export const summary = "move a running target/release/nvs aside so cargo can link, and sweep old copies: nv relink [--free]";

export async function run(args: string[]): Promise<number> {
  if (args.some((a) => a === "-h" || a === "--help")) {
    console.log(summary);
    return 0;
  }
  const unknown = args.filter((a) => a !== "--free");
  if (unknown.length > 0) {
    console.error(`nv relink: unknown argument ${unknown[0]}`);
    return 2;
  }
  const exe = releaseCli();
  if (args.includes("--free")) {
    if (!existsSync(exe)) {
      console.log(`nv relink: ${rel(exe)} does not exist, so there is nothing to free`);
    } else {
      const aside = free(exe);
      if (aside === null) {
        console.error(`nv relink: ${rel(exe)} could not be moved aside`);
        return 1;
      }
      console.log(`nv relink: moved ${rel(exe)} to ${rel(aside)}`);
    }
  }
  const gone = sweep(exe);
  console.log(`nv relink: deleted ${gone} moved-aside cop${gone === 1 ? "y" : "ies"} nobody runs any longer`);
  return 0;
}
