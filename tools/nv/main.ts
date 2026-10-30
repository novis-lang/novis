// `bun nv <command> [args]`: the one entry point of the repository's tools. Each command is a module
// under `cmd/` exporting a `summary` line and `run(args)`, which returns the exit status. Every command
// runs with the console held in the modes `lib/tty.ts` names, whatever the programs it starts do to it.
//
// A command's module is loaded only when it runs, so what `bun nv <command>` loads is this file, its
// three imports and that module's own imports. When `NV_READS_LOG` names a file, `lib/reads.ts` records
// what the command reads before it is loaded, and the modules Bun's registry holds when it exits.
// Every command's children inherit `LLVM_PROFILE_FILE`, set to `DISCARD_PROFILE` when it is unset, so an
// instrumented `nvs` that no run records writes its counters under `.agent-tmp/`.
//
// A command's own tests are under `test/`, and `test/surfaces.test.ts` runs the one a session reaches by
// name against this tree: `brief --where`.

import { DISCARD_PROFILE } from "./lib/paths.ts";
import { ENV as READS_ENV, install } from "./lib/reads.ts";
import { holdConsole } from "./lib/tty.ts";

interface Command {
  summary: string;
  run(args: string[]): Promise<number>;
}

const COMMANDS: Record<string, () => Promise<Command>> = {
  affected: () => import("./cmd/affected.ts"),
  app: () => import("./cmd/app.ts"),
  bench: () => import("./cmd/bench.ts"),
  benches: () => import("./cmd/benches.ts"),
  "bench-compare": () => import("./cmd/bench-compare.ts"),
  "bench-load": () => import("./cmd/bench-load.ts"),
  "bench-proxied": () => import("./cmd/bench-proxied.ts"),
  brief: () => import("./cmd/brief.ts"),
  check: () => import("./cmd/check.ts"),
  "checkout-acl": () => import("./cmd/checkout-acl.ts"),
  ci: () => import("./cmd/ci.ts"),
  "ci-changes": () => import("./cmd/ci-changes.ts"),
  "db-matrix": () => import("./cmd/db-matrix.ts"),
  debt: () => import("./cmd/debt.ts"),
  deps: () => import("./cmd/deps.ts"),

  directives: () => import("./cmd/directives.ts"),
  discord: () => import("./cmd/discord.ts"),
  disk: () => import("./cmd/disk.ts"),
  "exe-icons": () => import("./cmd/exe-icons.ts"),
  "ext-templates": () => import("./cmd/ext-templates.ts"),
  gaps: () => import("./cmd/gaps.ts"),
  garden: () => import("./cmd/garden.ts"),
  "gen-attribution": () => import("./cmd/gen-attribution.ts"),
  guard: () => import("./cmd/guard.ts"),
  holes: () => import("./cmd/holes.ts"),
  layout: () => import("./cmd/layout.ts"),
  "linux-test": () => import("./cmd/linux-test.ts"),
  links: () => import("./cmd/links.ts"),
  lints: () => import("./cmd/lints.ts"),
  machine: () => import("./cmd/machine.ts"),
  "nvs-fmt": () => import("./cmd/nvs-fmt.ts"),
  origin: () => import("./cmd/origin.ts"),
  owners: () => import("./cmd/owners.ts"),
  "pdf-custom-data": () => import("./cmd/pdf-custom-data.ts"),
  peek: () => import("./cmd/peek.ts"),
  proofs: () => import("./cmd/proofs.ts"),
  prose: () => import("./cmd/prose.ts"),
  reference: () => import("./cmd/reference.ts"),
  release: () => import("./cmd/release.ts"),
  relink: () => import("./cmd/relink.ts"),
  render: () => import("./cmd/render.ts"),
  rules: () => import("./cmd/rules.ts"),
  scaling: () => import("./cmd/scaling.ts"),
  select: () => import("./cmd/select.ts"),
  selftest: () => import("./cmd/selftest.ts"),
  side: () => import("./cmd/side.ts"),
  site: () => import("./cmd/site.ts"),
  services: () => import("./cmd/services.ts"),
  soundness: () => import("./cmd/soundness.ts"),
  splice: () => import("./cmd/splice.ts"),
  suite: () => import("./cmd/suite.ts"),
  sweep: () => import("./cmd/sweep.ts"),
  try: () => import("./cmd/try.ts"),
  valgrind: () => import("./cmd/valgrind.ts"),
  verify: () => import("./cmd/verify.ts"),
  viewer: () => import("./cmd/viewer.ts"),
  vsix: () => import("./cmd/vsix.ts"),
  "webcrypto-vectors": () => import("./cmd/webcrypto-vectors.ts"),
  "webp-lib": () => import("./cmd/webp-lib.ts"),

};

async function usage(): Promise<void> {
  console.log("usage: bun nv <command> [args]\n");
  const width = Math.max(...Object.keys(COMMANDS).map((n) => n.length));
  for (const [name, load] of Object.entries(COMMANDS)) console.log(`  ${name.padEnd(width)}  ${(await load()).summary}`);
}

const log = process.env[READS_ENV];
if (log) install(log);
process.env.LLVM_PROFILE_FILE ??= DISCARD_PROFILE;
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
