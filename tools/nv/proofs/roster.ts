// The roster: every feature Novis ships, read from five live sources and never from a list. A `Core`
// class member, exception, enum, interface and configuration directive come from the binary's own
// `nvs meta --json`. A language feature is a `#` heading in a chapter's source,
// `docs/examples/lang/<chapter>/chapter.md` (`chapters.ts`), and a tool feature is a `#` heading in a
// chapter under `docs/reference/tools/`. A class of a package this repository publishes is a type
// declared in a file under `packages/<dir>/src/`. Each feature has one path, `core/Str/length`,
// that the example, attack and bench trees share; a member of a built-in component's `Novis\` class
// has one under `novis/`, `novis/Image-Image/open`, and a package's class one under `package/`,
// `package/Novis-Rest-Client`. `rule:testing/feature-proofs` is what each one owes.
//
// A package's unit is its class, not each method as for a `Core` class: a package is not in the binary,
// and its class's page is the one place a user reads it. Its help is the comment above the class, and
// its tests are the package's own `#[Test]` methods that carry a `covers:` marker.
// A class or enum `nvs meta --json` calls `internal` — a component's engine, and an enum only its
// signature uses — is no feature, since a program never calls it.
//
// An anchor, `crates/…/file.rs:NN`, is where the feature is implemented when the roster can find it. It
// keys the perf ledger's currency, and a feature with no anchor is still on the roster.

import { existsSync, readdirSync, readFileSync } from "node:fs";
import { classConsts, filesUnder, nameConstsSignature, registry, registryNames } from "../cmd/gaps.ts";
import { noteKey, unrecorded } from "../lib/reads.ts";
import { ALL_CARDS, ALL_CLASSES, cardKey, classKey } from "../select/keys.ts";
import { anchorKey } from "./markers.ts";
import { abs } from "../lib/paths.ts";
import { comparePaths } from "../lib/py.ts";
import { run as runProc } from "../lib/proc.ts";
import { progress } from "../lib/progress.ts";
import { chapterFile, chapterSources, LANG_CHAPTERS as LANG, LANG_EXAMPLES } from "./chapters.ts";

export const EXAMPLES = "docs/examples";
export const HOSTILE = "tests/hostile";
export const BENCHES = "benches/members";
/** A feature's website description, inside its example directory. It never counts as an example. */
export const ABOUT = "about.md";
/** A language feature's exact rules and edge cases, beside its `about.md`. It is optional, and it never
 * counts as an example either. */
export const DETAILS = "details.md";
const TOOLCHAPTERS = "docs/reference/tools";

export type Kind = "member" | "lang" | "exception" | "enum" | "interface" | "tool" | "directive" | "package";

/** The packages this repository publishes, one directory each with its `package.toml`. */
export const PACKAGES = "packages";

/** One shipped feature, and where each of its feature proofs belongs. */
export interface Entry {
  id: string;
  kind: Kind;
  group: string;
  /** The relative path the example, attack and bench trees share. */
  path: string;
  /** `crates/…/file.rs:NN`, when the roster knows one. */
  anchor: string;
  summary: string;
  /** What the binary's own help is missing, or "" when nothing. */
  help: string;
}

export const examplesDir = (e: Entry) => `${EXAMPLES}/${e.path}`;
export const aboutFile = (e: Entry) => `${examplesDir(e)}/${ABOUT}`;
export const detailsFile = (e: Entry) => `${examplesDir(e)}/${DETAILS}`;
export const hostileDir = (e: Entry) => `${HOSTILE}/${e.path}`;
export const benchFile = (e: Entry) => `${BENCHES}/${e.path}.nvs`;
export const implFile = (e: Entry) => (e.anchor ? e.anchor.split(":")[0]! : "");

/** A repo-relative file's text with its line endings made `\n`, or "" when it cannot be read. */
export function read(path: string): string {
  try {
    return readFileSync(abs(path), "utf8").replace(/\r\n?/g, "\n");
  } catch {
    return "";
  }
}

/** The names in a repo-relative directory ending `ext`, sorted as paths, or none when it is not one. */
export function namesIn(dir: string, ext: string): string[] {
  if (!existsSync(abs(dir))) return [];
  try {
    return readdirSync(abs(dir))
      .filter((n) => n.endsWith(ext))
      .sort(comparePaths);
  } catch {
    return [];
  }
}

