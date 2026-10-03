// `bun nv decisions`: the decision summary a person reads, and the bookkeeping that keeps it true.
//
//     bun nv decisions              the summary, rendered to the terminal
//     bun nv decisions --check      what the summary owes; quiet on success
//     bun nv decisions --gate       only the findings that are always wrong; the CI shape
//     bun nv decisions --work       the work order for a pass: only the decisions not summarized yet
//     bun nv decisions --apply FILE merge written entries, transactionally
//     bun nv decisions --render     rewrite `docs/decisions.md` from the source
//     bun nv decisions --json       the summary as JSON, on stdout
//     bun nv decisions --groups     the closed group list, with what belongs in each
//
// The records under `docs/decisions/` are every settled decision at full length. The summary is one
// plain-language paragraph per decision, grouped by topic, in the order the decisions were taken, with
// no cross-reference in the prose. Each entry is the `summary` field of the decision's record,
// `data/decisions/NNNN.json`, and every one is prose a person wrote. `docs/decisions.md` is generated
// from those fields: `--render` writes it, and `--check` reports it being stale. The website does not
// publish the summary.
//
// A pass does only the accepted decisions that have no summary yet. `--work` prints them as `[[entry]]`
// blocks to fill in, and `--apply` checks the filled file and writes each entry into its record.
//
// `--check` holds the prose to three mechanical rules. `refs`: no `ADR`, no `§`, no bare decision
// number, no markdown link, because the renderer puts the one link per entry. `size`: a
// headline is one line, and a body is two to four sentences. `jargon`: a short blocklist of words this
// repository uses and a reader does not know.
//
// The groups are a closed list in reading order, `internal` last. Inside a group, entries run in the
// order they were decided, and `pin = true` floats one entry to the top: the one that frames the group.
// At most one per group, checked.

