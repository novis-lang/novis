// What each feature on the roster has on disk, and what it still owes. `rule:testing/feature-proofs` is
// what a feature owes. Here it is data: `POLICY` per kind, with one feature excused from one proof by its
// skip file under `data/proofs/skip/`, whose value for that proof is the reason.
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
import { analyseAll, digest } from "../keys/scan.ts";
import { abs, ROOT } from "../lib/paths.ts";
import { comparePaths, splitlines } from "../lib/py.ts";
import { inPart, noteKey, unrecorded } from "../lib/reads.ts";
import { load } from "../lib/store.ts";
import { proofSkip } from "../schema/proofs.ts";
import { LEDGER, LEDGER_WHOLE, ledgerRows, perfKey } from "./ledger.ts";
import { anchorKey, CALL_ROOTS, callsIn, callsKey, coversKey, MARKER_ROOTS, markersIn } from "./markers.ts";
import { aboutFile, benchFile, DETAILS, detailsFile, examplesDir, hostileDir, implFile, introFile, noteRoster, programsIn, read, type Entry, type Kind } from "./roster.ts";

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
  /**
   * A member is named in code, in a backtick span or a fence, by its class's introduction under
   * `docs/reference/` when the class has one. Off unless the policy file turns it on.
   */
  intro?: boolean;
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
  // A package has no Rust side: its tests are its own `#[Test]` methods.
  package: { tests: 2, rust: 0, examples: 3, perf: true, hostile: 1, about: false, help: true },
};

const POLICY_FILE = "data/proofs/policy.json";
const HELP_BACKLOG = "data/proofs/help-backlog.json";
/** The skip reason every feature on the help backlog reads as. */
export const HELP_BACKLOG_REASON =
  "Landed before the help proof was owed; goal `core-class-cards` writes it and deletes this feature from data/proofs/help-backlog.json.";
const INTRO_BACKLOG = "data/proofs/intro-backlog.json";
/** The skip reason every member on the introduction backlog reads as. */
export const INTRO_BACKLOG_REASON =
  "Its class introduction was written before it had to name every member; the session that next rewrites that introduction names it and deletes this member from data/proofs/intro-backlog.json.";
/** Each backlog file, the proof its features skip, and the reason they read as. Nothing is ever added to one. */
const BACKLOGS: [file: string, proof: string, reason: string][] = [
  [HELP_BACKLOG, "help", HELP_BACKLOG_REASON],
  [INTRO_BACKLOG, "intro", INTRO_BACKLOG_REASON],
];

export type Policy = Record<Kind, Owes>;
/** Per feature id, per proof it is excused from, the reason. */
export type Skips = Map<string, Record<string, string>>;

type Table = Record<string, unknown>;
const isTable = (v: unknown): v is Table => typeof v === "object" && v !== null && !Array.isArray(v);

/**
 * `POLICY` with the policy file's `owes` applied, `all` before each kind, and the skip files' map, with every
 * feature on a backlog skipping that backlog's proof. `noPerf`, and `NVS_PROOFS_NO_PERF=1` in the environment, stop
 * the perf proof from being owed and leave the ledger and the benches as they are.
 */
