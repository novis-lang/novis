// What each feature on the roster has on disk, and what it still owes. `rule:testing/feature-proofs` is
// what a feature owes. Here it is data: `POLICY` per kind, overridden by `tools/data/dossier-policy.toml`,
// with one feature excused from one proof by a skip entry whose value is the reason.
//
// A test is attributed to a feature by a `covers:` marker, in a `.nvst` case or above a Rust `#[test]`.
// A `Core` member is also credited by a case that calls it as `Class::member(`. An example, an attack and
// a bench are attributed by path. A perf figure is current when its record's `impl_hash` is `implHash`
// of the implementing file as it is now (`rule:testing/member-perf-ledger`). A record from any machine is current for what a feature owes, and
// the one taken on this machine is what `--id` prints beside it.

import { createHash } from "node:crypto";
import { existsSync, readdirSync, statSync } from "node:fs";
import { cpus, machine as osMachine } from "node:os";
import { join } from "node:path";
import { parse as parseToml } from "smol-toml";
import { analyse, digest } from "../keys/scan.ts";
import { abs, ROOT } from "../lib/paths.ts";
import { comparePaths, splitlines } from "../lib/py.ts";
import { aboutFile, benchFile, examplesDir, hostileDir, implFile, namesIn, read, type Entry, type Kind } from "./roster.ts";

/** Every proof a feature can owe, in the order the audit prints them. */
export const PROOFS = ["tests", "examples", "perf", "hostile", "about", "help"] as const;
export type Proof = (typeof PROOFS)[number];

/** What one kind owes. `tests` counts cases from either side and `rust` how many of them are Rust tests. */
export interface Owes {
  tests: number;
  rust: number;
  examples: number;
  perf: boolean;
  hostile: number;
  about: boolean;
  help: boolean;
  /** Every program among the proofs is inside the plain-comment bounds. Off unless the policy file turns it on. */
  comments?: boolean;
}

// `help` is off for an exception, an interface and a directive: the binary has no card or index entry to
// show for one yet. `about` is off for every kind until the policy file's override for `all` turns it on.
const POLICY: Record<Kind, Owes> = {
  member: { tests: 2, rust: 1, examples: 3, perf: true, hostile: 1, about: false, help: true },
  lang: { tests: 2, rust: 0, examples: 3, perf: true, hostile: 1, about: false, help: true },
  exception: { tests: 1, rust: 0, examples: 1, perf: false, hostile: 1, about: false, help: false },
  enum: { tests: 1, rust: 0, examples: 1, perf: false, hostile: 0, about: false, help: true },
  interface: { tests: 1, rust: 0, examples: 1, perf: false, hostile: 0, about: false, help: false },
  tool: { tests: 1, rust: 0, examples: 1, perf: false, hostile: 1, about: false, help: true },
  directive: { tests: 1, rust: 0, examples: 1, perf: false, hostile: 1, about: false, help: false },
};

const POLICY_FILE = "tools/data/dossier-policy.toml";
const HELP_BACKLOG = "tools/data/help-backlog.toml";
/** The skip reason every feature on the help backlog reads as. */
export const HELP_BACKLOG_REASON =
  "Landed before the help proof was owed; goal `core-class-cards` writes it and deletes this feature from tools/data/help-backlog.toml.";

export type Policy = Record<Kind, Owes>;
/** Per feature id, per proof it is excused from, the reason. */
export type Skips = Map<string, Record<string, string>>;

type Table = Record<string, unknown>;
const isTable = (v: unknown): v is Table => typeof v === "object" && v !== null && !Array.isArray(v);

/**
 * `POLICY` with the policy file's `[all]` and `[<kind>]` overrides applied, and the skip map, with every
 * help-backlog feature skipping `help`. `noPerf`, and `NVS_PROOFS_NO_PERF=1` in the environment, stop
 * the perf proof from being owed and leave the ledger and the benches as they are.
 */