import { existsSync, mkdirSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { parse as parseToml } from "smol-toml";
import { ROOT, rel } from "../lib/paths.ts";
import { ArgError, fill, parseArgs, pyInt, pyRepr, squash, wrap } from "../lib/py.ts";
import type { RecordType } from "../lib/schema.ts";
import { load, write as writeRecord } from "../lib/store.ts";
import { decision } from "../schema/decision.ts";
import { parse as parseRecord, type Prose } from "./records.ts";

export const summary = "the plain-language decision summary: nv decisions [--check|--gate|--work|--apply FILE|--render|--json|--groups]";

const RENDER_MD = join(ROOT, "docs", "decisions.md");
const RECORDS = join(ROOT, "docs", "decisions");

/** Where a decision record is read on the web. `website/config/site.mjs` holds the site's own copy. */
const GITHUB_BLOB = "https://github.com/novis-lang/novis/blob/main";

/** What a message tells the reader to run, and the command the generated files' headers name. */
const CLI = "bun nv decisions";

/** [id, heading, what belongs here]. The third column is the only description of a group anywhere. */
const GROUPS: [string, string, string][] = [
  [
    "foundations",
    "What Novis is",
    "Who the language is for, what it refuses to be, and the ground the rest stands on: " +
      "how errors travel, what a running script is, what ships in the box.",
  ],
  [
    "types",
    "Types and values",
    "What a value can be and how its type behaves: the type system's own rules, the scalar " +
      "types, conversion, equality, and the shapes a type can take.",
  ],
  [
    "syntax",
    "How code is written",
    "Spelling. What parses and what does not, which PHP forms were kept and which were " +
      "rejected, naming, visibility, and the shape of a file.",
  ],
  [
    "language",
    "Language features",
    "What the language gives you to build with: classes and their members, interfaces, " +
      "closures, iteration, concurrency, attributes, testing.",
  ],
  [
    "security",
    "Security and isolation",
    "The decisions that exist because the code and the data are not trusted: qualifiers on " +
      "values, what a request can reach, what an extension may do, what the doors are.",
  ],
  [
    "core",
    "The standard library and runtime",
    "What is built in and how it behaves: the Core namespace's own conventions, the " +
      "components that ship with it, and how the runtime serves a request.",
  ],
  [
    "tooling",
    "Tools, editors and shipping",
    "Everything around the language: the command-line tool, the editor experience, " +
      "formatting, packaging, deployment, and what the tools may and may not do for you.",
  ],
  [
    "internal",
    "Engineering decisions",
    "Decisions about how Novis itself is built and measured. Real decisions, kept for the " +
      "record, but a reader learning the language can skip the group entirely.",
  ],
];
const GROUP_IDS = GROUPS.map((g) => g[0]);

const HEADLINE_MAX = 78;
const BODY_MIN_WORDS = 20;
const BODY_MAX_WORDS = 95;

/** Words this repository uses precisely and a reader arriving from PHP does not know. */
const JARGON = [
  "lowering", "lowers to", "safepoint", "monomorph", "arity", "codegen", "ABI", "IR",
  "vtable", "trampoline", "prologue", "epilogue", "call site", "sans-io", "reentrant",
  "idempotent", "invariant", "orthogonal", "canonicalize", "normative",
];

const W = "[\\p{L}\\p{N}_]";
const BANNED_REFS: [RegExp, string][] = [
  [new RegExp(`(?<!${W})ADRs?(?!${W})`, "u"), "names an ADR"],
  [/§/u, "uses a section mark"],
  [/(?<![\p{L}\p{N}_.])0\p{Nd}{3}(?![\p{L}\p{N}_.])/u, "cites a decision by number"],
  [/\]\(/u, "carries a markdown link"],
  [new RegExp(`(?<!${W})amend(s|ed|ment|ments)?(?!${W})`, "iu"), "talks about amendments"],
  [new RegExp(`(?<!${W})supersed(e|es|ed|ing)(?!${W})`, "iu"), "talks about superseding"],
];

const pyLen = (s: string) => [...s].length;
const escapeRe = (s: string) => s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");

/** A record's `summary` field, which `apply` writes after `check` has vouched for its group. */
type Summary = NonNullable<(typeof decision extends RecordType<infer T> ? T : never)["summary"]>;

interface Entry {
  adr: string;
  group: string;
  pin: boolean;
  headline: string;
  body: string;
}

// ------------------------------------------------------------------------------------ the corpus side

interface Record {
  num: string;
  title: string;
  prose: Prose;
}

/** The decision itself, without the `ADR NNNN —` its heading carries. */
function titleOf(a: Record): string {
  return a.title.replace(/^ADR\s+\d{4}\s*[—-]\s*/u, "").trim();
}

/** The `In short` blockquote, as one line of plain text. */
function inShort(a: Record): string {
  const out: string[] = [];
  for (const line of a.prose.lines) {
    if (line.startsWith("> **In short:**")) out.push(line.slice("> **In short:**".length));
    else if (out.length > 0 && line.startsWith(">")) out.push(line.slice(1));
    else if (out.length > 0) break;
  }
  return squash(out.join(" ")).replace(/\[([^\]]*)\]\([^)]*\)/g, "$1");
}

/** The Decision section's numbered headings: what the decision is made of. */
function shape(a: Record): string[] {
  const [start, end] = a.prose.sections.get("Decision") ?? [0, 0];
  const out: string[] = [];
  for (const line of a.prose.lines.slice(start, end)) {
    const m = /^#{3,4} (\d+[a-z]?\.\s*)?(.+)$/.exec(line);
    if (m) out.push(m[2]!.trim());
  }
  return out;
}

/** The record's YAML block: `status`, and under `changes:` the `creates` and `modifies` rule ids. */
function frontmatter(a: Record): { status: string; creates: string[]; modifies: string[] } {
  const out = { status: "", creates: [] as string[], modifies: [] as string[] };
  const lines = a.prose.lines;
  if (lines.length === 0 || lines[0] !== "---") return out;
  let current: "creates" | "modifies" | null = null;
  for (const line of lines.slice(1)) {
    if (line === "---") break;
    let m = /^(status):\s*(.+?)\s*$/.exec(line);
    if (m) {
      out.status = m[2]!;
      current = null;
      continue;
    }
    m = /^\s+(creates|modifies):\s*$/.exec(line);
    if (m) {
      current = m[1] as "creates" | "modifies";
      continue;
    }
    m = /^\s+-\s+(\S+)\s*$/.exec(line);
    if (m && current) out[current].push(m[1]!);
  }
  return out;
}

