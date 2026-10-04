// The chain: the order the loop walks the goals in, and the goal it stands on. `goals` is a list of
// slugs, and a goal's position is its place in that list, printed as `N of M` and never part of a file
// name. `live` is the goal the driver works on. The goal switch deletes the goal it leaves, moves `live`
// and commits it, so every clone, CI and a git hook read the same live goal.

import { defineRecord, s } from "../lib/schema.ts";

export const chainShape = s.object({
  live: s.ref("goal"),
  goals: s.array(s.ref("goal")),
});

export const chain = defineRecord({
  name: "chain",
  dir: "chain",
  single: true,
  schema: chainShape,
  checks: [
    {
      name: "a goal is in the chain once",
      sql: `SELECT min(path) AS path, value || ' is listed ' || count(*) || ' times' AS detail
            FROM chain__goals GROUP BY value HAVING count(*) > 1`,
    },
    {
      name: "the live goal is on the chain",
      sql: `SELECT path, live || ' is live and not in goals' AS detail FROM chain
            WHERE live NOT IN (SELECT value FROM chain__goals)`,
    },
  ],
});
