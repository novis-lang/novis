// `bun nv layout`: CONTRIBUTING.md's repository-layout block, checked against the tree.
//
//     bun nv layout            the block's findings, or that it matches the tree
//     bun nv layout --check    quiet on success; exit 1 on a finding
//     bun nv layout --rows     draft the rows the block is missing, read off disk
//
// A gate: it exits 1 on any finding. The block is the one piece of prose a build invalidates: adding a
// crate never touches CONTRIBUTING.md, so the block stops describing the workspace and nothing else
// notices. The rule it is held to: every row names something that exists, and every crate, bench
// package and tracked top-level directory has a row. No description is read. What this owns is the set
// of rows, and a row for something not built yet is a finding, because the plan schedules that.
//
// The four kinds of finding:
//
// - `missing`   a row names a path that is not on disk.
// - `unlisted`  a crate, a bench package or a tracked top-level directory has no row. `--rows` drafts
//               one for each, from the crate's own `//!` first sentence.
// - `unsafe`    the `[audited unsafe]` marker disagrees with the crate's own `[lints]`. The workspace
//               forbids `unsafe_code`, and a crate that needs it overrides that with its own `deny`
//               (Cargo.toml § *Lint policy*), so which crates carry the marker is derived.
// - `shape`     the markers are gone, or what sits between them is not one fenced listing.

import { existsSync, readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { tracked } from "../lib/git.ts";
import { ROOT } from "../lib/paths.ts";

export const summary = "CONTRIBUTING.md's layout block against the tree: nv layout [--check | --rows]";

const BLOCK = "CONTRIBUTING.md";
const BEGIN = "<!-- layout:begin";
const END = "<!-- layout:end";
const MARKER = "[audited unsafe]";
/** A crate opts out of the workspace's `unsafe_code = "forbid"` with this line in its own manifest. */
const OPT_OUT = /^\s*unsafe_code\s*=\s*"deny"/m;

/** Python's `str.splitlines`: every line boundary it knows, and no empty line after the last one. */
function splitlines(text: string): string[] {
  const lines = text.split(/\r\n|[\n\r\v\f\x1c-\x1e\x85\u2028\u2029]/);
  if (lines.length > 0 && lines[lines.length - 1] === "") lines.pop();
  return lines;
}

function read(path: string): string {
  return readFileSync(join(ROOT, path), "utf8");
}

function byName(a: string, b: string): number {
  const [x, y] = process.platform === "win32" ? [a.toLowerCase(), b.toLowerCase()] : [a, b];
  return x < y ? -1 : x > y ? 1 : 0;
}

/** Every Cargo package under `crates/` and `benches/`, as a repo-relative directory. */
function packages(): string[] {
  const found: string[] = [];
  for (const parent of ["crates", "benches"]) {
    let names: string[];
    try {
      names = readdirSync(join(ROOT, parent)).sort(byName);
    } catch {
      continue;
    }
    for (const name of names) {
      if (existsSync(join(ROOT, parent, name, "Cargo.toml"))) found.push(`${parent}/${name}`);
    }
  }
  return found;
}

/**
 * Every tracked top-level directory, or null when git cannot say. Tracked rather than listed, so the
 * answer is what a fresh clone holds: `target/` is on this machine and in nobody's checkout.
 */
async function trackedDirs(): Promise<string[] | null> {
  let files: string[];
  try {
    files = await tracked();
  } catch {
    return null;
  }
  const dirs = new Set<string>();
  for (const f of files) {
    const cut = f.indexOf("/");
    if (cut >= 0) dirs.add(f.slice(0, cut));
  }
  return [...dirs].sort();
}

/** The packages carrying their own `unsafe_code = "deny"`. */
function auditedUnsafe(): string[] {
  return packages().filter((p) => OPT_OUT.test(read(`${p}/Cargo.toml`)));
}

/** A package's `//!` opening sentence, flattened, for `--rows` to draft a description from. */
function firstSentence(pkg: string): string {
  for (const name of ["src/lib.rs", "src/main.rs"]) {
    const source = join(ROOT, pkg, name);
    if (!existsSync(source)) continue;
    const doc: string[] = [];
    for (const line of splitlines(readFileSync(source, "utf8"))) {
      const stripped = line.trim();
      if (stripped.startsWith("//!")) doc.push(stripped.slice(3).trim());
      else if (doc.length > 0) break;
      else if (stripped && !stripped.startsWith("//")) break;
    }
    const text = doc.join(" ").replace(/\[([^\]]+)\]\([^)]*\)/g, "$1").trim().replace(/\s+/g, " ");
    const head = text.split(/(?<=[.:])\s/, 1)[0]!;
    return head.replace(/[.:]+$/, "");
  }
  return "";
}

interface Row {
  line: number;
  path: string;
  marked: boolean;
}

