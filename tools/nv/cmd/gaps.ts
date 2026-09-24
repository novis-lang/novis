// `bun nv gaps`: what the conformance corpus does not ask yet, as a worklist a session can take an item off.
//
//     bun nv gaps                          all three lists, counts and a sample
//     bun nv gaps --coverage               cases per member, per class, thinnest first
//     bun nv gaps --differential           every member with a PHP twin and no oracle case
//     bun nv gaps --errors                 every Fault site no case asserts
//     bun nv gaps --member 'Core\Arr::chunk'   what the corpus already asks of one member
//     bun nv gaps --limit 0                no truncation
//     bun nv gaps --json                   one JSON object instead
//
// Every registered member has a case, so the question a session asks is which claim to pin next, and
// the tree already answers it three ways:
//
// - **A differential gap.** `docs/spec/01-core-library.md`'s **Replaces** column names the PHP
//   built-ins a member subsumes, which is the twin a `--ORACLE--` case needs. A member with a named twin
//   that no case in `tests/differential/` calls is a case whose expected output PHP computes.
// - **A thin class.** Depth is the median number of cases per member, and floor is the worst member,
//   so a big class is not thin merely for being big. A case belongs to a class when it names it, or
//   when it holds one of its values: a written `Owner::member(` that returns an instance of another
//   class attributes the case to that class too, and `->member(` on a class the case already holds
//   does the same, to a fixed point.
// - **An unasserted error path.** Every `Fault::` site in `nvs-stdlib` is a boundary. A case that pins
//   one echoes the message, so a message stem that appears in no case is a boundary nothing asks about.
//
// An entry is a candidate, not a plan: a `Fault::fatal` may be an invariant no program reaches, and a
// member whose PHP twin diverges by decision wants `--ORACLE-DIVERGES--` instead. The member table is
// read positionally out of the `CoreClass` literals: a member belongs to the class whose literal most
// recently opened above it, and a class whose name does not resolve still opens a run, so its members
// are skipped rather than credited to the class above it. `crates/nvs-stdlib/tests/corpus/mod.rs`'s
// `Attribution::new` is the Rust half of the instance attribution, and the two agree by hand.

