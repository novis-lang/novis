// `sh <invocation>`: Claude Code's shell prefix. It runs `bash -c <invocation>`, and records a Bash call
// in a project as a job (`../jobs/sh.ts`). Its output is bash's and its exit code is bash's. It takes no
// `--json`.

import { runSh } from "../jobs/sh.ts";
import type { Command } from "./cli.ts";

export const shCommand: Command = {
  name: "sh",
  about: "Claude Code's shell prefix: runs `bash -c <invocation>`, and records a Bash call in a project as a job.",
  json: false,
  run: async (args) => ({ code: await runSh(args), own: true }),
};
