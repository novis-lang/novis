// `bun nv <command> [args]`: the one entry point of the repository's tools. Each command is a module
// under `cmd/` exporting a `summary` line and `run(args)`, which returns the exit status. Every command
// runs with the console held in the modes `lib/tty.ts` names, whatever the programs it starts do to it.
//
// A command's module is loaded only when it runs, so what `bun nv <command>` loads is this file, its
// two imports and that module's own imports: `tools/nv/keys/modules.ts` keys a check on exactly those.
// When `NV_READS_LOG` names a file, `lib/reads.ts` records what the command reads before it is loaded.

import { ENV as READS_ENV, install } from "./lib/reads.ts";
import { holdConsole } from "./lib/tty.ts";

interface Command {
  summary: string;
  run(args: string[]): Promise<number>;
}

const COMMANDS: Record<string, () => Promise<Command>> = {
  audit: () => import("./cmd/audit.ts"),
  bench: () => import("./cmd/bench.ts"),
  "bench-load": () => import("./cmd/bench-load.ts"),
  "bench-proxied": () => import("./cmd/bench-proxied.ts"),
  bg: () => import("./cmd/bg.ts"),
  brief: () => import("./cmd/brief.ts"),
  chain: () => import("./cmd/chain.ts"),
  check: () => import("./cmd/check.ts"),
  "ci-changes": () => import("./cmd/ci-changes.ts"),
  "ci-green": () => import("./cmd/ci-green.ts"),
  "class-cards": () => import("./cmd/class-cards.ts"),
  "db-matrix": () => import("./cmd/db-matrix.ts"),
  decisions: () => import("./cmd/decisions.ts"),
  directives: () => import("./cmd/directives.ts"),
  disk: () => import("./cmd/disk.ts"),
  "exe-icons": () => import("./cmd/exe-icons.ts"),
  gaps: () => import("./cmd/gaps.ts"),
  "gen-attribution": () => import("./cmd/gen-attribution.ts"),
  goal: () => import("./cmd/goal.ts"),
  guard: () => import("./cmd/guard.ts"),
  holes: () => import("./cmd/holes.ts"),
  impact: () => import("./cmd/impact.ts"),
  import: () => import("./cmd/import.ts"),
  layout: () => import("./cmd/layout.ts"),
  links: () => import("./cmd/links.ts"),
  lints: () => import("./cmd/lints.ts"),
  loop: () => import("./cmd/loop.ts"),
  "loop-stats": () => import("./cmd/loop-stats.ts"),
  machine: () => import("./cmd/machine.ts"),
  migration: () => import("./cmd/migration.ts"),
  orient: () => import("./cmd/orient.ts"),
  origin: () => import("./cmd/origin.ts"),
  owners: () => import("./cmd/owners.ts"),
  peek: () => import("./cmd/peek.ts"),
  plan: () => import("./cmd/plan.ts"),
  playbook: () => import("./cmd/playbook.ts"),
  proofs: () => import("./cmd/proofs.ts"),
  query: () => import("./cmd/query.ts"),
  records: () => import("./cmd/records.ts"),
  reference: () => import("./cmd/reference.ts"),
  release: () => import("./cmd/release.ts"),
  relink: () => import("./cmd/relink.ts"),
  render: () => import("./cmd/render.ts"),
  rules: () => import("./cmd/rules.ts"),
  selftest: () => import("./cmd/selftest.ts"),
  session: () => import("./cmd/session.ts"),
  splice: () => import("./cmd/splice.ts"),
  try: () => import("./cmd/try.ts"),
  verify: () => import("./cmd/verify.ts"),
  "webcrypto-vectors": () => import("./cmd/webcrypto-vectors.ts"),
  why: () => import("./cmd/why.ts"),
};

async function usage(): Promise<void> {
  console.log("usage: bun nv <command> [args]\n");
  const width = Math.max(...Object.keys(COMMANDS).map((n) => n.length));
  for (const [name, load] of Object.entries(COMMANDS)) console.log(`  ${name.padEnd(width)}  ${(await load()).summary}`);
}

const log = process.env[READS_ENV];
if (log) install(log);
holdConsole();
const [name, ...args] = process.argv.slice(2);
if (name === undefined || name === "help" || name === "--help" || name === "-h") {
  await usage();
  process.exit(name === undefined ? 2 : 0);
}
const load = COMMANDS[name];
if (!load) {
  console.error(`nv: no command ${name}\n`);
  await usage();
  process.exit(2);
}
process.exit(await (await load()).run(args));
