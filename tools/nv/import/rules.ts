// The rulebook's legacy homes: `docs/rules/_index.json`, which lists the topics, and one
// `docs/rules/<topic>.json` per topic, which repeats its topic's title and order and holds its rules.
// A topic becomes `data/rules/<topic>.json` and each rule `data/rules/<topic>/<slug>.json`; the
// topic's copy of the index entry must agree with the index, and a disagreement is reported.

import { rule } from "../schema/rule.ts";
import { topic } from "../schema/topic.ts";
import { exists, extraKeys, text, type Importer, type ImportResult } from "./lib.ts";

const INDEX = "docs/rules/_index.json";
const RULE_KEYS = ["id", "title", "status", "because", "divergesFromPhp", "seeAlso", "guardedBy"] as const;

interface LegacyRule {
  id: string;
  title: string;
  status: string;
  because?: string[];
  divergesFromPhp?: string;
  seeAlso?: string[];
  guardedBy?: string[];
}

interface LegacyTopic {
  topic: string;
  title: string;
  order: number;
  rules?: LegacyRule[];
}

export const rules: Importer = {
  name: "rules",
  read(root) {
    const out: ImportResult = { records: [], unread: [], files: 1 };
    const index = JSON.parse(text(root, INDEX)) as { topics: LegacyTopic[] };
    out.unread.push(...extraKeys(INDEX, "the index", index, ["topics"]));
    for (const entry of index.topics) {
      const path = `docs/rules/${entry.topic}.json`;
      if (!exists(root, path)) {
        out.unread.push({ path: INDEX, reason: `lists topic ${entry.topic}, and ${path} does not exist` });
        continue;
      }
      out.files++;
      const t = JSON.parse(text(root, path)) as LegacyTopic;
      out.unread.push(...extraKeys(path, "the topic", t, ["topic", "title", "order", "rules"]));
      for (const k of ["topic", "title", "order"] as const) {
        if (t[k] !== entry[k]) out.unread.push({ path, reason: `its ${k} ${JSON.stringify(t[k])} is not the index's ${JSON.stringify(entry[k])}` });
      }
      const legacy = t.rules ?? [];
      out.records.push({
        type: topic,
        id: entry.topic,
        value: { title: entry.title, order: entry.order, rules: legacy.map((r) => r.id) },
        from: path,
      });
      for (const r of legacy) {
        out.unread.push(...extraKeys(path, `rule ${r.id}`, r, RULE_KEYS));
        const value: Record<string, unknown> = {
          title: r.title,
          status: r.status,
          because: r.because ?? [],
          seeAlso: r.seeAlso ?? [],
          guardedBy: r.guardedBy ?? [],
        };
        if (r.divergesFromPhp !== undefined) value.divergesFromPhp = r.divergesFromPhp;
        out.records.push({ type: rule, id: r.id, value, from: path });
        if (!exists(root, `docs/rules/${r.id}.md`)) out.unread.push({ path, reason: `rule ${r.id} has no fragment docs/rules/${r.id}.md` });
      }
    }
    return out;
  },
};
