// The perf ledger, `docs/perf/members.ndjson`: one JSON record per line, each naming the feature it
// measures in `id`. `nv proofs` reads it without recording the file, and names a `perf:#<feature>` key
// for each feature whose rows it consults (`perfKey`). A reader that uses every row names
// `LEDGER_WHOLE`. A change to the ledger moves the keys of the features whose rows differ between its
// old and new text (`ledgerMoved`), so a new figure for one feature selects only that feature's proofs
// checks. It also moves the file's own `file:` key, which selects every other check that reads the
// file directly.

import { createHash } from "node:crypto";

export const LEDGER = "docs/perf/members.ndjson";

/** Every `perf:` key of one feature, as the prefix a selection looks up. */
export const PERF_ANY = "perf:#";

/** The key of a reader that uses every row of the ledger. Any change to the ledger moves it. */
export const LEDGER_WHOLE = "perf:ledger";

export const perfKey = (feature: string) => `${PERF_ANY}${feature}`;

/** Every record in `text`, grouped by feature, oldest first. A blank line, a `#` line and a line that
 * is not JSON are skipped. */
export function ledgerRows(text: string): Map<string, Record<string, unknown>[]> {
  const out = new Map<string, Record<string, unknown>[]>();
  for (const raw of text.split("\n")) {
    const line = raw.trim();
    if (!line || line.startsWith("#")) continue;
    let rec: Record<string, unknown>;
    try {
      rec = JSON.parse(line);
    } catch {
      continue;
    }
    const id = String(rec.id ?? "");
    if (!out.has(id)) out.set(id, []);
    out.get(id)!.push(rec);
  }
  return out;
}

/** A digest of each feature's rows in `text`, in order. A feature with no row has no entry, and so
 * does every feature when `text` is null (no ledger). */
export function ledgerSigns(text: string | null): Record<string, string> {
  const out: Record<string, string> = {};
  if (text === null) return out;
  for (const [id, recs] of ledgerRows(text)) out[id] = createHash("sha1").update(JSON.stringify(recs), "utf8").digest("hex").slice(0, 16);
  return out;
}

/** The `perf:` keys of the features whose rows differ between `before` and `after` (`ledgerSigns`),
 * sorted. */
export function ledgerMoved(before: Record<string, string>, after: Record<string, string>): string[] {
  const ids = new Set([...Object.keys(before), ...Object.keys(after)]);
  return [...ids].filter((id) => before[id] !== after[id]).sort().map(perfKey);
}
