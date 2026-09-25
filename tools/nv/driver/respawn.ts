// The run above the turns: `bun nv loop` started by hand starts one turn after another, each a fresh
// `bun nv loop` process, and starts the next one every time a turn exits with `AGAIN`. Any other exit
// ends the run with that code. A turn that asks again within `QUICK` seconds `MAX_QUICK` times in a row
// ends it too, so a turn that no longer starts properly cannot spin.
//
// A fresh process per turn is what makes a session's change to the driver live at the next session. So
// this holds no loop logic: it names the run in every turn's environment (`NOVIS_LOOP_RUN`, which is how
// a turn tells its own run's `.loop/running` from another run's), and waits. The console belongs to the
// turn: stdin, stdout and stderr are passed through, and a Ctrl-C, which reaches both processes, is left
// to the turn, which sweeps the session and exits; this then ends with the turn's exit code.
//
// `tools/respawn.py` does the same for a run started with `python tools/loop.py`.

import { join } from "node:path";
import { ROOT } from "../lib/paths.ts";
import { AGAIN, RUN_ENV, runName } from "./launch.ts";

/** A turn is a session and a sweep, which is minutes; this many quick ones in a row end the run. */
const QUICK = 5;
const MAX_QUICK = 5;

export async function respawn(args: string[]): Promise<number> {
  const env = { ...process.env, [RUN_ENV]: runName() };
  // Ignored here, not handled: the turn owns the Ctrl-C, and this waits for it to finish.
  const ignore = () => {};
  process.on("SIGINT", ignore);
  try {
    let quick = 0;
    for (;;) {
      const started = performance.now();
      const turn = Bun.spawn([process.execPath, join(ROOT, "tools/nv/main.ts"), "loop", ...args], { cwd: ROOT, env, stdin: "inherit", stdout: "inherit", stderr: "inherit" });
      const code = await turn.exited;
      if (code !== AGAIN) return code;
      quick = performance.now() - started < QUICK * 1000 ? quick + 1 : 0;
      if (quick >= MAX_QUICK) {
        console.error(`nv loop: a turn asked to be started again ${quick} times in a row within ${QUICK}s each, so the run ends here. If .loop/running is still there, delete it before the next run.`);
        return 1;
      }
    }
  } finally {
    process.off("SIGINT", ignore);
  }
}