import { existsSync, readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { ROOT } from "../lib/paths.ts";

export const summary = "what the corpus does not ask yet: nv gaps [--coverage | --differential | --errors | --member NAME] [--limit N] [--json]";

const SPEC = "docs/spec/01-core-library.md";
const STDLIB = "crates/nvs-stdlib/src";
const CONFORMANCE = "tests/conformance";
const DIFFERENTIAL = "tests/differential";
/** Where the tree-wide name-const map is read from: the stdlib, and the runtime crate a stdlib file forwards a carrier's name out of. */
const CONST_ROOTS = [STDLIB, "crates/nvs-runtime/src"];

/** `CoreMethod { name: "chunk", … }`, or `name: HAS` through a `&str` const. */
const METHOD_RE = /CoreMethod\s*\{\s*name:\s*(?:"([^"]+)"|([A-Za-z_][A-Za-z0-9_]*))/gu;
const SYMBOL_RE = /symbol:\s*"([^"]+)"/u;
/** `CoreClass { name: r"Core\Arr" }`, or `name: NAME` through a file-level const. */
const CLASS_RE = /CoreClass\s*\{\s*name:\s*(?:r"([^"]+)"|([A-Za-z_][A-Za-z0-9_]*))/gu;
/** `const NAME: &str = r"Core\Csv";`, or the plain string with its backslash doubled. */
const NAME_CONST_RE = /const\s+([A-Za-z_][A-Za-z0-9_]*)\s*:\s*&str\s*=\s*(?:r"([^"]+)"|"((?:[^"\\]|\\.)*)")/gu;
/** `const NAME: &str = nvs_runtime::CARRIER_CLI_TEXT;`, resolved by the path's last segment. */
const ALIAS_CONST_RE = /const\s+([A-Za-z_][A-Za-z0-9_]*)\s*:\s*&str\s*=\s*(?:[A-Za-z_][A-Za-z0-9_]*::)+([A-Za-z_][A-Za-z0-9_]*)\s*;/gu;
/** `return_ty: CoreTy::Instance(DATETIME_NAME)` or `CoreTy::InstanceAt(ROWS_NAME, &[...])`: the class a member returns an instance of. */
const RETURNS_RE = /return_ty:\s*CoreTy::Instance(?:At)?\(\s*(?:r"([^"]+)"|([A-Za-z_][A-Za-z0-9_]*))\s*[,)]/u;
const IMPL_FN_RE = /\bfn\s+(nvs_core_[a-z0-9_]+)\s*\(/gu;
/** A backticked PHP function name in the **Replaces** column: `strpos` yes, `$s == ""` no. */
const PHP_NAME_RE = /`([a-z_][a-z0-9_]*)`/gu;
/** A heading may name more than one class, which is why the registry decides which one owns a row. */
const SPEC_CLASS_RE = /`(Core(?:\\[A-Za-z]+)*)`/gu;
const FAULT_RE = /Fault::(thrown_as|thrown|fatal)\s*\(/gu;
/** A message literal, read to its first unescaped quote, so a message quoting its operand as `\"` is read whole. */
const QUOTED_RE = /"((?:[^"\\]|\\[\s\S]){10,400})"/u;
const ARROW_RE = /->([a-z][A-Za-z0-9]*)\s*\(/gu;

type Pos = [string, number, string];

/** A median that is a whole list element prints as an int, and one that averages two prints as a float, as Python's does. */
class Median {
  constructor(readonly value: number, readonly float: boolean) {}
}

interface Diff {
  member: string;
  php: string[];
  anchor: string;
}

interface Err {
  anchor: string;
  kind: string;
  fn: string;
  stem: string;
  message: string;
}

interface Coverage {
  class: string;
  members: number;
  cases: number;
  depth: Median;
  floor: number;
  thin: { member: string; cases: number; anchor: string }[];
  uncalled: { member: string; anchor: string }[];
}

/** A file's text with its line endings made `\n`, which is how Python reads it. */
function read(path: string): string {
  return readFileSync(join(ROOT, path), "utf8").replace(/\r\n?/g, "\n");
}

function escapeRe(s: string): string {
  return s.replace(/[.*+?^${}()|[\]\\\/]/g, "\\$&");
}

function byCodePoint(a: string, b: string): number {
  return a < b ? -1 : a > b ? 1 : 0;
}

/** The first `n` characters of `s`, counted as code points. */
function head(s: string, n: number): string {
  return [...s].slice(0, n).join("");
}

/** `s` padded on the right to `n` code points. */
function pad(s: string, n: number): string {
  return s + " ".repeat(Math.max(0, n - [...s].length));
}

/** `count` code points of `s` from UTF-16 offset `from`. */
function cpSlice(s: string, from: number, count: number): string {
  let i = from;
  for (let n = 0; n < count && i < s.length; n++) {
    const c = s.charCodeAt(i);
    i += c >= 0xd800 && c < 0xdc00 && i + 1 < s.length ? 2 : 1;
  }
  return s.slice(from, i);
}

/** Every file under `dir` ending in `ext`, repo-relative, in the order Python sorts its paths: component by component. */
function filesUnder(dir: string, ext: string): string[] {
  if (!existsSync(join(ROOT, dir))) return [];
  const out: string[][] = [];
  const walk = (parts: string[]) => {
    for (const e of readdirSync(join(ROOT, ...parts), { withFileTypes: true })) {
      if (e.isDirectory()) walk([...parts, e.name]);
      else if (e.name.endsWith(ext)) out.push([...parts, e.name]);
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

const cases = (root: string) => filesUnder(root, ".nvst");
const corpus = (root: string) => cases(root).map(read).join("\n");

function lineOf(text: string, offset: number): number {
  let n = 1;
  for (let i = text.indexOf("\n"); i !== -1 && i < offset; i = text.indexOf("\n", i + 1)) n++;
  return n;
}

// ------------------------------------------------------------------------ the registry

/** `{const name: the class name it holds}` for one file, both spellings unescaped. */
function nameConsts(text: string): Map<string, string> {
  const out = new Map<string, string>();
  for (const m of text.matchAll(NAME_CONST_RE)) out.set(m[1]!, m[2] !== undefined ? m[2] : m[3]!.replaceAll("\\\\", "\\"));
  return out;
}

let treeConsts: Map<string, string> | null = null;

/**
 * The name consts a `CoreClass` or `CoreMethod` literal in `text` may spell its `name:` with, the
 * later winning: the tree's unambiguous ones, the parent module's (`db/registry.rs` opens `Core\Db` as
 * `name: NAME` under a `use super::*`), the ones forwarded from another crate by alias, and the file's
 * own. A name declared in more than one file with different values is left out of the tree's map, so
 * a literal spelling an imported const never resolves to whichever file sorts last.
 */
function classConsts(path: string, text: string): Map<string, string> {
  if (treeConsts === null) {
    const seen = new Map<string, Set<string>>();
    for (const root of CONST_ROOTS) {
      for (const p of filesUnder(root, ".rs")) {
        for (const [name, value] of nameConsts(read(p))) {
          if (!seen.has(name)) seen.set(name, new Set());
          seen.get(name)!.add(value);
        }
      }
    }
    treeConsts = new Map();
    for (const [name, v] of seen) if (v.size === 1) treeConsts.set(name, [...v][0]!);
  }
  const slash = path.lastIndexOf("/");
  const sibling = `${path.slice(0, slash)}/mod.rs`;
  const parent = path.slice(slash + 1) !== "mod.rs" && existsSync(join(ROOT, sibling)) ? nameConsts(read(sibling)) : new Map<string, string>();
  const out = new Map(treeConsts);
  for (const [k, v] of parent) out.set(k, v);
  for (const m of text.matchAll(ALIAS_CONST_RE)) if (treeConsts.has(m[2]!)) out.set(m[1]!, treeConsts.get(m[2]!)!);
  for (const [k, v] of nameConsts(text)) out.set(k, v);
  return out;
}

/** The class run each offset in `text` sits in: `[start, class name]` per `CoreClass` literal. */
function classStarts(text: string, consts: Map<string, string>): [number, string][] {
  return [...text.matchAll(CLASS_RE)].map((m) => [m.index!, m[1] || consts.get(m[2]!) || ""]);
}

function ownerAt(starts: [number, string][], offset: number): string {
  let owner = "";
  for (const [pos, name] of starts) {
    if (pos < offset) owner = name;
    else break;
  }
  return owner;
}

/** `Class::member` -> `[file, line, symbol]`, read out of the `CoreClass` literals themselves. */
function registry(): Map<string, Pos> {
  const found = new Map<string, Pos>();
  for (const path of filesUnder(STDLIB, ".rs")) {
    const text = read(path);
    const consts = classConsts(path, text);
    const starts = classStarts(text, consts);
    if (starts.length === 0) continue;
    for (const m of text.matchAll(METHOD_RE)) {
      const owner = ownerAt(starts, m.index!);
      const member = m[1] || consts.get(m[2]!) || "";
      if (!owner || !member) continue;
      const end = m.index! + m[0].length;
      const sym = SYMBOL_RE.exec(cpSlice(text, end, 600));
      found.set(`${owner}::${member}`, [path, lineOf(text, m.index!), sym ? sym[1]! : ""]);
    }
  }
  return found;
}

/** `Class::member` -> the `Core` class an instance of which that member returns. */
function producers(): Map<string, [string, string, string]> {
  const found = new Map<string, [string, string, string]>();
  for (const path of filesUnder(STDLIB, ".rs")) {
    const text = read(path);
    const consts = classConsts(path, text);
    const starts = classStarts(text, consts);
    if (starts.length === 0) continue;
    // A literal's own extent rather than a fixed window: an options bag written inline puts a
    // member's `return_ty` far below its name.
    const methods = [...text.matchAll(METHOD_RE)];
    methods.forEach((m, index) => {
      const owner = ownerAt(starts, m.index!);
      const stop = index + 1 < methods.length ? methods[index + 1]!.index! : text.length;
      const made = RETURNS_RE.exec(text.slice(m.index! + m[0].length, stop));
      if (!owner || !made) return;
      const name = made[1] || consts.get(made[2]!) || "";
      const member = m[1] || consts.get(m[2]!) || "";
      if (name && member) found.set(`${owner}::${member}`, [owner, member, name]);
    });
  }
  return found;
}

/** `nvs_core_arr_range` -> where that function is declared, so an anchor lands on the implementation. */
function symbolLines(): Map<string, [string, number]> {
  const out = new Map<string, [string, number]>();
  for (const path of filesUnder(STDLIB, ".rs")) {
    const text = read(path);
    for (const m of text.matchAll(IMPL_FN_RE)) if (!out.has(m[1]!)) out.set(m[1]!, [path, lineOf(text, m.index!)]);
  }
  return out;
}

function anchorOf(reg: Map<string, Pos>, syms: Map<string, [string, number]>, key: string): string {
  const [path, line, sym] = reg.get(key)!;
  const impl = syms.get(sym);
  return impl ? `${impl[0]}:${impl[1]}` : `${path}:${line}`;
}

// ----------------------------------------------------------------------------- the spec

/** `[classes named by the enclosing heading, member, signature, replaces]`, one per member row. */
function specRows(): [string[], string, string, string][] {
  const rows: [string[], string, string, string][] = [];
  let heading: string[] = [];
  for (const line of read(SPEC).split(/[\n\x0b\x0c\x1c\x1d\x1e\x85\u2028\u2029]/)) {
    if (line.startsWith("## ")) heading = [...line.matchAll(SPEC_CLASS_RE)].map((m) => m[1]!);
    if (!line.startsWith("|") || line.split("|").length - 1 < 4) continue;
    const cells = line.replace(/^\|+|\|+$/g, "").split("|").map((c) => c.trim());
    if (cells.length < 3 || !cells[0]!.startsWith("`") || !cells[1]!.includes("(")) continue;
    rows.push([heading, cells[0]!.replace(/^`+|`+$/g, ""), cells[1]!, cells[2]!]);
  }
  return rows;
}

/** The PHP built-ins a **Replaces** cell names, or nothing when it names none. */
function phpTwins(cell: string): string[] {
  if (cell.toLowerCase().includes("nothing")) return [];
  return [...cell.matchAll(PHP_NAME_RE)].map((m) => m[1]!);
}

// -------------------------------------------------------------------------- the gaps

/** Every `Core\X::member` and `->member(` the given corpus text calls. */
function calledMembers(text: string): Set<string> {
  const out = new Set<string>();
  for (const m of text.matchAll(/(Core(?:\\[A-Za-z]+)+)::([A-Za-z][A-Za-z0-9]*)/gu)) out.add(`${m[1]}::${m[2]}`);
  for (const m of text.matchAll(ARROW_RE)) out.add(`->${m[1]}`);
  return out;
}

/** Members whose spec entry names a PHP built-in and whose name no differential case calls. */
function differentialGaps(): Diff[] {
  const reg = registry();
  const syms = symbolLines();
  const called = calledMembers(corpus(DIFFERENTIAL));
  const out: Diff[] = [];
  for (const [classes, member, , replaces] of specRows()) {
    const twins = phpTwins(replaces);
    if (twins.length === 0) continue;
    // A member the spec names and the registry does not is the registry ratchet's gap, not this one.
    const owner = classes.find((c) => reg.has(`${c}::${member}`)) ?? "";
    if (!owner) continue;
    if (called.has(`${owner}::${member}`) || called.has(`->${member}`)) continue;
    out.push({ member: `${owner}::${member}`, php: twins, anchor: anchorOf(reg, syms, `${owner}::${member}`) });
  }
  return out;
}

/** `Fault::` sites in `nvs-stdlib` whose message stem appears in no case of either suite. */
function errorGaps(): Err[] {
  const seen = `${corpus(CONFORMANCE)}\n${corpus(DIFFERENTIAL)}`;
  const out: Err[] = [];
  for (const path of filesUnder(STDLIB, ".rs")) {
    const text = read(path);
    const owners = [...text.matchAll(IMPL_FN_RE)].map((m): [number, string] => [m.index!, m[1]!]);
    for (const m of text.matchAll(FAULT_RE)) {
      const quoted = QUOTED_RE.exec(cpSlice(text, m.index! + m[0].length, 700));
      if (!quoted) continue;
      // A trailing backslash continues a Rust literal and eats the indent after it, and then each
      // remaining escape is the one character it spells, in one pass so `\\"` is a backslash and a quote.
      let message = quoted[1]!.replace(/\\\n\s*/gu, "");
      message = message.replace(/\\(["\\])/gu, "$1");
      const stem = message.split(/[{}]/)[0]!.trim();
      if ([...stem].length < 14 || seen.includes(stem)) continue;
      out.push({
        anchor: `${path}:${lineOf(text, m.index!)}`,
        kind: m[1]!,
        fn: ownerAt(owners, m.index!),
        // `stem` is the literal run before the first format hole, which is all a case's frozen output
        // can be matched against; `message` is what a reader needs.
        stem,
        message: message.split(/\s+/u).filter(Boolean).join(" "),
      });
    }
  }
  // `thrown` first: a program can reach it and a case can catch it, where most `fatal` rows are
  // argument type-guards the checker already refuses.
  const order: Record<string, number> = { thrown: 0, thrown_as: 1, fatal: 2 };
  out.sort((a, b) => (order[a.kind] ?? 3) - (order[b.kind] ?? 3) || byCodePoint(a.anchor, b.anchor));
  return out;
}

function median(sorted: number[]): Median {
  if (sorted.length === 0) return new Median(0, true);
  const mid = sorted.length >> 1;
  return sorted.length % 2 === 1 ? new Median(sorted[mid]!, false) : new Median((sorted[mid - 1]! + sorted[mid]!) / 2, true);
}

/** Per `Core` class: registered members, the cases that handle it, and which members none calls. */
function coverage(): Coverage[] {
  const reg = registry();
  const syms = symbolLines();
  const perClass = new Map<string, string[]>();
  for (const key of reg.keys()) {
    const cut = key.lastIndexOf("::");
    const owner = key.slice(0, cut);
    if (!perClass.has(owner)) perClass.set(owner, []);
    perClass.get(owner)!.push(key.slice(cut + 2));
  }
  const builds = new Map<string, Map<string, string>>();
  for (const [owner, member, made] of producers().values()) {
    if (!builds.has(owner)) builds.set(owner, new Map());
    builds.get(owner)!.set(member, made);
  }
  // `Core\Time` matches `Core\Time::now` and `Core\Time $t`, never `Core\Time\Duration`.
  const owns = [...perClass.keys()].map((name): [string, RegExp] => [name, new RegExp(`${escapeRe(name)}(?![A-Za-z0-9_\\\\])`, "u")]);

  const holders = (text: string): Set<string> => {
    const held = new Set(owns.filter(([, pattern]) => pattern.test(text)).map(([name]) => name));
    for (const m of text.matchAll(/(Core(?:\\[A-Za-z][A-Za-z0-9]*)*)::([A-Za-z][A-Za-z0-9]*)/gu)) {
      const made = builds.get(m[1]!)?.get(m[2]!);
      if (made) held.add(made);
    }
    const arrows = new Set([...text.matchAll(ARROW_RE)].map((m) => m[1]!));
    for (let growing = true; growing; ) {
      growing = false;
      for (const name of [...held]) {
        for (const [member, made] of builds.get(name) ?? []) {
          if (arrows.has(member) && !held.has(made)) {
            held.add(made);
            growing = true;
          }
        }
      }
    }
    return held;
  };

  const texts = cases(CONFORMANCE).map(read);
  const handled = texts.map(holders);
  const out: Coverage[] = [];
  for (const [owner, members] of perClass) {
    const mine = texts.filter((_, i) => handled[i]!.has(owner));
    const asked = new Map(members.map((m) => [m, 0]));
    const namedRe = new RegExp(`${escapeRe(owner)}::([A-Za-z][A-Za-z0-9]*)`, "gu");
    for (const text of mine) {
      const named = new Set([...text.matchAll(namedRe)].map((m) => m[1]!));
      for (const m of text.matchAll(ARROW_RE)) named.add(m[1]!);
      for (const member of named) if (asked.has(member)) asked.set(member, asked.get(member)! + 1);
    }
    const anchor = (member: string) => anchorOf(reg, syms, `${owner}::${member}`);
    const names = [...asked.keys()];
    const counts = [...asked.values()].sort((a, b) => a - b);
    const order = [...names].sort((a, b) => asked.get(a)! - asked.get(b)! || byCodePoint(a, b));
    out.push({
      class: owner,
      members: members.length,
      cases: mine.length,
      depth: median(counts),
      floor: counts.length > 0 ? counts[0]! : 0,
      thin: order.slice(0, 3).map((m) => ({ member: m, cases: asked.get(m)!, anchor: anchor(m) })),
      uncalled: names.filter((m) => asked.get(m) === 0).sort(byCodePoint).map((m) => ({ member: m, anchor: anchor(m) })),
    });
  }
  out.sort((a, b) => a.depth.value - b.depth.value || a.floor - b.floor || b.members - a.members);
  return out;
}

/** Which cases already call one member, so a session can see what is asked before adding. */
function memberReport(name: string): string[] {
  const lines: string[] = [];
  for (const [label, root] of [["conformance", CONFORMANCE], ["differential", DIFFERENTIAL]] as const) {
    for (const path of cases(root)) {
      const text = read(path);
      if (!text.includes(name)) continue;
      const all = text.split("\n");
      const at = all.findIndex((ln) => ln.trim() === "--TEST--");
      const title = at !== -1 && at + 1 < all.length ? all[at + 1]!.trim() : "";
      lines.push(`  ${pad(label, 12)} ${path}`);
      if (title) lines.push(`               ${head(title, 110)}`);
    }
  }
  return lines;
}

// ------------------------------------------------------------------------------- driver

/** JSON the way Python's `json.dumps(..., indent=1)` writes it: non-ASCII escaped, and a float median as `2.0`. */
function pythonJson(value: unknown): string {
  const text = JSON.stringify(value, (_, v) => (v instanceof Median ? `\u0000F:${v.float && Number.isInteger(v.value) ? `${v.value}.0` : v.value}` : v), 1);
  return text
    .replace(/"\\u0000F:([^"]*)"/g, "$1")
    .replace(/[\u007f-\uffff]/g, (c) => `\\u${c.charCodeAt(0).toString(16).padStart(4, "0")}`);
}

function show<T>(rows: T[], limit: number, render: (row: T) => string): void {
  const shown = limit <= 0 ? rows : rows.slice(0, limit);
  for (const row of shown) console.log(render(row));
  if (shown.length < rows.length) console.log(`  ... and ${rows.length - shown.length} more (--limit 0 for all)`);
}

const USAGE = "usage: bun nv gaps [--differential] [--errors] [--coverage] [--member MEMBER] [--limit LIMIT] [--json]";

export async function run(args: string[]): Promise<number> {
  const flags = new Set<string>();
  let member = "";
  let limit = 25;
  for (let i = 0; i < args.length; i++) {
    const a = args[i]!;
    const eq = /^(--member|--limit)=(.*)$/s.exec(a);
    const opt = eq ? eq[1]! : a;
    if (opt === "--member" || opt === "--limit") {
      const value = eq ? eq[2]! : args[++i];
      if (value === undefined) {
        console.error(`${USAGE}\nnv gaps: ${opt} takes a value`);
        return 2;
      }
      if (opt === "--member") member = value;
      else if (/^\s*[-+]?\d+\s*$/.test(value)) limit = Number(value);
      else {
        console.error(`${USAGE}\nnv gaps: --limit takes a number, not ${JSON.stringify(value)}`);
        return 2;
      }
    } else if (["--differential", "--errors", "--coverage", "--json"].includes(a)) flags.add(a);
    else {
      console.error(USAGE);
      return 2;
    }
  }

  if (!existsSync(join(ROOT, SPEC))) {
    console.error(`missing ${SPEC}`);
    return 2;
  }

  if (member) {
    const found = memberReport(member);
    console.log(`== ${member}`);
    console.log(found.length > 0 ? found.join("\n") : "  no case calls it");
    return 0;
  }

  const both = !(flags.has("--differential") || flags.has("--errors") || flags.has("--coverage"));
  const diff = both || flags.has("--differential") ? differentialGaps() : [];
  const errs = both || flags.has("--errors") ? errorGaps() : [];
  const cov = both || flags.has("--coverage") ? coverage() : [];

  if (flags.has("--json")) {
    console.log(pythonJson({ differential: diff, errors: errs, coverage: cov }));
    return 0;
  }

  if (both || flags.has("--coverage")) {
    const thin = cov.filter((r) => r.uncalled.length > 0);
    console.log(`== CONFORMANCE DEPTH BY CLASS  (${cov.length} classes, thinnest first; ${thin.length} with a member no case calls)`);
    console.log("-- DEPTH is the MEDIAN cases per member and FLOOR its worst member, so a big class");
    console.log("-- is not thin merely for being big -- take the group from the members named at the");
    console.log("-- right, which are the three each class asks least, with their anchors.");
    show(cov, limit, (r) =>
      `  ${r.depth.value.toFixed(1).padStart(5)}${String(r.floor).padStart(6)}${String(r.cases).padStart(7)}${String(r.members).padStart(9)}` +
      `   ${pad(r.class, 22)}` +
      (r.uncalled.length > 0
        ? ` no case calls ${r.uncalled.slice(0, 3).map((u) => `${u.member} ${u.anchor}`).join(", ")}`
        : ` ${r.thin.map((t) => `${t.member} ${t.cases}`).join(", ")}`));
    console.log("     ^depth ^floor ^cases ^members  ^thinnest members, and their case counts");
    console.log();
  }

  if (both || flags.has("--differential")) {
    const have = cases(DIFFERENTIAL).length;
    console.log(`== DIFFERENTIAL GAP  (${diff.length} members with a PHP twin and no oracle case; the suite holds ${have})`);
    console.log("-- the twin is the spec's Replaces column; PHP computes the expectation, so a case");
    console.log("-- here needs no frozen output. Never in tests/conformance/ (conventions.md).");
    show(diff, limit, (r) => `  ${pad(r.member, 34)} <- ${pad(head(r.php.join(", "), 44), 46)} ${r.anchor}`);
    console.log();
  }

  if (both || flags.has("--errors")) {
    const byKind = new Map<string, number>();
    for (const e of errs) byKind.set(e.kind, (byKind.get(e.kind) ?? 0) + 1);
    const kinds = [...byKind].sort(([a], [b]) => byCodePoint(a, b)).map(([k, v]) => `${v} ${k}`).join(", ");
    console.log(`== UNASSERTED ERROR PATHS  (${errs.length}: ${kinds})`);
    console.log("-- a `Fault::fatal` may be an internal invariant no program can reach; a");
    console.log("-- `thrown` is a boundary a case can catch and echo. Judge before writing.");
    show(errs, limit, (r) => `  ${pad(r.anchor, 36)} ${pad(r.kind, 10)} ${head(r.message, 76)}`);
  }

  return 0;
}
