// `bun nv parity <group>...`: runs a Python tool and the `nv` command that replaces it on the same tree,
// once per case, and compares what each printed and the status it exited with. A group and its cases
// are `tools/nv/parity/groups.json`. The comparison ignores only what `tools/nv/parity/known.json`
// declares: a rewrite of one side's text, a regex and its replacement, each carrying its reason. The
// `*` entries apply to every group. A group may also declare `unordered`: the entries whose order is not
// part of the output, compared sorted, with the reason. A Python tool is deleted only after its group
// matches here.
//
// With no group, prints the groups. With `--all`, runs every group. Exits 0 when every case matches,
// 1 when one differs, and 2 on a bad argument.

import { readFileSync } from "node:fs";
import { join } from "node:path";
import { ROOT } from "../lib/paths.ts";
import { run as runProc, type RunResult } from "../lib/proc.ts";

export const summary = "compare a Python tool with its nv replacement: nv parity <group>... | --all";

const GROUPS = "tools/nv/parity/groups.json";
const KNOWN = "tools/nv/parity/known.json";
/** Python writes to a pipe in the Windows code page unless told otherwise, and Bun writes UTF-8. */
const PYTHON_ENV = { env: { PYTHONIOENCODING: "utf-8" } };

interface Group {
  python: string[];
  nv: string[];
  /** The argument lists both programs are run with, one case each. */
  cases: string[][];
  /** Lines whose order is not part of the output, declared with the reason. */
  unordered?: Unordered;
}

/**
 * A line matching `pattern` opens an entry, and each line after it that is indented deeper and
 * matches nothing joins it. Every run of adjacent entries is compared sorted, on both sides.
 */
export interface Unordered {
  pattern: string;
  why: string;
}

function indent(line: string): number {
  return line.length - line.trimStart().length;
}

/** `text` with every run of adjacent entries `u` declares sorted. */
export function sortEntries(text: string, u: Unordered): string {
  const re = new RegExp(u.pattern);
  const lines = text.split("\n");
  const out: string[] = [];
  let run: string[][] = [];
  const flush = () => {
    out.push(...run.sort((a, b) => (a[0]! < b[0]! ? -1 : a[0]! > b[0]! ? 1 : 0)).flat());
    run = [];
  };
  for (const line of lines) {
    const open = run.length > 0 ? run[run.length - 1]! : null;
    if (re.test(line)) run.push([line]);
    else if (open && line.trim() !== "" && indent(line) > indent(open[0]!)) open.push(line);
    else {
      flush();
      out.push(line);
    }
  }
  flush();
  return out.join("\n");
}

export interface Known {
  /** Whose output the rewrite applies to. */
  side: "python" | "nv" | "both";
  /** A JavaScript regex, matched with the `g` and `m` flags. */
  pattern: string;
  replace: string;
  why: string;
}

function readJson<T>(path: string): T {
  return JSON.parse(readFileSync(join(ROOT, path), "utf8")) as T;
}

/** `text` with every declared rewrite for `side` applied, counting each rewrite that matched. */
export function normalize(text: string, side: "python" | "nv", known: Known[], used: Set<Known>): string {
  let out = text.replace(/\r\n/g, "\n");
  for (const k of known) {
    if (k.side !== side && k.side !== "both") continue;
    const re = new RegExp(k.pattern, "gm");
    if (re.test(out)) used.add(k);
    out = out.replace(new RegExp(k.pattern, "gm"), k.replace);
  }
  return out;
}

/** The first line where `a` and `b` differ, as report lines, or none when they are the same. */
function firstDifference(label: string, a: string, b: string): string[] {
  if (a === b) return [];
  const al = a.split("\n");
  const bl = b.split("\n");
  let i = 0;
  while (i < al.length && i < bl.length && al[i] === bl[i]) i++;
  const show = (s: string | undefined) => (s === undefined ? "(no line)" : JSON.stringify(s));
  return [
    `     ${label} line ${i + 1} of ${al.length} / ${bl.length}:`,
    `       python: ${show(al[i])}`,
    `       nv:     ${show(bl[i])}`,
  ];
}

/** Report lines for every way the two runs differ once normalized, or none when they match. */
export function compare(py: RunResult, nv: RunResult, known: Known[], used: Set<Known>, unordered?: Unordered): string[] {
  const out: string[] = [];
  const norm = (text: string, side: "python" | "nv") => {
    const n = normalize(text, side, known, used);
    return unordered ? sortEntries(n, unordered) : n;
  };
  if (py.code !== nv.code) out.push(`     exit status: python ${py.code}, nv ${nv.code}`);
  out.push(...firstDifference("stdout", norm(py.stdout, "python"), norm(nv.stdout, "nv")));
  out.push(...firstDifference("stderr", norm(py.stderr, "python"), norm(nv.stderr, "nv")));
  return out;
}

async function runGroup(name: string, group: Group, known: Known[]): Promise<boolean> {
  const used = new Set<Known>();
  let same = 0;
  console.log(`parity ${name}: ${group.cases.length} case(s), ${group.python.join(" ")} against ${group.nv.join(" ")}`);
  for (const args of group.cases) {
    const [py, nv] = await Promise.all([runProc([...group.python, ...args], PYTHON_ENV), runProc([...group.nv, ...args])]);
    const diff = compare(py, nv, known, used, group.unordered);
    const shown = args.length === 0 ? "(no arguments)" : args.join(" ");
    if (diff.length === 0) {
      same++;
      console.log(`  same     ${shown}`);
    } else {
      console.log(`  differs  ${shown}`);
      for (const line of diff) console.log(line);
    }
  }
  for (const k of known) {
    if (!used.has(k)) console.log(`  unused   a declared difference matched nothing: /${k.pattern}/ (${k.why})`);
  }
  console.log(`parity ${name}: ${same} of ${group.cases.length} case(s) match`);
  return same === group.cases.length;
}

export async function run(args: string[]): Promise<number> {
  const groups = readJson<Record<string, Group>>(GROUPS);
  const knownByGroup = readJson<Record<string, Known[]>>(KNOWN);
  const all = args.includes("--all");
  const names = all ? Object.keys(groups) : args;
  if (names.length === 0) {
    console.log("usage: bun nv parity <group>... | --all\n");
    for (const [name, g] of Object.entries(groups)) console.log(`  ${name}  ${g.cases.length} case(s)`);
    return 2;
  }
  const unknown = names.filter((n) => !(n in groups));
  if (unknown.length > 0) {
    console.error(`nv parity: no group ${unknown.join(", ")} in ${GROUPS}`);
    return 2;
  }
  let ok = true;
  for (const name of names) {
    const known = [...(knownByGroup["*"] ?? []), ...(knownByGroup[name] ?? [])];
    if (!(await runGroup(name, groups[name]!, known))) ok = false;
  }
  return ok ? 0 : 1;
}
