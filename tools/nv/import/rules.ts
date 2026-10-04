// The rulebook has no legacy home left: its home is its records, `data/rules/<topic>.json` for a topic
// and `data/rules/<topic>/<slug>.json` for each rule. This importer carries them through as they stand,
// so `nv import --write` keeps them rather than deleting every record no importer built. A record that
// does not load, and a rule with no fragment `docs/rules/<id>.md`, are reported.

import type { RecordType } from "../lib/schema.ts";
import { load } from "../lib/store.ts";
import { rule } from "../schema/rule.ts";
import { topic } from "../schema/topic.ts";
import { exists, type Importer, type ImportResult } from "./lib.ts";

export const rules: Importer = {
  name: "rules",
  read(root) {
    const out: ImportResult = { records: [], unread: [], files: 0 };
    for (const type of [topic, rule] as RecordType<unknown>[]) {
      for (const r of load(type, root)) {
        out.files++;
        if (r.value === undefined) {
          out.unread.push({ path: r.path, reason: r.issues.map((i) => i.message).join("; ") });
          continue;
        }
        out.records.push({ type, id: r.id, value: r.value, from: r.path });
        if (type === rule && !exists(root, `docs/rules/${r.id}.md`)) {
          out.unread.push({ path: r.path, reason: `rule ${r.id} has no fragment docs/rules/${r.id}.md` });
        }
      }
    }
    return out;
  },
};
