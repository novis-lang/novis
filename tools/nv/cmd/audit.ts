// `bun nv audit goals | checks | eol | ci | python`: what the cutover must leave behind, each as one
// yes-or-no over the tree (the goal's § *Stage 9*, § *Stage 11* and § *Stage 12*). With no argument it
// runs every audit.
//
//   goals   no tracked goal file is named `N-<slug>`, and no copy of a goal's files is tracked: the
//           driver's `docs/agent/loop-goal.*`, `docs/agent/handoff.md`, and a goal's `.toml` or
//           `.handoff.md` beside its prose. The goal's record under `data/goals/` is its one home.
//   checks  no check a goal record or the driver's toml holds runs Python, and the old name of the
//           feature proofs is in no check's name or `argv`, no directive line and no tracked file
//           under `tools/`, by path or by text.
//   eol     every tracked text file is LF in the index.
//   ci      no workflow step runs `python tools/...`, no git hook calls Python outside a comment, and the
//           website's own sync scripts are gone, because `bun nv render --website` writes their data.
//   python  no `.py` file is tracked but the Python twins under `benches/userland/`, which are a bench
//           engine's workload rather than a tool. No document that describes the present names a Python
//           tool, as a `tools/<x>.py` path or as a tool's bare `<x>.py`, or cites a home the cutover
//           deleted (`LEGACY`), as a link, in backticks or as plain text; and no record under `data/`
//           does either. What describes the past is not read: decision records and their records under
//           `data/decisions/`, `CHANGELOG.md` and the live goal's prose.
//
// A passing audit prints its `audit:` lines. A failing one prints a count and every offender under it,
// and exits 1. An unknown audit name exits 2.

