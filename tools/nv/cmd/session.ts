// `bun nv session`: AGENTS.md § *Session workflow* steps 4 and 5, the end of a session.
//
//     bun nv session --template        a wrap file skeleton, with this tree's counts already in it
//     bun nv session --check           what steps 4 and 5 still owe, read off the tree
//     bun nv session --counts          conformance, differential and decision record counts
//     bun nv session --scrub           a commit message on stdin, written trailer-free on stdout
//     bun nv session --wrap F [--dry-run]   apply a wrap file: the whole session tail
//
// A wrap file is markdown whose `## <kind>: <arg>` headings are instructions, and `parseWrap` is its
// one reader. `--help` prints the format. `--wrap` runs `validate` over the whole file and refuses it
// with every problem at once, then hands a valid file to `tools/session.py --wrap`, which applies every
// section or none.
//
// `--check` judges no content. It prints the counts the plan's prose should agree with, each status
// field's size against its ceiling, the handoff's shape, the dead links this session made, the
// rulebook, record and migration gates for a session that edited those trees, and what is
// uncommitted. It fails on a stale count or a link this session broke. A link that was already dead
// where the session opened is printed and refuses nothing: it is CI's to report.

import { existsSync, readdirSync, readFileSync } from "node:fs";
import { join, posix, relative, sep } from "node:path";
import { goalValue } from "../import/goals.ts";
import { chainGoals, liveGoal } from "../lib/chain.ts";
import { ROOT } from "../lib/paths.ts";
import { passthrough, run as runProc } from "../lib/proc.ts";
import { ArgError, parseArgs, pyRepr } from "../lib/py.ts";
import { load } from "../lib/store.ts";
import { goal as goalType, sideGoal as sideGoalType } from "../schema/goal.ts";
import { playbookSection } from "../schema/playbook.ts";
import { DOC_EXTS, fileFindings, findingsIn, deadMentions, isGenerated, MENTION_EXTS, readText, type Resolver, SOURCE_EXTS, sortKey, trackedFiles } from "./links.ts";
import { ANCHOR_RE, type GoalValue, manifestFindings } from "./orient.ts";
import { type Entry, fieldLimits, milestones, planFields } from "./plan.ts";
import { CITATION, Rulebook } from "./rules.ts";
import { NUMBER_CITE, OWN_HEADER } from "./chain.ts";
import { anchors } from "../import/playbook.ts";

export const summary = "the session tail: nv session --template | --check | --counts | --scrub | --wrap F [--dry-run]";

const PLAN = "docs/implementation-plan.md";
const PLAYBOOK_DIR = "docs/agent/playbook";
/** Where the driver leaves the commit the running session opened on. Absent outside the loop. */
const SESSION_BASE = ".loop/session-start.json";
/** Set to a side goal's slug in every process of a side run; `tools/goals.py`'s `SIDE_ENV`. */
const SIDE_ENV = "NOVIS_SIDE_GOAL";
const HANDOFF_REQUIRED = ["## State", "## Next group", "## Backlog"];
const HANDOFF_TARGET_LINES = 60;
/** How far a `--- old` quote may be widened before it is given up on as un-quotable. */
const QUOTE_MAX = 200;

const HELP = `bun nv session: ${summary}

A wrap file is plain markdown. Every \`## \` heading is an instruction, and the text under it is
that instruction's payload. The order in the file does not matter: sections are applied as plan
fields, plan edits, milestones, playbook, handoff, commits in the order written, then the status.
Nothing is applied until every section validates.

    ## plan-edit: Open now
    --- old
    a run of words the field holds now, exactly once
    --- new
    what it says instead; an empty fragment drops the old one

    ## plan: On disk
    the whole field, for a real rewrite

    ## milestone: M4S
    the whole body of docs/plan/m4s.md below its heading

    ## playbook: Tooling
    - **A new trap, as a bullet.**

    ## handoff
    ## State
    ...the whole handoff, verbatim; its own \`## \` headings stay inside it...

    ## commit: path/one.rs path/two.rs
    feat(scope): the subject line

    ## status
    CONTINUE one line saying what landed

Inside \`## handoff\` only a known directive ends the body, so the handoff's \`## State\` is text.
A plan, plan-edit, milestone, playbook or commit section needs an argument; handoff and status
take none; every section needs a body. Attribution trailers in a commit body are stripped.`;

// ------------------------------------------------------------------------- the wrap file

/** One `## kind: arg` section of a wrap file, with its 1-based heading line. */
export interface Section {
  kind: string;
  arg: string;
  body: string;
  line: number;
}

const KNOWN = ["plan", "plan-edit", "milestone", "playbook", "handoff", "commit", "status"];
/** `## kind: arg`. The argument is `[^\n]*` so a CR left by a CRLF file lands in it and is trimmed. */
const DIRECTIVE = /^##\s+([A-Za-z][A-Za-z-]*)\s*:?\s*([^\n]*)$/;

/** The first `n` characters of `s`, counted as Python counts them. */
function head(s: string, n: number): string {
  return [...s].slice(0, n).join("");
}

/**
 * The `## kind: arg` headings and their bodies, in file order, and what is malformed. The handoff body
 * is markdown with its own `## ` headings, so inside `## handoff` only a known directive starts a new
 * section, and `## State` stays in the body.
 */
export function parseWrap(text: string): { sections: Section[]; errors: string[] } {
  const sections: Section[] = [];
  const errors: string[] = [];
  let cur: Section | null = null;
  let buf: string[] = [];
  const flush = () => {
    if (cur !== null) {
      cur.body = buf.join("\n").replace(/^\n+|\n+$/g, "");
      sections.push(cur);
    }
    cur = null;
    buf = [];
  };
  text.split("\n").forEach((raw, i) => {
    const m = DIRECTIVE.exec(raw);
    const kind = m ? m[1]!.toLowerCase() : "";
    if (m && KNOWN.includes(kind)) {
      if (cur?.kind === "handoff" && kind === "handoff") errors.push(`line ${i + 1}: a second \`## handoff\` section`);
      flush();
      cur = { kind, arg: m[2]!.trim(), body: "", line: i + 1 };
      return;
    }
    if (cur === null) {
      if (raw.trim()) errors.push(`line ${i + 1}: text before the first \`## \` directive: ${pyRepr(head(raw.trim(), 60))}`);
      return;
    }
    buf.push(raw);
  });
  flush();

  for (const s of sections) {
    if (["plan", "plan-edit", "milestone", "playbook", "commit"].includes(s.kind) && !s.arg) {
      errors.push(`line ${s.line}: \`## ${s.kind}:\` needs an argument`);
    }
    if ((s.kind === "handoff" || s.kind === "status") && s.arg) {
      errors.push(`line ${s.line}: \`## ${s.kind}\` takes no argument, got ${pyRepr(s.arg)}`);
    }
    if (!s.body.trim()) errors.push(`line ${s.line}: \`## ${s.kind}\` has an empty body`);
  }
  return { sections, errors };
}

/** A field's text as the plan stores it: one paragraph, single-spaced. */
export function normalize(text: string): string {
  return text.split(/\s+/).filter(Boolean).join(" ");
}

