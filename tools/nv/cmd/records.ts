// `bun nv records`: the audit of the decision records, and the questions a cleanup pass asks of the set.
//
//     bun nv records                 the full audit, grouped by check
//     bun nv records --check         the same, quiet on success, exit 1 on a finding
//     bun nv records --only links    one named check, by the name the audit prints; repeatable
//     bun nv records --residue       the changelog residue check alone, exit 1 on a finding
//     bun nv records --stats         one line per record: size, section shape, rationale share
//     bun nv records --orphans       the records no other record links to, and the most cited
//     bun nv records --graph 0066    the rules one record creates and amends, and who else shaped them
//
// The set is the `decision` records under `data/decisions/`. Each one's reasoning is prose,
// `docs/decisions/NNNN.md`, whose front matter and bold field lines are rendered from the record, so
// the body starts after the front matter and no check reads a field out of it. Which rules a decision
// created or amended is the rules' `because` lists, so `changes` checks those lists from the rules'
// side. The full audit exits 0 whatever it finds; `--check` is the gate.
//
// A size counts characters rather than UTF-8 bytes, and a heading written twice counts once, as the
// later of the two. A finding names its file repo-relative.

import { existsSync, readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { ROOT } from "../lib/paths.ts";
import { load } from "../lib/store.ts";
import { decision } from "../schema/decision.ts";
import { rule } from "../schema/rule.ts";

export const summary = "the decision records, read-only: nv records [--check] [--only <check>] | --residue | --stats | --orphans | --graph NNNN";

/** The bold field lines a record's prose may carry between its title and its `In short` block. */
const FIELDS = ["Scope", "Depends on", "Validated by"];

/** The closed heading set, in the order every record uses it. */
const CANONICAL = [
  "Context",
  "Investigation",
  "Options considered",
  "Decision",
  "Diagnostics",
  "Consequences",
  "Alternatives rejected",
  "Revisiting",
  "Verification",
];

/** The sections that argue for a decision rather than state it. */
const RATIONALE = new Set(["Context", "Investigation", "Options considered", "Alternatives rejected", "Revisiting"]);

/** Prose that describes an earlier version of a frozen body, and what kind of residue each is. */
const RESIDUE: [RegExp, string][] = [
  [/\bthis body (?:now )?states the current rule\b/i, "fold boilerplate"],
  [/\bpreviously (?:said|read|stated|carried)\b/i, "describes a prior version"],
  [/\b(?:is|are) (?:now )?withdrawn\b/i, "tombstone"],
  [/\bthis section (?:originally|once|used to)\b/i, "tombstone"],
  [/\bWithdrawn\b/i, "tombstone heading"],
  [/\bthat sentence (?:is|was) (?:now )?(?:corrected|replaced)\b/i, "describes an edit"],
  [/\bformerly listed here\b/i, "tombstone"],
  [/\bin (?:an|its) earlier draft\b/i, "describes a prior version"],
  [/\bthis ADR (?:first|originally) (?:said|admitted|specified)\b/i, "describes a prior version"],
];
/** These sections name rejected and withdrawn things for a living, so residue is not looked for there. */
const RESIDUE_SKIP = ["Alternatives rejected", "Verification"];

/** Running totals spelled out in prose, which go stale the day the total moves. */
const COUNTERS = [
  /\bthe (?:tenth|eleventh|twelfth|thirteenth|fourteenth) divergence\b/i,
  /\bnow holds (?:three|four|five|six|seven|eight|nine|ten) names\b/i,
  /\bexactly those (?:three|four|five|six|seven) names\b/i,
  /\bthe (?:third|fourth|fifth|sixth|seventh) forcing case\b/i,
];

/** The documents that link to records without being one. */
const INDEX_DOCS = ["docs/adr/README.md", "docs/adr/tooling-parity.md"];

/** A link to a record, from a record (`(0066.md)`) or from an index document (`(../decisions/0066.md)`). */
const RECORD_LINK = /\]\((?:\.\.\/decisions\/)?(\d{4})\.md(?:#[^)]*)?\)/g;
/** A relative link that leaves the directory, whose target must be on disk. */
const PARENT_LINK = /\]\((\.\.\/[^)#]+)(?:#[^)]*)?\)/g;
/** A link's target collapsed to `]`, so `[0007](0007.md) § 4` reads as `[0007] § 4`. */
const LINK_TARGET = /\]\((?:\.\.\/decisions\/)?\d{4}\.md(?:#[^)]*)?\)/g;
const SECTION_CITE = /§§?\s*(\d+[a-z]?)/g;
/**
 * A `§ N` cites another record only when that record is named right in front of it: `[0067] § 13`,
 * `[0104]'s § 3`. Prose in between means the section is the writing record's own. A digit run after
 * a letter is a diagnostic code, and a record number always leads with a zero, so `2026` is a year.
 */
const CROSS_CITE = /(?<![A-Za-z0-9])(0\d{3})\]?(?:'s)?[\s,]*$/;
/** How far back from a `§` the record it cites is looked for, in characters. */
const CROSS_CITE_WINDOW = 40;

/** One record's prose, parsed the way every check reads it. Line numbers are 1-based. */
export interface Prose {
  num: string;
  /** Repo-relative. */
  file: string;
  text: string;
  lines: string[];
  /** The line the front matter closes on, or 0 when there is none; the body is after it. */
  bodyAfter: number;
  /** Bold field names outside `FIELDS`, before the first heading. */
  unknown: [number, string][];
  inShort: boolean;
  headings: [number, string][];
  /** Each `## ` section: its heading's line, and the last line before the next heading. */
  sections: Map<string, [number, number]>;
  /** The numbers of its `### N.` and `#### N.` subsections. */
  subsections: Set<string>;
}

export function parse(num: string, file: string, text: string): Prose {
  const lines = text.split("\n");
  let bodyAfter = 0;
  if (lines[0] === "---") {
    const close = lines.indexOf("---", 1);
    if (close > 0) bodyAfter = close + 1;
  }
  const p: Prose = {
    num, file, text, lines, bodyAfter, unknown: [], inShort: false, headings: [], sections: new Map(), subsections: new Set(),
  };
  lines.forEach((line, i) => {
    const at = i + 1;
    if (at <= bodyAfter) return;
    const field = /^- \*\*([^:*]+):\*\*\s*(.*)$/.exec(line);
    if (field && p.headings.length === 0) {
      const name = field[1]!.trim();
      if (!FIELDS.includes(name)) p.unknown.push([at, name]);
      return;
    }
    if (line.startsWith("> **In short:**")) p.inShort = true;
    if (line.startsWith("## ")) p.headings.push([at, line.slice(3).trim()]);
    const sub = /^#{3,4} (\d+[a-z]?)\./.exec(line);
    if (sub) p.subsections.add(sub[1]!);
  });
  p.headings.forEach(([at, name], k) => {
    const next = p.headings[k + 1];
    p.sections.set(name, [at, next ? next[0] - 1 : lines.length]);
  });
  return p;
}

/** The lines of a section's body, after its heading. */
function bodyOf(p: Prose, name: string): string {
  const range = p.sections.get(name);
  return range ? p.lines.slice(range[0], range[1]).join("\n") : "";
}

/** Every record this one links to, by number. */
function refs(p: Prose): Set<string> {
  return new Set([...p.text.matchAll(RECORD_LINK)].map((m) => m[1]!));
}

/** `[line, target record, section]` for every `[0007] § 3`-shaped citation of another record. */
function sectionRefs(p: Prose): [number, string, string][] {
  const out: [number, string, string][] = [];
  p.lines.forEach((line, i) => {
    const plain = line.replace(LINK_TARGET, "]");
    for (const m of plain.matchAll(SECTION_CITE)) {
      const cite = CROSS_CITE.exec(plain.slice(Math.max(0, m.index! - CROSS_CITE_WINDOW), m.index!));
      if (cite && cite[1] !== p.num) out.push([i + 1, cite[1]!, m[1]!]);
    }
  });
  return out;
}

/** The first `n` characters of `s`, counted as code points. */
function head(s: string, n: number): string {
  return [...s].slice(0, n).join("");
}

function read(path: string): string {
  return readFileSync(join(ROOT, path), "utf8").replace(/\r\n/g, "\n");
}

const loadDecisions = () => load(decision);
type DecisionRecord = ReturnType<typeof loadDecisions>[number];

interface RecordSet {
  /** Every decision record, by number. */
  records: Map<string, DecisionRecord>;
  /** Each record's prose, for the records that have it. */
  prose: Map<string, Prose>;
  /** Prose files under `docs/decisions/` with no record. */
  strays: string[];
}

function loadSet(): RecordSet {
  const records = new Map<string, DecisionRecord>();
  const prose = new Map<string, Prose>();
  for (const r of loadDecisions().sort((a, b) => (a.id < b.id ? -1 : a.id > b.id ? 1 : 0))) {
    records.set(r.id, r);
    const file = `docs/decisions/${r.id}.md`;
    if (existsSync(join(ROOT, file))) prose.set(r.id, parse(r.id, file, read(file)));
  }
  const strays: string[] = [];
  const glob = new Bun.Glob("0*.md");
  for (const name of glob.scanSync({ cwd: join(ROOT, "docs", "decisions") })) {
    if (!records.has(name.slice(0, 4))) strays.push(`docs/decisions/${name}`);
  }
  return { records, prose, strays: strays.sort() };
}

type Finding = [string, number, string];

function checkMetadata(set: RecordSet): Finding[] {
  const out: Finding[] = [];
  for (const [num, r] of set.records) {
    for (const issue of r.issues) out.push([r.path, 1, issue.at ? `${issue.at}: ${issue.message}` : issue.message]);
    if (r.issues.length > 0) continue;
    if (!r.value.scope.trim()) out.push([r.path, 1, "`scope` is empty"]);
    for (const dep of r.value.dependsOn) {
      if (!set.records.has(dep)) out.push([r.path, 1, `\`dependsOn\` names ${dep}, which is not a record`]);
    }
    const p = set.prose.get(num);
    if (!p) {
      out.push([r.path, 1, `no prose -- docs/decisions/${num}.md does not exist`]);
      continue;
    }
    for (const [line, name] of p.unknown) {
      out.push([p.file, line, `unknown metadata field \`${name}\` -- the bullets are ${FIELDS.join(", ")}`]);
    }
    if (!p.inShort) out.push([p.file, 1, "no `> **In short:**` block"]);
  }
  for (const file of set.strays) out.push([file, 1, "no record -- data/decisions/ has no entry for this prose"]);
  return out;
}

function checkChanges(set: RecordSet): Finding[] {
  const out: Finding[] = [];
  for (const r of load(rule)) {
    if (r.issues.length > 0) continue;
    const seen = new Set<string>();
    for (const num of r.value.because) {
      if (!set.records.has(num)) out.push([r.path, 1, `${r.id}'s \`because\` names ${num}, which is not a record`]);
      else if (seen.has(num)) out.push([r.path, 1, `${r.id}'s \`because\` names ${num} twice`]);
      seen.add(num);
    }
  }
  return out;
}

function checkStructure(set: RecordSet): Finding[] {
  const out: Finding[] = [];
  for (const p of set.prose.values()) {
    const names = p.headings.map(([, n]) => n);
    if (!names.includes("Decision")) {
      out.push([p.file, 1, "no `## Decision`"]);
      continue;
    }
    for (const n of names) {
      if (!CANONICAL.includes(n)) out.push([p.file, p.headings.find(([, name]) => name === n)![0], `non-canonical heading \`## ${n}\``]);
    }
    const ordered = names.filter((n) => CANONICAL.includes(n));
    const rank = ordered.map((n) => CANONICAL.indexOf(n));
    if (rank.some((r, i) => i > 0 && r < rank[i - 1]!)) {
      out.push([p.file, 1, `sections out of canonical order: ${ordered.join(" -> ")}`]);
    }
    for (const [n, [start, end]] of p.sections) {
      if (!p.lines.slice(start, end).join("\n").trim()) out.push([p.file, start, `\`## ${n}\` is empty`]);
    }
  }
  return out;
}

function checkLinks(set: RecordSet): Finding[] {
  const out: Finding[] = [];
  const sources: [string, string[]][] = [...set.prose.values()].map((p) => [p.file, p.lines]);
  for (const doc of INDEX_DOCS) if (existsSync(join(ROOT, doc))) sources.push([doc, read(doc).split("\n")]);
  for (const [file, lines] of sources) {
    const base = dirname(join(ROOT, file));
    lines.forEach((line, i) => {
      for (const m of line.matchAll(RECORD_LINK)) {
        if (!set.records.has(m[1]!)) out.push([file, i + 1, `broken record link -> ${m[1]}`]);
      }
      for (const m of line.matchAll(PARENT_LINK)) {
        if (!existsSync(resolve(base, m[1]!))) out.push([file, i + 1, `broken relative link -> ${m[1]}`]);
      }
    });
  }
  return out;
}

/** Section numbers in reading order: `2` before `10`, `3` before `3a`. */
function bySection(a: string, b: string): number {
  return a.length - b.length || (a < b ? -1 : a > b ? 1 : 0);
}

function checkSectionRefs(set: RecordSet): Finding[] {
  const out: Finding[] = [];
  for (const p of set.prose.values()) {
    for (const [line, num, sec] of sectionRefs(p)) {
      const t = set.prose.get(num);
      if (!set.records.has(num)) out.push([p.file, line, `cites record ${num}, which does not exist`]);
      else if (t && t.subsections.size > 0 && !t.subsections.has(sec)) {
        out.push([p.file, line, `cites ${num} § ${sec}; that record has §§ ${[...t.subsections].sort(bySection).join(", ")}`]);
      }
    }
  }
  return out;
}

function checkResidue(set: RecordSet): Finding[] {
  const out: Finding[] = [];
  for (const p of set.prose.values()) {
    const bodyStart = p.headings[0]?.[0] ?? p.lines.length;
    const skip = RESIDUE_SKIP.flatMap((n) => (p.sections.has(n) ? [p.sections.get(n)!] : []));
    p.lines.forEach((line, i) => {
      const at = i + 1;
      if (at < bodyStart || skip.some(([s, e]) => s <= at && at <= e)) return;
      const hit = RESIDUE.find(([pat]) => pat.test(line));
      if (hit) out.push([p.file, at, `${hit[1]}: ${head(line.trim(), 100)}`]);
    });
  }
  return out;
}

function checkCounters(set: RecordSet): Finding[] {
  const out: Finding[] = [];
  for (const p of set.prose.values()) {
    p.lines.forEach((line, i) => {
      if (COUNTERS.some((pat) => pat.test(line))) {
        out.push([p.file, i + 1, `running count in prose -- keep the total in one home: ${head(line.trim(), 90)}`]);
      }
    });
  }
  return out;
}

const CHECKS: [string, (set: RecordSet) => Finding[]][] = [
  ["metadata", checkMetadata],
  ["changes", checkChanges],
  ["structure", checkStructure],
  ["links", checkLinks],
  ["section refs", checkSectionRefs],
  ["changelog residue", checkResidue],
  ["stale counters", checkCounters],
];

function byFinding(a: Finding, b: Finding): number {
  if (a[0] !== b[0]) return a[0] < b[0] ? -1 : 1;
  if (a[1] !== b[1]) return a[1] - b[1];
  return a[2] < b[2] ? -1 : a[2] > b[2] ? 1 : 0;
}

/** Runs the checks `only` names, or every one; prints each one's findings. Returns how many there were. */
function report(set: RecordSet, only: Set<string> | null, quiet: boolean): number {
  let total = 0;
  for (const [name, fn] of CHECKS) {
    if (only && !only.has(name)) continue;
    const found = fn(set);
    total += found.length;
    if (found.length === 0) {
      if (!quiet) console.log(`ok   ${name}`);
      continue;
    }
    console.log(`\n== ${name} (${found.length})`);
    for (const [f, line, msg] of found.sort(byFinding)) console.log(`  ${f}:${line}  ${msg}`);
  }
  return total;
}

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

function measure(p: Prose): Stat {
  let rationale = 0;
  for (const name of p.sections.keys()) if (RATIONALE.has(name)) rationale += chars(bodyOf(p, name));
  return { num: p.num, lines: p.lines.length, chars: chars(p.text), rationale, headings: p.headings.map(([, n]) => n) };
}

function stats(set: RecordSet): number {
  const found = [...set.prose.values()].map(measure);
  console.log(`${"adr".padEnd(6)}${"lines".padStart(6)}${"bytes".padStart(8)}  ${"rationale".padStart(9)}  sections`);
  let lines = 0, total = 0, rationale = 0;
  for (const s of found) {
    lines += s.lines;
    total += s.chars;
    rationale += s.rationale;
    const pct = `${s.chars ? Math.floor((100 * s.rationale) / s.chars) : 0}%`;
    const shape = s.headings.map((h) => head(h, 4)).join(" ");
    console.log(`${s.num.padEnd(6)}${String(s.lines).padStart(6)}${String(s.chars).padStart(8)}  ` +
      `${String(s.rationale).padStart(6)} ${pct.padStart(2)}  ${shape}`);
  }
  console.log(`\n${found.length} records, ${lines} lines, ${total} bytes; ` +
    `${rationale} bytes (${total ? Math.floor((100 * rationale) / total) : 0}%) in rationale sections`);
  const missing = [...set.records.keys()].filter((num) => !set.prose.has(num));
  for (const num of missing) console.log(`nv records: !! docs/decisions/${num}.md does not exist, and its record does`);
  return missing.length > 0 ? 1 : 0;
}

/** The records nothing links to, then the twelve most cited; a tie is listed by number. */
function orphans(set: RecordSet): number {
  const inbound = new Map<string, Set<string>>();
  for (const p of set.prose.values()) {
    for (const t of refs(p)) {
      if (t === p.num) continue;
      if (!inbound.has(t)) inbound.set(t, new Set());
      inbound.get(t)!.add(p.num);
    }
  }
  console.log("records no other record links to:");
  for (const [num, r] of set.records) {
    if (!inbound.has(num)) console.log(`  ${num}  ${head(`ADR ${num} — ${r.value.title}`, 88)}`);
  }
  console.log("\nmost-cited:");
  const ranked = [...inbound].sort(([a, x], [b, y]) => y.size - x.size || (a < b ? -1 : 1)).slice(0, 12);
  for (const [num, from] of ranked) console.log(`  ${num}  ${String(from.size).padStart(3)} inbound`);
  return 0;
}

/** A field as the prose's bullet writes it on one line: its continuation lines joined by a space. */
function oneLine(value: string): string {
  return value.split("\n").map((l) => l.trim()).join(" ").trim();
}

/**
 * One record's place in the set: its fields, then each rule it created or amended with who else shaped
 * it, then the records it links to and the ones that cite it. A rule's `because` is creator-first, so a
 * record created the rules whose list it opens and amended the rest it appears in; the rules are listed
 * by id, and the other records by number, which is the order the decisions were taken in.
 */
function graph(set: RecordSet, num: string): number {
  const r = set.records.get(num);
  const p = set.prose.get(num);
  if (!r || !p) {
    console.log(`no record ${num}`);
    return 1;
  }
  const v = r.value;
  const fields: [string, string | undefined][] = [
    ["scope", v.scope === "" ? undefined : v.scope],
    ["depends on", v.dependsOnText ?? (v.dependsOn.length > 0 ? v.dependsOn.map((n) => `[${n}](${n}.md)`).join(", ") : undefined)],
    ["validated by", v.validatedBy],
  ];
  console.log(`${num}.md\n  ADR ${num} — ${v.title}\n`);
  console.log(`  ${"status:".padEnd(15)}${v.status}`);
  for (const [name, value] of fields) {
    if (value === undefined) continue;
    const line = oneLine(value);
    console.log(`  ${`${name}:`.padEnd(15)}${head(line, 110)}${[...line].length > 110 ? "..." : ""}`);
  }
  const touched: Record<"creates" | "modifies", [string, string[]][]> = { creates: [], modifies: [] };
  for (const rr of load(rule).sort((a, b) => (a.id < b.id ? -1 : a.id > b.id ? 1 : 0))) {
    const because = rr.value.because;
    if (!because.includes(num)) continue;
    touched[because[0] === num ? "creates" : "modifies"].push([rr.id, because]);
  }
  for (const key of ["creates", "modifies"] as const) {
    console.log(`\n  ${key} (${touched[key].length}):`);
    for (const [id, because] of touched[key]) {
      const others = because.filter((n) => n !== num).sort();
      const rest = others.filter((n) => n !== because[0]);
      const note = key === "creates"
        ? others.length > 0 ? `also shaped by ${others.join(", ")}` : "no other record"
        : `created by ${because[0]}` + (rest.length > 0 ? `; also ${rest.join(", ")}` : "");
      console.log(`    ${id.padEnd(58)} ${note}`);
    }
  }
  const links = [...refs(p)].filter((n) => n !== num).sort();
  const citedBy = [...set.prose.values()].filter((o) => o.num !== num && refs(o).has(num)).map((o) => o.num).sort();
  const sections = [...p.subsections].sort((a, b) => a.length - b.length || (a < b ? -1 : a > b ? 1 : 0));
  console.log(`\n  links to:      ${links.join(", ") || "-"}`);
  console.log(`  cited by:      ${citedBy.join(", ") || "-"}`);
  console.log(`  sections:      ${sections.join(", ") || "-"}`);
  return 0;
}

const USAGE = "usage: bun nv records [--check] [--only <check>]... | --residue | --stats | --orphans | --graph NNNN";

export async function run(args: string[]): Promise<number> {
  let check = false;
  const only = new Set<string>();
  let mode: string | null = null;
  let graphOf: string | null = null;
  for (let i = 0; i < args.length; i++) {
    const a = args[i]!;
    if (a === "--check") check = true;
    else if (a === "--graph" && i + 1 < args.length) graphOf = args[++i]!;
    else if (a === "--only" && i + 1 < args.length) only.add(args[++i]!);
    else if (a === "--stats" || a === "--orphans" || a === "--residue") mode = a;
    else {
      console.error(USAGE);
      return 2;
    }
  }
  const set = loadSet();
  if (graphOf) return graph(set, graphOf);
  if (mode === "--stats") return stats(set);
  if (mode === "--orphans") return orphans(set);
  if (mode === "--residue") return report(set, new Set(["changelog residue"]), false) > 0 ? 1 : 0;
  const n = report(set, only.size > 0 ? only : null, check);
  const count = set.records.size;
  if (n > 0) console.log(`\n${n} finding(s) across ${count} records`);
  else if (!check) console.log(`\nclean: ${count} records`);
  return n > 0 && check ? 1 : 0;
}
