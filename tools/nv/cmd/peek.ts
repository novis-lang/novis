// `bun nv peek <target>...`: reads many places in many files in one call. A target is a path and a
// locator, and `TARGET_FORMS` below is the one home of their spelling. A path may be a glob, and then
// the locator runs against every file it matches.
//
// `--locate <symbol>...` prints `file:line  <the defining line>` for each symbol and no bodies: it is
// what a handoff's anchors are made of. `--outline <path>...` prints one line per `fn`, `struct`,
// `enum`, `trait`, `impl` and `mod` seam (for Python, `def` and `class`), with the line it starts on
// and how long it runs; every line is a `:@name` target. Top-level seams only unless `--deep`.
//
// Nothing is truncated silently. A target that produced nothing says so on its own line, a whole file
// over `--max-lines` is refused with its size, and the footer says what the call printed. The footer
// also carries advice read from a per-session ledger, `.agent-tmp/peek-ledger.json`, which
// `tools/peek.py` shares: a file fetched `REFETCH_NOTE_AT` times is pointed at `--outline`, and a run
// of `SOLO_NOTE_AT` one-target calls is told that one call takes many targets. The advice never
// changes the exit status, and a ledger that cannot be read or written is no advice.

import { Glob } from "bun";
import { existsSync, mkdirSync, readdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { dirname, isAbsolute, join, resolve } from "node:path";
import { rel, ROOT } from "../lib/paths.ts";

export const summary = "read many places in one call: nv peek <path:locator>... | --locate <symbol>... | --outline <path>...";

export const TARGET_FORMS = `Target forms, all of them \`path\` followed by \`:\` and a locator:

    path                  the whole file (refused over --max-lines, which says so and stops)
    path:120-160          those lines
    path:120+30           30 lines starting at 120
    path:@name            the line that *defines* \`name\`, plus --window lines of context
    path:re:regex         every matching line and nothing around it, \`grep -n\` style -- a match
                          on a heading or a \`//!\` line almost always wants context
    path:re:regex:3       every matching line with 3 lines of context either side; \`--context 3\`
                          says that for every target in the call, and a suffix wins over it
    path:/regex/          the same, in the familiar spelling -- but Git Bash on Windows
                          rewrites a leading \`/\` into a Win32 path before this tool sees it,
                          so prefer \`re:\` there
    path:"## Heading"     a markdown heading and its body, to the next same-or-higher heading
    rule:topic/slug       that rule's fragment -- the citation token itself, as a target
    rule:topic            the whole generated chapter (usually too big; name the rule instead)`;

/** Source text here tokenizes at about this many bytes per token. It prices a call; it refuses none. */
const BYTES_PER_TOKEN = 2.5;
const DEFAULT_WINDOW = 12;
/** AGENTS.md rule 3's "whole file under ~400 lines". */
const DEFAULT_MAX_LINES = 400;

/**
 * What counts as *defining* a name, across the languages this repository holds, most specific first.
 * A hit here beats a plain mention, which is the difference between `--locate` and a `grep`. `[^\n]`
 * stands where Python's `.` would, because a line of a CRLF file keeps its `\r`.
 */
const DEFINITION = [
  String.raw`^\s*(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?(?:const\s+|unsafe\s+|extern\s+"[^"]*"\s+)*fn\s+NAME\b`,
  String.raw`^\s*(?:pub(?:\([^)]*\))?\s+)?(?:struct|enum|trait|union|type|mod|macro_rules!)\s+NAME\b`,
  String.raw`^\s*(?:pub(?:\([^)]*\))?\s+)?(?:static|const)\s+NAME\b`,
  String.raw`^\s*impl\b[^\n]*\bNAME\b`,
  String.raw`^\s*NAME\s*[{(,]`, // an enum variant, which has no other declarer
  String.raw`^\s*(?:def|class)\s+NAME\b`, // python
  String.raw`^\s*NAME\s*=`, // a top-level binding, python or toml
  String.raw`^#+\s*[^\n]*\bNAME\b`, // a markdown heading naming it
];

const SKIP_DIRS = new Set([".git", "target", "node_modules", "__pycache__", ".agent-tmp", ".loop"]);
const TEXT_SUFFIXES = new Set([".rs", ".py", ".md", ".toml", ".nvs", ".nvst", ".txt", ".json", ".yml",
  ".yaml", ".sh", ".ps1", ".snap", ".php", ".lock", ".cfg", ".ini"]);

/** A Rust seam. A method inside an `impl` is indented, which is how `--deep` tells it apart. */
const OUTLINE = new RegExp(
  String.raw`^(\s*)(` +
    String.raw`(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?(?:const\s+|unsafe\s+|extern\s+"[^"]*"\s+)*fn\s+\w+` +
    String.raw`|(?:pub(?:\([^)]*\))?\s+)?(?:struct|enum|trait|union|mod)\s+\w+` +
    String.raw`|impl(?:<[^>]*>)?\s+[^{;]+` +
    ")",
);

/**
 * A Python seam, kept apart from `OUTLINE` because `class Adder` is also Novis, and the Rust here holds
 * Novis fixtures in string literals that would otherwise outline as test classes.
 */
const OUTLINE_PY = /^(\s*)((?:async\s+)?(?:def|class)\s+\w+)/;

const LEDGER = join(ROOT, ".agent-tmp", "peek-ledger.json");
/** Fetches of one file in one session past which the footer points at `--outline`. */
const REFETCH_NOTE_AT = 3;
/**
 * One-target calls in a row past which the footer says so. A run of three can be reads each chosen by
 * what the last one printed; by four, a session is walking a file set it already knew.
 */
const SOLO_NOTE_AT = 4;
/** Idle time that ends a session when no loop log names one. */
const LEDGER_IDLE_SECONDS = 2 * 60 * 60;

/** `rule:<topic>/<slug>`, backticks optional, since a doc comment writes the token inside them. */
const RULE_TARGET = /^`?rule:([a-z0-9][a-z0-9-]*)(?:\/([a-z0-9][a-z0-9-]*))?`?$/;

const out: string[] = [];
const say = (line = "") => out.push(line);

/** The length Python gives `s`: code points, not UTF-16 units, so the footer's count matches `peek.py`'s. */
function len(s: string): number {
  let n = s.length;
  for (let i = 0; i < s.length; i++) {
    const c = s.charCodeAt(i);
    if (c >= 0xd800 && c <= 0xdbff) n--;
  }
  return n;
}

function escape(s: string): string {
  return s.replace(/[.*+?^${}()|[\]\\/-]/g, "\\$&");
}

function thousands(n: number): string {
  return Math.round(n).toString().replace(/\B(?=(\d{3})+(?!\d))/g, ",");
}

function readLines(path: string): string[] | null {
  try {
    return readFileSync(path, "utf8").split("\n");
  } catch {
    return null;
  }
}

function isFile(path: string): boolean {
  try {
    return statSync(path).isFile();
  } catch {
    return false;
  }
}

/** A repo-relative, forward-slash path for `path`, or `path` itself when it is outside the tree. */
function shown(path: string): string {
  const r = rel(resolve(path));
  return r.startsWith("..") || isAbsolute(r) ? path.split("\\").join("/") : r;
}

/** A file suffix the way Python's `Path.suffix` reads one: none for `.gitignore` or `name.`. */
function suffix(name: string): string {
  const i = name.lastIndexOf(".");
  return i <= 0 || i === name.length - 1 ? "" : name.slice(i);
}

/** Every text file under the tree in the order `os.walk` gives them: a directory's files, then its subdirectories. */
function* walkRepo(dir: string = ROOT): Generator<string> {
  let entries;
  try {
    entries = readdirSync(dir, { withFileTypes: true });
  } catch {
    return;
  }
  const dirs: string[] = [];
  for (const e of entries) {
    const p = join(dir, e.name);
    if (e.isDirectory()) {
      if (!SKIP_DIRS.has(e.name)) dirs.push(p);
    } else if (e.isSymbolicLink() && linksToDir(p)) {
      // a linked directory is listed and never entered, as `os.walk` does
    } else if (TEXT_SUFFIXES.has(suffix(e.name))) yield p;
  }
  for (const d of dirs) yield* walkRepo(d);
}

function linksToDir(p: string): boolean {
  try {
    return statSync(p).isDirectory();
  } catch {
    return false;
  }
}

/** `rule:types/conversion` -> the fragment's path, `rule:types` -> the chapter's, anything else -> null. */
function ruleTarget(spec: string): string | null {
  const m = RULE_TARGET.exec(spec.trim());
  if (!m) return null;
  return m[2] ? `docs/rules/${m[1]}/${m[2]}.md` : `docs/rules/${m[1]}.md`;
}

/** `path:locator` -> [path, locator], split on the first colon past a drive letter, since a locator may hold colons. */
function splitTarget(spec: string): [string, string | null] {
  const start = spec.length > 1 && spec[1] === ":" && /[A-Za-z]/.test(spec[0]!) ? 2 : 0;
  const i = spec.indexOf(":", start);
  if (i < 0 || i === spec.length - 1) return [spec, null];
  return [spec.slice(0, i), spec.slice(i + 1)];
}

/** Python's order for a list of paths: part by part, and case-folded on Windows. */
function pathOrder(a: string, b: string): number {
  const fold = process.platform === "win32" ? (s: string) => s.toLowerCase() : (s: string) => s;
  const pa = a.split(/[\\/]/).filter((p) => p !== "").map(fold);
  const pb = b.split(/[\\/]/).filter((p) => p !== "").map(fold);
  for (let i = 0; i < Math.min(pa.length, pb.length); i++) {
    if (pa[i] !== pb[i]) return pa[i]! < pb[i]! ? -1 : 1;
  }
  return pa.length - pb.length;
}

function globFrom(base: string, pattern: string): string[] {
  const parts = pattern.split(/[\\/]/);
  const lit: string[] = [];
  while (parts.length > 1 && !/[*?[]/.test(parts[0]!)) lit.push(parts.shift()!);
  if (!/[*?[]/.test(parts[0] ?? "")) {
    const p = resolve(base, pattern);
    return isFile(p) ? [p] : [];
  }
  const cwd = resolve(base, lit.length ? lit.join("/") || "/" : ".");
  try {
    return [...new Glob(parts.join("/")).scanSync({ cwd, onlyFiles: true, dot: false })].map((f) => join(cwd, f));
  } catch {
    return [];
  }
}

/** A path or a glob -> the files it names, from the working directory or else from the root, sorted. */
function expand(pattern: string): string[] {
  if (isFile(pattern)) return [pattern];
  let found = globFrom(process.cwd(), pattern);
  if (found.length === 0) found = globFrom(ROOT, pattern);
  return found.filter(isFile).sort(pathOrder);
}

/** The line index that defines `name`, or the first that mentions it when none defines it. */
function locateDefinition(lines: string[], name: string): number | null {
  for (const t of DEFINITION) {
    const rx = new RegExp(t.replaceAll("NAME", escape(name)));
    const i = lines.findIndex((l) => rx.test(l));
    if (i >= 0) return i;
  }
  const rx = new RegExp(`\\b${escape(name)}\\b`);
  const i = lines.findIndex((l) => rx.test(l));
  return i >= 0 ? i : null;
}

/**
 * A markdown heading and its body, to the next heading at the same or a higher level. A heading
 * matches on its words, so `## 4`, `4` and `4. What a write checks` find the same section.
 */
function headingSpan(lines: string[], wanted: string): [number, number] | null {
  const norm = (t: string) =>
    t.replace(/[`*_#]/g, "").trim().toLowerCase()
      .replace(/^(\d+[a-z]?)\s*[.)]?\s*/, "$1 ")
      .replace(/\s+/g, " ").trim();
  const key = norm(wanted);
  const heads: [number, number, string][] = [];
  lines.forEach((line, i) => {
    const m = /^(#{1,6})\s+([^\n]*)$/.exec(line);
    if (m) heads.push([i, m[1]!.length, m[2]!]);
  });
  for (let n = 0; n < heads.length; n++) {
    const [idx, level, title] = heads[n]!;
    const t = norm(title);
    if (t === key || t.startsWith(key + " ") || t.startsWith(key + ".")) {
      const later = heads.slice(n + 1).find(([, l]) => l <= level);
      return [idx, later ? later[0] : lines.length];
    }
  }
  return null;
}

function parseRange(locator: string, total: number): [number, number] | null {
  let m = /^(\d+)-(\d+)$/.exec(locator);
  if (m) return [Math.max(0, +m[1]! - 1), Math.min(total, +m[2]!)];
  m = /^(\d+)\+(\d+)$/.exec(locator);
  if (m) return [Math.max(0, +m[1]! - 1), Math.min(total, +m[1]! - 1 + +m[2]!)];
  m = /^(\d+)$/.exec(locator);
  if (m) return [Math.max(0, +m[1]! - 1), Math.min(total, +m[1]!)];
  return null;
}

/** One numbered window under a header naming what asked for it. Returns the characters printed. */
function emitSpan(path: string, lines: string[], start: number, end: number, note: string): number {
  const header = `===== ${shown(path)}:${start + 1}-${end}  ${note}`;
  say(header);
  const width = String(end).length;
  let printed = len(header);
  for (let i = start; i < Math.min(end, lines.length); i++) {
    const b = `${String(i + 1).padStart(width)}  ${lines[i]}`;
    say(b);
    printed += len(b) + 1;
  }
  say();
  return printed;
}

/** `grep -n` with `context` lines either side, overlapping windows printed once. */
function emitMatches(path: string, lines: string[], rx: RegExp, source: string, context: number): number {
  const hits = lines.flatMap((l, i) => (rx.test(l) ? [i] : []));
  if (hits.length === 0) return 0;
  const spans: [number, number][] = [];
  for (const i of hits) {
    const a = Math.max(0, i - context);
    const b = Math.min(lines.length, i + context + 1);
    const last = spans[spans.length - 1];
    if (last && a <= last[1]) last[1] = Math.max(last[1], b);
    else spans.push([a, b]);
  }
  const header = `===== ${shown(path)}  /${source}/  ${hits.length} hit(s)`;
  say(header);
  let printed = len(header);
  const width = String(lines.length).length;
  spans.forEach(([a, b], n) => {
    if (n) {
      say("  --");
      printed += 4;
    }
    for (let i = a; i < b; i++) {
      const line = `${String(i + 1).padStart(width)}  ${lines[i]}`;
      say(line);
      printed += len(line) + 1;
    }
  });
  say();
  return printed;
}

/** One target -> [characters printed, targets that produced nothing]. */
function peekOne(spec: string, window: number, maxLines: number, context: number): [number, number] {
  const asRule = ruleTarget(spec);
  const [pattern, locator] = splitTarget(asRule ?? spec);
  const files = expand(pattern);
  if (files.length === 0) {
    say(asRule
      ? `===== ${spec}  -- NO SUCH RULE (looked in ${pattern}). \`bun nv rules --list\` is every rule id.`
      : `===== ${pattern}  -- NO SUCH FILE`);
    say();
    return [0, 1];
  }

  // Over a glob, a file with no hit is the normal case: a sweep is empty only when nothing matched.
  const sweep = files.length > 1;
  let printed = 0;
  let empty = 0;
  let sweepHits = 0;
  for (const path of files) {
    const lines = readLines(path);
    if (lines === null) {
      say(`===== ${shown(path)}  -- UNREADABLE`);
      say();
      empty++;
      continue;
    }
    if (lines.length > 0 && lines[lines.length - 1] === "") lines.pop();

    if (locator === null) {
      if (lines.length > maxLines) {
        const r = shown(path);
        say(`===== ${r}  -- ${lines.length} lines, over --max-lines ${maxLines}. Name a region: ` +
          `\`${r}:1-${maxLines}\`, \`${r}:@symbol\`, or \`${r}:/regex/\`.`);
        say();
        empty++;
        continue;
      }
      printed += emitSpan(path, lines, 0, lines.length, "whole file");
      continue;
    }

    if (locator.startsWith("@")) {
      const name = locator.slice(1);
      const idx = locateDefinition(lines, name);
      if (idx === null) {
        if (!sweep) {
          say(`===== ${shown(path)}  -- no definition or mention of \`${name}\``);
          say();
          empty++;
        }
        continue;
      }
      sweepHits++;
      printed += emitSpan(path, lines, Math.max(0, idx - 2), Math.min(lines.length, idx + window), `@${name}`);
      continue;
    }

    // `/re/` is the familiar spelling and `re:` the one that survives Git Bash, which rewrites an
    // argument starting with a slash into a Win32 path.
    const m = /^\/([\s\S]*)\/(\d*)$/.exec(locator) ?? /^re:([\s\S]*?)(?::(\d+))?$/.exec(locator);
    if (m) {
      let rx: RegExp;
      try {
        rx = new RegExp(m[1]!);
      } catch (e) {
        say(`===== ${pattern}  -- bad regex /${m[1]}/: ${(e as Error).message}`);
        say();
        return [printed, empty + 1];
      }
      // A `:0` suffix is an answer, so the suffix wins whenever it was written at all.
      const n = emitMatches(path, lines, rx, m[1]!, m[2] ? Number(m[2]) : context);
      printed += n;
      if (n) sweepHits++;
      else if (!sweep) empty++;
      continue;
    }

    const range = parseRange(locator, lines.length);
    if (range) {
      printed += emitSpan(path, lines, range[0], range[1], "lines");
      continue;
    }

    const heading = headingSpan(lines, locator.replace(/^["']+|["']+$/g, ""));
    if (heading) {
      printed += emitSpan(path, lines, heading[0], heading[1], `§ ${locator}`);
      continue;
    }

    say(`===== ${shown(path)}  -- no heading matching ${pyRepr(locator)}, and it is not a line range. ` +
      `Forms: 120-160, 120+30, @symbol, re:pattern (re:pattern:3 for context), "## Heading".`);
    say();
    empty++;
  }

  if (sweep && !sweepHits) {
    say(`===== ${pattern}  -- ${files.length} file(s) matched the glob, none matched ${pyRepr(locator ?? "")}`);
    say();
    empty++;
  }
  return [printed, empty];
}

/** `repr` of a string the way Python writes it, since a message quotes the locator that way. */
function pyRepr(s: string): string {
  const q = s.includes("'") && !s.includes('"') ? '"' : "'";
  const body = s.replace(/\\/g, "\\\\").replace(/\n/g, "\\n").replace(/\r/g, "\\r").replace(/\t/g, "\\t");
  return q + (q === "'" ? body.replace(/'/g, "\\'") : body) + q;
}

/** Symbols in, `file:line  <the defining line>` out. Returns how many were not found. */
function locate(names: string[], scope: string | null): number {
  const files = scope ? expand(scope) : [...walkRepo()];
  const compiled = new Map(names.map((n) => [n, DEFINITION.map((t) => new RegExp(t.replaceAll("NAME", escape(n))))]));
  const found = new Map<string, [string, number, string][]>(names.map((n) => [n, []]));
  for (const path of files) {
    let text: string;
    try {
      text = readFileSync(path, "utf8");
    } catch {
      continue;
    }
    const lines = text.split("\n");
    for (const name of names) {
      if (!text.includes(name)) continue;
      for (const rx of compiled.get(name)!) {
        const hit = lines.findIndex((l) => rx.test(l));
        if (hit >= 0) {
          found.get(name)!.push([shown(path), hit + 1, lines[hit]!.trim()]);
          break;
        }
      }
    }
  }
  let missing = 0;
  for (const name of names) {
    const hits = found.get(name)!;
    if (hits.length === 0) {
      say(`${name}: NOT FOUND`);
      missing++;
      continue;
    }
    for (const [where, line, body] of hits.slice(0, 6)) say(`${where}:${line}  ${[...body].slice(0, 110).join("")}`);
    if (hits.length > 6) say(`  ... and ${hits.length - 6} more definition(s) of ${name}`);
  }
  return missing;
}

/** One line per seam of each file, with the line it starts on and how many lines it runs for. */
function outline(patterns: string[], deep: boolean): number {
  let seen = 0;
  let nested = 0;
  for (const pattern of patterns) {
    for (const path of expand(pattern)) {
      const lines = readLines(path);
      if (lines === null) {
        say(`===== ${pattern}  -- cannot read`);
        continue;
      }
      const rx = path.endsWith(".py") ? OUTLINE_PY : OUTLINE;
      const hits: [number, number, string][] = [];
      lines.forEach((line, i) => {
        const m = rx.exec(line);
        const t = line.trimStart();
        if (m && !t.startsWith("//") && !t.startsWith("#") && !t.startsWith("*")) hits.push([i + 1, m[1]!.length, m[2]!.trim()]);
      });
      const hidden = deep ? 0 : hits.filter((h) => h[1] !== 0).length;
      say(`===== ${shown(path)}  ${thousands(lines.length)} lines, ${hits.length} seam(s)` +
        (hidden ? `, ${hidden} nested one(s) not shown` : ""));
      hits.forEach(([n, indent, sig], i) => {
        if (!deep && indent !== 0) return;
        const next = i + 1 < hits.length ? hits[i + 1]![0] : lines.length + 1;
        say(`${String(n).padStart(6)}  ${"  ".repeat(Math.min(Math.floor(indent / 4), 3))}${sig}   [${next - n} lines]`);
      });
      say();
      seen++;
      nested += hidden;
    }
  }
  if (!seen) {
    say("-- outline: nothing matched");
    return 1;
  }
  say("-- outline: every line above is a `:@name` target that lands on that seam exactly.");
  if (nested) {
    say(`-- ${nested} seam(s) nested inside an \`impl\` are not shown; \`--deep\` prints them, and`);
    say("   `bun nv peek --locate <name>` finds one by name without printing any.");
  }
  return 0;
}

/** Under the loop driver each session has its own transcript, so the newest log names it. */
function sessionKey(): string {
  const logs = join(ROOT, ".loop", "logs");
  try {
    let newest: [string, number] | null = null;
    for (const name of readdirSync(logs)) {
      if (!name.endsWith(".log")) continue;
      const t = statSync(join(logs, name)).mtimeMs;
      if (!newest || t > newest[1]) newest = [name, t];
    }
    if (newest) return newest[0];
  } catch {
    // no loop logs: an interactive session, which the idle timer separates
  }
  return "interactive";
}

interface Ledger {
  session: string;
  at: number;
  files: Record<string, number>;
  solo: number;
}

/**
 * This call counted into the session's ledger, and the advice lines that count now earns. The two
 * tallies share one read-modify-write, so neither drops the other's.
 */
function noteReads(targets: string[]): string[] {
  const now = Date.now() / 1000;
  const key = sessionKey();
  const record: Ledger = { session: key, at: now, files: {}, solo: 0 };
  try {
    if (existsSync(LEDGER)) {
      const held = JSON.parse(readFileSync(LEDGER, "utf8"));
      const fresh = held && typeof held === "object" && held.session === key && now - Number(held.at ?? 0) < LEDGER_IDLE_SECONDS;
      if (fresh && held.files && typeof held.files === "object" && !Array.isArray(held.files)) {
        record.files = held.files;
        record.solo = Number(held.solo ?? 0) || 0;
      }
    }
  } catch {
    // an unreadable ledger starts a fresh one
  }

  const lines: string[] = [];
  // A glob is one target that reads a whole crate, so it is not a solo call.
  const globbed = targets.some((t) => t.includes("*") || t.includes("?"));
  record.solo = targets.length > 1 || globbed ? 0 : record.solo + 1;
  if (record.solo >= SOLO_NOTE_AT) {
    lines.push(`-- that is ${record.solo} calls in a row carrying one target. This tool takes as many as you ` +
      "have questions, and a session's wall clock is very nearly its round-trip count: " +
      '`nv peek a.rs:120-160 b.rs:@sym c.md:"## 4"` is one call, not three.');
    record.solo = 0;
  }

  const paths = [...new Set(targets.map((t) => splitTarget(ruleTarget(t) ?? t)[0]))]
    .filter((p) => !p.includes("*") && !p.includes("?"))
    .sort();
  const hot: [string, number][] = [];
  for (const p of paths) {
    const name = p.split("\\").join("/");
    record.files[name] = Number(record.files[name] ?? 0) + 1;
    if (record.files[name]! >= REFETCH_NOTE_AT) hot.push([name, record.files[name]!]);
  }
  try {
    mkdirSync(dirname(LEDGER), { recursive: true });
    writeFileSync(LEDGER, JSON.stringify(record), "utf8");
  } catch {
    // advice only: a ledger that cannot be written is no advice
  }

  if (hot.length) {
    const [worst, count] = hot.reduce((a, b) => (b[1] > a[1] ? b : a));
    lines.push(`-- you have now fetched ${worst} ${count} times this session. ` +
      `\`bun nv peek --outline ${worst}\` prints its seams once, and every line of ` +
      "that is a `:@name` target that lands first time.");
  }
  return lines;
}

const HELP = `usage: bun nv peek [options] <path:locator>...
       bun nv peek --locate <symbol>... [--in <glob>]
       bun nv peek --outline <path>... [--deep]

${TARGET_FORMS}

options:
  --locate SYMBOL...  symbols in, file:line out, no bodies
  --outline PATH...   one line per fn/struct/impl seam, with its span -- the map of a big file
  --deep              with --outline, include seams nested inside an impl
  --in GLOB           restrict --locate to these files
  --window N          lines of body after an @symbol hit, and nothing else (default ${DEFAULT_WINDOW});
                      a re: target takes --context
  --context N, -C N   lines either side of every re: match (default 0); a target's own
                      re:pattern:N wins over this
  --max-lines N       refuse a whole file over this many lines (default ${DEFAULT_MAX_LINES})
  --quiet             omit the cost footer`;

/** A bad argument: the message, and the target forms, which answer what the wrong flag was asking. */
function refuse(message: string): number {
  console.error(`nv peek: error: ${message}\n\n${TARGET_FORMS}`);
  return 2;
}

interface Options {
  targets: string[];
  locate: string[] | null;
  outline: string[] | null;
  deep: boolean;
  scope: string | null;
  window: number;
  context: number;
  maxLines: number;
  quiet: boolean;
  help: boolean;
}

function parse(args: string[]): Options | string {
  const o: Options = { targets: [], locate: null, outline: null, deep: false, scope: null, window: DEFAULT_WINDOW,
    context: 0, maxLines: DEFAULT_MAX_LINES, quiet: false, help: false };
  const ints: Record<string, "window" | "context" | "maxLines"> = { "--window": "window", "--context": "context", "-C": "context", "--max-lines": "maxLines" };
  for (let i = 0; i < args.length; i++) {
    let a = args[i]!;
    let inline: string | null = null;
    const eq = a.startsWith("--") ? a.indexOf("=") : -1;
    if (eq > 0) [a, inline] = [a.slice(0, eq), a.slice(eq + 1)];
    else if (/^-C\d+$/.test(a)) [a, inline] = ["-C", a.slice(2)];
    const value = (): string | null => inline ?? (i + 1 < args.length ? args[++i]! : null);
    if (a === "--locate" || a === "--outline") {
      const list: string[] = inline !== null ? [inline] : [];
      while (i + 1 < args.length && !args[i + 1]!.startsWith("-")) list.push(args[++i]!);
      if (list.length === 0) return `argument ${a}: expected at least one argument`;
      if (a === "--locate") o.locate = list;
      else o.outline = list;
    } else if (a in ints) {
      const v = value();
      if (v === null || !/^-?\d+$/.test(v)) return `argument ${a}: expected an integer`;
      o[ints[a]!] = Number(v);
    } else if (a === "--in") {
      const v = value();
      if (v === null) return "argument --in: expected one argument";
      o.scope = v;
    } else if (a === "--deep") o.deep = true;
    else if (a === "--quiet") o.quiet = true;
    else if (a === "--help" || a === "-h") o.help = true;
    else if (a.startsWith("-") && a.length > 1) return `unrecognized arguments: ${a}`;
    else o.targets.push(a);
  }
  return o;
}

export async function run(args: string[]): Promise<number> {
  const o = parse(args);
  if (typeof o === "string") return refuse(o);
  if (o.help) {
    console.log(HELP);
    return 0;
  }
  out.length = 0;
  const flush = () => {
    if (out.length) console.log(out.join("\n"));
  };

  if (o.outline) {
    const code = outline(o.outline, o.deep);
    flush();
    return code;
  }
  if (o.locate) {
    const missing = locate(o.locate, o.scope);
    flush();
    return missing ? 1 : 0;
  }
  if (o.targets.length === 0) {
    console.log(HELP);
    return 2;
  }

  let total = 0;
  let empty = 0;
  for (const spec of o.targets) {
    const [printed, missed] = peekOne(spec, o.window, o.maxLines, o.context);
    total += printed;
    empty += missed;
  }
  // The ledger counts this call whatever `--quiet` says, or the next call's advice would be wrong.
  const advice = noteReads(o.targets);
  if (!o.quiet) {
    say(`-- peek: ${o.targets.length} target(s) in one call, ${thousands(total)} B (~${thousands(total / BYTES_PER_TOKEN)} tok)` +
      (empty ? `, ${empty} produced nothing` : ""));
    for (const line of advice) say(line);
  }
  flush();
  return 0;
}
