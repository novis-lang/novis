// `bun nv holes`: every shape the language still refuses, as a worklist a session can take an item off.
//
//     bun nv holes                  the summary: sites per item, and what is unattributed
//     bun nv holes --item 7         one item: its anchors, its refusal sites, its cases
//     bun nv holes --unattributed   only the sites no item claims
//     bun nv holes --cases          only the named `.nvst` artefacts still missing
//     bun nv holes --sites          every site, file by file, with its message
//     bun nv holes --guarded        every `guarded_by!` site and the code it names
//     bun nv holes --json           one JSON object instead
//
// A refusal site is a shape that compiles in the front end and then refuses below it. It lives in
// `nvs-ir` or `nvs-codegen`, and it is read from the source itself, so a hole that stops refusing
// leaves the list and a new one joins it without anyone editing a list. A site is a string literal
// that sits after a refusal construct and says what it refuses:
//
// - A `panic!`/`todo!`/`unimplemented!`/`assert!` counts only when its message claims a shape it will
//   not take ("only lowers X", "has no arm for Y"). Most panic-family sites in these crates are engine
//   invariants no program reaches, and counting them would keep this list from ever reaching zero.
// - A `CodegenError::Unsupported` is a refusal whatever it says, because the type is the claim. The
//   ones whose message says the unit was assembled wrong are engine bugs, and are left out.
//
// A `guarded_by!` site is the other close a refusal can take: the shape never arrives, because the
// diagnostic it names refuses it where it is written. `--guarded` lists each with the code the
// registry in `crates/nvs-diagnostics/src/lib.rs` gives its constant, and `no-such-constant` for one it
// does not declare. `crates/nvs-ir/tests/refusals.rs` holds each code to a conformance case.
//
// The items are the live goal's numbered items in its prose, `docs/agent/goals/<slug>.md`, and the
// carried ones in `docs/agent/carried-refusals.md`, which number from 900. A site belongs to the item
// that names its enclosing function in backticks, or else to the item with the nearest `crates/…:NN`
// anchor in the same file; a site neither claims is unattributed. The named cases are the `cases`
// lists of the live goal's checks, in `data/goals/<slug>.json`. Nothing here is a gate.

