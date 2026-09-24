// `bun nv <command> [args]`: the one entry point of the repository's tools. Each command is a module
// under `cmd/` exporting a `summary` line and `run(args)`, which returns the exit status.

import * as check from "./cmd/check.ts";
import * as query from "./cmd/query.ts";
import * as render from "./cmd/render.ts";
import * as selftest from "./cmd/selftest.ts";

interface Command {
  summary: string;
  run(args: string[]): Promise<number>;
}

const COMMANDS: Record<string, Command> = { check, query, render, selftest };

function usage(): void {
  console.log("usage: bun nv <command> [args]\n");
  const width = Math.max(...Object.keys(COMMANDS).map((n) => n.length));
  for (const [name, cmd] of Object.entries(COMMANDS)) console.log(`  ${name.padEnd(width)}  ${cmd.summary}`);
}

const [name, ...args] = process.argv.slice(2);
if (name === undefined || name === "help" || name === "--help" || name === "-h") {
  usage();
  process.exit(name === undefined ? 2 : 0);
}
const cmd = COMMANDS[name];
if (!cmd) {
  console.error(`nv: no command ${name}\n`);
  usage();
  process.exit(2);
}
process.exit(await cmd.run(args));
