// The proof policy's legacy homes. `tools/data/dossier-policy.toml`'s `[report]` and `[skip]` become
// `data/proofs/policy.json`, with `tools/dossier.py`'s `REPORT` filling a report key the file leaves
// out. `tools/data/help-backlog.toml`'s `features` becomes `data/proofs/help-backlog.json`.
//
// The file's per-kind sections, `[all]` and `[<kind>]`, have no record: what each kind owes is
// `tools/dossier.py`'s `POLICY` until the proof tool replaces it. One found in the file is reported,
// so an override is never dropped without a line saying so.

import { parse as parseToml } from "smol-toml";
import { helpBacklog, proofPolicy } from "../schema/proofs.ts";
import { exists, extraKeys, text, type Importer, type ImportResult } from "./lib.ts";

const POLICY = "tools/data/dossier-policy.toml";
const BACKLOG = "tools/data/help-backlog.toml";
const PROOFS = ["tests", "examples", "perf", "hostile", "about", "help"] as const;
const REPORT_DEFAULT = { outlierFactor: 5, ceiling: {} as Record<string, number> };

type Table = Record<string, unknown>;
const isTable = (v: unknown): v is Table => typeof v === "object" && v !== null && !Array.isArray(v);

export const proofs: Importer = {
  name: "proofs",
  read(root) {
    const out: ImportResult = { records: [], unread: [], files: 0 };

    const policy: Table = exists(root, POLICY) ? (out.files++, parseToml(text(root, POLICY))) : {};
    for (const key of Object.keys(policy).filter((k) => k !== "report" && k !== "skip")) {
      out.unread.push({ path: POLICY, reason: `[${key}] overrides what a kind owes, which no record holds` });
    }
    const report = isTable(policy.report) ? policy.report : {};
    out.unread.push(...extraKeys(POLICY, "[report]", report, ["outlier_factor", "ceiling"]));
    const skip: Record<string, Record<string, string>> = {};
    for (const [feature, entry] of Object.entries(isTable(policy.skip) ? policy.skip : {})) {
      if (!isTable(entry)) {
        out.unread.push({ path: POLICY, reason: `[skip."${feature}"] is not a table` });
        continue;
      }
      out.unread.push(...extraKeys(POLICY, `[skip."${feature}"]`, entry, PROOFS));
      skip[feature] = Object.fromEntries(Object.entries(entry).filter(([k]) => (PROOFS as readonly string[]).includes(k))) as Record<string, string>;
    }
    const value = {
      report: {
        outlierFactor: report.outlier_factor ?? REPORT_DEFAULT.outlierFactor,
        ceiling: isTable(report.ceiling) ? report.ceiling : REPORT_DEFAULT.ceiling,
      },
      skip,
    };
    out.records.push({ type: proofPolicy, id: proofPolicy.name, value, from: POLICY });

    const backlog: Table = exists(root, BACKLOG) ? (out.files++, parseToml(text(root, BACKLOG))) : {};
    out.unread.push(...extraKeys(BACKLOG, "the file", backlog, ["features"]));
    const features = Array.isArray(backlog.features) ? backlog.features : [];
    out.records.push({ type: helpBacklog, id: helpBacklog.name, value: { features }, from: BACKLOG });
    return out;
  },
};
