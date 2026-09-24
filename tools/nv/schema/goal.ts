// A goal: one entry in the loop's chain, and its acceptance test as data. Its prose, what the goal is
// for and each stage's reasoning, is `docs/agent/goals/<slug>.md`, and this record is everything a tool
// reads about it. A side goal has the same shape and lives under `data/goals/side/`, off the chain.
//
// A check's `stage` is a stage number of its own goal, and its `id` is unique within the goal and never
// changes, so a memo, a ledger line or a handoff can name one check across edits to its neighbours.

import { defineRecord, plainIdOf, s } from "../lib/schema.ts";

/** What `orient` prints for a goal, each list a set of selectors of its own kind. */
const context = s.object({
  modules: s.optional(s.array(s.string())),
  rules: s.optional(s.array(s.string())),
  adrs: s.optional(s.array(s.string())),
  shapes: s.optional(s.array(s.string())),
  playbook: s.optional(s.array(s.string())),
  plan: s.optional(s.array(s.string())),
  milestones: s.optional(s.array(s.string())),
});

const stage = s.object({
  number: s.int(),
  title: s.string(),
  /** Added to the goal's own `context` while a handoff's next group names this stage. */
  context: s.optional(context),
});

const check = s.object({
  id: s.slug(),
  kind: s.enum("command", "cargo-named", "nvs-suite", "exact", "ordered", "contains", "min-bytes"),
  stage: s.int(),
  /** What the driver prints for the check; a check over one `file` may go without. */
  name: s.optional(s.string()),
  argv: s.optional(s.array(s.string())),
  args: s.optional(s.array(s.string())),
  cwd: s.optional(s.string()),
  tests: s.optional(s.array(s.string())),
  cases: s.optional(s.array(s.string())),
  file: s.optional(s.string()),
  stream: s.optional(s.enum("stdout", "stderr")),
  want: s.optional(s.array(s.string())),
  exit: s.optional(s.enum("nonzero")),
  stdoutContains: s.optional(s.array(s.string())),
  stderrContains: s.optional(s.array(s.string())),
  minBytes: s.optional(s.int()),
  minPassing: s.optional(s.int()),
  needs: s.optional(s.string()),
  setup: s.optional(s.boolean()),
  memoize: s.optional(s.boolean()),
  overlap: s.optional(s.boolean()),
});

/** What a goal's checks need from the machine beyond a build. */
const env = s.object({
  docker: s.optional(
    s.object({
      compose: s.string(),
      services: s.array(s.string()),
      memoizeOn: s.optional(s.array(s.string())),
    }),
  ),
  valgrind: s.optional(s.object({ skip: s.array(s.string()) })),
  wsl: s.optional(s.object({ targetDir: s.string() })),
});

export const goalShape = s.object({
  title: s.string(),
  /** Null for a goal that lands in no milestone. */
  milestone: s.nullable(s.ref("milestone")),
  /** `last` pins the goal behind every goal not pinned, whatever the chain's order. */
  position: s.optional(s.enum("last")),
  /** The fixtures its checks run, carried into the next goal's floor with them. */
  files: s.array(s.string()),
  context,
  stages: s.array(stage),
  checks: s.array(check),
  env,
});

/** The invariants a goal type holds that no foreign key declares, over the table `t`. */
function goalChecks(t: string): { name: string; sql: string }[] {
  return [
    {
      name: "a check names a stage of its own goal",
      sql: `SELECT c.path, 'check ' || c.id || ' names stage ' || c.stage || ', which the goal does not have' AS detail
            FROM ${t}__checks c
            WHERE NOT EXISTS (SELECT 1 FROM ${t}__stages st WHERE st.owner = c.owner AND st.number = c.stage)`,
    },
    {
      name: "a check id is used once per goal",
      sql: `SELECT min(path) AS path, 'check id ' || id || ' is used ' || count(*) || ' times' AS detail
            FROM ${t}__checks GROUP BY owner, id HAVING count(*) > 1`,
    },
    {
      name: "a stage number is used once per goal",
      sql: `SELECT min(path) AS path, 'stage ' || number || ' is declared ' || count(*) || ' times' AS detail
            FROM ${t}__stages GROUP BY owner, number HAVING count(*) > 1`,
    },
  ];
}

export const goal = defineRecord({
  name: "goal",
  dir: "goals",
  schema: goalShape,
  idOf: (sub) => (sub.includes("/") ? null : plainIdOf(sub)),
  checks: [
    ...goalChecks("goal"),
    {
      name: "every goal is in the chain",
      sql: `SELECT path, id || ' is not in data/chain.json' AS detail FROM goal
            WHERE id NOT IN (SELECT value FROM chain__items)`,
    },
  ],
});

export const sideGoal = defineRecord({
  name: "side_goal",
  dir: "goals/side",
  schema: goalShape,
  checks: goalChecks("side_goal"),
});
