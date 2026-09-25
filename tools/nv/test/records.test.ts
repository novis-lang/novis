import { afterEach, describe, expect, test } from "bun:test";
import { Index } from "../lib/index.ts";
import { pathOf, write } from "../lib/store.ts";
import { chain } from "../schema/chain.ts";
import { gap } from "../schema/gap.ts";
import { goal } from "../schema/goal.ts";
import { handoff } from "../schema/handoff.ts";
import { RECORDS } from "../schema/index.ts";
import { milestone } from "../schema/milestone.ts";
import { playbookBullet, playbookSection } from "../schema/playbook.ts";
import { rule } from "../schema/rule.ts";
import { topic } from "../schema/topic.ts";
import { decision } from "../schema/decision.ts";
import { scratch, type Scratch } from "./scratch.ts";

let tmp: Scratch;
afterEach(() => tmp?.cleanup());

const TOOLS = {
  title: "the tools are one program",
  milestone: "m12",
  files: [],
  context: { modules: ["tools/nv/main.ts"] },
  stages: [
    { number: 1, title: "floor", summary: "Every check of every walked goal still passes." },
    { number: 4, title: "the records", summary: "Reads every legacy home into records.", context: { rules: ["testing/feature-proofs"] } },
  ],
  checks: [
    { id: "nv-check", kind: "command" as const, stage: 4, name: "every record is valid", argv: ["bun", "nv", "check"], want: ["nv check: 0 findings"] },
  ],
  env: { valgrind: { skip: [] } },
};

/** One valid record of each type that has references, all pointing at each other. */
function seed(): Index {
  tmp = scratch();
  write(milestone, "m12", { title: "Measurement", order: 12, loopDays: "measurement-bound", state: "ongoing" }, tmp.root);
  write(goal, "tooling-overhaul", TOOLS, tmp.root);
  write(chain, "chain", ["tooling-overhaul"], tmp.root);
  write(handoff, "tooling-overhaul", { state: "Stage 3 landed.", next: { stage: 4, title: "the importer", files: ["tools/nv/import/**"], items: [{ done: false, text: "Record types" }] }, backlog: [] }, tmp.root);
  write(decision, "0134", { title: "Features owe proofs", status: "accepted", scope: "What a feature owes.", dependsOn: [] }, tmp.root);
  write(topic, "testing", { title: "Testing", order: 90, rules: ["testing/feature-proofs"] }, tmp.root);
  write(rule, "testing/feature-proofs", { title: "A feature is finished when its proofs exist", status: "shipped", because: ["0134"], seeAlso: [], guardedBy: ["tools/nv/proofs/collect.ts"] }, tmp.root);
  write(gap, "nvs-cli/suite-is-serial", { module: "crates/nvs-cli/src/runner.rs", title: "The suite is serial.", text: "Open.", milestone: "m12" }, tmp.root);
  write(playbookSection, "tooling", { title: "Tooling", order: 1 }, tmp.root);
  write(playbookBullet, "tooling/one-call-reads", { lead: "One call reads many places.", body: "Use `tools/peek.py`.", files: ["tools/peek.py"], until: { kind: "gone", arg: "tools/peek.py" } }, tmp.root);
  return new Index({ root: tmp.root, types: RECORDS, prose: [] });
}

describe("record types", () => {
  test("every type publishes a JSON Schema and has a unique name", () => {
    const names = RECORDS.map((t) => t.name);
    expect(new Set(names).size).toBe(names.length);
    for (const t of RECORDS) expect(t.schema.jsonSchema()).toBeObject();
  });

  test("records sharing a directory each land in their own type", () => {
    const index = seed();
    index.refresh();
    expect(index.query("SELECT type, id FROM records ORDER BY type, id")).toEqual([
      { type: "chain", id: "chain" },
      { type: "decision", id: "0134" },
      { type: "gap", id: "nvs-cli/suite-is-serial" },
      { type: "goal", id: "tooling-overhaul" },
      { type: "handoff", id: "tooling-overhaul" },
      { type: "milestone", id: "m12" },
      { type: "playbook_bullet", id: "tooling/one-call-reads" },
      { type: "playbook_section", id: "tooling" },
      { type: "rule", id: "testing/feature-proofs" },
      { type: "topic", id: "testing" },
    ]);
    expect(index.check()).toEqual([]);
    index.close();
  });

  test("each invariant no foreign key declares is a finding", () => {
    const index = seed();
    write(goal, "stray", { ...TOOLS, stages: [TOOLS.stages[0]!, { number: 4, title: "the records" }], checks: [...TOOLS.checks, { ...TOOLS.checks[0]!, stage: 7 }] }, tmp.root);
    write(gap, "nvs-cli/unowned", { module: "crates/nvs-cli/src/runner.rs", title: "No owner.", text: "Open." }, tmp.root);
    write(topic, "testing", { title: "Testing", order: 90, rules: [] }, tmp.root);
    write(playbookBullet, "gone/a-bullet", { lead: "A.", body: "B.", files: [], until: { kind: "exists", arg: "tools/x.py" } }, tmp.root);
    index.refresh();
    const messages = index.check().map((f) => `${f.path}: ${f.message}`);
    const stray = pathOf(goal, "stray");
    index.close();
    expect(messages.sort()).toEqual([
      `${pathOf(gap, "nvs-cli/unowned")}: a gap has exactly one owner: names no owner`,
      `${stray}: a check id is used once per goal: check id nv-check is used 2 times`,
      `${stray}: a check names a stage of its own goal: check nv-check names stage 7, which the goal does not have`,
      `${stray}: a stage says what it does: stage 4 (the records) has no summary`,
      `${stray}: every goal is in the chain: stray is not in data/chain.json`,
      `${pathOf(playbookBullet, "gone/a-bullet")}: a bullet is in a section: no section gone`,
      `${pathOf(playbookBullet, "gone/a-bullet")}: a bullet names a file: names no file`,
      `${pathOf(rule, "testing/feature-proofs")}: a rule is in its topic's list: testing/feature-proofs is not in its topic's rules`,
    ]);
  });
});