/**
 * The block's rows, or a `shape` finding as a string. A row is `<indent><name><spaces><description>`.
 * An unindented name ending in `/` opens a group, and each indented row under it hangs off that group.
 */
function parse(text: string): Row[] | string {
  const lines = splitlines(text);
  const starts = lines.flatMap((l, i) => (l.startsWith(BEGIN) ? [i] : []));
  const ends = lines.flatMap((l, i) => (l.startsWith(END) ? [i] : []));
  if (starts.length !== 1 || ends.length !== 1 || ends[0]! < starts[0]!) {
    return `expected exactly one ${BEGIN} …> and one ${END} …> after it`;
  }
  const start = starts[0]!;
  const body = lines.slice(start + 1, ends[0]!);
  const fences = body.flatMap((l, i) => (l.startsWith("```") ? [i] : []));
  if (fences.length !== 2) return "what sits between the markers is not one fenced listing";

  const rows: Row[] = [];
  let group = "";
  body.slice(fences[0]! + 1, fences[1]!).forEach((line, offset) => {
    if (!line.trim()) return;
    const name = line.trim().split(/\s+/)[0]!;
    let path: string;
    if (/^\s/.test(line)) path = group + name;
    else {
      group = name.endsWith("/") ? name : "";
      path = name;
    }
    rows.push({ line: start + 2 + fences[0]! + 1 + offset, path: path.replace(/\/+$/, ""), marked: line.includes(MARKER) });
  });
  return rows;
}

/** Every finding as a kind and its detail, and the packages that owe a row. */
async function check(): Promise<{ findings: [string, string][]; owed: string[] }> {
  const rows = parse(read(BLOCK));
  if (typeof rows === "string") return { findings: [["shape", `${BLOCK}: ${rows}`]], owed: [] };

  const findings: [string, string][] = [];
  const listed = new Set(rows.map((r) => r.path));
  for (const r of rows) {
    if (!existsSync(join(ROOT, r.path))) findings.push(["missing", `${BLOCK}:${r.line}  ->  ${r.path}`]);
  }
  const owed = packages().filter((p) => !listed.has(p));
  for (const p of owed) findings.push(["unlisted", `${p}  (a Cargo package with no row)`]);

  const dirs = await trackedDirs();
  if (dirs === null) findings.push(["shape", "`git ls-files` failed, so top-level directories went unchecked"]);
  else {
    for (const name of dirs) {
      if (!listed.has(name) && ![...listed].some((p) => p.startsWith(name + "/"))) {
        findings.push(["unlisted", `${name}/  (a tracked top-level directory with no row)`]);
      }
    }
  }

  const expected = new Set(auditedUnsafe());
  for (const r of rows) {
    if (r.marked && !expected.has(r.path)) {
      findings.push(["unsafe", `${BLOCK}:${r.line}  ${r.path} carries ${MARKER}, but its manifest does not override the workspace's forbid`]);
    }
  }
  const marked = new Set(rows.filter((r) => r.marked).map((r) => r.path));
  for (const p of [...expected].filter((p) => !marked.has(p)).sort()) {
    if (listed.has(p)) findings.push(["unsafe", `${p} declares its own \`unsafe_code = "deny"\` and its row does not say ${MARKER}`]);
  }
  return { findings, owed };
}

const USAGE = "usage: nv layout [--check] [--rows] [-h]";

export async function run(args: string[]): Promise<number> {
  const unknown = args.filter((a) => !["--check", "--rows", "-h", "--help"].includes(a));
  if (unknown.length > 0) {
    console.error(`${USAGE}\nnv layout: error: unrecognized arguments: ${unknown.join(" ")}`);
    return 2;
  }
  if (args.includes("-h") || args.includes("--help")) {
    console.log(`nv layout: ${summary}`);
    return 0;
  }
  const { findings, owed } = await check();
  const out: string[] = [];

  if (args.includes("--rows")) {
    if (owed.length === 0) out.push("nothing on disk owes a row");
    for (const p of owed) out.push(`  ${p.slice(p.lastIndexOf("/") + 1).padEnd(16)}  ${firstSentence(p)}`);
  } else {
    for (const [kind, detail] of findings) out.push(`  ${kind.padEnd(9)} ${detail}`);
    if (findings.length > 0) {
      out.push(
        "",
        `${findings.length} finding(s) in ${BLOCK}'s *Repository layout* listing.`,
        "The block owns the set of rows; `bun nv layout --rows` drafts the missing ones.",
        "What the workspace has not built yet belongs to docs/implementation-plan.md, not here.",
      );
    } else if (!args.includes("--check")) {
      const rows = parse(read(BLOCK)) as Row[];
      out.push(
        `every one of the ${rows.length} rows in ${BLOCK}'s layout listing is on disk, ` +
          "and every crate, bench package and tracked top-level directory has one",
      );
    }
  }
  if (out.length > 0) process.stdout.write(out.join("\n") + "\n");
  return findings.length > 0 ? 1 : 0;
}
