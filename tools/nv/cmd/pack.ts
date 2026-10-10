// `bun nv pack`: the sections of the orientation pack only Novis has a form for, for one goal and one
// stage. They are the rules, the decision-record sections, the scoped map, the convention shapes, the
// plan fields, the milestones and the next free numbers, in the order `bun nv orient` prints them, and
// `orient` calls the `run*` functions here for its own pack. The traps stay in `orient`.
//
// The goal is `--goal`, else `LOOPER_GOAL_ENV`, else the chain's live goal, read through `lib/chain.ts`.
// The stage is `--stage`, else `LOOPER_STAGE_ENV`, else the stage the goal's handoff names. The goal's
// `context` is the base selection and the stage's `context` adds to it (`manifest`). A goal with no
// `context` at all prints no section.
//
// The output is one buffer for the process: `begin` empties it, every section appends to it, and
// `printed` and `costs` read it back, so `orient` interleaves its own sections with these in one pack.
// Every reader takes the root `begin` names, which is how a test runs the pack over a scratch tree.
//
//   bun nv pack                         the live goal, at the stage its handoff names
//   bun nv pack --goal SLUG --stage N   goal SLUG at stage N

import { existsSync, readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { goalHandoff, LOOPER_STAGE_ENV, namedPlan } from "../lib/chain.ts";
import { tracked } from "../lib/git.ts";
import { ROOT } from "../lib/paths.ts";
import { fill, pyRepr } from "../lib/py.ts";
import { load } from "../lib/store.ts";
import { planStatus } from "../schema/plan-status.ts";
import { rule as ruleType } from "../schema/rule.ts";
import { topic as topicType } from "../schema/topic.ts";
import { bodyOf, leadParagraph, milestones, verifyParagraph } from "./plan.ts";

export const summary = "the orientation pack's Novis-only sections for one goal and stage: nv pack [--goal <slug>] [--stage <n>]";

const USAGE = "usage: bun nv pack [--goal <slug>] [--stage <n>]";

export const CONVENTIONS = "docs/agent/conventions.md";
export const DECISIONS = "docs/decisions";
const DIAGNOSTICS = "crates/nvs-diagnostics/src/lib.rs";

/** The longest list printed whole; a longer one prints as a count. */
const NAMED_BEFORE_COUNT = 6;

/** How many files the manifest's patterns outside `crates/` and `editors/` may list. */
const NAMED_FILES_MAX = 40;

/** How long a summary derived from a file that is not a crate module may be. */
const SUMMARY_MAX = 200;

/** Shapes that only make sense together: a decision is the record and the rule's fragment. */
export const SHAPE_IMPLIES: [string, string][] = [["A decision record", "A rule fragment"]];

/** The plan's status fields as the plan writes them, and each one's key in the record. */
const PLAN_FIELDS: [string, string][] = [
  ["Status", "status"],
  ["Done", "done"],
  ["On disk", "onDisk"],
  ["Toolchain", "toolchain"],
  ["ADR slices landed", "adrSlicesLanded"],
  ["Open now", "openNow"],
  ["Blocking", "blocking"],
];

export interface Context {
  modules?: string[];
  rules?: string[];
  adrs?: string[];
  shapes?: string[];
  playbook?: string[];
  plan?: string[];
  milestones?: string[];
}

export interface GoalValue {
  context: Context;
  stages: { number: number; title: string; summary?: string; context?: Context }[];
  checks?: RecordCheck[];
}

/** The fields of an acceptance check the pack names it by and prints. */
export interface RecordCheck {
  id: string;
  stage: number;
  kind: string;
  name?: string;
  file?: string;
  [field: string]: unknown;
}

// ----------------------------------------------------------------------------- output

let root = ROOT;
let who = "pack";
let out: string[] = [];
let ledger: [string, number][] = [];

/** Empties the output and points every reader at `at`; `name` is the command the warnings name. */
export function begin(name: string, at: string = ROOT): void {
  who = name;
  root = at;
  out = [];
  ledger = [];
}

/** Every line printed since `begin`. */
export function printed(): string[] {
  return out;
}

/** Each section's title and its size in bytes, once `closeLedger` has sized the last one. */
export function costs(): [string, number][] {
  return ledger;
}

export function emit(line = ""): void {
  out.push(line);
}

export function warn(msg: string): void {
  emit("");
  emit(`!! nv ${who}: ${msg}`);
}

export function nbytes(text: string): number {
  return Buffer.byteLength(text, "utf8");
}

export function closeLedger(): void {
  const last = ledger[ledger.length - 1];
  if (last) ledger[ledger.length - 1] = [last[0], nbytes(out.slice(last[1]).join("\n"))];
}

export function section(title: string, source: string): void {
  closeLedger();
  emit();
  emit(`== ${title}`);
  emit(`-- source: ${source}`);
  emit();
  ledger.push([title, out.length]);
}

/** A repo-relative file's text with every line ending read as `\n`, or "" when it cannot be read. */
export function read(path: string): string {
  try {
    return readFileSync(join(root, path), "utf8").replace(/\r\n?/g, "\n");
  } catch {
    return "";
  }
}

/** The command that runs tool `name`. */
export function tool(name: string): string {
  return `bun nv ${name}`;
}

/** `[0002](../decisions/0002.md)` -> `0002`. */
export function stripLinks(text: string): string {
  return text.replace(/\[([^\]]*)\]\([^)]*\)/g, "$1");
}

