// `bun nv records`: the decision records left to merge into the rules, and the checks that keep the
// merge from leaving a dangling citation behind.
//
//     bun nv records                         every check, grouped
//     bun nv records --check                 the same, one line on success, exit 1 on a finding
//     bun nv records --left                  every record still on disk, grouped by topic
//     bun nv records --left 0002-0039        the records in that range, inclusive
//     bun nv records --left adr              the files under `docs/adr/`
//
// A record is `docs/decisions/NNNN.md`, and it is merged by moving its reasons and its declined
// alternatives into the rules whose `because` names it, then deleting it. `--left` groups the records
// in a scope by the topic most of those rules sit in, and gives each one its rule ids and how many
// files outside `docs/decisions/` cite it. Its last line is `<n> left in <scope>`.
//
// `--check` runs two checks. `links`: a link from a record or from a `docs/adr/` file to a record, or
// up out of the directory, has its target on disk, so a merged record's links in the records left are
// unlinked to plain text. `citations`: no tracked file outside `docs/decisions/` cites a record that is
// not on disk, by the `DECISION` pattern `lib/prose.ts` reads citations with. This goal's own file
// names records it deletes, and the test of that pattern names record numbers as fixtures, so
// neither is read.

import { existsSync, readdirSync, readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { DECISION } from "../lib/prose.ts";
import { ROOT } from "../lib/paths.ts";
import { load } from "../lib/store.ts";
import { rule } from "../schema/rule.ts";

export const summary = "the decision records left to merge: nv records [--check] | --left [<from>-<to> | adr]";

const RECORDS_DIR = "docs/decisions";
const ADR_DIR = "docs/adr";
/** The live goal's file, which names the records it merges and deletes. */
const GOAL_FILE = "docs/goals/decision-records-retired.md";
/** The test of `DECISION`, whose fixtures cite record numbers that need not be on disk. */
const PROSE_TEST = "tools/nv/test/prose.test.ts";

/** A link to a record, from a record (`(NNNN.md)`) or from a `docs/adr/` file (`(../decisions/NNNN.md)`). */
const RECORD_LINK = /\]\((?:\.\.\/decisions\/)?(\d{4})\.md(?:#[^)]*)?\)/g;
/** A relative link that leaves the directory, whose target must be on disk. */
const PARENT_LINK = /\]\((\.\.\/[^)#]+)(?:#[^)]*)?\)/g;
/** What `git grep` looks for before `DECISION` reads the line: a superset of it, in ERE. */
const GREP = "ADRs? [0-9]{4}|decisions/[0-9]{4}\\.md|0[0-9]{3} §";

type Finding = [string, number, string];

function read(path: string): string {
  return readFileSync(join(ROOT, path), "utf8").replace(/\r\n/g, "\n");
}

/** The record numbers on disk, ascending. */
function onDisk(): string[] {
  if (!existsSync(join(ROOT, RECORDS_DIR))) return [];
  return readdirSync(join(ROOT, RECORDS_DIR)).filter((f) => /^\d{4}\.md$/.test(f)).map((f) => f.slice(0, 4)).sort();
}

/** The files under `docs/adr/`, repo-relative and sorted. */
function adrFiles(): string[] {
  if (!existsSync(join(ROOT, ADR_DIR))) return [];
  return readdirSync(join(ROOT, ADR_DIR)).filter((f) => f.endsWith(".md")).map((f) => `${ADR_DIR}/${f}`).sort();
}

/** `[file, line, record]` for every citation of a record in a tracked file outside the records. */
function citations(): Finding[] {
  const proc = Bun.spawnSync(
    ["git", "-c", "core.quotepath=off", "grep", "-n", "-I", "-E", GREP, "--", ".", `:!${RECORDS_DIR}/`, `:!${GOAL_FILE}`, `:!${PROSE_TEST}`],
    { cwd: ROOT, stdout: "pipe", stderr: "pipe" },
  );
  if (proc.exitCode !== 0 && proc.exitCode !== 1) throw new Error(`git grep failed: ${proc.stderr.toString()}`);
  const out: Finding[] = [];
  for (const row of proc.stdout.toString().split("\n")) {
    const m = /^([^:]+):(\d+):(.*)$/.exec(row);
    if (!m) continue;
    const seen = new Set<string>();
    for (const c of m[3]!.matchAll(DECISION)) {
      const n = c[1] ?? c[2] ?? c[3];
      if (n && !seen.has(n)) out.push([m[1]!, Number(m[2]), n]);
      if (n) seen.add(n);
    }
  }
  return out;
}

function checkLinks(records: Set<string>): Finding[] {
  const out: Finding[] = [];
  const files = [...[...records].map((n) => `${RECORDS_DIR}/${n}.md`), ...adrFiles()];
  for (const file of files) {
    const base = dirname(join(ROOT, file));
    read(file).split("\n").forEach((line, i) => {
      for (const m of line.matchAll(RECORD_LINK)) {
        if (!records.has(m[1]!)) out.push([file, i + 1, `broken record link -> ${m[1]}`]);
      }
      for (const m of line.matchAll(PARENT_LINK)) {
        if (!existsSync(resolve(base, m[1]!))) out.push([file, i + 1, `broken relative link -> ${m[1]}`]);
      }
    });
  }
  return out;
}

function checkCitations(records: Set<string>): Finding[] {
  return citations()
    .filter(([, , n]) => !records.has(n))
    .map(([file, line, n]) => [file, line, `cites record ${n}, which is not on disk`] as Finding);
}

function byFinding(a: Finding, b: Finding): number {
  if (a[0] !== b[0]) return a[0] < b[0] ? -1 : 1;
  if (a[1] !== b[1]) return a[1] - b[1];
  return a[2] < b[2] ? -1 : a[2] > b[2] ? 1 : 0;
}

function report(quiet: boolean): number {
  const records = new Set(onDisk());
  const checks: [string, () => Finding[], string][] = [
    ["links", () => checkLinks(records), "every record link has its target on disk"],
    ["citations", () => checkCitations(records), "no citation names a missing record"],
  ];
  let total = 0;
  for (const [name, fn, clean] of checks) {
    const found = fn();
    total += found.length;
    if (found.length === 0) {
      if (!quiet) console.log(`ok   ${name}: ${clean}`);
      continue;
    }
    console.log(`\n== ${name} (${found.length})`);
    for (const [f, line, msg] of found.sort(byFinding)) console.log(`  ${f}:${line}  ${msg}`);
  }
  if (total > 0) console.log(`\n${total} finding(s), ${records.size} records on disk`);
  else if (quiet) console.log("no citation names a missing record");
  return total;
}

/** The topic most of `ids` sit in; a tie goes to the topic named first alphabetically. */
function topicOf(ids: string[]): string {
  const counts = new Map<string, number>();
  for (const id of ids) {
    const t = id.slice(0, id.indexOf("/"));
    counts.set(t, (counts.get(t) ?? 0) + 1);
  }
  const ranked = [...counts].sort(([a, x], [b, y]) => y - x || (a < b ? -1 : 1));
  return ranked[0]?.[0] ?? "(no rule)";
}

function left(scope: string | null): number {
  if (scope === "adr") {
    const files = adrFiles();
    const proc = Bun.spawnSync(["git", "grep", "-l", "-I", "-F", "adr/", "--", ".", `:!${ADR_DIR}/`, `:!${GOAL_FILE}`], { cwd: ROOT, stdout: "pipe" });
    const citing = proc.stdout.toString().split("\n").filter(Boolean);
    for (const f of files) {
      const name = f.slice(ADR_DIR.length + 1);
      const n = citing.filter((c) => read(c).includes(`adr/${name}`)).length;
      console.log(`  ${f}  cited by ${n} file(s)`);
    }
    console.log(`${files.length} left in ${ADR_DIR}`);
    return 0;
  }
  let from = "0000", to = "9999";
  if (scope !== null) {
    const m = /^(\d{4})-(\d{4})$/.exec(scope);
    if (!m) {
      console.error(`nv records: --left takes <from>-<to> as two four-digit numbers, or adr; got ${scope}`);
      return 2;
    }
    [from, to] = [m[1]!, m[2]!];
  }
  const nums = onDisk().filter((n) => n >= from && n <= to);
  const rules = new Map<string, string[]>();
  for (const r of load(rule)) {
    if (r.issues.length > 0) continue;
    for (const n of r.value.because) {
      if (!rules.has(n)) rules.set(n, []);
      rules.get(n)!.push(r.id);
    }
  }
  const citing = new Map<string, Set<string>>();
  for (const [file, , n] of citations()) {
    if (!citing.has(n)) citing.set(n, new Set());
    citing.get(n)!.add(file);
  }
  const groups = new Map<string, string[]>();
  for (const n of nums) {
    const t = topicOf(rules.get(n) ?? []);
    if (!groups.has(t)) groups.set(t, []);
    groups.get(t)!.push(n);
  }
  for (const t of [...groups.keys()].sort()) {
    console.log(t);
    for (const n of groups.get(t)!) {
      const ids = (rules.get(n) ?? []).sort();
      console.log(`  ${n}  cited by ${citing.get(n)?.size ?? 0} file(s)  ${ids.join(" ") || "-"}`);
    }
  }
  console.log(`${nums.length} left in ${scope ?? RECORDS_DIR}`);
  return 0;
}

const USAGE = "usage: bun nv records [--check] | --left [<from>-<to> | adr]";

export async function run(args: string[]): Promise<number> {
  let check = false;
  let leftScope: string | null | undefined;
  for (let i = 0; i < args.length; i++) {
    const a = args[i]!;
    if (a === "--check") check = true;
    else if (a === "--left") leftScope = i + 1 < args.length && !args[i + 1]!.startsWith("--") ? args[++i]! : null;
    else if (a === "-h" || a === "--help") {
      console.log(`${USAGE}\n\nnv records: ${summary}`);
      return 0;
    } else {
      console.error(USAGE);
      return 2;
    }
  }
  if (leftScope !== undefined) return left(leftScope);
  const n = report(check);
  return n > 0 && check ? 1 : 0;
}
