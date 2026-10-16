// `bun nv query "<sql>"`: runs read-only SQL over the index, brought up to date first. Prints one
// tab-separated row per line under a header row, or with `--json` one JSON array. A statement that
// would write is refused by SQLite itself.

import { Index } from "../lib/index.ts";
import { RECORDS } from "../schema/index.ts";

export const summary = 'run read-only SQL over the index: nv query [--json] "<sql>"';

export async function run(args: string[]): Promise<number> {
  const json = args.includes("--json");
  const rest = args.filter((a) => a !== "--json");
  if (rest.length !== 1 || !rest[0]) {
    console.error('nv query: wants one SQL statement, as nv query [--json] "<sql>"');
    return 2;
  }
  const index = new Index({ types: RECORDS });
  try {
    index.refresh();
    let rows: Record<string, unknown>[];
    try {
      rows = index.query(rest[0]);
    } catch (e) {
      console.error(`nv query: ${(e as Error).message}`);
      return 1;
    }
    if (json) {
      console.log(JSON.stringify(rows, null, 2));
      return 0;
    }
    const first = rows[0];
    if (!first) return 0;
    const cols = Object.keys(first);
    console.log(cols.join("\t"));
    for (const row of rows) console.log(cols.map((c) => String(row[c] ?? "")).join("\t"));
    return 0;
  } finally {
    index.close();
  }
}