export function loadPolicy(noPerf: boolean): { policy: Policy; skips: Skips } {
  const policy = Object.fromEntries(Object.entries(POLICY).map(([k, v]) => [k, { ...v }])) as Policy;
  const skips: Skips = new Map();
  const doc: Table = existsSync(abs(POLICY_FILE)) ? parseToml(read(POLICY_FILE)) : {};
  for (const [kind, fields] of Object.entries(doc)) {
    if (!isTable(fields)) continue;
    if (kind === "skip") {
      for (const [fid, reasons] of Object.entries(fields)) if (isTable(reasons)) skips.set(fid, { ...(reasons as Record<string, string>) });
    } else if (kind === "all") {
      for (const k of Object.keys(policy) as Kind[]) Object.assign(policy[k], fields);
    } else if (kind in policy) {
      Object.assign(policy[kind as Kind], fields);
    }
  }
  if (existsSync(abs(HELP_BACKLOG))) {
    const backlog: Table = parseToml(read(HELP_BACKLOG));
    for (const fid of Array.isArray(backlog.features) ? (backlog.features as string[]) : []) {
      const reasons = skips.get(fid) ?? {};
      if (!("help" in reasons)) reasons.help = HELP_BACKLOG_REASON;
      skips.set(fid, reasons);
    }
  }
  const env = process.env.NVS_PROOFS_NO_PERF ?? "";
  if (noPerf || (env !== "" && env !== "0")) for (const k of Object.keys(policy) as Kind[]) policy[k].perf = false;
  return { policy, skips };
}

/** The proofs some kind still owes: the audit's columns, so a proof switched off shows no column. */
export function shownProofs(policy: Policy): Proof[] {
  const kinds = Object.values(policy);
  return PROOFS.filter((p) => (p !== "perf" && p !== "about" && p !== "help") || kinds.some((k) => k[p]));
}

/** The words a description aims for, and the most the check accepts once one is owed. */
const ABOUT_WORDS: [number, number] = [40, 160];
const ABOUT_WORDS_ACCEPTED = 200;

/** What is wrong with a description's shape, or "". Whether it reads well is a person's to check. */
export function aboutProblem(path: string): string {
  const text = read(path).trim();
  if (text.startsWith("#") || text.startsWith("---")) return "opens with a heading or front matter, and the page supplies the title";
  if (text.includes("```")) return "carries a code block, and the examples are where code goes";
  const words = text.split(/\s+/).filter(Boolean).length;
  if (words < ABOUT_WORDS[0] || words > ABOUT_WORDS_ACCEPTED) {
    return `${words} words, outside ${ABOUT_WORDS[0]}-${ABOUT_WORDS_ACCEPTED} (the target is ${ABOUT_WORDS[0]}-${ABOUT_WORDS[1]})`;
  }
  return "";
}

// The three bounds of AGENTS.md § *Text an end user reads* a script can count. Which words a comment
// uses is the writer's to get right, and nothing here reads them.
const COMMENT_TOP_LINES = 4;
const COMMENT_STEP_LINES = 2;
const COMMENT_SENTENCE_WORDS = 25;
const COMMENT_DIRECTIVE_RE = /^\/\/\s*(?:bench|hostile|covers|proof|requires):/;

/**
 * What is wrong with the shape of a proof program's comments, as `line: what`. A directive line is a
 * tool's and is skipped, and so is an indented comment line, which is how a configuration example shows
 * the block it is about.
 */
export function commentProblems(path: string): string[] {
  const blocks: [number, string[]][] = [];
  let current: string[] = [];
  splitlines(read(path)).forEach((line, i) => {
    const text = line.trim();
    if (text.startsWith("//") && !COMMENT_DIRECTIVE_RE.test(text)) {
      const body = text.slice(2);
      if (body.startsWith("     ") || !body.trim()) return;
      if (current.length === 0) {
        current = [];
        blocks.push([i + 1, current]);
      }
      current.push(body.trim());
    } else if (!text.startsWith("//")) {
      current = [];
    }
  });
  const problems: string[] = [];
  blocks.forEach(([number, lines], index) => {
    const top = index === 0 && number <= 3;
    const bound = top ? COMMENT_TOP_LINES : COMMENT_STEP_LINES;
    if (lines.length > bound) {
      problems.push(`${number}: ${top ? "the top comment" : "a comment above a step"} is ${lines.length} lines, and ${bound} is the bound`);
    }
    const prose = lines.join(" ");
    if (prose.includes("—") || prose.includes(" -- ")) problems.push(`${number}: a dash joins two sentences; write two`);
    for (const sentence of prose.split(/(?<=[.!?:])\s+/)) {
      const words = sentence.split(/\s+/).filter(Boolean);
      if (words.length > COMMENT_SENTENCE_WORDS) {
        problems.push(`${number}: a ${words.length}-word sentence opening \`${words.slice(0, 5).join(" ")} ...\`; ${COMMENT_SENTENCE_WORDS} is the bound`);
      }
    }
  });
  return problems;
}

