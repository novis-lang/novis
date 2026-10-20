// The table of the kit's commands, which `main.ts` hands to the frame in `cli.ts`. Each kit area adds
// its commands here.

import pkg from "../../package.json";
import type { Command } from "./cli.ts";
import { checkCommand } from "./check.ts";
import { claimsCommand } from "./claims.ts";
import { cleanCommand } from "./clean.ts";
import { decisionCommand } from "./decision.ts";
import { doctorCommand } from "./doctor.ts";
import { eventCommand } from "./event.ts";
import { execCommand } from "./exec.ts";
import { hookCommand, layoutCommand, linksCommand, proseCommand } from "./gates.ts";
import { goal, harvestedCommand } from "./goal.ts";
import { guardCommand } from "./guard.ts";
import { importCommand } from "./import.ts";
import { initCommand } from "./init.ts";
import { launchCommand } from "./launch.ts";
import { lint, renderCommand } from "./lint.ts";
import { logCommand, loopCommand, statusCommand } from "./loop.ts";
import { orient } from "./orient.ts";
import { playbook } from "./playbook.ts";
import { projectCommand } from "./project.ts";
import { shCommand } from "./sh.ts";
import { syncCommand } from "./sync.ts";
import { affectedCommand, verifyCommand } from "./verify.ts";
import { wrapCommand } from "./wrap.ts";

const version: Command = {
  name: "version",
  about: "Prints the kit's version.",
  json: true,
  run: () => ({ doc: { version: pkg.version }, text: pkg.version }),
};

export const commands: readonly Command[] = [
  version,
  lint,
  renderCommand,
  initCommand,
  projectCommand,
  syncCommand,
  importCommand,
  goal,
  playbook,
  harvestedCommand,
  orient,
  linksCommand,
  layoutCommand,
  proseCommand,
  hookCommand,
  claimsCommand,
  guardCommand,
  doctorCommand,
  cleanCommand,
  execCommand,
  eventCommand,
  shCommand,
  launchCommand,
  verifyCommand,
  affectedCommand,
  checkCommand,
  loopCommand,
  decisionCommand,
  statusCommand,
  logCommand,
  wrapCommand,
];