/** The two kinds of example: an `NN-title.nvs` program, and an `NN-title.cmd` command line
 * (`docs/examples/README.md`). */
export const PROGRAM_EXTS = [".nvs", ".cmd"];

/** The examples and attacks directly in a repo-relative directory, `.nvs` and `.cmd` alike, sorted as
 * paths, or none when it is not one. */
export function programsIn(dir: string): string[] {
  return PROGRAM_EXTS.flatMap((ext) => namesIn(dir, ext)).sort(comparePaths);
}

export function slugify(text: string): string {
  const out = text.toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-+|-+$/g, "");
  return out || "unnamed";
}

/** `Core\Db\Row` -> `Db-Row` and the class-owned enum `Core\Arr::Order` -> `Arr-Order`, the spelling
 * every tree and the website use. */
export function classTail(name: string): string {
  return (name.startsWith("Core\\") ? name.slice(5) : name).replaceAll("\\", "-").replaceAll("::", "-");
}

/** A class member's path: `Core\Db\Row::get` -> `core/Db-Row/get`, and a built-in component's
 * `Novis\Image\Image::open` -> `novis/Image-Image/open`. */
export function memberPath(cname: string, mname: string): string {
  if (cname.startsWith("Novis\\")) return `novis/${cname.slice(6).replaceAll("\\", "-")}/${mname}`;
  return `core/${classTail(cname)}/${mname}`;
}

const lineAt = (text: string, offset: number) => text.slice(0, offset).split("\n").length;

