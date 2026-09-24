// `bun nv session`: AGENTS.md § *Session workflow* steps 4 and 5, the end of a session.
//
//     bun nv session --template        a wrap file skeleton, with this tree's counts already in it
//     bun nv session --check           what steps 4 and 5 still owe, read off the tree
//     bun nv session --counts          conformance, differential and decision record counts
//     bun nv session --scrub           a commit message on stdin, written trailer-free on stdout
//     bun nv session --wrap F [--dry-run]   apply a wrap file: the whole session tail
//
// A wrap file is markdown whose `## <kind>: <arg>` headings are instructions, and `parseWrap` is its
// one reader. `--help` prints the format. `--wrap` still runs `tools/session.py --wrap`, which applies
// every section or none; the parser here refuses the same malformed input that one does.
//
// `--check` judges no content. It prints the counts the plan's prose should agree with, each status
// field's size against its ceiling, the handoff's shape, the dead links this session made, the
// rulebook, record and migration gates for a session that edited those trees, and what is
// uncommitted. It fails on a stale count or a link this session broke. A link that was already dead
// where the session opened is printed and refuses nothing: it is CI's to report.

import { existsSync, readdirSync, readFileSync } from "node:fs";
import { join, posix, relative, sep } from "node:path";
import { ROOT } from "../lib/paths.ts";
import { passthrough, run as runProc } from "../lib/proc.ts";
import { ArgError, parseArgs, pyRepr } from "../lib/py.ts";
import { load } from "../lib/store.ts";
import { playbookSection } from "../schema/playbook.ts";
import { DOC_EXTS, fileFindings, findingsIn, deadMentions, isGenerated, MENTION_EXTS, readText, type Resolver, SOURCE_EXTS, sortKey, trackedFiles } from "./links.ts";
import { ANCHOR_RE } from "./orient.ts";
import { fieldLimits, planFields } from "./plan.ts";

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
    const { errors } = parseWrap(readFileSync(wrap, "utf8").replace(/\r\n/g, "\n"));
    if (errors.length) {
      console.log(`nv session: ${wrap} is malformed, and nothing was written:`);
      for (const e of errors) console.log(`  - ${e}`);
      return 2;
    }
    return await passthrough(["python", "tools/session.py", "--wrap", wrap, ...(opts.flags.has("--dry-run") ? ["--dry-run"] : [])]);
  }
  console.log(HELP);
  return 2;
}
