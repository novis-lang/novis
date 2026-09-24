// `bun nv records --stats`: one line per decision record, with its size, its section shape and the
// share of it in rationale sections, then the totals over the set.
//
// The set is the `decision` records under `data/decisions/`. The sizes are the prose's,
// `docs/decisions/NNNN.md`: its lines, its characters, and its `## ` headings below the front matter.
// A size counts characters rather than UTF-8 bytes, and a heading written twice counts once, as the
// later of the two.

import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { ROOT } from "../lib/paths.ts";
import { load } from "../lib/store.ts";
import { decision } from "../schema/decision.ts";

export const summary = "the decision records, read-only: nv records --stats";

/** The sections that argue for a decision rather than state it. */
const RATIONALE = new Set(["Context", "Investigation", "Options considered", "Alternatives rejected", "Revisiting"]);

interface Stat {
  num: string;
  lines: number;
  chars: number;
  rationale: number;
  headings: string[];
}

/** Characters as code points, the unit a size is counted in. */
function chars(s: string): number {
  let pairs = 0;
  for (let i = 0; i < s.length; i++) {
    const c = s.charCodeAt(i);
    if (c >= 0xd800 && c <= 0xdbff) pairs++;
  }
  return s.length - pairs;
}

function measure(num: string, text: string): Stat {
  const lines = text.split("\n");
  // The front matter is a `---` first line and the next `---`; unclosed, there is none.
  let bodyFrom = 0;
  if (lines[0] === "---") {
    const close = lines.indexOf("---", 1);
    if (close > 0) bodyFrom = close + 1;
  }
  const headings: [number, string][] = [];
  lines.forEach((line, i) => {
    if (i >= bodyFrom && line.startsWith("## ")) headings.push([i, line.slice(3).trim()]);
  });
  const sections = new Map<string, string>();
  headings.forEach(([at, name], k) => {
    const end = headings[k + 1]?.[0] ?? lines.length;
    sections.set(name, lines.slice(at + 1, end).join("\n"));
  });
  let rationale = 0;
  for (const [name, body] of sections) if (RATIONALE.has(name)) rationale += chars(body);
  return { num, lines: lines.length, chars: chars(text), rationale, headings: headings.map(([, n]) => n) };
}

function stats(): number {
  const found: Stat[] = [];
  const missing: string[] = [];
  for (const r of load(decision).sort((a, b) => (a.id < b.id ? -1 : a.id > b.id ? 1 : 0))) {
    const path = `docs/decisions/${r.id}.md`;
    if (!existsSync(join(ROOT, path))) {
      missing.push(path);
      continue;
    }
    found.push(measure(r.id, readFileSync(join(ROOT, path), "utf8").replace(/\r\n/g, "\n")));
  }
  console.log(`${"adr".padEnd(6)}${"lines".padStart(6)}${"bytes".padStart(8)}  ${"rationale".padStart(9)}  sections`);
  let lines = 0, total = 0, rationale = 0;
  for (const s of found) {
    lines += s.lines;
    total += s.chars;
    rationale += s.rationale;
    const pct = `${s.chars ? Math.floor((100 * s.rationale) / s.chars) : 0}%`;
    const shape = s.headings.map((h) => [...h].slice(0, 4).join("")).join(" ");
    console.log(`${s.num.padEnd(6)}${String(s.lines).padStart(6)}${String(s.chars).padStart(8)}  ` +
      `${String(s.rationale).padStart(6)} ${pct.padStart(2)}  ${shape}`);
  }
  console.log(`\n${found.length} records, ${lines} lines, ${total} bytes; ` +
    `${rationale} bytes (${total ? Math.floor((100 * rationale) / total) : 0}%) in rationale sections`);
  for (const path of missing) console.log(`nv records: !! ${path} does not exist, and its record does`);
  return missing.length > 0 ? 1 : 0;
}

export async function run(args: string[]): Promise<number> {
  if (args.length === 1 && args[0] === "--stats") return stats();
  console.error("usage: bun nv records --stats\n  the records' other modes are still `python tools/records.py`'s");
  return 2;
}