/** Every accepted record, by number. A retired one is not a decision the summary owes. */
function corpus(): Map<string, Record> {
  const out = new Map<string, Record>();
  const files = readdirSync(RECORDS).filter((f) => f.startsWith("0") && f.toLowerCase().endsWith(".md")).sort();
  for (const file of files) {
    const text = readFileSync(join(RECORDS, file), "utf8").replace(/\r\n?/g, "\n");
    const prose = parseRecord(file.slice(0, 4), `docs/decisions/${file}`, text);
    const heading = prose.lines.find((l) => l.startsWith("# "));
    const title = heading === undefined ? "" : heading.replace(/^[# ]+/, "").trim();
    const a: Record = { num: file.slice(0, 4), title, prose };
    if (frontmatter(a).status === "accepted") out.set(a.num, a);
  }
  return out;
}

// ---------------------------------------------------------------------------------------- the entries

/** Each decision record's `summary`, by number. A record with none is not in the map. */
function loadEntries(): Map<string, Entry> {
  const out = new Map<string, Entry>();
  for (const r of load(decision)) {
    const s = r.value?.summary;
    if (!s) continue;
    out.set(r.id, { adr: r.id, group: s.group, pin: Boolean(s.pin), headline: squash(s.headline), body: squash(s.body) });
  }
  return out;
}

/** The `[[entry]]` blocks of a file `--apply` is handed, in the shape `--work` prints them. */
function readEntries(path: string): Map<string, Entry> {
  const out = new Map<string, Entry>();
  if (!existsSync(path)) return out;
  const data = parseToml(readFileSync(path, "utf8")) as { entry?: { [k: string]: unknown }[] };
  for (const raw of data.entry ?? []) {
    const adr = String(raw.adr ?? "").padStart(4, "0");
    out.set(adr, {
      adr,
      group: String(raw.group ?? ""),
      pin: Boolean(raw.pin ?? false),
      headline: squash(String(raw.headline ?? "")),
      body: squash(String(raw.body ?? "")),
    });
  }
  return out;
}

const sortedKeys = <V>(m: Map<string, V>) => [...m.keys()].sort();

// ------------------------------------------------------------------------------------------- checks

function proseFindings(num: string, e: Entry): string[] {
  const out: string[] = [];
  const head = e.headline ?? "";
  const body = e.body ?? "";
  if (!head) out.push("headline is empty");
  if (pyLen(head) > HEADLINE_MAX) out.push(`headline is ${pyLen(head)} chars, over ${HEADLINE_MAX}`);
  if (head.endsWith(".")) out.push("headline ends in a period");
  const words = body.split(/\s+/).filter(Boolean).length;
  if (words < BODY_MIN_WORDS) out.push(`body is ${words} words, under ${BODY_MIN_WORDS}`);
  if (words > BODY_MAX_WORDS) out.push(`body is ${words} words, over ${BODY_MAX_WORDS}`);
  for (const [pattern, why] of BANNED_REFS) {
    for (const [field, text] of [["headline", head], ["body", body]] as const) {
      const m = pattern.exec(text);
      if (m) out.push(`${field} ${why}: ${pyRepr(m[0])}`);
    }
  }
  const lowered = `${head} ${body}`.toLowerCase();
  for (const word of JARGON) {
    if (new RegExp(`(?<![\\p{L}\\p{N}_-])${escapeRe(word.toLowerCase())}(?![\\p{L}\\p{N}_-])`, "u").test(lowered)) {
      out.push(`uses ${pyRepr(word)}, which the audience does not read`);
    }
  }
  return out.map((f) => `${num}: ${f}`);
}

type Found = Map<string, string[]>;

/** Every finding, grouped by kind. An empty map is a clean tree. */
function check(entries: Map<string, Entry>, adrs: Map<string, Record>): Found {
  const found: Found = new Map();
  const add = (kind: string, msg: string) => {
    if (!found.has(kind)) found.set(kind, []);
    found.get(kind)!.push(msg);
  };
  for (const num of sortedKeys(adrs)) {
    if (!entries.has(num)) add("missing", `${num}: ${[...titleOf(adrs.get(num)!)].slice(0, 70).join("")}`);
  }
  for (const num of sortedKeys(entries)) {
    const e = entries.get(num)!;
    const a = adrs.get(num);
    if (!a) {
      add("orphan", `${num}: summarized, but no accepted decision has that number`);
      continue;
    }
    if (!GROUP_IDS.includes(e.group)) add("group", `${num}: unknown group ${e.group === undefined ? "None" : pyRepr(e.group)}`);
    for (const f of proseFindings(num, e)) add("prose", f);
  }
  for (const gid of GROUP_IDS) {
    const pinned = sortedKeys(entries).filter((n) => entries.get(n)!.group === gid && entries.get(n)!.pin);
    if (pinned.length > 1) add("pin", `${gid}: ${pinned.join(", ")} are all pinned — a group is framed once`);
  }
  if (entries.size > 0) {
    const onDisk = existsSync(RENDER_MD) ? readFileSync(RENDER_MD, "utf8").replace(/\r\n?/g, "\n") : null;
    if (onDisk !== renderMd(entries)) add("render", `${rel(RENDER_MD)} is not what the source renders to -- run --render`);
  }
  return found;
}

// ---------------------------------------------------------------------------------------- rendering

/** Each group with entries, and its entries in decision order with the pinned one first. */
function grouped(entries: Map<string, Entry>): [[string, string, string], Entry[]][] {
  const out: [[string, string, string], Entry[]][] = [];
  for (const g of GROUPS) {
    const nums = sortedKeys(entries).filter((n) => entries.get(n)!.group === g[0]);
    // A stable sort: the pinned entry moves to the front, and the rest keep decision order.
    nums.sort((a, b) => Number(!entries.get(a)!.pin) - Number(!entries.get(b)!.pin));
    if (nums.length > 0) out.push([g, nums.map((n) => entries.get(n)!)]);
  }
  return out;
}

function renderMd(entries: Map<string, Entry>): string {
  const out = [
    "# What Novis has decided",
    "",
    `<!-- Generated by \`${CLI} --render\` from the summaries in data/decisions/.`,
    "     Do not edit: the next render overwrites it without saying so. -->",
    "",
    "Every decision this project has taken, in plain language, grouped by what it is about and",
    "in the order it was decided. Each entry says what is true now, not how it got there.",
    "",
    "The full reasoning behind any one of them -- the alternatives weighed, the costs accepted,",
    "the exact wording -- lives in the frozen records under `docs/decisions/`, each reached through",
    "the rule it changed in [the rulebook](ground-rules.md).",
    "",
  ];
  for (const [[, heading, blurb], rows] of grouped(entries)) {
    out.push(`## ${heading}`, "", blurb, "");
    for (const e of rows) {
      out.push(`**${e.headline}**`, "", fill(e.body, 100, { breakLongWords: false, breakOnHyphens: false }), "");
    }
  }
  return out.join("\n").trimEnd() + "\n";
}

/** The summary as JSON. No timestamp, so a render that changes nothing writes the same bytes. */
function renderJson(entries: Map<string, Entry>): string {
  const data = {
    groups: grouped(entries).map(([[id, title, blurb], rows]) => ({
      id,
      title,
      blurb,
      entries: rows.map((e) => ({
        adr: e.adr,
        headline: e.headline,
        body: e.body,
        // The site publishes the rulebook and not the frozen rationale behind it, so the full
        // record is read in the repository.
        url: `${GITHUB_BLOB}/docs/decisions/${e.adr}.md`,
      })),
    })),
  };
  return JSON.stringify(data, null, 2) + "\n";
}

// ----------------------------------------------------------------------------------------- commands

function fail(msg: string): number {
  console.error(`error: ${msg}`);
  return 1;
}

const LABELS: { [kind: string]: string } = {
  missing: `not summarized yet (${CLI} --work)`,
  orphan: "summarizes a decision that is not there",
  group: "not in a group the tool knows",
  pin: "more than one entry pinned to the top of a group",
  prose: "breaks a rule the summary is held to",
  render: "a generated file is behind the source",
};

function report(found: Found, out: string[]): void {
  for (const kind of ["missing", "orphan", "group", "pin", "prose", "render"]) {
    const rows = found.get(kind);
    if (!rows || rows.length === 0) continue;
    out.push(`${kind} — ${LABELS[kind]} (${rows.length})`, ...rows.map((r) => `  ${r}`), "");
  }
}

function work(entries: Map<string, Entry>, adrs: Map<string, Record>, group: string | undefined, limit: number, out: string[]): number {
  // An owed decision has no entry, so no group either: `--group` narrows the list to nothing.
  const owed = sortedKeys(adrs).filter((num) => !entries.has(num) && !group);
  const total = owed.length;
  if (total === 0) {
    out.push(`nothing owed: all ${adrs.size} decisions are summarized`);
    return 0;
  }
  const shown = limit ? owed.slice(0, limit) : owed;
  out.push(
    `# ${total} decision(s) owed; ${shown.length} below. Write an entry for each, then apply:`,
    `#   ${CLI} --apply <file>`,
    `# Groups: ${GROUP_IDS.join(", ")}   (${CLI} --groups)`,
    "",
  );
  for (const num of shown) {
    const a = adrs.get(num)!;
    out.push(`# ${"-".repeat(94)}`, `# ${num} — ${titleOf(a)}`);
    for (const line of wrap(inShort(a), 94)) out.push(`#   ${line}`);
    const parts = shape(a);
    if (parts.length > 0) out.push("# the decision has these parts:", ...parts.map((p) => `#   - ${p}`));
    out.push("", "[[entry]]", `adr      = '${num}'`, "group    = ''", "headline = ''", "body     = '''\n'''", "");
  }
  if (total > shown.length) out.push(`# ${total - shown.length} more owed; re-run --work after applying these.`);
  return 0;
}

function write(path: string, text: string): void {
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, text);
}

function apply(path: string, entries: Map<string, Entry>, adrs: Map<string, Record>, dryRun: boolean, out: string[]): number {
  const incoming = readEntries(path);
  if (incoming.size === 0) return fail(`${path} holds no [[entry]] blocks`);
  const merged = new Map(entries);
  const touched: string[] = [];
  for (const [num, e] of incoming) {
    if (!adrs.has(num)) return fail(`${num} is not an accepted decision — nothing to summarize`);
    merged.set(num, e);
    touched.push(num);
  }
  const findings = check(merged, adrs);
  const blocking: Found = new Map([...findings].filter(([k]) => ["group", "pin", "prose", "orphan"].includes(k)));
  if (blocking.size > 0) {
    const n = [...blocking.values()].reduce((s, v) => s + v.length, 0);
    out.push(`refused — ${n} finding(s), nothing written:\n`);
    report(blocking, out);
    return 1;
  }
  if (dryRun) {
    out.push(
      `would write ${touched.length} ${touched.length === 1 ? "entry" : "entries"}: ${touched.join(", ")}`,
      `would render ${rel(RENDER_MD)}`,
    );
    return 0;
  }
  const records = new Map(load(decision).map((r) => [r.id, r.value]));
  for (const num of touched) {
    const e = merged.get(num)!;
    const body = fill(e.body, 96, { breakLongWords: false, breakOnHyphens: false });
    const summary = { group: e.group, headline: e.headline, body, ...(e.pin ? { pin: true } : {}) } as Summary;
    writeRecord(decision, num, { ...records.get(num)!, summary });
  }
  write(RENDER_MD, renderMd(merged));
  const still = [...adrs.keys()].filter((n) => !merged.has(n)).length;
  out.push(`applied ${touched.length}: ${touched.join(", ")}`, `${merged.size} of ${adrs.size} decisions summarized; ${still} still owed`);
  return 0;
}

const USAGE = [
  "usage: nv decisions [-h] [--check] [--gate] [--work] [--group ID]",
  "                    [--limit LIMIT] [--apply FILE] [--dry-run] [--render]",
  "                    [--json] [--groups]",
].join("\n");

export async function run(args: string[]): Promise<number> {
  let flags: Set<string>;
  let values: Map<string, string>;
  let limit = 25;
  try {
    ({ flags, values } = parseArgs(args, {
      flags: ["--check", "--gate", "--work", "--dry-run", "--render", "--json", "--groups"],
      valued: ["--group", "--limit", "--apply"],
    }));
    const raw = values.get("--limit");
    if (raw !== undefined) {
      const n = pyInt(raw);
      if (n === null) throw new ArgError(`argument --limit: invalid int value: ${pyRepr(raw)}`);
      limit = n;
    }
  } catch (e) {
    if (!(e instanceof ArgError)) throw e;
    console.error(`${USAGE}\nnv decisions: error: ${e.message}`);
    return 2;
  }
  if (flags.has("--help")) {
    console.log(`${USAGE}\n\nnv decisions: ${summary}`);
    return 0;
  }
  const out: string[] = [];
  const flush = (code: number) => {
    if (out.length > 0) process.stdout.write(out.join("\n") + "\n");
    return code;
  };

  if (flags.has("--groups")) {
    for (const [gid, heading, blurb] of GROUPS) {
      out.push(`${gid.padEnd(12)} ${heading}`, ...wrap(blurb, 84).map((l) => `${"".padEnd(12)} ${l}`), "");
    }
    return flush(0);
  }

  const adrs = corpus();
  const entries = loadEntries();
  const group = values.get("--group");
  if (group && !GROUP_IDS.includes(group)) return fail(`unknown group ${pyRepr(group)} — one of ${GROUP_IDS.join(", ")}`);

  const applyPath = values.get("--apply");
  if (applyPath) return flush(apply(applyPath, entries, adrs, flags.has("--dry-run"), out));
  if (flags.has("--work")) return flush(work(entries, adrs, group, limit, out));
  if (flags.has("--json")) {
    process.stdout.write(renderJson(entries));
    return 0;
  }
  if (flags.has("--render")) {
    if (entries.size === 0) return fail("no record under data/decisions/ has a summary — nothing to render");
    write(RENDER_MD, renderMd(entries));
    console.log(`rendered ${entries.size} entries to ${rel(RENDER_MD)}`);
    return 0;
  }

  const found = check(entries, adrs);
  const n = [...found.values()].reduce((s, v) => s + v.length, 0);
  if (flags.has("--gate")) {
    // A decision not summarized yet is a pass that has not run, which is a schedule and not a fault.
    // The other kinds are always a mistake.
    const bad: Found = new Map([...found].filter(([k]) => k !== "missing"));
    const m = [...bad.values()].reduce((s, v) => s + v.length, 0);
    if (m) {
      report(bad, out);
      out.push(`${m} finding(s) that are always wrong`);
    }
    return flush(m ? 1 : 0);
  }
  if (flags.has("--check")) {
    if (n) {
      report(found, out);
      out.push(`${n} finding(s); ${entries.size} of ${adrs.size} decisions summarized`);
    }
    return flush(n ? 1 : 0);
  }
  if (entries.size === 0) {
    console.log(`the summary is empty — start with: ${CLI} --work`);
    return 0;
  }
  process.stdout.write(renderMd(entries));
  if (n) console.log(`\n[${n} finding(s) — ${CLI} --check]`);
  return 0;
}