/** A Python list of strings, the way the pack has always quoted a manifest field. */
export function pyList(items: string[]): string {
  return `[${items.map(pyRepr).join(", ")}]`;
}

function indentedFill(text: string, indent: string, first = indent): string {
  return fill(text, 100, { initialIndent: first, subsequentIndent: indent, breakLongWords: false, breakOnHyphens: false });
}

function escapeRe(s: string): string {
  return s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

// --------------------------------------------------------------------- markdown slicing

interface MdHeading {
  index: number;
  level: number;
  title: string;
}

/** Every heading outside a ``` fence, with its 0-based line index. */
function headings(text: string): MdHeading[] {
  const found: MdHeading[] = [];
  let fenced = false;
  text.split("\n").forEach((line, index) => {
    if (line.trimStart().startsWith("```")) fenced = !fenced;
    if (fenced) return;
    const m = /^(#{1,6})\s+(.*?)\s*$/.exec(line);
    if (m) found.push({ index, level: m[1]!.length, title: m[2]! });
  });
  return found;
}

/** `### 2. Both operands ...` -> `2 both operands ...`, so a section is named by number or by words. */
export function normalize(title: string): string {
  let t = title.replace(/[`*_]/g, "").trim().toLowerCase();
  t = t.replace(/^(\d+[a-z]?)\s*[.)]?\s*/, "$1 ");
  return t.replace(/\s+/g, " ").trim();
}

export function titleMatches(title: string, key: string): boolean {
  const norm = normalize(title);
  return norm === key || norm.startsWith(key + " ") || norm.startsWith(key + ".");
}

/** The named heading and its body, up to the next heading at the same or a higher level. */
export function sliceSection(text: string, wanted: string): string | null {
  const lines = text.split("\n");
  const hs = headings(text);
  const key = normalize(wanted);
  const at = hs.findIndex((h) => titleMatches(h.title, key));
  if (at < 0) return null;
  const h = hs[at]!;
  const next = hs.slice(at + 1).find((later) => later.level <= h.level);
  return lines.slice(h.index, next ? next.index : lines.length).join("\n").trimEnd();
}

/** The first `##` section whose title opens with `words`. */
export function sectionOpening(text: string, words: string): string | null {
  const h = headings(text).find((x) => x.level === 2 && normalize(x.title).startsWith(words));
  return h ? sliceSection(text, h.title) : null;
}

/** A decision record's title, metadata and *In short*: everything before its first `##`. */
function sliceHead(text: string): string {
  const h = headings(text).find((x) => x.level === 2);
  return h ? text.split("\n").slice(0, h.index).join("\n").trimEnd() : text.trimEnd();
}

/** A frozen record's text without the YAML block it opens on. */
export function stripFrontmatter(text: string): string {
  if (!text.startsWith("---\n")) return text;
  const end = text.indexOf("\n---\n", 4);
  return end < 0 ? text : text.slice(end + 5).replace(/^\n+/, "");
}

// ------------------------------------------------------------------------ the manifest

/** The fields a stage's `context` may add to. */
export const STAGE_FIELDS = ["rules", "adrs", "shapes", "playbook", "milestones"] as const;

export interface Manifest {
  present: boolean;
  /** Every stage that carries a `context` of its own. */
  staged: number[];
  stage: number | null;
  applied: number | null;
  modules: string[];
  rules: string[];
  adrs: string[];
  shapes: string[];
  playbook: string[];
  plan: string[];
  milestones: string[];
}

export function manifest(g: GoalValue, stage: number | null): Manifest {
  const ctx = g.context ?? {};
  const over = stage === null ? undefined : g.stages.find((s) => s.number === stage)?.context;
  const field = (name: keyof Context): string[] => {
    const base = [...(ctx[name] ?? [])].map(String);
    if (over && (STAGE_FIELDS as readonly string[]).includes(name)) {
      for (const x of over[name] ?? []) if (!base.includes(x)) base.push(x);
    }
    return base;
  };
  return {
    present: Object.keys(ctx).length > 0,
    staged: g.stages.filter((s) => s.context !== undefined).map((s) => s.number),
    stage,
    applied: over ? stage : null,
    modules: field("modules"),
    rules: field("rules"),
    adrs: field("adrs"),
    shapes: field("shapes"),
    playbook: field("playbook"),
    plan: ctx.plan ?? ["Open now", "Blocking"],
    milestones: field("milestones"),
  };
}

// --------------------------------------------------------------------------- the rules

export interface BookRule {
  id: string;
  title: string;
  status: string;
  because: string[];
  guardedBy: string[];
  topicOrder: number;
  order: number;
}

export function rulebook(): Map<string, BookRule> {
  const topics = load(topicType, root);
  const place = new Map<string, [number, number]>();
  for (const t of topics) t.value.rules.forEach((id, i) => place.set(id, [t.value.order, i]));
  const book = new Map<string, BookRule>();
  for (const r of load(ruleType, root)) {
    const [topicOrder, order] = place.get(r.id) ?? [Number.MAX_SAFE_INTEGER, 0];
    book.set(r.id, { id: r.id, title: r.value.title, status: r.value.status, because: r.value.because, guardedBy: r.value.guardedBy, topicOrder, order });
  }
  return book;
}

function capped(label: string, items: string[], tail: string, indent = "  "): void {
  if (items.length === 0) return;
  const rest = items.length > NAMED_BEFORE_COUNT ? `, and ${items.length - NAMED_BEFORE_COUNT} more (${tail})` : "";
  emit(indentedFill(`${label}: ` + items.slice(0, NAMED_BEFORE_COUNT).join(", ") + rest, indent));
}

export function runRules(m: Manifest): void {
  section("THE RULES THIS GOAL LIVES INSIDE", `docs/rules/, selected by [context] rules = ${m.rules.length ? pyList(m.rules) : "[]"}`);
  emit("AGENTS.md's priority ordering and its numbered rules are already in your context. These are");
  emit("the rules this goal's own work sits inside. A rule named by id is printed whole, because");
  emit("the fragment IS the rule; a record number expands to the rules it created or changed.");
  emit(`A short changed list is named whole; a long one is only its count, and \`${tool("brief")} --where\``);
  emit("routes a topic to the chapter that holds the rest.");
  emit();
  if (m.rules.length === 0) {
    warn("[context] rules is empty, so no rule and no decision is named as binding this goal");
    return;
  }
  const book = rulebook();
  const byPlace = (a: BookRule, b: BookRule) => a.topicOrder - b.topicOrder || a.order - b.order;
  for (const entry of m.rules) {
    if (entry.includes("/")) {
      runOneRule(book, entry);
      continue;
    }
    const all = [...book.values()];
    const created = all.filter((r) => r.because[0] === entry).sort(byPlace);
    const changed = all.filter((r) => r.because.slice(1).includes(entry)).sort(byPlace);
    if (created.length === 0 && changed.length === 0) {
      warn(`[context] rules names ${entry}, but no rule's \`because\` names that record and the rulebook has no rule with that id`);
      continue;
    }
    emit(`ADR ${entry} -- ${created.length} rule(s) created, ${changed.length} changed:`);
    for (const r of created) emit(`- \`rule:${r.id}\` -- ${r.title}${r.status === "shipped" ? "" : "  (designed)"}`);
    const guards = new Set(created.flatMap((r) => r.guardedBy));
    if (guards.size > 0) emit(`  guarded by ${guards.size} path(s) (\`${tool("rules")} --show <id>\` lists a rule's own)`);
    if (changed.length > 0 && changed.length <= NAMED_BEFORE_COUNT) {
      emit(indentedFill("changed: " + changed.map((r) => `rule:${r.id}`).join(", "), "  "));
    }
    emit();
  }
}

function runOneRule(book: Map<string, BookRule>, id: string): void {
  const r = book.get(id);
  if (!r) {
    warn(`[context] rules names rule:${id}, and the rulebook has no rule with that id -- \`${tool("rules")} --list\` is every one of them`);
    return;
  }
  emit(`---- rule:${r.id}${r.status === "shipped" ? "" : "  (designed, not yet shipped)"}`);
  emit(`     ${r.title}`);
  if (r.because.length > 0) {
    emit(`     decided in ${r.because.join(", ")}${r.because.length === 1 ? "" : " (first created it, the rest amended it)"}`);
  }
  capped("guarded by", r.guardedBy, `\`${tool("rules")} --show\` lists them all`, "     ");
  emit();
  emit(read(`docs/rules/${id}.md`).replace(/^\n+|\n+$/g, ""));
  emit();
}

export function runAdrs(m: Manifest): void {
  if (m.adrs.length === 0) return;
  section("THE ADR SECTIONS IN SCOPE", "docs/decisions/*.md, sliced live -- never a copy");
  emit("A section, not the file. If you need one this does not print, open the record at that");
  emit("heading and add the section to [context] adrs so the next session does not pay twice.");
  for (const entry of m.adrs) {
    const parts = entry.replaceAll("§", " ").split(/\s+/).filter(Boolean);
    if (parts.length === 0) continue;
    const [number, ...rest] = parts as [string, ...string[]];
    const wanted = rest.join(" ");
    const path = `${DECISIONS}/${number}.md`;
    if (!existsSync(join(root, path))) {
      warn(`[context] adrs names ADR ${number}, and docs/decisions/ has no ${number}.md`);
      continue;
    }
    const text = stripFrontmatter(read(path));
    const body = wanted ? sliceSection(text, wanted) : sliceHead(text);
    if (body === null) {
      warn(`ADR ${number} has no section matching ${pyRepr(wanted)} -- it was renamed or renumbered`);
      continue;
    }
    emit();
    emit(`---- ${path}` + (wanted ? `  §${wanted}` : "  (In short)"));
    emit();
    emit(body);
  }
}

// ---------------------------------------------------------------------------- the map

/** Python's `fnmatch.fnmatch`: `*` crosses `/`, and on Windows the match ignores case. */
function fnmatch(name: string, pat: string): boolean {
  let re = "";
  for (let i = 0; i < pat.length; i++) {
    const c = pat[i]!;
    if (c === "*") re += ".*";
    else if (c === "?") re += ".";
    else if (c === "[") {
      const end = pat.indexOf("]", i + 2);
      if (end < 0) re += "\\[";
      else {
        let cls = pat.slice(i + 1, end).replace(/\\/g, "\\\\");
        if (cls.startsWith("!")) cls = "^" + cls.slice(1);
        re += `[${cls}]`;
        i = end;
      }
    } else re += escapeRe(c);
  }
  return new RegExp(`^(?:${re})$`, process.platform === "win32" ? "is" : "s").test(name);
}

function globHits(full: string, pat: string): boolean {
  return fnmatch(full, pat) || fnmatch(full, pat.replace(/\/+$/, "") + "/**");
}

/** `Foo the bar. Then baz.` -> `Foo the bar`. */
function firstSentence(text: string): string {
  const t = stripLinks(text).replace(/\[(`[^`\]]+`)\]/g, "$1");
  return t.split(/(?<=[a-z)`])\.\s+(?=[A-Z[`])/, 1)[0]!.trim().replace(/\.+$/, "");
}

/** The first paragraph of a Rust module's own `//!` block, or "". */
function moduleDoc(text: string): string {
  const para: string[] = [];
  for (const line of text.split("\n")) {
    const m = /^\s*\/\/!\s?(.*)$/.exec(line);
    if (m) {
      const body = m[1]!.trim();
      if (!body) {
        if (para.length > 0) break;
        continue;
      }
      para.push(body);
    } else if (para.length > 0) break;
    else if (!/^\s*(#!\[|\/\/[^!]|\/\/$|$)/.test(line)) break;
  }
  return para.join(" ");
}

/** The first paragraph of a TypeScript file's leading comment, or "". */
function headerDoc(text: string): string {
  const para: string[] = [];
  for (const raw of text.split("\n")) {
    let line = raw.trim();
    if (line.startsWith("/*")) line = line.replace(/^[/*]+/, "").trim();
    else if (line.startsWith("*/")) break;
    else if (line.startsWith("*")) line = line.slice(1).trim();
    else if (line.startsWith("//")) line = line.slice(2).trim();
    else if (para.length > 0 || line) break;
    if (!line) {
      if (para.length > 0) break;
      continue;
    }
    para.push(line);
  }
  return para.join(" ");
}

/** The opening paragraph of a Python file's module docstring, or "". */
function pyDoc(text: string): string {
  const m = /^(?:[ \t]*(?:#[^\n]*)?\n)*[ \t]*[rRuU]?("""|''')([\s\S]*?)\1/.exec(text);
  if (!m) return "";
  const lines = m[2]!.replace(/\t/g, "        ").split("\n");
  const margins = lines.slice(1).filter((l) => l.trim()).map((l) => l.length - l.trimStart().length);
  const margin = margins.length > 0 ? Math.min(...margins) : 0;
  const cleaned = [lines[0]!.trimStart(), ...lines.slice(1).map((l) => l.slice(margin))].join("\n").trim();
  return cleaned.split("\n\n", 1)[0]!.trim();
}

/** A Markdown file's first `#` heading and the opening sentence under it, as one line. */
function mdDoc(text: string): string {
  let head = "";
  const para: string[] = [];
  for (const raw of text.split("\n")) {
    const line = raw.trim();
    if (!head) {
      if (line.startsWith("# ")) head = line.slice(2).trim();
      continue;
    }
    if (line.startsWith("#")) break;
    if (line) para.push(line);
    else if (para.length > 0) break;
  }
  const body = para.length > 0 ? firstSentence(para.join(" ")) : "";
  return head && body ? `${head} -- ${body}` : head || body;
}

/** One line saying what any file in the tree is, or "" for a shape that carries no header. */
function pathSummary(path: string): string {
  const text = read(path);
  const ext = path.slice(path.lastIndexOf("."));
  const fn: Record<string, (t: string) => string> = {
    ".py": (t) => firstSentence(pyDoc(t)),
    ".md": mdDoc,
    ".rs": (t) => firstSentence(moduleDoc(t)),
    ".ts": (t) => firstSentence(headerDoc(t)),
    ".tsx": (t) => firstSentence(headerDoc(t)),
  };
  const f = fn[ext];
  if (!text || !f || !path.slice(path.lastIndexOf("/") + 1).includes(".")) return "";
  const line = f(text).split(/\s+/).filter(Boolean).join(" ");
  return line.length <= SUMMARY_MAX ? line : line.slice(0, SUMMARY_MAX).replace(/[ ,;:-]+$/, "") + "...";
}

/** Every file under `dir` whose name passes `keep`, as paths relative to `dir`, sorted. */
function filesUnder(dir: string, keep: (rel: string) => boolean): string[] {
  let names: string[];
  try {
    names = readdirSync(join(root, dir), { recursive: true }) as string[];
  } catch {
    return [];
  }
  return names.map((n) => n.split("\\").join("/")).filter(keep).sort();
}

/** Every crate module and then every editor module, grouped, each with its header summary. */
function sourceModules(): [string, string, [string, string][]][] {
  const groups: [string, string, [string, string][]][] = [];
  const byKey = (first: string) => (a: [string, string], b: [string, string]) =>
    Number(a[0] !== first) - Number(b[0] !== first) || (a[0] < b[0] ? -1 : a[0] > b[0] ? 1 : a[1] < b[1] ? -1 : a[1] > b[1] ? 1 : 0);
  const packages = (base: string) => {
    try {
      return readdirSync(join(root, base)).filter((d) => existsSync(join(root, base, d, "src"))).sort();
    } catch {
      return [];
    }
  };
  for (const crate of packages("crates")) {
    const entries = filesUnder(`crates/${crate}/src`, (p) => p.endsWith(".rs") && !p.split("/").includes("snapshots"))
      .map((p): [string, string] => [`src/${p}`, firstSentence(moduleDoc(read(`crates/${crate}/src/${p}`)))]);
    if (entries.length > 0) groups.push([crate, `crates/${crate}`, entries.sort(byKey("src/lib.rs"))]);
  }
  for (const pkg of packages("editors")) {
    const entries = filesUnder(`editors/${pkg}/src`, (p) => (p.endsWith(".ts") || p.endsWith(".tsx")) && !p.split("/").includes("node_modules"))
      .map((p): [string, string] => [`src/${p}`, firstSentence(headerDoc(read(`editors/${pkg}/src/${p}`)))]);
    if (entries.length > 0) groups.push([`editors/${pkg}`, `editors/${pkg}`, entries.sort(byKey("src/extension.ts"))]);
  }
  return groups;
}

export async function runMap(m: Manifest): Promise<void> {
  section("THE MAP, SCOPED", `what each file the goal names says it is, filtered to [context] modules (${m.modules.length} pattern(s))`);
  const groups = sourceModules();
  if (groups.length === 0) {
    warn("no `crates/*/src/**/*.rs` or `editors/*/src/**/*.ts` found -- those are the source");
    return;
  }
  if (m.modules.length === 0) {
    warn("[context] modules is empty, so the map is not printed at all. A goal that touches code must name the files it touches; `bun nv orient --full` prints all of them.");
    return;
  }
  let shown = 0;
  let total = 0;
  const unmatched = new Set(m.modules);
  for (const [name, prefix, entries] of groups) {
    const keep: [string, string][] = [];
    for (const [within, summary] of entries) {
      total++;
      const hit = m.modules.filter((pat) => globHits(`${prefix}/${within}`, pat));
      if (hit.length > 0) {
        keep.push([within, summary || "(no header doc comment)"]);
        for (const pat of hit) unmatched.delete(pat);
      }
    }
    if (keep.length === 0) continue;
    emit();
    emit(name);
    const width = Math.max(...keep.map(([w]) => w.length));
    for (const [within, summary] of keep) {
      emit(`  ${within.padEnd(width)}  ${summary}`);
      shown++;
    }
  }

  const rest = [...unmatched].sort();
  const named = new Map<string, [string, string][]>();
  const hitPats = new Set<string>();
  let kept = 0;
  for (const path of rest.length > 0 ? await tracked([], root) : []) {
    const matched = rest.filter((pat) => globHits(path, pat));
    if (matched.length === 0) continue;
    for (const pat of matched) hitPats.add(pat);
    kept++;
    if (kept > NAMED_FILES_MAX) continue;
    const slash = path.lastIndexOf("/");
    const group = slash < 0 ? "." : path.slice(0, slash);
    if (!named.has(group)) named.set(group, []);
    named.get(group)!.push([path.slice(slash + 1), pathSummary(path)]);
  }
  const cut = Math.max(0, kept - NAMED_FILES_MAX);
  let extra = 0;
  for (const group of [...named.keys()].sort()) {
    const entries = named.get(group)!.sort((a, b) => (a[0] < b[0] ? -1 : a[0] > b[0] ? 1 : 0));
    extra += entries.length;
    emit();
    emit(group);
    const width = Math.max(...entries.map(([n]) => n.length));
    for (const [n, summary] of entries) emit(summary ? `  ${n.padEnd(width)}  ${summary}` : `  ${n}`);
  }
  if (cut > 0) {
    emit();
    emit(`...and ${cut} more file(s) the manifest's patterns match, not listed. A selector this broad is naming the tree rather than the goal's file set.`);
  }
  emit();
  const also = extra > 0 ? `, plus ${extra} file(s) it names outside them` : "";
  emit(`${shown} of ${total} crate and editor module(s) are in scope${also}. For one that is not,`);
  emit("`bun nv orient --full` prints the whole map -- and if you needed it, the manifest is");
  emit("missing a pattern.");
  for (const pat of rest.filter((p) => !hitPats.has(p))) {
    warn(`[context] modules pattern ${pyRepr(pat)} matches no file in the tree -- it moved, or the glob is wrong. The shape does not have to be Rust: any tracked file resolves.`);
  }
}

// -------------------------------------------------------------------------- the shapes

export function runShapes(m: Manifest): void {
  if (m.shapes.length === 0) return;
  const text = read(CONVENTIONS);
  if (!text) {
    warn(`${CONVENTIONS} is missing`);
    return;
  }
  for (const [shape, implied] of SHAPE_IMPLIES) {
    if (m.shapes.includes(shape) && !m.shapes.includes(implied)) {
      warn(`[context] shapes names ${pyRepr(shape)} without ${pyRepr(implied)}. One decision is both files, in one commit, and \`nv session --wrap\` refuses a rulebook left half-written -- add ${pyRepr(implied)} to the manifest`);
    }
  }
  section("THE SHAPES YOU ARE ABOUT TO WRITE", `${CONVENTIONS}, filtered to [context] shapes`);
  for (const name of m.shapes) {
    const body = sliceSection(text, name);
    if (body === null) {
      warn(`[context] shapes names ${pyRepr(name)}, and ${CONVENTIONS} has no such heading`);
      continue;
    }
    emit();
    emit(body);
  }
}

// ------------------------------------------------------------------- plan and numbers

export function runPlan(m: Manifest): void {
  const status = load(planStatus, root)[0]?.value as Record<string, string> | undefined;
  if (!status) return;
  const picked = PLAN_FIELDS.filter(([label, key]) => m.plan.includes(label) && status[key] !== undefined);
  if (picked.length === 0) return;
  section("THE PLAN, THE FIELDS THIS GOAL READS", `docs/implementation-plan.md, ${pyList(m.plan)}`);
  for (const [label, key] of picked) emit(indentedFill(`${label}: ${stripLinks(status[key]!.trim())}`, "    ", ""));
  emit();
  emit(`These fields are rewritten by \`${tool("session")} --wrap\`, never edited by hand.`);
}

export function runMilestones(m: Manifest): void {
  if (m.milestones.length === 0) return;
  const index = milestones(root);
  section("THE MILESTONES THIS GOAL IS INSIDE", `docs/plan/, filtered to [context] milestones = ${pyList(m.milestones)}`);
  for (const wanted of m.milestones) {
    const colon = wanted.indexOf(":");
    const spec = colon < 0 ? wanted : wanted.slice(0, colon);
    const part = colon < 0 ? "" : wanted.slice(colon + 1);
    const entry = index.find((e) => e.id === spec.trim().toUpperCase());
    if (!entry) {
      warn(`[context] milestones names ${pyRepr(spec)}, which the plan's table does not list`);
      continue;
    }
    if (!existsSync(join(root, entry.rel))) {
      warn(`[context] milestones names ${pyRepr(spec)}, whose file ${entry.rel} is missing`);
      continue;
    }
    emit();
    if (part === "verify") {
      emit(`-- ${entry.id} acceptance (${entry.rel})`);
      emit(stripLinks(verifyParagraph(entry, root)));
    } else if (part === "lead") {
      emit(`-- ${entry.id} lead (${entry.rel})`);
      emit(stripLinks(leadParagraph(entry)));
    } else {
      emit(`-- ${entry.id} (${entry.rel})`);
      emit(bodyOf(entry, root));
    }
  }
  emit();
  emit("`Mn` is the whole milestone; `Mn:lead` and `Mn:verify` are the two paragraphs that");
  emit(`usually answer the question. A \`## milestone: Mn\` section in \`${tool("session")} --wrap\``);
  emit("rewrites one.");
}

/** The next free diagnostic code in each band, and the next free decision number. */
export function runNumbers(): void {
  section("THE NEXT FREE NUMBER", "nvs-diagnostics (every `Code::new`) and docs/decisions/ filenames");
  const text = read(DIAGNOSTICS);
  if (!text) {
    warn(`could not read ${DIAGNOSTICS} -- it is the diagnostic-code registry`);
  } else {
    const legend = new Map<string, string>();
    const highest = new Map<string, number>();
    for (const line of text.split("\n")) {
      const m = /^\/\/\/\s*\|\s*`E(\d{2})xx`\s*\|\s*([^|]+?)\s*\|/.exec(line);
      if (m) legend.set(m[1]!, m[2]!.trim());
    }
    // A retired code is never reused, so the comment naming it holds its band's ceiling up.
    const decl = [/Code::new\("(E(\d{2})\d{2})"\)/g, /`(E(\d{2})\d{2})`[^`]{0,80}?\bretired\b/g, /\bretired\b[^`]{0,80}?`(E(\d{2})\d{2})`/g];
    for (const re of decl) {
      for (const m of text.matchAll(re)) highest.set(m[2]!, Math.max(highest.get(m[2]!) ?? 0, Number(m[1]!.slice(1))));
    }
    if (highest.size === 0) {
      warn(`no \`Code::new("Ennnn")\` declarations in ${DIAGNOSTICS}`);
    } else {
      emit("diagnostic codes -- next free in each band (max + 1; a retired code is never");
      emit("reused, so this is deliberately not the lowest hole):");
      for (const band of [...highest.keys()].sort()) {
        const top = highest.get(band)!;
        const meaning = (legend.get(band) ?? "(no row for this band in that file's legend table)").padEnd(48);
        // Max-plus-one leaves the band at `Enn00`, and a filled band's next code comes from the
        // legend's continuation row rather than from the next band's digits.
        if ((top + 1) % 100 === 0) emit(`  E${band}xx  ${meaning} FULL at E${String(top).padStart(4, "0")}`);
        else emit(`  E${band}xx  ${meaning} next: E${String(top + 1).padStart(4, "0")}`);
      }
      const unlisted = [...legend.keys()].filter((b) => !highest.has(b)).sort();
      if (unlisted.length > 0) emit(`  bands with a legend row but no code yet: ${unlisted.map((b) => `E${b}xx`).join(", ")}`);
    }
  }
  let numbers: number[];
  try {
    numbers = readdirSync(join(root, DECISIONS)).flatMap((f) => {
      const m = /^(\d{4})\.md$/.exec(f);
      return m ? [Number(m[1])] : [];
    });
  } catch {
    warn(`no ${DECISIONS}/ directory`);
    return;
  }
  emit();
  if (numbers.length === 0) {
    warn(`no \`NNNN.md\` files in ${DECISIONS}/`);
    return;
  }
  const top = Math.max(...numbers);
  emit(`ADRs: ${numbers.length} on disk, highest ${String(top).padStart(4, "0")} -- next free is ${String(top + 1).padStart(4, "0")}`);
  emit("Claim it by creating the file, and re-check this immediately before you do: another");
  emit("agent working the same tree derives the same answer from the same directory.");
}

// ---------------------------------------------------------------------------- driver

/** Every section in `orient`'s order, for the goal and stage `args` name, over the tree at `at`. */
export async function pack(args: string[], at: string = ROOT): Promise<{ code: number; text: string }> {
  let goalFlag: string | undefined;
  let stageFlag: number | null = null;
  if (args.includes("--help") || args.includes("-h")) return { code: 0, text: `${USAGE}\n\n${summary}` };
  for (let i = 0; i < args.length; i++) {
    const [flag, inline] = args[i]!.includes("=") ? (args[i]!.split(/=(.*)/s, 2) as [string, string]) : [args[i]!, undefined];
    if (flag !== "--goal" && flag !== "--stage") return { code: 2, text: `nv pack: unknown argument ${pyRepr(args[i]!)}\n${summary}` };
    const v = inline ?? args[++i];
    if (flag === "--goal") {
      if (!v) return { code: 2, text: `nv pack: --goal needs a goal slug\n${summary}` };
      goalFlag = v;
    } else {
      if (!v || !/^\d+$/.test(v)) return { code: 2, text: `nv pack: --stage needs a number\n${summary}` };
      stageFlag = Number(v);
    }
  }
  const named = namedPlan(goalFlag, at);
  if (named === null) return { code: 2, text: `nv pack: ${goalFlag ?? "the live goal"} has no goal record, so there is no context to print` };
  const envStage = (process.env[LOOPER_STAGE_ENV] ?? "").trim();
  const stage = stageFlag ?? (/^\d+$/.test(envStage) ? Number(envStage) : (goalHandoff(named.slug, at)?.next.stage ?? null));
  const m = manifest(named.goal as unknown as GoalValue, stage);
  if (!m.present) return { code: 0, text: `nv pack: goal ${named.slug} has no \`context\`, so no section is printed` };

  begin("pack", at);
  runRules(m);
  runAdrs(m);
  await runMap(m);
  runShapes(m);
  runPlan(m);
  runMilestones(m);
  runNumbers();
  closeLedger();
  return { code: 0, text: printed().join("\n").replace(/^\n+/, "") };
}

export async function run(args: string[]): Promise<number> {
  const r = await pack(args);
  if (r.code === 2) console.error(r.text);
  else process.stdout.write(r.text + "\n");
  return r.code;
}