import { existsSync, readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { parse as parseToml } from "smol-toml";
import { liveGoal } from "../lib/chain.ts";
import { indexEol, tracked } from "../lib/git.ts";
import { OLD, ROOT } from "../lib/paths.ts";
import { load } from "../lib/store.ts";
import { goal, sideGoal } from "../schema/goal.ts";

export const summary = "what the cutover must leave behind: nv audit [goals | checks | eol | ci | python]";

const OLD_RE = new RegExp(OLD, "i");
const DIRECTIVE_RE = new RegExp(`^\\s*(?://|#)\\s*${OLD}\\s*:`, "im");

const GOALS = "docs/agent/goals/";
const DRIVER_TOML = "docs/agent/loop-goal.toml";

/** What one audit found: the line it prints when it passes, and each offender when it does not. */
export interface Finding {
  pass: string;
  fail: string;
  offenders: string[];
}

/** Tracked goal files whose name starts with a position. */
export function numberedGoalFiles(paths: string[]): string[] {
  return paths.filter((p) => (p.startsWith(GOALS) || p.startsWith("data/goals/")) && /^\d+-/.test(p.slice(p.lastIndexOf("/") + 1)));
}

/** Tracked copies of what a goal's record now holds. */
export function goalCopies(paths: string[]): string[] {
  return paths.filter(
    (p) =>
      p.startsWith("docs/agent/loop-goal.") ||
      p === "docs/agent/handoff.md" ||
      (p.startsWith(GOALS) && (p.endsWith(".toml") || p.endsWith(".handoff.md"))),
  );
}

export interface CheckLine {
  /** Where the check is written: a record's path, or the driver's toml. */
  where: string;
  name: string;
  argv: string[];
}

/** A check runs Python when its program is a Python interpreter or any argument is a `.py` file. */
export function runsPython(c: CheckLine): boolean {
  const program = (c.argv[0] ?? "").split(/[\\/]/).pop()!;
  return /^(?:python[\d.]*|py)(?:\.exe)?$/i.test(program) || c.argv.some((a) => /\.py$/i.test(a));
}

/** Every check that says the old name, in its name or in its `argv`. */
export function checksSayingOld(checks: CheckLine[]): string[] {
  return checks
    .filter((c) => OLD_RE.test(c.name) || c.argv.some((a) => OLD_RE.test(a)))
    .map((c) => `${c.where}: ${JSON.stringify(c.name)}`);
}

/** A tracked file's text says the old name as a directive: a `//` or `#` comment line naming it with a colon. */
export function saysOldDirective(text: string): boolean {
  return DIRECTIVE_RE.test(text);
}

/** A tool says the old name when its path or its text does. */
export function toolSayingOld(path: string, text: string): string | null {
  if (OLD_RE.test(path)) return `${path}: its path`;
  const n = text.split(/\r?\n/).filter((l) => OLD_RE.test(l)).length;
  return n > 0 ? `${path}: ${n} line(s)` : null;
}

/** Tracked text files the index holds with a CR in a line ending. */
export function crlfFiles(rows: { path: string; eol: string }[]): string[] {
  return rows.filter((r) => r.eol === "crlf" || r.eol === "mixed").map((r) => `${r.path}: ${r.eol}`);
}

/** Every check a goal or side goal record holds, and every one the driver's toml holds while it is on disk. */
export function allChecks(root: string = ROOT): CheckLine[] {
  const out: CheckLine[] = [];
  for (const type of [goal, sideGoal]) {
    for (const g of load(type, root)) {
      for (const c of g.value.checks) out.push({ where: g.path, name: c.name ?? c.id, argv: [...(c.argv ?? []), ...(c.args ?? [])] });
    }
  }
  const toml = join(root, DRIVER_TOML);
  if (existsSync(toml)) {
    const doc = parseToml(readFileSync(toml, "utf8")) as { check?: { name?: string; argv?: string[]; args?: string[] }[] };
    for (const c of doc.check ?? []) out.push({ where: DRIVER_TOML, name: c.name ?? "", argv: [...(c.argv ?? []), ...(c.args ?? [])] });
  }
  return out;
}

function readText(path: string): string | null {
  const full = join(ROOT, path);
  if (!existsSync(full)) return null;
  const text = readFileSync(full, "utf8");
  return text.includes("\0") ? null : text;
}

async function auditGoals(): Promise<Finding[]> {
  const paths = await tracked();
  return [
    { pass: "audit: no goal file carries a number", fail: "goal file(s) carry a number", offenders: numberedGoalFiles(paths) },
    { pass: "audit: no copy of a goal's files is tracked", fail: "copy(ies) of a goal's files are tracked", offenders: goalCopies(paths) },
  ];
}

async function auditChecks(): Promise<Finding[]> {
  const checks = allChecks();
  const python = checks.filter(runsPython).map((c) => `${c.where}: ${JSON.stringify(c.name)}`);
  const old = checksSayingOld(checks);
  for (const p of await tracked()) {
    if (p.startsWith("tools/")) {
      const hit = toolSayingOld(p, readText(p) ?? "");
      if (hit) old.push(hit);
    } else if (/\.(?:nvs|nvst|rs)$/.test(p)) {
      const text = readText(p);
      if (text !== null && saysOldDirective(text)) old.push(`${p}: a directive`);
    }
  }
  return [
    { pass: "audit: no check runs python", fail: "check(s) run python", offenders: python },
    { pass: `audit: nothing says ${OLD}`, fail: `place(s) say ${OLD}`, offenders: old },
  ];
}

async function auditEol(): Promise<Finding[]> {
  return [{ pass: "audit: every tracked text file is LF", fail: "tracked text file(s) are not LF", offenders: crlfFiles(await indexEol()) }];
}

const WORKFLOWS = [".github/workflows/ci.yml", ".github/workflows/pages.yml", ".github/workflows/release.yml", ".github/workflows/release-promote.yml"];
const HOOKS = ["tools/git-hooks/commit-msg"];
const WEBSITE_SYNCS = ["website/scripts/sync-rules.mjs", "website/scripts/sync-core.mjs"];

/** The lines of a file that are not blank and not a `#` comment, each with its 1-based number. */
function codeLines(text: string): { n: number; line: string }[] {
  return text
    .split(/\r?\n/)
    .map((line, i) => ({ n: i + 1, line }))
    .filter(({ line }) => line.trim() !== "" && !line.trimStart().startsWith("#"));
}

/** Every line of a workflow that runs a Python tool from `tools/`. */
export function pythonToolSteps(path: string, text: string): string[] {
  return codeLines(text)
    .filter(({ line }) => /\bpython[\d.]*\s+tools\//.test(line))
    .map(({ n, line }) => `${path}:${n}: ${line.trim()}`);
}

/** Every line of a hook that calls Python. */
export function pythonCalls(path: string, text: string): string[] {
  return codeLines(text)
    .filter(({ line }) => /\bpython[\d.]*\b/.test(line))
    .map(({ n, line }) => `${path}:${n}: ${line.trim()}`);
}

async function auditCi(): Promise<Finding[]> {
  const steps = WORKFLOWS.flatMap((p) => pythonToolSteps(p, readText(p) ?? ""));
  const hooks = HOOKS.flatMap((p) => pythonCalls(p, readText(p) ?? ""));
  const syncs = WEBSITE_SYNCS.filter((p) => existsSync(join(ROOT, p))).map((p) => `${p}: still exists`);
  return [
    { pass: "audit: no workflow or hook calls python", fail: "workflow step(s) or git hook line(s) call Python", offenders: [...steps, ...hooks] },
    { pass: "audit: no website script parses a record", fail: "website sync script(s) still parse the data", offenders: syncs },
  ];
}

/** The Python column of the cross-language bench: `bun nv bench` runs these as a peer engine's workload. */
const PY_WORKLOAD = "benches/userland/";
/** Documents that are history: frozen records, and a changelog written from `git log`. */
const HISTORY = ["docs/decisions/", "CHANGELOG.md"];
/** Records that are history: a decision's record is as frozen as its prose. */
const RECORD_HISTORY = ["data/decisions/"];
/** Scratch, never a document. */
const SCRATCH = ".agent-tmp/";

/** Tracked `.py` files that are not a bench workload. */
export function strayPython(paths: string[]): string[] {
  return paths.filter((p) => p.endsWith(".py") && !p.startsWith(PY_WORKLOAD));
}

/**
 * The names of Python tools whose port took another name, so no `nv` subcommand carries theirs: the
 * feature-proof tool is `bun nv proofs`, `check-links` is `bun nv links` and `guard-read` is `bun nv guard`.
 */
export const RENAMED_TOOLS = [OLD, "check-links", "guard-read"];

/**
 * The homes the cutover deleted, each as a pattern over a citation of it: the driver's copies of the live
 * goal, the handoff file, the decision and rule indexes, the TOML data under `tools/data/`, the generated
 * goals' directory and a goal file named by its position, the website's sync scripts and their npm
 * names, and the Python driver's memo. Each is gone, so a line naming one describes the past.
 */
export const LEGACY: RegExp[] = [
  /\bloop-goal\.(?:md|toml)\b/,
  /(?<![\w.-])handoff\.md\b/,
  /\bdocs\/decisions\.toml\b/,
  /\bdocs\/rules\/_index\.json\b/,
  /\btools\/data\/[\w-]+\.toml\b/,
  new RegExp(`\\bdocs/agent/goals/(?:${OLD}/|\\d+-[a-z])`),
  /\bwebsite\/scripts\/sync-(?:core|rules)\.mjs\b/,
  /\bsync:(?:core|rules)\b/,
  /\bgoal-green\.json\b/,
];

/**
 * What names a Python tool: any `tools/<x>.py` path, or a bare `<x>.py` when `<x>` is a tool's name.
 * A tool's name is its `nv` subcommand's, since each port kept the name of the tool it replaced, one of
 * `RENAMED_TOOLS`, or a `tools/*.py` still tracked.
 */
export function toolNameRe(names: string[]): RegExp {
  const bare = names.map((n) => n.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")).join("|");
  return new RegExp(`tools/[\\w/-]+\\.py\\b${bare ? `|(?<![\\w/.-])(?:${bare})\\.py\\b` : ""}`);
}

/** What a present-tense document or record may not say: a Python tool by `toolNameRe`'s rule, or any `LEGACY` home. */
export function pastRe(names: string[]): RegExp {
  return new RegExp([toolNameRe(names).source, ...LEGACY.map((r) => r.source)].join("|"));
}

/** A document naming a Python tool or a deleted home, as `path:<first line>: N line(s)`, or null when it names none. */
export function docNamingPython(path: string, text: string, re: RegExp): string | null {
  const hits = text
    .split(/\r?\n/)
    .map((l, i) => (re.test(l) ? i + 1 : 0))
    .filter((n) => n > 0);
  return hits.length > 0 ? `${path}:${hits[0]}: ${hits.length} line(s)` : null;
}

/**
 * The tracked Markdown a reader follows today: everything but history and the live goal's prose, which
 * names the tools it deletes.
 */
export function currentDocs(paths: string[], live: string | null): string[] {
  return paths.filter((p) => {
    if (!p.endsWith(".md") || p.startsWith(SCRATCH) || HISTORY.some((h) => p === h || (h.endsWith("/") && p.startsWith(h)))) return false;
    return p !== `docs/agent/goals/${live}.md`;
  });
}

/**
 * The tracked records under `data/` that describe the present: all but the decisions'. The live goal's
 * records are read, since they are its state now.
 */
export function currentRecords(paths: string[]): string[] {
  return paths.filter((p) => p.startsWith("data/") && p.endsWith(".json") && !RECORD_HISTORY.some((h) => p.startsWith(h)));
}

async function auditPython(): Promise<Finding[]> {
  const paths = await tracked();
  const live = liveGoal()?.slug ?? null;
  const names = new Set(readdirSync(join(ROOT, "tools/nv/cmd")).filter((f) => f.endsWith(".ts")).map((f) => f.slice(0, -3)));
  for (const n of RENAMED_TOOLS) names.add(n);
  for (const p of paths) if (/^tools\/[\w-]+\.py$/.test(p)) names.add(p.slice(6, -3));
  const re = pastRe([...names]);
  const docs = currentDocs(paths, live).flatMap((p) => docNamingPython(p, readText(p) ?? "", re) ?? []);
  const records = currentRecords(paths).flatMap((p) => docNamingPython(p, readText(p) ?? "", re) ?? []);
  return [
    { pass: "audit: no Python file is tracked beyond the bench's workload", fail: "Python file(s) are tracked beyond the bench's workload", offenders: strayPython(paths) },
    { pass: "audit: no document names a Python tool or a deleted home", fail: "document(s) name a Python tool or a deleted home", offenders: docs },
    { pass: "audit: no record names a Python tool or a deleted home", fail: "record(s) name a Python tool or a deleted home", offenders: records },
  ];
}

const AUDITS: Record<string, () => Promise<Finding[]>> = { goals: auditGoals, checks: auditChecks, eol: auditEol, ci: auditCi, python: auditPython };

/** The lines a set of findings prints, and whether every one passed. */
export function report(findings: Finding[]): { lines: string[]; ok: boolean } {
  const lines: string[] = [];
  for (const f of findings) {
    if (f.offenders.length === 0) lines.push(f.pass);
    else lines.push(`audit: ${f.offenders.length} ${f.fail}`, ...f.offenders.map((o) => `  ${o}`));
  }
  return { lines, ok: findings.every((f) => f.offenders.length === 0) };
}

export async function run(args: string[]): Promise<number> {
  const names = args.length === 0 ? Object.keys(AUDITS) : args;
  const unknown = names.filter((n) => !(n in AUDITS));
  if (unknown.length > 0) {
    console.error(`nv audit: no audit named ${unknown.join(", ")}; the audits are ${Object.keys(AUDITS).join(", ")}`);
    return 2;
  }
  const findings: Finding[] = [];
  for (const n of names) findings.push(...(await AUDITS[n]!()));
  const { lines, ok } = report(findings);
  for (const l of lines) console.log(l);
  return ok ? 0 : 1;
}