/** The `--- old` / `--- new` fragment pairs of a `## plan-edit:` body, and what is malformed. */
export function parseEdits(body: string): { pairs: [string, string][]; errors: string[] } {
  const pairs: [string, string][] = [];
  const errors: string[] = [];
  let slot: "old" | "new" | null = null;
  const buf = { old: [] as string[], new: [] as string[] };
  const flush = () => {
    if (buf.old.length || buf.new.length) pairs.push([normalize(buf.old.join("\n")), normalize(buf.new.join("\n"))]);
    buf.old = [];
    buf.new = [];
  };
  for (const raw of body.split("\n")) {
    const m = /^---\s*(old|new)\s*$/i.exec(raw.trim());
    if (m) {
      const next = m[1]!.toLowerCase() as "old" | "new";
      if (next === "old") flush();
      else if (buf.old.length === 0) errors.push("a `--- new` fragment with no `--- old` in front of it");
      slot = next;
      continue;
    }
    if (slot === null) {
      if (raw.trim()) errors.push(`text before the first \`--- old\`: ${pyRepr(head(raw.trim(), 60))}`);
      continue;
    }
    buf[slot].push(raw);
  }
  flush();
  if (pairs.length === 0) errors.push("no `--- old` / `--- new` fragment pair");
  for (const [old] of pairs) {
    if (!old) errors.push("an empty `--- old` fragment -- it must quote what is there now");
  }
  return { pairs, errors };
}

// ------------------------------------------------------------------------------ the tree

/** The side goal this process belongs to, or null in a chain run; `tools/goals.py`'s `side_goal`. */
function sideGoal(): string | null {
  const slug = (process.env[SIDE_ENV] ?? "").trim();
  if (!/^[a-z0-9]+(?:-[a-z0-9]+)*$/.test(slug)) return null;
  return existsSync(join(ROOT, "docs", "agent", "goals", "side", `${slug}.md`)) ? slug : null;
}

function handoffPath(side: string | null): string {
  return side ? `docs/agent/goals/side/${side}.handoff.md` : "docs/agent/handoff.md";
}

function countFiles(dir: string, ext: string): number {
  let n = 0;
  const walk = (d: string) => {
    for (const e of readdirSync(d, { withFileTypes: true })) {
      if (e.isDirectory()) walk(join(d, e.name));
      else if (e.name.endsWith(ext)) n++;
    }
  };
  if (existsSync(dir)) walk(dir);
  return n;
}

/** The counts the plan's prose should agree with, keyed the way `--template` names them. */
export function counts(): Record<"conformance" | "differential" | "adrs" | "highest_adr", number> {
  const adrs = readdirSync(join(ROOT, "docs", "decisions")).filter((f) => /^\d{4}\.md$/.test(f)).sort();
  return {
    conformance: countFiles(join(ROOT, "tests", "conformance"), ".nvst"),
    differential: countFiles(join(ROOT, "tests", "differential"), ".nvst"),
    adrs: adrs.length,
    highest_adr: adrs.length ? Number(adrs[adrs.length - 1]!.slice(0, 4)) : 0,
  };
}

/** How many times `sub` occurs in `s` without overlapping, as Python's `str.count`. */
function occurrences(s: string, sub: string): number {
  return s.split(sub).length - 1;
}

/** Widen `body[lo:hi]` a word at a time until it appears exactly once, or give up. */
function uniqueSpan(body: string, lo: number, hi: number): [number, number] | null {
  while (occurrences(body, body.slice(lo, hi)) !== 1) {
    if (hi - lo >= QUOTE_MAX) return null;
    const end = lo > 0 ? Math.max(lo - 1, 0) : 0;
    const left = lo > 0 && end > 0 ? body.lastIndexOf(" ", end - 1) : -1;
    const right = hi < body.length ? body.indexOf(" ", Math.min(hi + 1, body.length)) : -1;
    if (left === -1 && right === -1) return null;
    if (left !== -1) lo = left + 1;
    if (right !== -1) hi = right;
  }
  return [lo, hi];
}

/** `[field, old, new]` for every count in the plan that disagrees with the tree; an empty `old` could not be quoted. */
export function staleEdits(): [string, string, string][] {
  const c = counts();
  const out: [string, string, string][] = [];
  for (const [name, body] of planFields()) {
    for (const [label, value] of [["conformance", c.conformance], ["differential", c.differential]] as const) {
      for (const m of body.matchAll(new RegExp(`${label}\\D{0,12}(\\d{2,4})`, "gid"))) {
        if (Number(m[1]) === value) continue;
        const start = m.index!;
        const span = uniqueSpan(body, start, start + m[0].length);
        if (span === null) {
          out.push([name, "", `${label} ${m[1]} -> ${value}`]);
          continue;
        }
        const [lo, hi] = span;
        const old = body.slice(lo, hi);
        const at = m.indices![1]![0] - lo;
        out.push([name, old, old.slice(0, at) + String(value) + old.slice(at + m[1]!.length)]);
      }
    }
  }
  return out;
}

/** The `## ` headings a `## playbook:` section may name, in the playbook's order. */
function playbookHeadings(): string[] {
  return load(playbookSection)
    .sort((a, b) => a.value.order - b.value.order)
    .map((s) => s.value.title);
}

function nbytes(s: string): number {
  return Buffer.byteLength(s, "utf8");
}

// ----------------------------------------------------------------------------- the handoff