export function loadPolicy(noPerf: boolean): { policy: Policy; skips: Skips } {
  const policy = Object.fromEntries(Object.entries(POLICY).map(([k, v]) => [k, { ...v }])) as Policy;
  const skips: Skips = new Map();
  const doc: Table = existsSync(abs(POLICY_FILE)) ? JSON.parse(read(POLICY_FILE)) : {};
  const owes = isTable(doc.owes) ? doc.owes : {};
  if (isTable(owes.all)) for (const k of Object.keys(policy) as Kind[]) Object.assign(policy[k], owes.all);
  for (const k of Object.keys(policy) as Kind[]) if (isTable(owes[k])) Object.assign(policy[k], owes[k]);
  for (const { value } of load(proofSkip)) {
    if (!isTable(value)) continue;
    const { feature, ...reasons } = value;
    skips.set(feature, { ...skips.get(feature), ...(reasons as Record<string, string>) });
  }
  for (const [file, proof, reason] of BACKLOGS) {
    if (!existsSync(abs(file))) continue;
    const backlog: Table = JSON.parse(read(file));
    for (const fid of Array.isArray(backlog.features) ? (backlog.features as string[]) : []) {
      const reasons = skips.get(fid) ?? {};
      if (!(proof in reasons)) reasons[proof] = reason;
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

/** The code of a class introduction's text: every fence's lines and every backtick span, one per line. */
export function introCodeOf(text: string): string {
  const fences = [...text.matchAll(/^```[^\n]*\n([\s\S]*?)^```/gm)].map((m) => m[1]!);
  const spans = [...text.replace(/^```[^\n]*\n[\s\S]*?^```/gm, "").matchAll(/`([^`\n]+)`/g)].map((m) => m[1]!);
  return [...fences, ...spans].join("\n");
}

const introCodes = new Map<string, string>();
/** `introCodeOf` the introduction at `path`, read once per process, since every member of its class asks. */
function introCode(path: string): string {
  let code = introCodes.get(path);
  if (code === undefined) {
    code = introCodeOf(read(path));
    introCodes.set(path, code);
  }
  return code;
}

/** Whether `code` has `name` as a whole word, so `at` is not found inside `format`. */
export function namesInCode(code: string, name: string): boolean {
  return new RegExp(`(?<![\\w$])${name.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}(?!\\w)`).test(code);
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
const COMMENT_PAYLOAD_RE = /^\/\/\s*hostile:\s*comment-payload\s*$/;

/**
 * What is wrong with the shape of a proof program's comments, as `line: what`. A `///` doc comment is
 * judged like a `//` one. A directive line is a tool's and is skipped, and so is an empty or indented
 * comment line, which is how a configuration example shows the block it is about.
 * `// hostile: comment-payload` skips the comment block directly under it: that block is an attack's
 * payload, which no reader is meant to read as prose.
 */
export function commentProblems(path: string): string[] {
  const blocks: [number, string[]][] = [];
  let current: string[] = [];
  let payload = false;
  splitlines(read(path)).forEach((line, i) => {
    const text = line.trim();
    if (COMMENT_PAYLOAD_RE.test(text)) {
      payload = true;
      current = [];
      return;
    }
    if (payload && text.startsWith("//")) return;
    payload = false;
    if (text.startsWith("//") && !COMMENT_DIRECTIVE_RE.test(text)) {
      const body = text.replace(/^\/\/\/?/, "");
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
    problems.push(...proseProblems(number, lines.join(" ")));
  });
  return problems;
}

/** The sentence and dash bounds over one run of prose that starts on line `number`, as `line: what`. */
function proseProblems(number: number, prose: string): string[] {
  const problems: string[] = [];
  if (prose.includes("—") || prose.includes(" -- ")) problems.push(`${number}: a dash joins two sentences; write two`);
  for (const sentence of prose.split(/(?<=[.!?:])\s+/)) {
    const words = sentence.split(/\s+/).filter(Boolean);
    if (words.length > COMMENT_SENTENCE_WORDS) {
      problems.push(`${number}: a ${words.length}-word sentence opening \`${words.slice(0, 5).join(" ")} ...\`; ${COMMENT_SENTENCE_WORDS} is the bound`);
    }
  }
  return problems;
}

/**
 * What is wrong with the shape of a feature's `details.md`, as `line: what`: a heading or front matter
 * at the top, a code block, and each paragraph and `-` list item held to a comment's sentence and dash
 * bounds. It has no word limit.
 */
export function detailsProblems(path: string): string[] {
  const text = read(path);
  const problems: string[] = [];
  const head = text.trimStart();
  if (head.startsWith("#") || head.startsWith("---")) problems.push("1: opens with a heading or front matter, and the page supplies the title");
  let block: [number, string[]] | null = null;
  const flush = () => {
    if (block) problems.push(...proseProblems(block[0], block[1].join(" ")));
    block = null;
  };
  splitlines(text).forEach((line, i) => {
    const t = line.trim();
    if (t.startsWith("```")) problems.push(`${i + 1}: a code block, and the examples are where code goes`);
    if (!t || t.startsWith("```")) return flush();
    if (/^[-*] /.test(t)) flush();
    if (!block) block = [i + 1, []];
    block[1].push(t.replace(/^[-*] /, ""));
  });
  flush();
  return problems;
}

/** What is wrong with the plain-text bounds of `path`: a `details.md` or a proof program. */
export function textProblems(path: string): string[] {
  return path.endsWith(`/${DETAILS}`) || path === DETAILS ? detailsProblems(path) : commentProblems(path);
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
  /** The exact rules beside the description, when it is on disk, or "". */
  details: string;
  /** A member's class introduction, when the class has one, or "", and whether it names the member in code. */
  intro: string;
  introNames: boolean;
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

/** Every `covers:` marker `markersIn` finds under `MARKER_ROOTS`, as feature id -> the tests carrying it.
 * The files are read `unrecorded`: `collect` names the `covers:` key of each feature it asks about. */
function scanMarkers(): Map<string, string[]> {
  const found = new Map<string, string[]>();
  unrecorded(() => {
    for (const [base, ext] of MARKER_ROOTS) {
      for (const path of walk(base, ext)) {
        for (const [name, label] of markersIn(path, read(path))) {
          if (!found.has(name)) found.set(name, []);
          found.get(name)!.push(label);
        }
      }
    }
  });
  return found;
}

/** Which cases under `CALL_ROOTS` call which member, as `callsIn` keys it. An instance member is
 * attributed by its `covers:` marker instead. Read `unrecorded`, as `scanMarkers` is. */
function scanCalls(): Map<string, string[]> {
  const out = new Map<string, string[]>();
  unrecorded(() => {
    for (const base of CALL_ROOTS) {
      for (const path of walk(base, ".nvst")) {
        for (const key of callsIn(read(path))) {
          if (!out.has(key)) out.set(key, []);
          out.get(key)!.push(path);
        }
      }
    }
  });
  return out;
}

export { LEDGER };

/** Every perf record, grouped by feature, oldest first. The ledger is read without recording the file.
 * With `keyed` the caller names `perfKey` for each feature whose rows it uses; without it this names
 * `LEDGER_WHOLE`, because the caller uses every row. */
export function ledgerRecords(keyed = false): Map<string, Record<string, unknown>[]> {
  if (!keyed) noteKey(LEDGER_WHOLE);
  return unrecorded(() => (existsSync(abs(LEDGER)) ? ledgerRows(read(LEDGER)) : new Map()));
}

const sha12 = (text: string) => createHash("sha1").update(text, "utf8").digest("hex").slice(0, 12);

/**
 * Modules many members read through, so a change to one changes what each of them does. A Rust
 * file that names one (`byte_source::`) is hashed with that module's `card` tier joined to its own,
 * and a change to the module stales every figure that file implements
 * (`rule:testing/member-perf-ledger`). The test is by text, so a file that names the module only in
 * its tests is staled too, which is the safe direction. An extension's member reads through the host's
 * `extension_source_range`, and its own files never name the module, so its figures are not joined.
 */
export const SHARED_IMPL: readonly { module: string; path: string }[] = [
  { module: "byte_source", path: "crates/nvs-stdlib/src/byte_source.rs" },
];

/** The shared modules `text` names, in `SHARED_IMPL`'s order; a module's own file names none. */
export function sharedIn(path: string, text: string): string[] {
  return SHARED_IMPL.filter((s) => s.path !== path && new RegExp(`\\b${s.module}::`).test(text)).map((s) => s.path);
}

/**
 * What a perf figure is current against: the implementing file's `card` tier, which is its tokens
 * without comments, layout, test code or reference cards (`keys/scan.ts`), joined to the tier of each
 * `SHARED_IMPL` module it reads through. A file that is not Rust, such as a reference chapter, is its
 * text with its line endings made `\n`. Empty when there is no such file.
 */
export function implHash(path: string): string {
  return implHashes([path]).get(path)!;
}

/** `implHash` of each path, with every Rust file among them read by one run of the scanner. */
export function implHashes(paths: string[]): Map<string, string> {
  const out = new Map<string, string>();
  const rust: string[] = [];
  for (const path of new Set(paths)) {
    let file = false;
    try {
      file = statSync(abs(path)).isFile();
    } catch {
      // A path with nothing behind it hashes to "".
    }
    if (!file) out.set(path, "");
    else if (path.endsWith(".rs")) rust.push(path);
    else out.set(path, digest(read(path)));
  }
  const texts = rust.map(read);
  const shared = texts.map((t, i) => sharedIn(rust[i]!, t));
  const extra = [...new Set(shared.flat())].filter((p) => !rust.includes(p) && existsSync(abs(p)));
  const cards = new Map<string, string>();
  analyseAll([...texts, ...extra.map(read)]).forEach((a, i) => cards.set(i < rust.length ? rust[i]! : extra[i - rust.length]!, a.card));
  rust.forEach((path, i) => {
    const own = cards.get(path)!;
    const joined = shared[i]!.filter((p) => cards.has(p));
    out.set(path, joined.length === 0 ? own : digest(own, ...joined.map((p) => cards.get(p)!)));
  });
  return out;
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

/** The tree-wide scans `collect` looks every entry up in. They note nothing, so one taking serves several
 * `collect` calls, each noting the keys of its own entries. */
export interface Scans {
  markers: Map<string, string[]>;
  calls: Map<string, string[]>;
}

export function scans(): Scans {
  return { markers: scanMarkers(), calls: scanCalls() };
}

/**
 * `collect` over each part in turn, a part being a label and its features, with what the roster and the
 * disk give each part noted under its label (`inPart`), so one process that runs several scopes records
 * what each of them read apart from the others. The tree-wide scans are taken once for all of them.
 */
export function collectParts(parts: Iterable<readonly [string, Entry[]]>): Map<string, Proofs> {
  const scanned = scans();
  const out = new Map<string, Proofs>();
  for (const [label, mine] of parts) {
    inPart([label], () => {
      noteRoster(mine, false);
      for (const [id, p] of collect(mine, scanned)) out.set(id, p);
    });
  }
  return out;
}

/** `collectParts` with one part per group of `groups`, holding that group's features of `scope`. */
export function collectGroups(scope: Entry[], groups: Iterable<string>): Map<string, Proofs> {
  return collectParts([...new Set(groups)].map((g) => [g, scope.filter((e) => e.group === g)] as const));
}

/** What every entry has on disk, by feature id. */
export function collect(entries: Entry[], scanned: Scans = scans()): Map<string, Proofs> {
  const { markers, calls } = scanned;
  const perf = ledgerRecords(true);
  const me = fingerprint().id;
  const hashes = implHashes(entries.map(implFile).filter((f) => f !== ""));
  const hashOf = (path: string) => hashes.get(path) ?? "";
  const out = new Map<string, Proofs>();
  for (const e of entries) {
    noteKey(coversKey(e.id));
    noteKey(anchorKey(e.id));
    const marked = markers.get(e.id) ?? [];
    const p: Proofs = {
      nvst: sortedSet(marked.filter((f) => f.endsWith(".nvst") || /\.nvs(::|$)/.test(f))),
      rust: sortedSet(marked.filter((f) => f.includes(".rs"))),
      examples: [],
      hostile: [],
      about: "",
      aboutProblem: "",
      details: "",
      intro: "",
      introNames: false,
      bench: "",
      perf: null,
      perfAny: null,
      inferred: 0,
      gaps: [],
    };
    if (e.kind === "member") {
      const [owner, member] = [e.id.slice(0, e.id.indexOf("::")), e.id.slice(e.id.indexOf("::") + 2)];
      const have = new Set(p.nvst);
      const call = `static:${owner.split("\\").pop()}::${member}`;
      noteKey(callsKey(call));
      const fresh = sortedSet((calls.get(call) ?? []).filter((f) => !have.has(f)));
      p.inferred = fresh.length;
      p.nvst = sortedSet([...p.nvst, ...fresh]);
    }
    const programs = (dir: string) => programsIn(dir).map((n) => `${dir}/${n}`);
    p.examples = programs(examplesDir(e)).sort();
    p.hostile = programs(hostileDir(e)).sort();
    if (existsSync(abs(aboutFile(e)))) {
      p.about = aboutFile(e);
      p.aboutProblem = aboutProblem(p.about);
    }
    if (existsSync(abs(detailsFile(e)))) p.details = detailsFile(e);
    if (e.kind === "member" && existsSync(abs(introFile(e.path)))) {
      p.intro = introFile(e.path);
      p.introNames = namesInCode(introCode(p.intro), e.id.slice(e.id.indexOf("::") + 2));
    }
    for (const dir of [examplesDir(e), hostileDir(e)]) {
      p.gaps.push(...programs(dir).sort(comparePaths).filter((f) => knownGap(read(f))));
    }
    if (existsSync(abs(benchFile(e)))) p.bench = benchFile(e);
    noteKey(perfKey(e.id));
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

/** Each of `named` that the feature `id`'s file under `data/proofs/skip/` skips, proof -> one sentence. It
 * is what `--gate --no-skip` adds to what the feature owes. */
export function skippedOf(id: string, skips: Skips, named: readonly string[]): Record<string, string> {
  const skip = skips.get(id) ?? {};
  return Object.fromEntries(named.filter((p) => p in skip).map((p) => [p, "skipped in data/proofs/skip/"]));
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
  if (want.intro && !("intro" in skip) && proofs.intro && !proofs.introNames) {
    const member = entry.id.slice(entry.id.indexOf("::") + 2);
    out.intro = `${proofs.intro} does not name \`${member}\` in code: say what it does where the introduction describes the class`;
  }
  if (want.comments && !("comments" in skip)) {
    // Not a proof of its own: it judges the programs the other proofs are made of, and `details.md`.
    const files = [...proofs.examples, ...proofs.hostile];
    if (proofs.bench) {
      const siblings = [".scale.nvs", ".twin.nvs"].map((s) => proofs.bench!.replace(/\.nvs$/, s));
      files.push(proofs.bench, ...siblings.filter((f) => existsSync(join(ROOT, f))));
    }
    if (proofs.details) files.push(proofs.details);
    const missed = files.map((f) => [f, textProblems(f)] as const).filter(([, problems]) => problems.length > 0);
    if (missed.length > 0) {
      const [first, problems] = missed[0]!;
      out.comments = `${missed.length} file(s) outside the plain-comment bounds, first ${first} line ${problems[0]}`;
    }
  }
  if (want.perf && !("perf" in skip)) {
    // A figure from any machine against this implementation text documents the feature.
    if (!proofs.bench) out.perf = `no bench at ${benchFile(entry)}`;
    else if (!proofs.perfAny) out.perf = implFile(entry) ? `stale: ${implFile(entry)} changed since it was last measured` : "never measured";
  }
  return out;
}
