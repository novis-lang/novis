// Every record type, in one list. The index builds a table for each, `nv check` checks each, and
// `data/schema/` publishes each one's JSON Schema. A type is declared in a file of its own beside this
// one, with `defineRecord` from `../lib/schema.ts`, and added here.
//
// A file under `data/` belongs to the first type in this list whose directory and `idOf` accept it.
// Two types share a directory only where their `idOf`s split it: a goal and its handoff, a topic and
// its rules, a playbook section and its bullets.

import type { RecordType } from "../lib/schema.ts";
import { chain } from "./chain.ts";
import { decision } from "./decision.ts";
import { gap } from "./gap.ts";
import { goal, sideGoal } from "./goal.ts";
import { handoff, sideHandoff } from "./handoff.ts";
import { milestone } from "./milestone.ts";
import { planStatus } from "./plan-status.ts";
import { playbookBullet, playbookSection } from "./playbook.ts";
import { impactProbes } from "./impact-probes.ts";
import { helpBacklog, proofPolicy } from "./proofs.ts";
import { referenceChapter } from "./reference.ts";
import { rule } from "./rule.ts";
import { specCoreMembers, specPhpMigration } from "./spec.ts";
import { topic } from "./topic.ts";

export const RECORDS: RecordType<any>[] = [
  goal,
  handoff,
  sideGoal,
  sideHandoff,
  chain,
  topic,
  rule,
  decision,
  planStatus,
  milestone,
  gap,
  playbookSection,
  playbookBullet,
  referenceChapter,
  specCoreMembers,
  specPhpMigration,
  proofPolicy,
  helpBacklog,
  impactProbes,
];
