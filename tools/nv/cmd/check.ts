// `bun nv check`: every schema, foreign-key and invariant finding over the records, one line each, led
// by the path of the file it is about. Exits 1 when there is any.

import { Index } from "../lib/index.ts";
import { RECORDS } from "../schema/index.ts";

export const summary = "check every record against its schema, its foreign keys and its invariants";

export async function run(args: string[]): Promise<number> {
  if (args.length > 0) {
    console.error(`nv check: takes no arguments, found ${args.join(" ")}`);
    return 2;
  }
  const index = new Index({ types: RECORDS });
  try {
    index.refresh();
    const findings = index.check();
    for (const f of findings) console.log(`${f.path}: ${f.message}`);
    const { records } = index.query(
      RECORDS.length > 0 ? "SELECT count(*) AS records FROM records" : "SELECT 0 AS records",
    )[0] as { records: number };
    const { prose } = index.query("SELECT count(*) AS prose FROM files WHERE kind = 'prose'")[0] as { prose: number };
    const n = findings.length;
    console.log(
      `nv check: ${n} finding${n === 1 ? "" : "s"}, in ${records} record(s) of ${RECORDS.length} type(s) and ${prose} prose file(s)`,
    );
    return findings.length > 0 ? 1 : 0;
  } finally {
    index.close();
  }
}
