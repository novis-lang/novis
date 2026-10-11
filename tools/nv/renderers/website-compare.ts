// The comparison page's data: `website/src/data/compare.json`, a copy of the record
// `bun nv bench-compare --record` writes to `benches/compare.json`. The site reads the copy through
// `website/src/lib/compare.ts`, so its build never reaches outside `website/`. The copy opens with the
// generated marker and `recorded`, which is `false` and alone before the first run is recorded, and the
// page then says no run is recorded. The record's shape is `buildRecord`'s in
// `tools/nv/cmd/bench-compare.ts`; this step checks only that it is one JSON object with its cases.

import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { MARKER, type Output, type Renderer } from "../lib/render.ts";

export const COMPARE_RECORD = "benches/compare.json";
export const WEBSITE_COMPARE = "website/src/data/compare.json";

/** The website's copy of `record`, the text of `benches/compare.json`, or of no record when it is `null`. */
export function renderWebsiteCompare(record: string | null): Output {
  const text = (data: object) => ({ path: WEBSITE_COMPARE, text: `${JSON.stringify({ comment: MARKER, ...data }, null, 2)}\n` });
  if (record === null) return text({ recorded: false });
  const parsed = JSON.parse(record) as unknown;
  if (parsed === null || typeof parsed !== "object" || Array.isArray(parsed) || typeof (parsed as { cases?: unknown }).cases !== "object") {
    throw new Error(`${COMPARE_RECORD} is not a bench-compare record: it has no cases`);
  }
  return text({ recorded: true, ...parsed });
}

export const websiteCompare: Renderer = {
  name: "website-compare",
  render: (root) => {
    const path = join(root, COMPARE_RECORD);
    return [renderWebsiteCompare(existsSync(path) ? readFileSync(path, "utf8") : null)];
  },
};
