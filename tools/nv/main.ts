// `bun nv <command> [args]`: the one entry point of the repository's tools. Each command is a module
// under `cmd/` exporting a `summary` line and `run(args)`, which returns the exit status.

import * as audit from "./cmd/audit.ts";
import * as bg from "./cmd/bg.ts";
import * as brief from "./cmd/brief.ts";
import * as chain from "./cmd/chain.ts";
import * as check from "./cmd/check.ts";
import * as decisions from "./cmd/decisions.ts";
import * as directives from "./cmd/directives.ts";
import * as disk from "./cmd/disk.ts";
import * as gaps from "./cmd/gaps.ts";
import * as goal from "./cmd/goal.ts";
import * as guard from "./cmd/guard.ts";
import * as holes from "./cmd/holes.ts";
import * as impact from "./cmd/impact.ts";
import * as importCmd from "./cmd/import.ts";
import * as layout from "./cmd/layout.ts";
import * as links from "./cmd/links.ts";
import * as loop from "./cmd/loop.ts";
import * as migration from "./cmd/migration.ts";
import * as owners from "./cmd/owners.ts";
import * as parity from "./cmd/parity.ts";
import * as peek from "./cmd/peek.ts";
import * as plan from "./cmd/plan.ts";
import * as playbook from "./cmd/playbook.ts";
import * as proofs from "./cmd/proofs.ts";
import * as query from "./cmd/query.ts";
import * as records from "./cmd/records.ts";
import * as reference from "./cmd/reference.ts";
import * as orient from "./cmd/orient.ts";
import * as render from "./cmd/render.ts";
import * as rules from "./cmd/rules.ts";
import * as selftest from "./cmd/selftest.ts";
import * as session from "./cmd/session.ts";
import * as splice from "./cmd/splice.ts";
import * as verify from "./cmd/verify.ts";
import * as why from "./cmd/why.ts";

interface Command {
  summary: string;
  run(args: string[]): Promise<number>;
}

const COMMANDS: Record<string, Command> = { audit, bg, brief, chain, check, decisions, directives, disk, gaps, goal, guard, holes, impact, import: importCmd, layout, links, loop, migration, orient, owners, parity, peek, plan, playbook, proofs, query, records, reference, render, rules, selftest, session, splice, verify, why };

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
