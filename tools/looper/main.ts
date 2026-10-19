// The kit's entry: `bun main.ts [--json] <command> [args]`. It sets the exit code rather than calling
// `process.exit`, so stdout is flushed before the process ends.

import { run } from "./src/cli/cli.ts";
import { commands } from "./src/cli/commands.ts";

process.exitCode = await run(process.argv.slice(2), commands);
