// The chain: the order the loop walks the goals in, as one list of slugs and nothing else. A goal's
// position is its place in this list, printed as `N of M`, and never part of a file name.

import { defineRecord, s } from "../lib/schema.ts";

export const chain = defineRecord({
  name: "chain",
  dir: "chain",
  single: true,
  schema: s.array(s.ref("goal")),
  checks: [
    {
      name: "a goal is in the chain once",
      sql: `SELECT min(path) AS path, value || ' is listed ' || count(*) || ' times' AS detail
            FROM chain__items GROUP BY value HAVING count(*) > 1`,
    },
  ],
});
