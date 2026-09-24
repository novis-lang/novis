// The impact probes have no legacy home: `data/impact-probes.json` was written as a record, so the
// import reads it back as it stands. An import that did not would delete it on `--write`.

import { impactProbes } from "../schema/impact-probes.ts";
import { exists, text, type Importer, type ImportResult } from "./lib.ts";

const PROBES = "data/impact-probes.json";

export const impactProbesImporter: Importer = {
  name: "impact_probes",
  read(root) {
    const out: ImportResult = { records: [], unread: [], files: 0 };
    if (!exists(root, PROBES)) return out;
    out.files++;
    out.records.push({ type: impactProbes, id: impactProbes.name, value: JSON.parse(text(root, PROBES)), from: PROBES });
    return out;
  },
};