/** `// proof: gap <gap id>` in a proof: a bug it found that nobody has fixed yet, recorded as
 * `data/gaps/<gap id>.json`. */
const GAP_RE = /(?:\/\/|#)\s*proof:\s*gap\s+(\S+)/;

/** The gap id a proof's text is marked with, or null. */
export function knownGap(source: string): string | null {
  return GAP_RE.exec(source)?.[1] ?? null;
}

/** The title of the gap record `id`, or null when `data/gaps/` has no such record. */
export function gapTitle(id: string): string | null {
  const path = `data/gaps/${id}.json`;
  return existsSync(abs(path)) ? (JSON.parse(read(path)) as { title: string }).title : null;
}

/** What one feature has on disk, and how each proof was attributed. */
export interface Proofs {
  nvst: string[];
  rust: string[];
  examples: string[];
  hostile: string[];
  /** The website description, when it is on disk, and what is wrong with its shape. */
  about: string;
  aboutProblem: string;
  bench: string;
  /** The newest record taken on this machine, which `--id` prints. */
  perf: Record<string, unknown> | null;
  /** The newest current record from any machine, which decides whether perf is owed. */
  perfAny: Record<string, unknown> | null;
  /** How many cases are credited by a call rather than by a marker. */
  inferred: number;
  /** The proofs carrying a known-gap marker. */
  gaps: string[];
}

/** `// covers: A, B`, in a `.nvst`, a `.nvs` or above a Rust `#[test]`. `#` lets it sit in TOML too. */
const COVERS_RE = /(?:\/\/|#)\s*covers:\s*(.+)/g;

/** Every file under a repo-relative directory whose name ends `ext`, skipping any `target` directory. */
export function walk(dir: string, ext: string): string[] {
  const out: string[] = [];
  const visit = (rel: string) => {
    let entries;
    try {
      entries = readdirSync(abs(rel), { withFileTypes: true });
    } catch {
      return;
    }
    for (const e of entries) {
      if (e.isDirectory()) {
        if (e.name !== "target") visit(`${rel}/${e.name}`);
      } else if (e.name.endsWith(ext)) out.push(`${rel}/${e.name}`);
    }
  };
  visit(dir);
  return out;
}

const names = (text: string) => text.split(",").map((p) => p.trim().replace(/^[`"']+|[`"']+$/g, "")).filter(Boolean);

/**
 * Every `covers:` marker in the two case trees and `crates/`, as feature id -> the files carrying it. A
 * Rust marker is labelled with the `#[test] fn` under it. The example, attack and bench trees are
 * attributed by path, so a marker in one is never read.
 */
function scanMarkers(): Map<string, string[]> {
  const found = new Map<string, string[]>();
  const roots: [string, string][] = [["tests/conformance", ".nvst"], ["tests/differential", ".nvst"], ["crates", ".rs"]];
  for (const [base, ext] of roots) {
    for (const path of walk(base, ext)) {
      const text = read(path);
      if (!text.includes("covers:")) continue;
      for (const m of text.matchAll(COVERS_RE)) {
        let label = path;
        if (ext === ".rs") {
          const end = m.index! + m[0].length;
          const fn = /\bfn\s+([a-zA-Z_][a-zA-Z0-9_]*)/.exec(text.slice(end, end + 400));
          if (fn) label = `${label}::${fn[1]}`;
        }
        for (const name of names(m[1]!)) {
          if (!found.has(name)) found.set(name, []);
          found.get(name)!.push(label);
        }
      }
    }
  }
  return found;
}

/**
 * Which cases call which member as `Class::member(`, keyed `static:<class tail>::<member>`. Only that
 * written form is credited: `->member(` cannot be tied to a class without a type checker, so an instance
 * member is attributed by its `covers:` marker instead.
 */
function scanCalls(): Map<string, string[]> {
  const re = /(?:Core\\)?([A-Za-z_][A-Za-z0-9_\\]*)::([a-zA-Z_][a-zA-Z0-9_]*)\s*\(/g;
  const out = new Map<string, string[]>();
  for (const base of ["tests/conformance", "tests/differential"]) {
    for (const path of walk(base, ".nvst")) {
      for (const m of read(path).matchAll(re)) {
        const key = `static:${m[1]!.split("\\").pop()}::${m[2]}`;
        if (!out.has(key)) out.set(key, []);
        out.get(key)!.push(path);
      }
    }
  }
  return out;
}

export const LEDGER = "docs/perf/members.ndjson";

/** Every perf record, grouped by feature, oldest first. */
export function ledgerRecords(): Map<string, Record<string, unknown>[]> {
  const out = new Map<string, Record<string, unknown>[]>();
  if (!existsSync(abs(LEDGER))) return out;
  for (const raw of read(LEDGER).split("\n")) {
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

const sha12 = (text: string) => createHash("sha1").update(text, "utf8").digest("hex").slice(0, 12);

/**
 * What a perf figure is current against: the implementing file's `card` tier from `analyse`, which is
 * its tokens without comments, layout, inline test modules or the initialisers of card constants. A file
 * that is not Rust, such as a reference chapter, is its text with its line endings made `\n`. Empty when
 * there is no such file.
 */
export function implHash(path: string): string {
  try {
    if (!statSync(abs(path)).isFile()) return "";
  } catch {
    return "";
  }
  const text = read(path);
  return path.endsWith(".rs") ? analyse(text).card : digest(text);
}

/** Python's `json.dumps(..., sort_keys=True)` of a flat object, which the fingerprint's id hashes. */
function pyFlatJson(fields: Record<string, string | number>): string {
  const esc = (s: string) => JSON.stringify(s).replace(/[\u0080-￿]/g, (c) => `\\u${c.charCodeAt(0).toString(16).padStart(4, "0")}`);
  return `{${Object.keys(fields)
    .sort()
    .map((k) => `${esc(k)}: ${typeof fields[k] === "number" ? fields[k] : esc(fields[k] as string)}`)
    .join(", ")}}`;
}

/**
 * This machine, as the ledger records it. `cores` is `.loop/machine.json`'s local entry when there is
 * one. On Windows `cpu` is the processor identifier; elsewhere it is `uname -m`, which is what `uname -p`
 * reports on the machines the ledger has been taken on.
 */
export function fingerprint(): Record<string, string | number> {
  const win = process.platform === "win32";
  const arch = win ? (process.env.PROCESSOR_ARCHITEW6432 ?? process.env.PROCESSOR_ARCHITECTURE ?? osMachine()) : osMachine();
  let cores = cpus().length;
  try {
    const cached = JSON.parse(read(".loop/machine.json")).contexts?.local?.cores;
    if (typeof cached === "number" && cached > 0) cores = cached;
  } catch {
    // No cache: the processor count stands.
  }
  const fields: Record<string, string | number> = {
    cpu: (win ? process.env.PROCESSOR_IDENTIFIER : "") || arch,
    os: win ? "windows" : process.platform,
    release: arch,
    cores,
  };
  fields.id = sha12(pyFlatJson(fields));
  return fields;
}

const sortedSet = (items: Iterable<string>) => [...new Set(items)].sort();

/** What every entry has on disk, by feature id. */
export function collect(entries: Entry[]): Map<string, Proofs> {
  const markers = scanMarkers();
  const calls = scanCalls();
  const perf = ledgerRecords();
  const me = fingerprint().id;
  const hashes = new Map<string, string>();
  const hashOf = (path: string) => {
    if (!hashes.has(path)) hashes.set(path, implHash(path));
    return hashes.get(path)!;
  };
  const out = new Map<string, Proofs>();
  for (const e of entries) {
    const marked = markers.get(e.id) ?? [];
    const p: Proofs = {
      nvst: sortedSet(marked.filter((f) => f.endsWith(".nvst"))),
      rust: sortedSet(marked.filter((f) => f.includes(".rs"))),
      examples: [],
      hostile: [],
      about: "",
      aboutProblem: "",
      bench: "",
      perf: null,
      perfAny: null,
      inferred: 0,
      gaps: [],
    };
    if (e.kind === "member") {
      const [owner, member] = [e.id.slice(0, e.id.indexOf("::")), e.id.slice(e.id.indexOf("::") + 2)];
      const have = new Set(p.nvst);
      const fresh = sortedSet((calls.get(`static:${owner.split("\\").pop()}::${member}`) ?? []).filter((f) => !have.has(f)));
      p.inferred = fresh.length;
      p.nvst = sortedSet([...p.nvst, ...fresh]);
    }
    const programs = (dir: string) => namesIn(dir, ".nvs").map((n) => `${dir}/${n}`);
    p.examples = programs(examplesDir(e)).sort();
    p.hostile = programs(hostileDir(e)).sort();
    if (existsSync(abs(aboutFile(e)))) {
      p.about = aboutFile(e);
      p.aboutProblem = aboutProblem(p.about);
    }
    for (const dir of [examplesDir(e), hostileDir(e)]) {
      p.gaps.push(...programs(dir).sort(comparePaths).filter((f) => knownGap(read(f))));
    }
    if (existsSync(abs(benchFile(e)))) p.bench = benchFile(e);
    const records = perf.get(e.id) ?? [];
    const mine = records.filter((r) => r.machine === me);
    p.perf = mine.at(-1) ?? null;
    const current = implFile(e) && hashOf(implFile(e));
    const fresh = records.filter((r) => !current || r.impl_hash === current);
    p.perfAny = fresh.at(-1) ?? null;
    out.set(e.id, p);
  }
  return out;
}

/** What `entry` still owes, proof -> one sentence. Empty means complete. */
export function owed(entry: Entry, proofs: Proofs, policy: Policy, skips: Skips): Record<string, string> {
  const want = policy[entry.kind];
  const skip = skips.get(entry.id) ?? {};
  const out: Record<string, string> = {};
  if (!("tests" in skip)) {
    const have = proofs.nvst.length + proofs.rust.length;
    if (have < want.tests) out.tests = `${have} of ${want.tests} cases`;
    else if (want.rust && proofs.rust.length === 0) out.tests = "no Rust-side test carries its `covers:` marker";
  }
  if (!("examples" in skip) && proofs.examples.length < want.examples) {
    out.examples = `${proofs.examples.length} of ${want.examples} in ${examplesDir(entry)}`;
  }
  if (!("hostile" in skip) && proofs.hostile.length < want.hostile) {
    out.hostile = `${proofs.hostile.length} of ${want.hostile} in ${hostileDir(entry)}`;
  }
  if (want.about && !("about" in skip)) {
    if (!proofs.about) out.about = `no description at ${aboutFile(entry)}`;
    else if (proofs.aboutProblem) out.about = `${proofs.about}: ${proofs.aboutProblem}`;
  }
  if (want.help && !("help" in skip) && entry.help) out.help = entry.help;
  if (want.comments && !("comments" in skip)) {
    // Not a proof of its own: it judges the programs the other proofs are made of.
    const programs = [...proofs.examples, ...proofs.hostile];
    if (proofs.bench) {
      const scale = proofs.bench.replace(/\.nvs$/, "") + ".scale.nvs";
      programs.push(proofs.bench, ...(existsSync(join(ROOT, scale)) ? [scale] : []));
    }
    const missed = programs.map((f) => [f, commentProblems(f)] as const).filter(([, problems]) => problems.length > 0);
    if (missed.length > 0) {
      const [first, problems] = missed[0]!;
      out.comments = `${missed.length} program(s) outside the plain-comment bounds, first ${first} line ${problems[0]}`;
    }
  }
  if (want.perf && !("perf" in skip)) {
    // A figure from any machine against this implementation text documents the feature.
    if (!proofs.bench) out.perf = `no bench at ${benchFile(entry)}`;
    else if (!proofs.perfAny) out.perf = implFile(entry) ? `stale: ${implFile(entry)} changed since it was last measured` : "never measured";
  }
  return out;
}