/** `("Throwable", None)` in `nvs_hir::errors::TREE` and `("Comparable", &[...])` in `nvs_hir::interfaces::RESERVED`. */
const TUPLE_ROW_RE = /^\s*\((?:r"([^"]+)"|"((?:[^"\\]|\\.)+)"),/gm;
/** `pub const FINISH_MARKER: &str = "Core\\Script\\Finished";`, an exception declared beside the tree. */
const CONST_ROW_RE = /const\s+[A-Za-z_][A-Za-z0-9_]*\s*:\s*&str\s*=\s*(?:r"([^"]+)"|"((?:[^"\\]|\\.)+)")\s*;/g;
/** `Directive { key: "cache.memory", ... }` in `nvs_config::DIRECTIVES`. */
const DIRECTIVE_ROW_RE = /Directive\s*\{\s*key:\s*"([^"]+)"/g;
/** A `CoreEnum` literal, named inline or through a name const. */
const ENUM_RE = /CoreEnum\s*\{\s*name:\s*(?:r"([^"]+)"|([A-Za-z_][A-Za-z0-9_]*))/g;

const TABLES: [Kind, string, RegExp][] = [
  ["exception", "crates/nvs-hir/src/errors.rs", TUPLE_ROW_RE],
  ["exception", "crates/nvs-hir/src/errors.rs", CONST_ROW_RE],
  ["interface", "crates/nvs-hir/src/interfaces.rs", TUPLE_ROW_RE],
  ["directive", "crates/nvs-config/src/directive.rs", DIRECTIVE_ROW_RE],
];

/** Per kind, name -> the table row or `CoreEnum` literal it is declared at. The first row wins. */
function tableAnchors(): Record<string, Map<string, string>> {
  const out: Record<string, Map<string, string>> = { exception: new Map(), interface: new Map(), directive: new Map(), enum: new Map() };
  for (const [kind, path, pattern] of TABLES) {
    const text = read(path);
    for (const m of text.matchAll(pattern)) {
      const name = m[1] !== undefined ? m[1] : (m[2] ?? "").replaceAll("\\\\", "\\");
      if (!out[kind]!.has(name)) out[kind]!.set(name, `${path}:${lineAt(text, m.index!)}`);
    }
  }
  // Read `unrecorded`: `collect` names the `anchor:` key of each feature it asks about.
  unrecorded(() => {
    for (const path of filesUnder("crates/nvs-stdlib/src", ".rs")) {
      const text = read(path);
      const consts = classConsts(path, text);
      for (const m of text.matchAll(ENUM_RE)) {
        const name = m[1] || consts.get(m[2]!) || "";
        if (name && !out.enum!.has(name)) out.enum!.set(name, `${path}:${lineAt(text, m.index!)}`);
      }
    }
  });
  return out;
}

/** The files the anchor scans read: the stdlib's sources, and the runtime's for the class-name consts
 * a stdlib file forwards. */
export function isAnchorFile(path: string): boolean {
  return path.endsWith(".rs") && (path.startsWith("crates/nvs-stdlib/src/") || path.startsWith("crates/nvs-runtime/src/"));
}

/** What the anchor scans read of one file: the `anchor:` key of each member and enum it declares, and
 * the signature of its class-name consts, which the owners of every other file are read through. */
export function anchorScan(path: string, text: string): { keys: string[]; consts: string } {
  const keys = new Set<string>();
  if (path.startsWith("crates/nvs-stdlib/src/")) {
    for (const name of registryNames(path, text)) keys.add(anchorKey(name));
    const consts = classConsts(path, text);
    for (const m of text.matchAll(ENUM_RE)) {
      const name = m[1] || consts.get(m[2]!) || "";
      if (name) keys.add(anchorKey(name));
    }
  }
  return { keys: [...keys], consts: nameConstsSignature(text) };
}


/**
 * Every `#` heading in a reference chapter is one feature of that chapter's topic. The front matter's
 * `id` names the chapter. A fenced block is skipped whole, because `# a comment` inside a sample program
 * is Novis and not a heading. The section under a heading is what `nvs agent show` prints, so it owes the
 * help proof unless it shows how the feature is written: a fenced block, or a table row holding code.
 */
function chapterFeatures(directory: string, area: "lang" | "tools"): Entry[] {
  const out: Entry[] = [];
  const judge = (entry: Entry | null, showsCode: boolean, path: string) => {
    if (entry !== null && !showsCode) {
      entry.help = `the section \`# ${entry.summary}\` in ${path} shows no code: add a short example of how it is written, which \`nvs agent show\` prints`;
    }
  };
  for (const name of namesIn(directory, ".md")) {
    const path = `${directory}/${name}`;
    const text = read(path);
    const m = /^---\n([\s\S]*?)\n---\n/.exec(text);
    const front = m ? m[1]! : "";
    const cid = /^id:\s*(\S+)/m.exec(front);
    const chapter = cid ? cid[1]! : name.replace(/\.md$/, "");
    const body = m ? text.slice(m[0].length) : text;
    let lineNo = m ? m[0].split("\n").length : 0;
    let fence = "";
    let current: Entry | null = null;
    let showsCode = false;
    for (const line of body.split("\n")) {
      lineNo++;
      const marker = line.trim().slice(0, 3);
      if (fence) {
        if (marker === fence) fence = "";
        continue;
      }
      if (marker === "```" || marker === "~~~") {
        fence = marker;
        showsCode = true;
        continue;
      }
      if (line.trimStart().startsWith("|") && line.includes("`")) showsCode = true;
      if (line.startsWith("# ")) {
        judge(current, showsCode, path);
        const title = line.slice(2).trim();
        const slug = slugify(title);
        current = {
          id: `${area}:${chapter}/${slug}`,
          kind: area === "lang" ? "lang" : "tool",
          group: `${area}:${chapter}`,
          path: `${area}/${chapter}/${slug}`,
          anchor: `${path}:${lineNo}`,
          summary: title,
          help: "",
        };
        showsCode = false;
        out.push(current);
      }
    }
    judge(current, showsCode, path);
  }
  return out;
}

type Doc = { short?: string } | null | undefined;
export interface Meta {
  classes?: { name: string; internal?: boolean; doc?: Doc; members?: { name: string; doc?: Doc; signature?: string }[] }[];
  exceptions?: unknown[];
  enums?: unknown[];
  interfaces?: unknown[];
  directives?: unknown[];
}

/** What `nvs meta --json` prints. A binary that cannot print it ends the command. It writes no footprint
 * log, since it prints every class and card: `noteRoster` names the part of it a verdict read. */
export async function metaJson(nvs: string): Promise<Meta> {
  progress("proofs: reading the roster (`nvs meta --json`)");
  const out = await runProc([nvs, "meta", "--json"], { timeoutMs: 120_000, env: { NVS_FOOTPRINT_LOG: "" } });
  if (out.code !== 0) throw new RosterError(`\`${nvs} meta --json\` failed:\n${out.stderr.trim()}`);
  return JSON.parse(out.stdout) as Meta;
}

export class RosterError extends Error {}

/** The kinds `nvs meta --json` lists outside a class, each as one list a verdict reads whole. */
const META_LISTS: ReadonlySet<Kind> = new Set(["exception", "enum", "interface", "directive"]);

/**
 * Notes what a verdict over `scope` read of `nvs meta --json`: the class and the card of each member's
 * class, which a change to that class's registry row or card moves. An exception, enum, interface or
 * directive, and the whole roster (`whole`), depend on every class and card, since the list they come
 * from moves with any of them. A language or tool feature is read from its chapter, which is noted as
 * a file.
 */
export function noteRoster(scope: Entry[], whole: boolean): void {
  if (whole || scope.some((e) => META_LISTS.has(e.kind))) {
    noteKey(ALL_CLASSES);
    noteKey(ALL_CARDS);
    return;
  }
  for (const e of scope) {
    if (e.kind !== "member") continue;
    noteKey(classKey(e.group));
    noteKey(cardKey(e.group));
  }
}

const isObject = (v: unknown): v is Record<string, unknown> => typeof v === "object" && v !== null && !Array.isArray(v);
/** A value as Python's `str()` prints it inside an f-string. */
const pyStr = (v: unknown) => (v === undefined ? "" : v === null ? "None" : String(v));

/** Every feature Novis ships, in the order the sources give them. `doc` is `nvs meta --json` when the
 * caller has already read it. */
export async function roster(nvs: string, doc?: Meta): Promise<Entry[]> {
  doc ??= await metaJson(nvs);
  const anchors = unrecorded(() => new Map([...registry()].map(([k, v]) => [k, `${v[0]}:${v[1]}`])));
  const tables = tableAnchors();
  const out: Entry[] = [];

  for (const klass of doc.classes ?? []) {
    if (klass.internal) continue;
    const cname = klass.name;
    // A member's own card is enforced by the registry test. What the binary can still be missing is
    // the class's card, which a completion list and `show` print above the member.
    const classHelp = klass.doc?.short
      ? ""
      : cname.startsWith("Novis\\")
        ? `\`${cname}\` has no class card: add a \`///\` comment above the class in its component's Novis source`
        : `\`${cname}\` has no class card: add its \`ClassDoc\` above the class's \`CLASS\` row and delete it from \`CLASSES_STILL_OWING_A_CARD\` in crates/nvs-stdlib/src/registry.rs`;
    for (const member of klass.members ?? []) {
      const mname = member.name;
      const key = `${cname}::${mname}`;
      out.push({
        id: key,
        kind: "member",
        group: cname,
        path: memberPath(cname, mname),
        anchor: anchors.get(key) ?? "",
        summary: member.doc?.short || member.signature || "",
        help: member.doc?.short ? classHelp : `\`${key}\` has no card, so \`nvs agent show\` prints nothing for it`,
      });
    }
  }

  for (const [kind, key] of [["exception", "exceptions"], ["enum", "enums"], ["interface", "interfaces"]] as const) {
    for (const item of doc[key] ?? []) {
      if (isObject(item) && item.internal === true) continue;
      const name = isObject(item) ? String(item.name) : pyStr(item);
      const short = isObject(item) ? ((item.doc as Doc)?.short ?? "") : "";
      out.push({
        id: name,
        kind,
        group: `types:${kind}`,
        path: `types/${classTail(name)}`,
        anchor: tables[kind]!.get(name) ?? "",
        summary: short,
        help: short ? "" : `\`${name}\` has no card, so \`nvs agent show\` prints nothing for it`,
      });
    }
  }

  for (const d of doc.directives ?? []) {
    const key = isObject(d) ? String(d.key) : pyStr(d);
    out.push({
      id: `directive:${key}`,
      kind: "directive",
      group: "config:directives",
      path: `config/${key.replaceAll(".", "-")}`,
      anchor: tables.directive!.get(key) ?? "",
      summary: isObject(d) ? `${pyStr(d.class)} directive, applied at ${pyStr(d.apply)}` : "",
      help: "",
    });
  }

  out.push(...langFeatures(), ...chapterFeatures(TOOLCHAPTERS, "tools"), ...packageFeatures());
  return out;
}

/** `namespace Novis\Rest;` in a package's source file. */
const NAMESPACE_RE = /^namespace\s+([A-Za-z_][A-Za-z0-9_\\]*)\s*;/m;
/** A top-level `final class Client`, `interface Store` or `enum Grant` declaration. */
const TYPE_RE = /^(?:(?:final|abstract|readonly)\s+)*(?:class|interface|enum)\s+([A-Za-z_][A-Za-z0-9_]*)/gm;

/**
 * Each type declared under `packages/<dir>/src/`, in path order. Its anchor is its declaration, so the
 * perf ledger keys its figures on the file that implements it. It owes the help proof until a `//`
 * comment sits directly above the declaration, or above the attributes on it.
 */
export function packageFeatures(): Entry[] {
  const out: Entry[] = [];
  const dirs = existsSync(abs(PACKAGES)) ? readdirSync(abs(PACKAGES), { withFileTypes: true }) : [];
  for (const dir of dirs.filter((d) => d.isDirectory()).map((d) => d.name).sort(comparePaths)) {
    const pkg = read(`${PACKAGES}/${dir}/package.toml`);
    const name = /^name\s*=\s*['"]([^'"]+)['"]/m.exec(pkg)?.[1];
    if (!name) continue;
    const src = `${PACKAGES}/${dir}/src`;
    for (const path of walk(src, ".nvs").sort(comparePaths)) {
      const text = read(path);
      const ns = NAMESPACE_RE.exec(text)?.[1] ?? "";
      const lines = text.split("\n");
      for (const m of text.matchAll(TYPE_RE)) {
        const id = ns ? `${ns}\\${m[1]}` : m[1]!;
        const line = lineAt(text, m.index!);
        let above = line - 2;
        while (above >= 0 && lines[above]!.trimStart().startsWith("#[")) above--;
        const comment: string[] = [];
        for (; above >= 0 && lines[above]!.trimStart().startsWith("//"); above--) comment.unshift(lines[above]!.trim().replace(/^\/\/\s?/, ""));
        out.push({
          id,
          kind: "package",
          group: `package:${name}`,
          path: `package/${id.replaceAll("\\", "-")}`,
          anchor: `${path}:${line}`,
          summary: comment[0] ?? "",
          help: comment.length ? "" : `\`${id}\` has no comment above its declaration in ${path}: add one that says what the class is for`,
        });
      }
    }
  }
  return out;
}

/** Every file under a repo-relative directory whose name ends `ext`. */
function walk(dir: string, ext: string): string[] {
  const out: string[] = [];
  const visit = (rel: string) => {
    for (const e of existsSync(abs(rel)) ? readdirSync(abs(rel), { withFileTypes: true }) : []) {
      if (e.isDirectory()) visit(`${rel}/${e.name}`);
      else if (e.name.endsWith(ext)) out.push(`${rel}/${e.name}`);
    }
  };
  visit(dir);
  return out;
}

/**
 * The language features, in the order each chapter's source gives them (`chapters.ts`). An entry's
 * anchor and help proof come from the generated chapter's section, which is what `nvs agent show`
 * prints and what the perf ledger and the site lock hash. A source with a problem, or a chapter whose
 * headings are not its source's, ends the command, because every id would be wrong.
 */
function langFeatures(): Entry[] {
  const sources = chapterSources();
  const problems = sources.flatMap((s) => s.problems);
  const sections = new Map(chapterFeatures(LANG, "lang").map((e) => [e.id, e]));
  const out: Entry[] = [];
  for (const source of sources) {
    const file = chapterFile(source);
    for (const f of source.features) {
      const id = `lang:${source.id}/${f.slug}`;
      const section = sections.get(id);
      sections.delete(id);
      if (!section) problems.push(`${source.path}:${f.line}: \`# ${f.heading}\` is not a heading in ${file}`);
      out.push({
        id,
        kind: "lang",
        group: `lang:${source.id}`,
        path: `lang/${source.id}/${f.slug}`,
        anchor: section?.anchor ?? "",
        summary: f.heading,
        help: section?.help ?? "",
      });
    }
  }
  for (const e of sections.values()) problems.push(`${e.anchor}: \`# ${e.summary}\` is in no chapter source under ${LANG_EXAMPLES}`);
  if (problems.length) throw new RosterError(`the language chapters disagree with their sources:\n  ${problems.join("\n  ")}`);
  return out;
}