/** `[start, end)` line indices of a `## heading` and its body, matched on its words. */
function headingIndex(text: string, wanted: string): [number, number] | null {
  const norm = (t: string) => t.replace(/[`*_#]/g, "").replace(/\s+/g, " ").trim().toLowerCase();
  const lines = text.split("\n");
  const key = norm(wanted);
  const heads: [number, number, string][] = [];
  lines.forEach((ln, i) => {
    const m = /^(#{1,6})\s+(.*)$/.exec(ln);
    if (m) heads.push([i, m[1]!.length, m[2]!]);
  });
  for (let n = 0; n < heads.length; n++) {
    const [idx, level, title] = heads[n]!;
    if (norm(title) !== key) continue;
    const later = heads.slice(n + 1).find(([, l]) => l <= level);
    return [idx, later ? later[0] : lines.length];
  }
  return null;
}

/** The `- [ ]` items under `## Next group`, each with its continuation lines, numbered from 1. */
export function nextGroupItems(body: string): [number, string][] {
  const span = headingIndex(body, "Next group");
  if (span === null) return [];
  const items: string[][] = [];
  for (const ln of body.split("\n").slice(span[0] + 1, span[1])) {
    if (/^\s*- \[/.test(ln)) items.push([ln]);
    else if (items.length) items[items.length - 1]!.push(ln);
  }
  return items.map((blk, n) => [n + 1, blk.join("\n")]);
}

/** An anchor written without its directory -- `ctx.rs:2285` -- which the next pack cannot expand. */
const BARE_ANCHOR_RE = /(?<![\w/.-])([\w-]+\.(?:rs|py|md|toml)):(\d+)\b/g;

/** Every open item must carry a repo-rooted `file:NN` anchor, which is what `nv orient` inlines into the next pack. */
function validateAnchors(body: string): string[] {
  const errors: string[] = [];
  for (const [n, item] of nextGroupItems(body)) {
    if (/^\s*- \[[xX]\]/.test(item)) continue;
    const first = item.split("\n")[0]!.replace(/^\s*- \[.\]\s*/, "");
    const claim = pyRepr(head(first.replace(/[`*_]/g, "").replace(/\s+/g, " ").trim(), 60));
    for (const m of item.matchAll(BARE_ANCHOR_RE)) {
      errors.push(
        `\`## handoff\` -- \`## Next group\` item ${n} (${claim}) anchors \`${m[1]}:${m[2]}\` ` +
          `without its directory, which nv orient cannot expand. Write it repo-rooted, as \`crates/.../${m[1]}:${m[2]}\`.`,
      );
    }
    if (item.search(ANCHOR_RE) < 0) {
      errors.push(
        `\`## handoff\` -- \`## Next group\` item ${n} (${claim}) carries no repo-rooted ` +
          "`file:NN` anchor. They are not optional: nv orient inlines the code at each one " +
          "into the next session's pack, from the item and nowhere else. Resolve them with " +
          "`bun nv peek --locate <symbol> ...`.",
      );
    }
  }
  return errors;
}

export function validateHandoff(body: string): string[] {
  const errors: string[] = [];
  for (const required of HANDOFF_REQUIRED) {
    if (!new RegExp(`^${required}\\b`, "m").test(body)) errors.push(`\`## handoff\` -- missing the required \`${required}\` heading`);
  }
  return [...errors, ...validateAnchors(body)];
}

// ------------------------------------------------------------------------- the manifest gate

/** One copy of a goal's manifest: its value, the path its findings are named by, and its prose. */
export interface ManifestCopy {
  value: GoalValue;
  where: string;
  prose: string | null;
}

/**
 * What `manifestFindings` refuses in any copy of the live goal's manifest, each problem once. `nv chain
 * --check` is on every goal's floor and the driver runs the floor after the session is gone, so a
 * manifest broken at wrap time is a DONE claim held for a hand. The wrap refuses it first.
 */
export function manifestProblems(copies: ManifestCopy[]): string[] {
  const problems: string[] = [];
  for (const c of copies) {
    for (const p of manifestFindings(c.value, c.where, c.prose).problems) if (!problems.includes(p)) problems.push(p);
  }
  return problems.map((p) => `${p} -- \`nv chain --check\` is on the floor and halts a DONE claim on this; fix the manifest before the wrap, or drop the line`);
}

/**
 * The live goal's manifest copies: its stored record, and the `loop-goal.toml` the driver installs,
 * which a session edits in place so the two can disagree. A side goal's live copy is its own toml. A
 * retired goal, or none, has no manifest to gate.
 */
function manifestCopies(): ManifestCopy[] {
  const side = sideGoal();
  let slug: string, md: string, toml: string, stored: any;
  if (side) {
    slug = side;
    md = `docs/agent/goals/side/${side}.md`;
    toml = `docs/agent/goals/side/${side}.toml`;
    stored = load<any>(sideGoalType).find((g) => g.id === side);
  } else {
    const live = liveGoal(chainGoals());
    if (!live || live.retired) return [];
    slug = live.slug;
    md = live.md ?? "docs/agent/loop-goal.md";
    toml = "docs/agent/loop-goal.toml";
    stored = load<any>(goalType).find((g) => g.id === live.slug);
  }
  const copies: ManifestCopy[] = [];
  if (stored) copies.push({ value: stored.value as GoalValue, where: side ? `data/goals/side/${slug}.json` : `data/goals/${slug}.json`, prose: md });
  if (existsSync(join(ROOT, toml))) {
    const v = goalValue(ROOT, { slug, md, toml, handoff: null }, []);
    if (v) copies.push({ value: v as unknown as GoalValue, where: toml, prose: md });
  }
  return copies;
}

// ---------------------------------------------------------------------------- the link gate

async function git(...args: string[]): Promise<{ code: number; stdout: string }> {
  const r = await runProc(["git", ...args]);
  return { code: r.code, stdout: r.stdout };
}

/** The commit this session opened on, checked to be an ancestor of HEAD, or `HEAD` when nothing names one. */
async function sessionBase(): Promise<string> {
  let base: unknown;
  try {
    base = JSON.parse(readFileSync(join(ROOT, SESSION_BASE), "utf8"))?.base;
  } catch {
    return "HEAD";
  }
  if (typeof base !== "string" || !/^[0-9a-f]{7,40}$/.test(base)) return "HEAD";
  if ((await git("cat-file", "-e", `${base}^{commit}`)).code !== 0) return "HEAD";
  if ((await git("merge-base", "--is-ancestor", base, "HEAD")).code !== 0) return "HEAD";
  return base;
}

/** Every path `ref` holds, as git spells it; empty when there is no such commit. */
async function treePaths(ref: string): Promise<Set<string>> {
  const done = await git("ls-tree", "-r", "--name-only", ref);
  return done.code === 0 ? new Set(done.stdout.split("\n").filter(Boolean)) : new Set();
}

/** A resolver answering from a set of repo-relative paths, case-exact, instead of from disk. */
function inTree(paths: Set<string>): Resolver {
  return (base, target) => {
    const prefix = relative(ROOT, base).split(sep).join("/");
    const rel = posix.normalize(posix.join(prefix, target)).replace(/\/+$/, "") || ".";
    if (rel === ".." || rel.startsWith("../")) return null;
    if (paths.has(rel) || [...paths].some((p) => p.startsWith(`${rel}/`))) return null;
    return "missing";
  };
}

/** The tracked files the link gate reads, and the untracked ones this session created. */
async function scannedFiles(): Promise<string[]> {
  const files = await trackedFiles([]);
  const done = await git("ls-files", "--others", "--exclude-standard");
  if (done.code === 0) {
    const exts = [...DOC_EXTS, ...SOURCE_EXTS, ...MENTION_EXTS];
    for (const ln of done.stdout.split("\n")) {
      if (ln && exts.some((e) => ln.endsWith(e))) files.push(join(ROOT, ln));
    }
  }
  return files;
}

/** The link targets already dead in `rel` at `ref`, which are not this session's. */
async function inheritedLinks(rel: string, paths: Set<string>, ref: string): Promise<Set<string>> {
  const done = await runProc(["git", "show", `${ref}:${rel}`]);
  if (done.code !== 0) return new Set();
  const text = done.stdout.replace(/\r\n?/g, "\n");
  if (MENTION_EXTS.some((e) => rel.endsWith(e))) return new Set(deadMentions(text).map((f) => f.target));
  const source = SOURCE_EXTS.some((e) => rel.endsWith(e));
  const base = posix.dirname(`${ROOT.split(sep).join("/")}/${rel}`);
  return new Set(findingsIn(text, source, base, inTree(paths)).map((f) => f.target));
}

/** Dead links in the working tree, split into those this session broke and those it found already dead. */
async function linkFindings(ref: string): Promise<{ broke: string[]; found: string[] }> {
  const broke: string[] = [];
  const found: string[] = [];
  let paths: Set<string> | null = null;
  const files = (await scannedFiles()).map((f) => ({ f, key: sortKey(f) })).sort((a, b) => (a.key < b.key ? -1 : a.key > b.key ? 1 : 0));
  for (const { f } of files) {
    const text = readText(f);
    if (text === null || isGenerated(text)) continue;
    const hits = fileFindings(f, text);
    if (hits.length === 0) continue;
    const rel = relative(ROOT, f).split(sep).join("/");
    paths ??= await treePaths(ref);
    const old = await inheritedLinks(rel, paths, ref);
    for (const h of hits) (old.has(h.target) ? found : broke).push(`${rel}:${h.line} -> ${h.target} (${h.kind})`);
  }
  return { broke, found };
}

// ------------------------------------------------------------------------ the tree gates

interface Gate {
  args: string[];
  why: string;
}

const RULEBOOK_GATES: Gate[] = [
  {
    args: ["rules", "--check"],
    why:
      "a rule does not load, or a `rule:` citation somewhere in the tree names one that is not there. " +
      "A renamed or deleted rule breaks its citations in files this session never opened.",
  },
  {
    args: ["rules", "--render", "--check"],
    why:
      "a generated page no longer matches the fragments it is rendered from. `bun nv rules --render` " +
      "writes them, and editing a fragment without re-running it commits the old rendering beside the new rule.",
  },
];
const RECORD_GATES: Gate[] = [
  {
    args: ["records", "--check"],
    why:
      "a decision record's field set, heading order, cross-links or derived counters are wrong -- most often " +
      "a `changes:` block with no `modifies:` list. `nv verify` does not run this one either.",
  },
];
const MIGRATION_GATES: Gate[] = [
  {
    args: ["migration"],
    why:
      "the migration table has a structural error -- a row for a name PHP does not have, an outcome outside " +
      "the vocabulary, or a duplicate. The tool's own output is the whole message.",
  },
];

/**
 * What the gates refuse over the whole tree, for a session that edited `tree`. A finding here is a property
 * of the whole set with nothing per file to compare against the session's base, so the trigger is the
 * session's own edit: a tree already red under a session that never opened it is CI's to report.
 */
async function treeGate(tree: string, gates: Gate[]): Promise<string[]> {
  const done = await git("status", "--porcelain", "--", tree);
  if (done.code !== 0 || !done.stdout.trim()) return [];
  const out: string[] = [];
  for (const { args, why } of gates) {
    const ran = await runProc([process.execPath, join(ROOT, "tools", "nv", "main.ts"), ...args]);
    if (ran.code === 0) continue;
    const detail = (ran.stdout + ran.stderr).trim().split("\n");
    const more = detail.length > 12 ? `\n      ... and ${detail.length - 12} more line(s)` : "";
    out.push(`\`bun nv ${args.join(" ")}\` fails, and this session edited \`${tree}/\`: ${why}\n      ${detail.slice(0, 12).join("\n      ")}${more}`);
  }
  return out;
}

async function touched(tree: string): Promise<boolean> {
  const done = await git("status", "--porcelain", "--", tree);
  return done.code === 0 && done.stdout.trim() !== "";
}

// ------------------------------------------------------------------------------- validate

const SUBJECT_MAX = 120;
const STATUS_WORDS = ["CONTINUE", "DONE", "BLOCKED"];
/** A bullet is the trap, why, and what to do instead: conventions.md § *A playbook bullet*. */
const PLAYBOOK_BULLET_MAX = 700;
/**
 * A `## plan:` body over `RETYPE_BYTES` whose 8-word runs are at least `RETYPE_OVERLAP` already in the
 * field, and which is not a cut below `RETYPE_SHRINK` of it, is the field typed out again: `## plan-edit:`
 * sends only the sentence that moved.
 */
const RETYPE_BYTES = 1_500;
const RETYPE_OVERLAP = 0.7;
const RETYPE_SHRINK = 0.6;
const EXPIRY = /\[until:\s*(test|exists|gone|rule|reviewed)\s+([^\]]+?)\s*\]\s*$/;

const LINK_WHY: Record<string, string> = {
  missing: "nothing is there",
  case: "the entry on disk is spelled with different case, which resolves on this machine and 404s on every Linux checkout",
  absolute: "a markdown file's links are relative to itself, and `/docs/...` is the site root",
  relative: "a source file's links are absolute from the repository root (`/docs/...`)",
};

/** The wrap's sections in the order they are applied, which is the order they are validated in. */
function inOrder(sections: Section[]): Section[] {
  return KNOWN.flatMap((kind) => sections.filter((s) => s.kind === kind));
}

function named(s: Section): string {
  return s.arg ? `\`## ${s.kind}: ${s.arg}\`` : `\`## ${s.kind}\``;
}

/** The fraction of `next`'s 8-word runs that appear verbatim in `old`. */
function verbatimOverlap(next: string, old: string): number {
  const grams = (t: string) => {
    const w = t.split(/\s+/).filter(Boolean);
    const out = new Set<string>();
    for (let i = 0; i + 8 <= w.length; i++) out.add(w.slice(i, i + 8).join("\0"));
    return out;
  };
  const gn = grams(next);
  if (gn.size === 0) return 0;
  const go = grams(old);
  return [...gn].filter((g) => go.has(g)).length / gn.size;
}

const thousands = (n: number) => n.toLocaleString("en-US");

/** A refusal only when the edit leaves the field both over the ceiling and bigger than it was. */
function growthRefusal(kind: string, field: string, before: string, after: string, ceiling: number): string | null {
  const was = nbytes(before);
  const now = nbytes(after);
  if (now <= was || now <= ceiling) return null;
  const cut = now - Math.max(ceiling, was);
  const how = kind === "plan-edit" ? "a `--- old` quoting the stale sentence with an empty `--- new` drops it" : "send the replacement shorter";
  return (
    `\`## ${kind}: ${field}\` -- leaves the field at ${thousands(now)} B, +${now - was} B ` +
    `and past its ${thousands(ceiling)} B ceiling. A field is status and is overwritten, not appended ` +
    `to. Cut at least ${cut} B of it in this same section (${how}), or take the new text ` +
    "where it belongs -- a per-file gap to that crate's module doc `# Known gaps`, a trap to " +
    "`## playbook:`, what landed to the commit body. An edit that leaves the field no bigger " +
    "than it is now is always taken."
  );
}

function resolveMilestone(arg: string): Entry | null {
  const want = arg.trim().toUpperCase();
  return milestones().find((m) => m.id === want) ?? null;
}

/** The directory of the playbook section a `## playbook: <heading>` names, matched on its words. */
function sectionDir(heading: string): string | null {
  const norm = (t: string) => t.replace(/[`*_#]/g, "").replace(/\s+/g, " ").trim().toLowerCase();
  const hit = load(playbookSection).find((s) => norm(s.value.title) === norm(heading));
  return hit ? `${PLAYBOOK_DIR}/${hit.id}` : null;
}

/** Every `- ` bullet and `NNN. ` entry at column 0 of a playbook body, as `tools/playbook.py`'s `blocks` reads them. */
function blocks(text: string): { body: string; lead: string }[] {
  const lines = text.split("\n");
  const spans: { first: string; start: number; end: number }[] = [];
  let cur: { first: string; start: number; end: number } | null = null;
  const starts = /^(- |\d+\. )/;
  lines.forEach((line, i) => {
    if (/^#{1,6}\s/.test(line)) {
      if (cur) spans.push(cur);
      cur = null;
    } else if (starts.test(line)) {
      if (cur) spans.push(cur);
      cur = { first: line, start: i, end: i };
    } else if (cur && !line.trim()) {
      const nxt = lines[i + 1] ?? "";
      if (nxt && !nxt.startsWith(" ") && !nxt.startsWith("\t") && !starts.test(nxt)) {
        spans.push(cur);
        cur = null;
      }
    } else if (cur) {
      cur.end = i;
    }
  });
  if (cur) spans.push(cur);
  return spans.map((b) => ({
    body: lines.slice(b.start, b.end + 1).join("\n").trimEnd(),
    lead: head(b.first.replace(/^(- (?:\[.\] )?|\d+\. )/, ""), 70),
  }));
}

/** The `[until: kind arg]` a bullet ends with, or null when it declares nothing or wraps its trailer over a line. */
function declaration(body: string): { kind: string; arg: string } | null {
  const m = EXPIRY.exec(body.trimEnd());
  return m && !m[2]!.includes("\n") ? { kind: m[1]!, arg: m[2]!.trim() } : null;
}

/** Why a new bullet may not end with `[until: reviewed <date>]`: nothing retires that kind, so it is for a trap no mechanical kind fits. */
function reviewedRefusals(which: string, body: string, until: { kind: string; arg: string }): string[] {
  if (until.kind !== "reviewed") return [];
  const out: string[] = [];
  const day = /^\d{4}-\d{2}-\d{2}$/.test(until.arg) ? new Date(`${until.arg}T00:00:00`) : null;
  if (day === null || Number.isNaN(day.getTime())) {
    out.push(`${which} -- \`${until.arg}\` is not a YYYY-MM-DD date.`);
  } else {
    const today = new Date();
    today.setHours(0, 0, 0, 0);
    if (day > today) out.push(`${which} is dated ${until.arg}, which is after today. A \`reviewed\` date is the day the bullet was read: write today's date.`);
  }
  const files = anchors(ROOT, body, until);
  if (files.length) {
    out.push(
      `${which} names \`${files[0]}\` and ends with \`[until: reviewed ...]\`. A trap about a path ` +
        `ends when that path changes, so declare that instead: \`[until: gone ${files[0]}:<a ` +
        "word the trap depends on>]`, `[until: exists <path>]` for a fix that is not there " +
        "yet, or `[until: test <fn>]` for a hole a test will close.",
    );
  }
  return out;
}

/** What a `## playbook:` section refuses, bullet by bullet. A lead-in that collides with another bullet's is `tools/session.py --wrap`'s to refuse, which the wrap still hands off to. */
function validatePlaybook(s: Section): string[] {
  const errors: string[] = [];
  if (sectionDir(s.arg) === null) errors.push(`\`## playbook: ${s.arg}\` -- no such section. The playbook has: ${playbookHeadings().join(", ")}`);
  if (!s.body.trimStart().startsWith("-")) {
    errors.push(`\`## playbook: ${s.arg}\` -- a playbook entry is a \`- \` bullet`);
    return errors;
  }
  const found = blocks(s.body);
  if (found.length === 0) errors.push(`\`## playbook: ${s.arg}\` -- a bullet starts with \`- \` at column 0; this one is indented, and the file's own parsers skip it`);
  for (const b of found) {
    const which = `\`## playbook: ${s.arg}\` -- ${pyRepr(b.lead.trim())}`;
    const m = EXPIRY.exec(b.body.trimEnd());
    const until = declaration(b.body);
    if (m && m[2]!.includes("\n")) {
      errors.push(
        `${which} ends with a trailer broken across two lines. Its argument would hold a newline no path, ` +
          "needle, name or date can, so nothing ever retires the bullet. Keep the whole `[until: ...]` on one line.",
      );
    } else if (until === null) {
      errors.push(`${which} declares nothing that retires it. End it with \`[until: <kind> <arg>]\`; the five kinds are in tools/playbook.py's module doc.`);
    } else {
      errors.push(...reviewedRefusals(which, b.body, until));
    }
    if (anchors(ROOT, b.body, until).length === 0) {
      errors.push(
        `${which} names no file in the tree. A trap is about a file: name it in backticks, as its path ` +
          "from the repository root (`tools/session.py`, `crates/nvs-ir/src/lib.rs`, `Cargo.toml`). A rule " +
          "every agent needs whatever it edits is not a trap: it belongs in docs/agent/commands.md or docs/agent/conventions.md.",
      );
    }
    const weight = nbytes(b.body.trim());
    if (weight > PLAYBOOK_BULLET_MAX) {
      errors.push(
        `${which} is ${weight} B, past the ${PLAYBOOK_BULLET_MAX} B a bullet may weigh. A bullet is the trap, ` +
          "why, and what to do instead -- three sentences, docs/agent/conventions.md § *A playbook bullet*. " +
          "The session's story (which stage, what was tried first) is git log's.",
      );
    }
  }
  return errors;
}

function validateCommit(s: Section): string[] {
  const errors: string[] = [];
  const subject = s.body.trim().split("\n")[0]!;
  if (!/^(feat|fix|docs|test|perf|refactor|chore|build|ci)(\([a-z0-9-]+\))?: .+/.test(subject)) {
    errors.push(`\`## commit:\` line ${s.line} -- subject is not \`type(scope): subject\`: ${pyRepr(head(subject, 60))}`);
  }
  const len = [...subject].length;
  if (len > SUBJECT_MAX) {
    errors.push(`\`## commit:\` line ${s.line} -- subject is ${len} chars, ${len - SUBJECT_MAX} over the ${SUBJECT_MAX} limit: ${pyRepr(subject)}`);
  }
  for (const path of s.arg.split(/\s+/).filter(Boolean)) {
    if (!existsSync(join(ROOT, path)) && !path.includes("*")) errors.push(`\`## commit:\` line ${s.line} -- no such path: ${path}`);
  }
  return errors;
}

/** Where a section's body lands, which is where its links resolve from; null for a body that is not a file. */
function bodyHome(s: Section): string | null {
  if (s.kind === "handoff") return handoffPath(sideGoal());
  if (s.kind === "plan" || s.kind === "plan-edit") return PLAN;
  if (s.kind === "playbook") {
    const first = load(playbookSection).sort((a, b) => a.value.order - b.value.order)[0];
    return first ? `${PLAYBOOK_DIR}/${first.id}/bullet.md` : null;
  }
  if (s.kind === "milestone") return resolveMilestone(s.arg)?.rel ?? null;
  return null;
}

/** The text a section puts into the tree: a `## plan-edit:` quotes the field in its `--- old` halves, so only the `--- new` halves count. */
function writtenText(s: Section): string {
  return s.kind === "plan-edit" ? parseEdits(s.body).pairs.map(([, next]) => next).join("\n") : s.body;
}

/** The tracked files this wrap's doc sections write. A playbook bullet is named by its section's directory. */
function writtenPaths(sections: Section[]): string[] {
  const out: string[] = [];
  for (const s of inOrder(sections)) {
    if (s.kind === "plan" || s.kind === "plan-edit") out.push(PLAN);
    else if (s.kind === "milestone") {
      const entry = resolveMilestone(s.arg);
      if (entry) out.push(entry.rel);
    } else if (s.kind === "playbook") {
      const dir = sectionDir(s.arg);
      if (dir) out.push(dir);
    } else if (s.kind === "handoff") out.push(handoffPath(sideGoal()));
  }
  return [...new Set(out)];
}

/** Dead links in the bodies this wrap is about to write, which it commits in the same call. */
function bodyLinks(sections: Section[]): string[] {
  const out: string[] = [];
  for (const s of sections) {
    const home = bodyHome(s);
    if (home === null) continue;
    const base = posix.dirname(`${ROOT.split(sep).join("/")}/${home}`);
    for (const f of findingsIn(writtenText(s), false, base)) {
      out.push(
        `${named(s)} cites ${pyRepr(f.target)}, and ${LINK_WHY[f.kind]}. A link in a wrap body resolves ` +
          `from ${home}, which is where the body lands -- and this wrap writes and ` +
          "commits in one call, so nothing reads it before CI's `docs` job does.",
      );
    }
  }
  return out;
}

/** `rule:` tokens in the bodies this wrap is about to write that name no rule. */
function bodyCitations(sections: Section[]): string[] {
  const book = new Rulebook();
  if (book.byId.size === 0) return [];
  const out: string[] = [];
  for (const s of sections) {
    if (!["handoff", "plan", "plan-edit", "playbook", "milestone"].includes(s.kind)) continue;
    writtenText(s).split("\n").forEach((line, i) => {
      for (const m of line.matchAll(CITATION)) {
        if (book.byId.has(m[1]!)) continue;
        out.push(
          `${named(s)} line ${i + 1} cites \`rule:${m[1]}\`, and no rule has that id. \`bun nv rules --check\` ` +
            "resolves every `rule:` token under `docs/`, the handoff and the playbook included, and the goal's " +
            "`1 floor` runs it -- so a placeholder id written here turns that check red for every later session. " +
            "Cite a real rule (`bun nv brief --where <keyword>` finds one), or describe the token in words.",
        );
      }
    });
  }
  return out;
}

/** A goal named by its number in a body this wrap is about to write, a commit message included. */
function bodyGoalNumbers(sections: Section[]): string[] {
  const out: string[] = [];
  for (const s of sections) {
    if (!["handoff", "plan", "plan-edit", "playbook", "milestone", "commit"].includes(s.kind)) continue;
    writtenText(s).split("\n").forEach((line, i) => {
      if (OWN_HEADER.test(line)) return;
      for (const m of line.matchAll(NUMBER_CITE)) {
        out.push(
          `${named(s)} line ${i + 1} writes \`${m[0]}\`, naming a goal by its number. Say ` +
            "the slug -- goal `parses`, never `goal 21` -- because a number is a position " +
            "and every insert in front of it moves the position without touching the " +
            "sentence. `bun nv chain --check` refuses this over the whole tree and " +
            "is on the goal's `1 floor`, so a number written here turns that check red for " +
            "every later session. A position beside a total (`29 of 43`) is not this.",
        );
      }
    });
  }
  return out;
}

/** Every test a `cargo-named` check of the live goal names that no `fn` in the tree carries; a DONE claim only. */
async function namedTestFindings(): Promise<string[]> {
  const done = await git("grep", "-h", "-o", "-E", "\\bfn [A-Za-z_][A-Za-z0-9_]*", "--", "*.rs");
  const fns = [...new Set(done.stdout.split("\n").map((l) => l.trim().slice(3)).filter(Boolean))];
  const missing: [string, string][] = [];
  for (const c of manifestCopies()) {
    for (const check of ((c.value as any).checks ?? []) as { kind: string; name?: string; tests?: string[] }[]) {
      if (check.kind !== "cargo-named") continue;
      const label = check.name ?? "?";
      for (const name of check.tests ?? []) {
        if (missing.some(([l, n]) => l === label && n === name)) continue;
        if (!fns.some((fn) => fn.includes(name))) missing.push([label, name]);
      }
    }
  }
  return missing.map(
    ([label, name]) =>
      `\`## status\` DONE -- check ${pyRepr(label)} names test \`${name}\`, and no \`fn ${name}\` is in ` +
      "the tree; the driver's sweep runs it, matches nothing and halts the claim. Rename the " +
      "test to what the toml names, or write it",
  );
}

/**
 * Everything that could refuse the wrap, before a byte is written. The plan sections are checked against
 * a simulated field, advanced in apply order, so a second `## plan-edit:` on one field quotes the text the
 * first leaves behind, which is the text its author was looking at.
 */
export async function validate(sections: Section[]): Promise<string[]> {
  const errors: string[] = [];
  const fields = planFields();
  const names = new Map(fields.map(([n]) => [n.toLowerCase(), n]));
  const working = new Map(fields.map(([n, t]) => [n.toLowerCase(), t]));
  const { ceiling } = fieldLimits();
  const side = sideGoal();
  const noField = (s: Section) => `\`## ${s.kind}: ${s.arg}\` -- no such field, and this tool does not add one. The block has: ${[...names.values()].join(", ")}`;

  for (const s of inOrder(sections)) {
    if (side && ["plan", "plan-edit", "milestone"].includes(s.kind)) {
      // The plan's status block and the milestone files are the chain run's state, and a side
      // branch that rewrote them would overwrite main's when it lands.
      errors.push(`\`## ${s.kind}: ${s.arg}\` -- a side run does not write the plan; say what it changed in the handoff, and the landing commit carries it`);
      continue;
    }
    const key = s.arg.toLowerCase();
    if (s.kind === "plan") {
      if (!names.has(key)) {
        errors.push(noField(s));
        continue;
      }
      const next = normalize(s.body);
      const old = working.get(key)!;
      const share = verbatimOverlap(next, old);
      const len = [...next].length;
      if (len > RETYPE_BYTES && share >= RETYPE_OVERLAP && len > [...old].length * RETYPE_SHRINK) {
        errors.push(
          `\`## plan: ${s.arg}\` -- ${thousands(len)} B, and ${Math.round(share * 100)}% of it is already ` +
            "on disk word for word. That is a retype, not a rewrite: use " +
            `\`## plan-edit: ${s.arg}\` with \`--- old\` / \`--- new\` fragments and send only ` +
            `the sentence that moved. (A real rewrite overlaps less than ${Math.round(RETYPE_OVERLAP * 100)}% and is taken as it stands.)`,
        );
        continue;
      }
      const grown = growthRefusal("plan", s.arg, old, next, ceiling);
      if (grown) {
        errors.push(grown);
        continue;
      }
      working.set(key, next);
    } else if (s.kind === "plan-edit") {
      if (!names.has(key)) {
        errors.push(noField(s));
        continue;
      }
      const { pairs, errors: bad } = parseEdits(s.body);
      errors.push(...bad.map((b) => `\`## plan-edit: ${s.arg}\` -- ${b}`));
      let text = working.get(key)!;
      for (const [old, next] of pairs) {
        if (!old) continue;
        const hits = occurrences(text, old);
        if (hits !== 1) {
          const where = hits === 0 ? "is not in that field" : `appears ${hits} times in it`;
          errors.push(
            `\`## plan-edit: ${s.arg}\` -- the \`--- old\` fragment ${where}, so nothing ` +
              "was changed. Quote a longer run, exactly as the field reads (it is one " +
              `paragraph, single-spaced; \`bun nv plan --get ${pyRepr(s.arg)}\` prints ` +
              `it): ${pyRepr(head(old, 70))}`,
          );
          continue;
        }
        text = text.replace(old, () => next);
      }
      const grown = growthRefusal("plan-edit", s.arg, working.get(key)!, text, ceiling);
      if (grown) {
        errors.push(grown);
        continue;
      }
      working.set(key, text);
    } else if (s.kind === "milestone") {
      const entry = resolveMilestone(s.arg);
      if (entry === null) {
        errors.push(`\`## milestone: ${s.arg}\` -- the plan's table lists no such milestone, and this tool does not add one. It has: ${milestones().map((m) => m.id).join(", ")}`);
      } else if (!existsSync(join(ROOT, entry.rel))) {
        errors.push(`\`## milestone: ${s.arg}\` -- ${entry.rel} does not exist`);
      }
    } else if (s.kind === "playbook") {
      errors.push(...validatePlaybook(s));
    } else if (s.kind === "status") {
      const lines = s.body.trim().split("\n");
      const first = lines[0]!;
      if (!STATUS_WORDS.some((w) => first.startsWith(w))) errors.push(`\`## status\` -- must start with one of ${STATUS_WORDS.join("/")}, got ${pyRepr(head(first, 40))}`);
      if (lines.length > 1) errors.push("`## status` -- one line only");
      if (first.startsWith("DONE")) errors.push(...(await namedTestFindings()));
    } else if (s.kind === "commit") {
      errors.push(...validateCommit(s));
    } else if (s.kind === "handoff") {
      errors.push(...validateHandoff(s.body));
    }
  }

  // A wrap that writes the docs and commits nothing leaves step 5 owing the files it just changed, and
  // inventing a subject for them would be this tool judging content, so it refuses and names them.
  const writes = writtenPaths(sections);
  if (writes.length && !sections.some((s) => s.kind === "commit")) {
    errors.push(
      `this wrap writes ${writes.join(", ")} and has no \`## commit:\` section, so step 5 ` +
        `would end with ${writes.length > 1 ? "them" : "it"} dirty. Add ` +
        `\`## commit: ${writes.join(" ")}\` -- every doc section is applied before any commit ` +
        "is staged, so one wrap does both.",
    );
  }

  errors.push(...bodyLinks(sections), ...bodyCitations(sections), ...bodyGoalNumbers(sections));
  const { broke } = await linkFindings(await sessionBase());
  if (broke.length) {
    const more = broke.length > 8 ? `; and ${broke.length - 8} more -- \`bun nv links\` lists them all` : "";
    errors.push(
      `${broke.length} link(s) resolved at HEAD and do not resolve now, so they are this ` +
        `session's: ${broke.slice(0, 8).join("; ")}${more}. Most often that is a file renamed under the citations of ` +
        "it, which is how the four this gate was added for got there. Nothing else catches it " +
        "before the push -- `nv links` is CI's `docs` job, and `bun nv verify` does not " +
        "run it, so a green verify says nothing here. " +
        "A link already dead at the commit this session opened on is not counted: that one is " +
        "not yours. A slice you committed by hand earlier in this session is still yours.",
    );
  }
  errors.push(
    ...(await treeGate("docs/rules", RULEBOOK_GATES)),
    ...(await treeGate("docs/decisions", RECORD_GATES)),
    ...(await treeGate("docs/spec", MIGRATION_GATES)),
    ...manifestProblems(manifestCopies()),
  );
  return errors;
}

// ---------------------------------------------------------------------------- the modes

async function check(): Promise<number> {
  const say = (line = "") => console.log(line);
  const c = counts();
  say("== COUNTS  (what the plan's prose should agree with)");
  say(`  conformance cases   ${c.conformance}`);
  say(`  differential cases  ${c.differential}`);
  say(`  ADRs on disk        ${c.adrs}, highest ${String(c.highest_adr).padStart(4, "0")}, next free ${String(c.highest_adr + 1).padStart(4, "0")}`);

  say();
  const { aim, ceiling } = fieldLimits();
  say(`== PLAN  (fields, any that name a stale count, and what each costs; aim ~${aim} B, ceiling ${ceiling} B)`);
  const edits = staleEdits();
  let total = 0;
  for (const [name, body] of planFields()) {
    const mine = edits.filter((e) => e[0] === name);
    let flag = mine.length ? `   <- ${mine.length} stale count(s); \`--template\` hands them back ready to apply` : "";
    const n = nbytes(body);
    total += n;
    if (n > ceiling) flag += "   OVER the ceiling: an edit that grows it is refused -- replace, do not add";
    else if (!flag && n > aim * 1.5) flag = `   ${Math.round(n / aim)}x the aim, ${ceiling - n} B of headroom`;
    say(`  ${name.padEnd(18)} ${String(n).padStart(5)} B${flag}`);
  }
  say(`  ${"".padEnd(18)} ${String(total).padStart(5)} B  shipped into every session. \`--wrap\` refuses an edit that`);
  say("                          leaves a field both over the ceiling and bigger than it was;");
  say("                          a shrink is always taken. `nv plan --check` is the same number.");

  say();
  say("== HANDOFF");
  const handoff = join(ROOT, handoffPath(sideGoal()));
  if (!existsSync(handoff)) {
    say("  MISSING");
  } else {
    const body = readFileSync(handoff, "utf8").replace(/\r\n?/g, "\n");
    const n = body.split("\n").length;
    const problems = validateHandoff(body);
    say(`  ${n} lines${n > 85 ? ` (target ~${HANDOFF_TARGET_LINES})` : ""}`);
    for (const p of problems) say(`  - ${p}`);
    if (problems.length === 0) say("  shape OK: State / Next group / Backlog, every open item with a repo-rooted file:NN anchor");
  }

  say();
  say("== LINKS  (bun nv links -- CI's `docs` job, which nv verify does not run)");
  const base = await sessionBase();
  const { broke, found } = await linkFindings(base);
  const where = base === "HEAD" ? "HEAD" : `${base.slice(0, 9)}, where this session opened`;
  for (const ln of broke) say(`  YOURS  ${ln}`);
  for (const ln of found.slice(0, 8)) say(`  inherited  ${ln}`);
  if (found.length > 8) say(`  ... and ${found.length - 8} more that were already dead at ${where}`);
  if (!broke.length && !found.length) say("  every link resolves, with matching case and form");
  else if (!broke.length) say(`  none of the ${found.length} is this session's -- all were dead at ${where} -- so \`--wrap\` will not refuse over them`);
  else say("  `--wrap` refuses while a YOURS line stands: fix the link, not the citation's line");

  const gates: [string, string, string, Gate[], string][] = [
    ["RULEBOOK", "bun nv rules -- the same job, and only when you edited docs/rules/", "docs/rules", RULEBOOK_GATES, "every rule loads, every citation resolves, and the rendered pages are current"],
    ["RECORDS", "bun nv records -- CI's `docs` job, and only when you edited them", "docs/decisions", RECORD_GATES, "every record's field set, heading order and derived counters are right"],
    ["MIGRATION", "bun nv migration -- only when you edited docs/spec/", "docs/spec", MIGRATION_GATES, "every row of the migration table is well formed"],
  ];
  for (const [title, about, tree, list, clean] of gates) {
    say();
    say(`== ${title}  (${about})`);
    const problems = await treeGate(tree, list);
    if (problems.length === 0) say((await touched(tree)) ? `  ${clean}` : "  clean -- this wrap is not gated on it");
    for (const p of problems) say(`  YOURS  ${p}`);
  }

  say();
  say("== TREE");
  const st = (await git("status", "--short")).stdout.trimEnd();
  if (!st) {
    say("  clean -- nothing to commit");
  } else {
    const lines = st.split("\n");
    for (const ln of lines.slice(0, 20)) say(`  ${ln}`);
    if (lines.length > 20) say(`  ... and ${lines.length - 20} more`);
  }
  say();
  say("Write one wrap file and apply it with `bun nv session --wrap <file>`;");
  say("its format is this tool's --help.");
  return edits.length || broke.length ? 1 : 0;
}

/**
 * A wrap file skeleton for this tree. Every count the tree has moved past is already an applicable
 * `## plan-edit:`, and otherwise the skeleton quotes a real run of words out of `Open now`, so the
 * unedited skeleton still validates except for its placeholder commit paths.
 */
function template(): number {
  const out: string[] = [];
  const say = (line = "") => out.push(line);
  const side = sideGoal();
  const c = counts();
  const edits = side ? [] : staleEdits();
  if (side) {
    // A side run writes no plan section, and the wrap refuses one.
  } else if (edits.length) {
    // Nothing but the pairs goes here: `parseEdits` refuses text in front of a `--- old` and folds text
    // after a `--- new` into the fragment, so a note would corrupt the edit it explained.
    for (const field of new Set(edits.map(([name]) => name))) {
      say(`## plan-edit: ${field}`);
      for (const [, old, next] of edits.filter((e) => e[0] === field)) {
        say("--- old");
        say(old || `QUOTE THE SENTENCE HOLDING ${next} -- widening past ${QUOTE_MAX} chars`);
        say("--- new");
        say(next);
      }
      say("");
    }
  } else {
    const field = planFields().find(([name]) => name === "Open now")?.[1] ?? "";
    const sample = field.split(/\s+/).filter(Boolean).slice(0, 9).join(" ") || "the run of words that is now wrong";
    say("## plan-edit: Open now");
    say("--- old");
    say(sample);
    say("--- new");
    say(`${sample}   <- REPLACE both fragments. Repeat the pair per place the field moved.`);
    say(`The tree's counts are ${Object.entries(c).map(([k, v]) => `${k} ${v}`).join(", ")} and no plan field`);
    say("names a stale one. `## plan: <Field>` replaces a field WHOLE instead, for a real");
    say("rewrite -- a big replacement mostly already on disk is refused as a retype.");
    say(`Open now is ${nbytes(field)} B of its ${fieldLimits().ceiling} B ceiling: an edit that`);
    say("leaves it both bigger than now AND over that is refused, so replace a sentence");
    say("rather than adding one -- an empty `--- new` drops the one that went stale.");
  }
  say("");
  say("## playbook: Tooling");
  say("- **DELETE THIS SECTION unless a trap cost you time.** A bullet is appended under the");
  say("  heading, never rewritten, so only add one that is not already there -- three sentences,");
  say("  trap / why / what to do (conventions.md § A playbook bullet), not the session's story,");
  say("  which git log holds. The headings are");
  for (const heading of playbookHeadings()) say(`  ${heading}`);
  say("");
  say("## handoff");
  say("## State");
  say("REPLACE. Where the work stands now -- not the path taken to get here.");
  say("");
  say("## Next group");
  say("- [ ] **The claim** -- anchored, as `crates/nvs-stdlib/src/arr.rs:2084`, so the next");
  say("      session does not re-derive what this one already had open.");
  say("");
  say("## Backlog");
  say("- What this session did not take.");
  say("");
  say("## commit: path/one.rs path/two.rs");
  say("test(stdlib): what is now true, lower case, no trailing period");
  say("");
  say("The body. No trailers of any kind -- they are stripped and counted.");
  say("");
  // The docs this wrap writes, committed by this wrap, pre-filled so no session hand-rolls a `git add` of them.
  const docs = side ? [PLAYBOOK_DIR, handoffPath(side)] : [PLAN, PLAYBOOK_DIR, handoffPath(side)];
  say(`## commit: ${docs.join(" ")}`);
  say(side ? "docs(agent): what the handoff now says" : "docs(agent): what the plan and the handoff now say");
  say("");
  say("Drop a path this wrap does not write. Anything it does write that no `## commit:`");
  say("names joins the last one regardless, so the docs cannot be left dirty.");
  say("");
  say("## status");
  say("CONTINUE one line saying what landed");
  process.stdout.write(out.join("\n") + "\n");
  return 0;
}

/** An attribution trailer, `Key: value`; conventions.md § *A commit message* is the rule. */
const TRAILER_RE =
  /^\s*(?:co-authored-by|signed-off-by|generated-(?:by|with)|assisted-by|authored-by|reviewed-by|on-behalf-of|committed-by|created-by|made-with)\s*:.*$/gim;
/** The same boilerplate in prose, `🤖 Generated with [tool](url)`. A backtick in front means the line quotes it. */
const BOILERPLATE_RE = /^[^A-Za-z\n`]*(?:generated|created|made)\s+(?:with|by)\s+\[.*$/gim;

/** The message with every attribution trailer removed, and how many went. */
export function stripTrailers(body: string): { text: string; removed: number } {
  let removed = 0;
  const cleaned = body
    .replace(TRAILER_RE, () => {
      removed++;
      return "";
    })
    .replace(BOILERPLATE_RE, () => {
      removed++;
      return "";
    });
  if (removed === 0) return { text: body, removed };
  return { text: `${cleaned.replace(/\n{3,}/g, "\n\n").trim()}\n`, removed };
}

export async function run(args: string[]): Promise<number> {
  let opts;
  try {
    opts = parseArgs(args, { flags: ["--dry-run", "--check", "--counts", "--template", "--scrub"], valued: ["--wrap"] });
  } catch (e) {
    if (!(e instanceof ArgError)) throw e;
    console.error(`nv session: error: ${e.message}`);
    return 2;
  }
  if (opts.flags.has("--help")) {
    console.log(HELP);
    return 0;
  }
  if (opts.flags.has("--scrub")) {
    process.stdout.write(stripTrailers(await Bun.stdin.text()).text);
    return 0;
  }
  if (opts.flags.has("--counts")) {
    for (const [k, v] of Object.entries(counts())) console.log(`${k} ${v}`);
    return 0;
  }
  if (opts.flags.has("--template")) return template();
  if (opts.flags.has("--check")) return await check();
  const wrap = opts.values.get("--wrap");
  if (wrap !== undefined) {
    if (!existsSync(wrap)) {
      console.log(`nv session: no such file: ${wrap}`);
      return 2;
    }
    const parsed = parseWrap(readFileSync(wrap, "utf8").replace(/\r\n/g, "\n"));
    if (!parsed.sections.length && !parsed.errors.length) {
      console.log(`nv session: ${wrap} holds no \`## \` directive`);
      return 2;
    }
    const errors = [...parsed.errors, ...(await validate(parsed.sections))];
    if (errors.length) {
      console.log(`nv session: ${errors.length} problem(s) -- NOTHING was written or committed:`);
      for (const e of errors) console.log(`  - ${e}`);
      return 1;
    }
    return await passthrough(["python", "tools/session.py", "--wrap", wrap, ...(opts.flags.has("--dry-run") ? ["--dry-run"] : [])]);
  }
  console.log(HELP);
  return 2;
}
