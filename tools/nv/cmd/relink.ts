// `bun nv relink`: frees `target/release/nvs` for the linker by hand, when a running copy of it holds the
// path, and builds it. `tools/nv/lib/relink.ts` owns why a held binary fails a Windows link and why moving
// it aside is the answer; the release builds the loop and `bun nv proofs` run already go through it.
//
//     bun nv relink                 delete every moved-aside copy nobody runs any longer
//     bun nv relink --free          move the release binary aside first, so the next build can link
//     bun nv relink --build         build the release binary, moving a held one aside
//     bun nv relink --build --hook  the same, as looper's `after_retire` hook
//
// **Every goal end rebuilds the release binary.** The user's editor and terminal run
// `target/release/nvs`, and nothing else keeps it current: a debug build never touches it, and the
// acceptance sweep builds it only for a check that measures it. So looper's `after_retire` hook runs
// `--build --hook` once the retirement commit has landed. In a side loop's lane (`LOOPER_LOOP_KIND` is
// `side`) the hook builds nothing, since the lane's `target/` is not the one anybody runs; its goal
// reaches the root's binary at the next goal end after it lands. What it spends: one release build per
// goal, minutes of the loop's time between two goals, and no memory a request holds.

import { existsSync } from "node:fs";
import { free, linked, RELEASE_ARGV, releaseCli, sweep } from "../lib/relink.ts";
import { rel } from "../lib/paths.ts";
import { run as runProc } from "../lib/proc.ts";
import { cargoLines } from "../lib/progress.ts";

export const summary = "move a running target/release/nvs aside so cargo can link, sweep old copies, and build it: nv relink [--free | --build [--hook]]";

const FLAGS = ["--free", "--build", "--hook"];

export async function run(args: string[], env: Record<string, string | undefined> = process.env): Promise<number> {
  if (args.some((a) => a === "-h" || a === "--help")) {
    console.log(summary);
    return 0;
  }
  const unknown = args.filter((a) => !FLAGS.includes(a));
  if (unknown.length > 0) {
    console.error(`nv relink: unknown argument ${unknown[0]}`);
    return 2;
  }
  if (args.includes("--hook") && !args.includes("--build")) {
    console.error("nv relink: --hook goes with --build");
    return 2;
  }
  const exe = releaseCli();
  if (args.includes("--build")) return await build(exe, args.includes("--hook") ? (env.LOOPER_LOOP_KIND ?? "") : null);
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

/** Builds `exe` through `linked`. `kind` is the loop's `LOOPER_LOOP_KIND` under `--hook`, and `null` by hand. */
async function build(exe: string, kind: string | null): Promise<number> {
  if (kind === "side") {
    console.log("nv relink: a side loop's lane builds no release binary; the root's is rebuilt at the next goal end after this goal lands");
    return 0;
  }
  const command = RELEASE_ARGV.join(" ");
  console.log(`nv relink: ${command}`);
  const r = await linked(exe, () => runProc(RELEASE_ARGV, { timeoutMs: 60 * 60 * 1000, onLine: cargoLines("nv relink") }), (r) => r.stderr);
  if (r.code !== 0 || !existsSync(exe)) {
    const tail = r.stderr.trimEnd().split("\n").slice(-15).join("\n");
    console.error(`nv relink: \`${command}\` failed (exit ${r.code}):\n${tail}`);
    return 1;
  }
  console.log(`nv relink: ${rel(exe)} is built from the tree as it stands`);
  return 0;
}
