// `bun nv selftest`: the tools' own gate. It type-checks them with `tsc --noEmit` and runs their tests
// with `bun test`, and prints one line for each half that passes. The two lines are what the loop's
// acceptance check and `tools/verify.py`'s `nv` step read.

import { join } from "node:path";
import { passthrough } from "../lib/proc.ts";
import { ROOT } from "../lib/paths.ts";

export const summary = "type-check the tools and run their tests";

export async function run(args: string[]): Promise<number> {
  if (args.length > 0) {
    console.error(`nv selftest: takes no arguments, found ${args.join(" ")}`);
    return 2;
  }
  const tsc = await passthrough(
    [process.execPath, join(ROOT, "node_modules", "typescript", "bin", "tsc"), "--noEmit", "-p", "tsconfig.json"],
    { timeoutMs: 5 * 60 * 1000 },
  );
  if (tsc !== 0) {
    console.log(`nv selftest: tsc failed with exit ${tsc}`);
    return 1;
  }
  console.log("nv selftest: the types check");
  const test = await passthrough([process.execPath, "test", "tools/nv/"], { timeoutMs: 5 * 60 * 1000 });
  if (test !== 0) {
    console.log(`nv selftest: bun test failed with exit ${test}`);
    return 1;
  }
  console.log("nv selftest: every test passes");
  return 0;
}
