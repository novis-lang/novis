// `bun nv origin`: the local origin `examples/http.nvs` talks to, served in the foreground until the
// process is stopped. `tools/nv/driver/origin.ts` owns what it answers and why each answer is held; the
// loop's sweeps hold the same listener up around their checks without this command.
//
//     bun nv origin     serve http://127.0.0.1:8099/ok, then run the example from another shell

import { holdOrigin } from "../driver/origin.ts";

export const summary = "serve the local origin examples/http.nvs talks to, until stopped: nv origin";

export async function run(args: string[]): Promise<number> {
  if (args.some((a) => a === "-h" || a === "--help")) {
    console.log(summary);
    return 0;
  }
  const origin = await holdOrigin();
  console.log(origin.line);
  if (!origin.line.startsWith("origin: serving")) return origin.line.includes("cannot bind") ? 1 : 0;
  return new Promise<number>(() => {});
}
