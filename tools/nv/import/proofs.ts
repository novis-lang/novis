// The proof policy's legacy homes. The policy file under `tools/data/` gives its `[report]` and
// `[skip]` to `data/proofs/policy.json`, with the Python proof tool's `REPORT` filling a report key the
// file leaves out, and its per-kind sections, `[all]` and `[<kind>]`, become `owes` under the same key.
// A section named for no kind, or a key no kind owes, is reported rather than carried.
// `tools/data/help-backlog.toml`'s `features` becomes `data/proofs/help-backlog.json`.

import { parse as parseToml } from "smol-toml";
import { helpBacklog, proofPolicy } from "../schema/proofs.ts";
import { exists, extraKeys, OLD, text, type Importer, type ImportResult } from "./lib.ts";

const POLICY = `tools/data/${OLD}-policy.toml`;
const BACKLOG = "tools/data/help-backlog.toml";
const PROOFS = ["tests", "examples", "perf", "hostile", "about", "help"] as const;
/** The keys a per-kind section may set, and the sections that may set them: `all` and each kind. */
const OWED = ["tests", "rust", "examples", "perf", "hostile", "about", "help", "comments"] as const;
const OWNERS = ["all", "member", "lang", "exception", "enum", "interface", "tool", "directive"];
const REPORT_DEFAULT = { outlierFactor: 5, ceiling: {} as Record<string, number> };

type Table = Record<string, unknown>;
const isTable = (v: unknown): v is Table => typeof v === "object" && v !== null && !Array.isArray(v);

export const proofs: Importer = {
  name: "proofs",
  read(root) {
    const out: ImportResult = { records: [], unread: [], files: 0 };

    const policy: Table = exists(root, POLICY) ? (out.files++, parseToml(text(root, POLICY))) : {};
    const owes: Record<string, Table> = {};
    for (const [key, entry] of Object.entries(policy).filter(([k]) => k !== "report" && k !== "skip")) {
      if (!OWNERS.includes(key) || !isTable(entry)) {
        out.unread.push({ path: POLICY, reason: `[${key}] is neither a kind nor \`all\`, or is not a table` });
        continue;
      }
      out.unread.push(...extraKeys(POLICY, `[${key}]`, entry, OWED));
      owes[key] = Object.fromEntries(Object.entries(entry).filter(([k]) => (OWED as readonly string[]).includes(k)));
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
      ...(Object.keys(owes).length ? { owes } : {}),
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
