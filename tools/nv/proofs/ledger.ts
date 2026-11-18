// The perf ledger, `data/perf/members.ndjson`: one JSON record per line, each naming the feature it
// measures in `id`. `nv proofs` reads it without recording the file, and names a `perf:#<feature>` key
// for each feature whose rows it consults (`perfKey`). A reader that uses every row names
// `LEDGER_WHOLE`. A change to the ledger moves the keys of the features whose rows differ between its
// old and new text (`ledgerMoved`), so a new figure for one feature selects only that feature's proofs
// checks. It also moves the file's own `file:` key, which selects every other check that reads the
// file directly. Every `--record-perf` writes the ledger back through `compactLedger`, so it holds only
// the rows a reader uses and does not grow with the number of runs.

import { createHash } from "node:crypto";

export const LEDGER = "data/perf/members.ndjson";

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

/** How many of a feature's newest rows `compactLedger` keeps per machine, and per feature among the rows
 * with counts. The report's `Δ` reads the last two, and the gate reads the last one. */
export const LEDGER_KEEP = 2;

/** `text` with only the rows a reader uses: the last `LEDGER_KEEP` of each (machine, feature), and the
 * last `LEDGER_KEEP` of each feature that carry counts, which the report's count `Δ` compares across
 * machines. Rows keep their order. A `#` line and a line that is not JSON are kept as they are, and a
 * blank line is dropped. An older figure is in git's history of the file. A row kept only for its
 * `impl_hash`, to spare a re-measure after a revert, is not kept: one per hash a feature ever had is
 * nearly the whole ledger, and a revert costs one `--record-perf`. */
export function compactLedger(text: string): { text: string; kept: number; dropped: number } {
  type Line = { raw: string; rec: Record<string, unknown> | null };
  const lines: Line[] = [];
  for (const raw of text.split("\n")) {
    const line = raw.trim();
    if (!line) continue;
    let rec: Record<string, unknown> | null = null;
    if (!line.startsWith("#")) {
      try {
        const parsed = JSON.parse(line);
        if (parsed && typeof parsed === "object" && !Array.isArray(parsed)) rec = parsed;
      } catch {
        rec = null;
      }
    }
    lines.push({ raw: line, rec });
  }
  const keep = new Set<number>();
  const newest = (key: (r: Record<string, unknown>) => string | null) => {
    const seen = new Map<string, number>();
    for (let i = lines.length - 1; i >= 0; i--) {
      const rec = lines[i]?.rec;
      const k = rec && key(rec);
      if (k === null || k === undefined) continue;
      const n = seen.get(k) ?? 0;
      if (n >= LEDGER_KEEP) continue;
      seen.set(k, n + 1);
      keep.add(i);
    }
  };
  newest((r) => `${String(r.machine ?? "")}\0${String(r.id ?? "")}`);
  newest((r) => ("allocations" in r ? String(r.id ?? "") : null));
  const out = lines.filter((l, i) => l.rec === null || keep.has(i));
  const records = lines.filter((l) => l.rec !== null).length;
  return { text: out.length ? out.map((l) => l.raw).join("\n") + "\n" : "", kept: keep.size, dropped: records - keep.size };
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
