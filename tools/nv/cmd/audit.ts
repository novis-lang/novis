// `bun nv audit goals | checks | eol`: what the cutover must leave behind, each as one yes-or-no over the
// tracked tree (loop-goal.md § *Stage 9*). With no argument it runs all three.
//
//   goals   no tracked goal file is named `N-<slug>`, and no copy of a goal's files is tracked: the
//           driver's `docs/agent/loop-goal.*`, `docs/agent/handoff.md`, and a goal's `.toml` or
//           `.handoff.md` beside its prose. The goal's record under `data/goals/` is its one home.
//   checks  no check a goal record or the driver's toml holds runs Python, and the old name of the
//           feature proofs is in no check's name or `argv`, no directive line and no tracked file
//           under `tools/`, by path or by text.
//   eol     every tracked text file is LF in the index.
//
// A passing audit prints its `audit:` lines. A failing one prints a count and every offender under it,
// and exits 1. An unknown audit name exits 2.

import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { parse as parseToml } from "smol-toml";
import { indexEol, tracked } from "../lib/git.ts";
import { ROOT } from "../lib/paths.ts";
import { load } from "../lib/store.ts";
import { goal, sideGoal } from "../schema/goal.ts";

export const summary = "what the cutover must leave behind: nv audit [goals | checks | eol]";

/** The word the feature proofs were once named by, built in halves so this file does not say it. */
const OLD = ["doss", "ier"].join("");
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

const AUDITS: Record<string, () => Promise<Finding[]>> = { goals: auditGoals, checks: auditChecks, eol: auditEol };

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