import { existsSync, readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { liveGoal } from "../lib/chain.ts";
import { ROOT } from "../lib/paths.ts";
import { loadFile } from "../lib/store.ts";
import { goal as goalType } from "../schema/goal.ts";

export const summary = "the shapes the language still refuses: nv holes [--item N | --unattributed | --cases | --sites | --guarded | --json]";

const CARRIED_MD = "docs/agent/carried-refusals.md";
/** Carried items number from here, so they cannot collide with a goal's own items. */
const CARRIED_BASE = 900;
const REGISTRY = "crates/nvs-diagnostics/src/lib.rs";
/** Where a refusal can live. Both crates lower; nothing else does. */
const SOURCES = ["crates/nvs-ir/src", "crates/nvs-codegen/src"];

/** The construct a refusal message is written in, ending right where its literal starts. */
const CONSTRUCT = /(?:panic|todo|unimplemented)!\s*\(\s*$|(?:debug_)?assert(?:_eq|_ne)?!\s*\([^;{}]*$/;
/** What a refusal claims: what it does take and nothing else, or that it has no arm. */
const REFUSAL = /does not (?:yet )?lower|no lowering for|has no arm for|only (?:lowers|converts|stages|emits|takes|accepts|handles)|(?:lowers|converts|stages|emits|reaches) [^.;]{0,90}?\bonly\b/i;
/** A `CodegenError::Unsupported(` or `CodegenError::Unsupported(format!(` that the literal opens. */
const UNSUPPORTED = /CodegenError::Unsupported\s*\(\s*(?:format!\s*\(\s*)?$/;
/** An engine bug that uses the same type. */
const ENGINE = /this is a bug|declares no (?:descriptor|slot)|which this unit/i;
/** How far back from a literal its construct is looked for, in characters: past an `assert!`'s condition. */
const WINDOW = 240;
const GUARDED = /guarded_by!\s*\(\s*(?:[A-Za-z_][A-Za-z0-9_]*::)*([A-Z][A-Z0-9_]*)/g;
const DECLARES_CODE = /pub const ([A-Z][A-Z0-9_]*): Code = Code::new\("([A-Z]\d+)"\)/g;
const ANCHOR = /(crates\/[A-Za-z0-9_\-./]+\.rs):(\d+)/g;
const ITEM = /^(\d+)\. \*\*(.+?)\*\*/gm;
/** A backticked snake_case word in an item's prose names a function it changes. */
const NAMED_FN = /`([a-z_][a-z0-9_]{4,})`/g;
const DEFINES_FN = /^\s*(?:pub(?:\([^)]*\))?\s+)?(?:const\s+)?(?:async\s+)?(?:unsafe\s+)?(?:extern\s+"[^"]+"\s+)?fn\s+([a-z_][a-z0-9_]*)/;

interface Site {
  file: string;
  line: number;
  fn: string;
  message: string;
}

interface Guard {
  file: string;
  line: number;
  fn: string;
  const: string;
  code: string;
}

interface Item {
  n: number;
  title: string;
  files: string[];
  anchors: string[];
  functions: string[];
}

interface Case {
  suite: string;
  stage: unknown;
  path: string;
  written: boolean;
}

function read(path: string): string {
  return readFileSync(join(ROOT, path), "utf8");
}

/** The first `n` characters of `s`, counted as code points. */
function head(s: string, n: number): string {
  return [...s].slice(0, n).join("");
}

function byCodePoint(a: string, b: string): number {
  return a < b ? -1 : a > b ? 1 : 0;
}

/**
 * Every `.rs` file under `dir`, repo-relative, in path order: component by component, so `lower/x.rs`
 * comes before `lower_y.rs`. A crate's own tests refuse things on purpose, and no item anchors one, so
 * `tests.rs` and anything directly in a `tests/` directory are left out.
 */
function rustFiles(dir: string): string[] {
  const out: string[][] = [];
  const walk = (parts: string[]) => {
    for (const e of readdirSync(join(ROOT, ...parts), { withFileTypes: true })) {
      if (e.isDirectory()) walk([...parts, e.name]);
      else if (e.name.endsWith(".rs") && e.name !== "tests.rs" && parts[parts.length - 1] !== "tests") out.push([...parts, e.name]);
    }
  };
  walk(dir.split("/"));
  const key = (s: string) => (process.platform === "win32" ? s.toLowerCase() : s);
  out.sort((a, b) => {
    for (let i = 0; i < Math.min(a.length, b.length); i++) {
      const c = byCodePoint(key(a[i]!), key(b[i]!));
      if (c !== 0) return c;
    }
    return a.length - b.length;
  });
  return out.map((p) => p.join("/"));
}

/**
 * Every string literal outside a `//` comment, as `[line, offset, body]`. A literal is read to its
 * closing quote, across a trailing-backslash continuation, and its whitespace runs collapse to one
 * space. Skipping comments keeps a module's own prose about what it does not lower out of the list.
 */
function* literals(text: string): Generator<[number, number, string]> {
  let i = 0, line = 1;
  while (i < text.length) {
    const ch = text[i];
    if (ch === "\n") {
      line++;
      i++;
      continue;
    }
    if (ch === "/" && text.startsWith("//", i)) {
      const nl = text.indexOf("\n", i);
      i = nl < 0 ? text.length : nl;
      continue;
    }
    if (ch !== '"') {
      i++;
      continue;
    }
    const start = i, started = line;
    let out = "";
    i++;
    while (i < text.length) {
      const c = text[i]!;
      if (c === "\\") {
        const next = text[i + 1] ?? "";
        if (next === "\n") {
          line++;
          i += 2;
          while (i < text.length && (text[i] === " " || text[i] === "\t")) i++;
          continue;
        }
        out += next;
        i += 2;
        continue;
      }
      if (c === "\n") {
        line++;
        i++;
        out += " ";
        continue;
      }
      i++;
      if (c === '"') break;
      out += c;
    }
    yield [started, start, out.replace(/\s+/g, " ").trim()];
  }
}

/** The function a line sits in, or "" at file scope. */
function enclosingFn(lines: string[], line: number): string {
  for (let n = Math.min(line, lines.length) - 1; n >= 0; n--) {
    const m = DEFINES_FN.exec(lines[n]!);
    if (m) return m[1]!;
  }
  return "";
}

/** Each source file's text, with CRLF read as LF. */
function sources(): [string, string][] {
  return SOURCES.flatMap((dir) => rustFiles(dir)).map((file) => [file, read(file).replace(/\r\n/g, "\n")]);
}

function sites(files: [string, string][]): Site[] {
  const found: Site[] = [];
  for (const [file, text] of files) {
    const lines = text.split("\n");
    for (const [line, at, message] of literals(text)) {
      const near = text.slice(Math.max(0, at - WINDOW), at);
      const unsupported = UNSUPPORTED.test(near);
      if (!(unsupported || CONSTRUCT.test(near))) continue;
      if (!(unsupported || REFUSAL.test(message))) continue;
      // `#[error("nvs-codegen does not lower {0} yet")]` is the variant's Display impl, not a site.
      if (ENGINE.test(message) || near.trimEnd().endsWith("#[error(")) continue;
      found.push({ file, line, fn: enclosingFn(lines, line), message: message || "(no literal message at the site)" });
    }
  }
  return found;
}

function registry(): Map<string, string> {
  const out = new Map<string, string>();
  if (!existsSync(join(ROOT, REGISTRY))) return out;
  for (const m of read(REGISTRY).matchAll(DECLARES_CODE)) out.set(m[1]!, m[2]!);
  return out;
}

function guarded(files: [string, string][]): Guard[] {
  const declared = registry();
  const found: Guard[] = [];
  for (const [file, text] of files) {
    const lines = text.split("\n");
    for (const m of text.matchAll(GUARDED)) {
      const bol = text.lastIndexOf("\n", m.index! - 1) + 1;
      const line = text.slice(0, m.index!).split("\n").length;
      // A match inside a comment is prose about the macro, not a site written in it.
      const remark = lines[line - 1]!.indexOf("//");
      if (remark !== -1 && remark < m.index! - bol) continue;
      found.push({ file, line, fn: enclosingFn(lines, line), const: m[1]!, code: declared.get(m[1]!) ?? "" });
    }
  }
  return found;
}

function itemsIn(path: string): Item[] {
  if (!existsSync(join(ROOT, path))) return [];
  const text = read(path).replace(/\r\n/g, "\n");
  const marks = [...text.matchAll(ITEM)];
  return marks.map((m, k) => {
    const body = text.slice(m.index!, marks[k + 1]?.index ?? text.length);
    const anchors = [...body.matchAll(ANCHOR)].map((a) => [a[1]!, Number(a[2])] as const);
    return {
      n: Number(m[1]),
      title: m[2]!.replace(/\s+/g, " ").trim(),
      files: [...new Set(anchors.map(([f]) => f))].sort(byCodePoint),
      anchors: anchors.map(([f, l]) => `${f}:${l}`),
      functions: [...new Set([...body.matchAll(NAMED_FN)].map((f) => f[1]!))].sort(byCodePoint),
    };
  });
}

/** The goal's items, then the carried ones. Null, after saying why, when a carried one is misnumbered. */
function items(): Item[] | null {
  const carried = itemsIn(CARRIED_MD);
  const low = carried.find((i) => i.n < CARRIED_BASE);
  if (low) {
    console.error(`${CARRIED_MD}: item ${low.n} must be numbered from ${CARRIED_BASE} so it cannot collide with a goal's own item ${low.n}`);
    return null;
  }
  const prose = goalProse();
  return [...(prose === null ? [] : itemsIn(prose)), ...carried];
}

/** The live goal's prose, or null when the chain names no live goal. */
function goalProse(): string | null {
  return liveGoal()?.md ?? null;
}

function namedCases(): Case[] {
  const live = liveGoal();
  const path = live === null ? "" : `data/goals/${live.slug}.json`;
  if (live === null || !existsSync(join(ROOT, path))) return [];
  const goal = loadFile(goalType, path).value;
  const labels = new Map(goal.stages.map((s) => [s.number, `${s.number} ${s.title}`]));
  return goal.checks.flatMap((check) =>
    (check.cases ?? []).map((path) => ({
      suite: check.name ?? "?",
      stage: labels.get(check.stage) ?? String(check.stage),
      path,
      written: existsSync(join(ROOT, path)),
    })),
  );
}

/** Each site under the item that claims it, by enclosing function first and nearest anchor second; 0 is nobody. */
function attribute(found: Site[], scheduled: Item[]): Map<number, (Site & { by: string })[]> {
  const byItem = new Map<number, (Site & { by: string })[]>();
  for (const site of found) {
    let claimed: number | null = null, how = "";
    const byFn = site.fn ? scheduled.find((i) => i.functions.includes(site.fn)) : undefined;
    if (byFn) {
      claimed = byFn.n;
      how = "fn";
    } else {
      let distance: number | null = null;
      for (const item of scheduled) {
        for (const anchor of item.anchors) {
          const cut = anchor.lastIndexOf(":");
          if (anchor.slice(0, cut) !== site.file) continue;
          const gap = Math.abs(Number(anchor.slice(cut + 1)) - site.line);
          if (distance === null || gap < distance) {
            claimed = item.n;
            how = "file";
            distance = gap;
          }
        }
      }
    }
    const key = claimed ?? 0;
    if (!byItem.has(key)) byItem.set(key, []);
    byItem.get(key)!.push({ ...site, by: how });
  }
  return byItem;
}

/** JSON the way Python's `json.dumps(..., indent=2)` writes it: every non-ASCII character escaped. */
function asciiJson(value: unknown): string {
  return JSON.stringify(value, null, 2).replace(/[\u0080-￿]/g, (c) => `\\u${c.charCodeAt(0).toString(16).padStart(4, "0")}`);
}

/** Lines grouped under their file, the way `--sites` and `--guarded` print them. */
function byFile<T extends { file: string }>(list: T[], row: (x: T) => string): void {
  let current = "";
  for (const x of list) {
    if (x.file !== current) {
      current = x.file;
      console.log(`  ${current}`);
    }
    console.log(row(x));
  }
}

const USAGE = "usage: bun nv holes [--item N | --unattributed | --cases | --sites | --guarded | --json]";

export async function run(args: string[]): Promise<number> {
  const flags = new Set<string>();
  let item: number | null = null;
  for (let i = 0; i < args.length; i++) {
    const a = args[i]!;
    const eq = /^--item=(.*)$/.exec(a);
    const value = eq ? eq[1]! : a === "--item" ? args[++i] : undefined;
    if (value !== undefined) {
      if (!/^-?\d+$/.test(value)) {
        console.error(`${USAGE}\nnv holes: --item takes a number, not ${JSON.stringify(value)}`);
        return 2;
      }
      item = Number(value);
    } else if (["--unattributed", "--cases", "--sites", "--guarded", "--json"].includes(a)) flags.add(a);
    else {
      console.error(USAGE);
      return 2;
    }
  }

  const files = sources();
  const found = sites(files);
  const scheduled = items();
  if (scheduled === null) return 1;
  const cases = namedCases();
  const guards = guarded(files);
  const byItem = attribute(found, scheduled);
  const absent = cases.filter((c) => !c.written);

  if (flags.has("--json")) {
    console.log(asciiJson({
      sites: found,
      guarded: guards,
      items: scheduled.map((i) => ({ ...i, sites: byItem.get(i.n)?.length ?? 0 })),
      unattributed: byItem.get(0) ?? [],
      cases,
    }));
    return 0;
  }

  if (flags.has("--guarded")) {
    console.log(`${guards.length} guarded site(s)\n`);
    // The code is one token, so the test reading this column fails on a constant nothing declares.
    byFile(guards, (g) => `    :${String(g.line).padEnd(5)} ${(g.code || "no-such-constant").padEnd(16)} ${g.const}  in ${g.fn || "(file scope)"}`);
    if (guards.length > 0) {
      console.log("\nEach says the shape never arrives because the code beside it refuses one where");
      console.log("it is written. crates/nvs-ir/tests/refusals.rs holds each to a conformance case.");
    }
    return 0;
  }

  if (item !== null) {
    const one = scheduled.find((i) => i.n === item);
    if (!one) {
      console.log(`no item ${item} in ${goalProse() ?? "the live goal's prose"} or ${CARRIED_MD}`);
      return 1;
    }
    console.log(`item ${one.n}: ${one.title}\n`);
    console.log("  anchors");
    for (const anchor of one.anchors.length > 0 ? one.anchors : ["(none -- add them to the goal)"]) console.log(`    ${anchor}`);
    const mine = byItem.get(one.n) ?? [];
    console.log(`\n  refusal sites in those files: ${mine.length}`);
    for (const site of mine) console.log(`    ${site.file}:${site.line}  ${head(site.message, 110)}`);
    const words = one.title.toLowerCase().split(/\s+/).filter((w) => [...w].length > 5);
    const owed = absent.filter((c) => words.some((w) => c.path.includes(w)));
    if (owed.length > 0) {
      console.log("\n  cases that may belong to it (name match, judge it yourself)");
      for (const c of owed) console.log(`    ${c.path}`);
    }
    return 0;
  }

  if (flags.has("--cases")) {
    console.log(`${absent.length} of ${cases.length} named case(s) not written yet\n`);
    for (const c of absent) console.log(`  [${c.stage}] ${c.path}`);
    return 0;
  }

  if (flags.has("--sites") || flags.has("--unattributed")) {
    const unattributed = flags.has("--unattributed");
    const show = unattributed ? byItem.get(0) ?? [] : found;
    console.log(`${show.length} ${unattributed ? "unattributed refusal site" : "refusal site"}(s)\n`);
    byFile(show, (s) => `    :${s.line}  ${head(s.message, 110)}`);
    if (unattributed && show.length > 0) {
      console.log("\nEach is a hole nobody scheduled or a decision nobody wrote down. Both are the");
      console.log("goal's business before the code is -- an item in the live goal's prose, or one in");
      console.log(`${CARRIED_MD}.`);
    }
    return 0;
  }

  console.log(`${found.length} refusal site(s) across ${SOURCES.length} crate(s), ${guards.length} guarded (--guarded), ` +
    `${scheduled.length} scheduled item(s), ${absent.length} of ${cases.length} named case(s) still to write\n`);
  console.log("  ITEMS WITH A REFUSAL SITE STILL STANDING");
  const live = scheduled.filter((i) => (byItem.get(i.n)?.length ?? 0) > 0);
  for (const i of live) {
    console.log(`    ${String(i.n).padStart(3)}  ${String(byItem.get(i.n)!.length).padStart(2)} site(s)  ${head(i.title, 88)}`);
  }
  if (live.length === 0) console.log("    none -- every scheduled item's files are clean");
  const orphan = byItem.get(0) ?? [];
  console.log(`\n  UNATTRIBUTED: ${orphan.length} site(s) in files no item anchors`);
  for (const site of orphan.slice(0, 12)) console.log(`    ${site.file}:${site.line}  ${head(site.message, 88)}`);
  if (orphan.length > 12) console.log(`    ... and ${orphan.length - 12} more (--unattributed)`);
  console.log(`\n  ${absent.length} named case(s) still to write (--cases)`);
  console.log("\n  bun nv holes --item N     one item in full");
  return 0;
}
