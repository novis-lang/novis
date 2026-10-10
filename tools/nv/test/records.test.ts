import { afterEach, describe, expect, test } from "bun:test";
import { Index } from "../lib/index.ts";
import { pathOf, write } from "../lib/store.ts";
import { gap } from "../schema/gap.ts";
import { RECORDS } from "../schema/index.ts";
import { milestone } from "../schema/milestone.ts";
import { rule } from "../schema/rule.ts";
import { topic } from "../schema/topic.ts";
import { decision } from "../schema/decision.ts";
import { scratch, type Scratch } from "./scratch.ts";

let tmp: Scratch;
afterEach(() => tmp?.cleanup());

/** One valid record of each type that has references, all pointing at each other. */
function seed(): Index {
  tmp = scratch();
  write(milestone, "m12", { title: "Measurement", order: 12, loopDays: "measurement-bound", state: "ongoing" }, tmp.root);
  write(decision, "0134", { title: "Features owe proofs", status: "accepted", scope: "What a feature owes.", dependsOn: [] }, tmp.root);
  write(topic, "testing", { title: "Testing", order: 90, rules: ["testing/feature-proofs"] }, tmp.root);
  write(rule, "testing/feature-proofs", { title: "A feature is finished when its proofs exist", status: "shipped", because: ["0134"], seeAlso: [], guardedBy: ["tools/nv/proofs/collect.ts"] }, tmp.root);
  write(gap, "nvs-cli/suite-is-serial", { module: "crates/nvs-cli/src/runner.rs", title: "The suite is serial.", text: "Open.", milestone: "m12" }, tmp.root);
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
      { type: "decision", id: "0134" },
      { type: "gap", id: "nvs-cli/suite-is-serial" },
      { type: "milestone", id: "m12" },
      { type: "rule", id: "testing/feature-proofs" },
      { type: "topic", id: "testing" },
    ]);
    expect(index.check()).toEqual([]);
    index.close();
  });

  test("each invariant no foreign key declares is a finding", () => {
    const index = seed();
    write(gap, "nvs-cli/unowned", { module: "crates/nvs-cli/src/runner.rs", title: "No owner.", text: "Open." }, tmp.root);
    write(topic, "testing", { title: "Testing", order: 90, rules: [] }, tmp.root);
    index.refresh();
    const messages = index.check().map((f) => `${f.path}: ${f.message}`);
    index.close();
    expect(messages.sort()).toEqual([
      `${pathOf(gap, "nvs-cli/unowned")}: a gap has exactly one owner: names no owner`,
      `${pathOf(rule, "testing/feature-proofs")}: a rule is in its topic's list: testing/feature-proofs is not in its topic's rules`,
    ]);
  });
});
