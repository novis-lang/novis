// The roster: every feature Novis ships, read from four live sources and never from a list. A `Core`
// class member, exception, enum, interface and configuration directive come from the binary's own
// `nvs meta --json`. A language feature is a `#` heading in a chapter under `docs/reference/lang/`, and
// a tool feature is one under `docs/reference/tools/`. Each feature has one path, `core/Str/length`,
// that the example, attack and bench trees share. `rule:testing/feature-proofs` is what each one owes.
//
// An anchor, `crates/…/file.rs:NN`, is where the feature is implemented when the roster can find it. It
// keys the perf ledger's currency, and a feature with no anchor is still on the roster.

import { existsSync, readdirSync, readFileSync } from "node:fs";
import { classConsts, filesUnder, nameConstsSignature, phpTwins, registry, registryNames, specRows } from "../cmd/gaps.ts";
import { noteKey, unrecorded } from "../lib/reads.ts";
import { ALL_CARDS, ALL_CLASSES, cardKey, classKey } from "../select/keys.ts";
import { anchorKey } from "./markers.ts";
import { abs } from "../lib/paths.ts";
import { comparePaths } from "../lib/py.ts";
import { run as runProc } from "../lib/proc.ts";
import { progress } from "../lib/progress.ts";

export const EXAMPLES = "docs/examples";
export const HOSTILE = "tests/hostile";
export const BENCHES = "benches/members";
/** A feature's website description, inside its example directory. It never counts as an example. */
export const ABOUT = "about.md";
const LANG = "docs/reference/lang";
const TOOLCHAPTERS = "docs/reference/tools";

export type Kind = "member" | "lang" | "exception" | "enum" | "interface" | "tool" | "directive";

/** One shipped feature, and where each of its feature proofs belongs. */
export interface Entry {
  id: string;
  kind: Kind;
  group: string;
  /** The relative path the example, attack and bench trees share. */
  path: string;
  /** `crates/…/file.rs:NN`, when the roster knows one. */
  anchor: string;
  /** The PHP built-ins it replaces, when the spec's **Replaces** column names them. */
  twin: string[];
  summary: string;
  /** What the binary's own help is missing, or "" when nothing. */
  help: string;
}

export const examplesDir = (e: Entry) => `${EXAMPLES}/${e.path}`;
export const aboutFile = (e: Entry) => `${examplesDir(e)}/${ABOUT}`;
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

export function slugify(text: string): string {
  const out = text.toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-+|-+$/g, "");
  return out || "unnamed";
}

/** `Core\Db\Row` -> `Db-Row`, the spelling every tree and the website use. */
export function classTail(name: string): string {
  return (name.startsWith("Core\\") ? name.slice(5) : name).replaceAll("\\", "-");
}

const lineAt = (text: string, offset: number) => text.slice(0, offset).split("\n").length;

/** `("Throwable", None)` in `nvs_hir::errors::TREE` and `("Comparable", &[...])` in `nvs_hir::interfaces::RESERVED`. */
const TUPLE_ROW_RE = /^\s*\((?:r"([^"]+)"|"((?:[^"\\]|\\.)+)"),/gm;
/** `pub const FINISH_MARKER: &str = "Core\\Script\\Finished";`, an exception declared beside the tree. */
const CONST_ROW_RE = /const\s+[A-Za-z_][A-Za-z0-9_]*\s*:\s*&str\s*=\s*(?:r"([^"]+)"|"((?:[^"\\]|\\.)+)")\s*;/g;
/** `Directive { key: "cache.local", ... }` in `nvs_config::DIRECTIVES`. */
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

/** `Class::member` -> the PHP built-ins the spec's **Replaces** column names for it. */
function twins(): Map<string, string[]> {
  const out = new Map<string, string[]>();
  for (const [owners, member, , replaces] of specRows()) {
    const names = phpTwins(replaces);
    if (names.length > 0) for (const owner of owners) out.set(`${owner}::${member}`, names);
  }
  return out;
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
          twin: [],
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
  classes?: { name: string; doc?: Doc; members?: { name: string; doc?: Doc; signature?: string }[] }[];
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
  const phpNames = twins();
  const out: Entry[] = [];

  for (const klass of doc.classes ?? []) {
    const cname = klass.name;
    const tail = classTail(cname);
    // A member's own card is enforced by the registry test. What the binary can still be missing is
    // the class's card, which a completion list and `show` print above the member.
    const classHelp = klass.doc?.short
      ? ""
      : `\`${cname}\` has no class card: add its \`ClassDoc\` above the class's \`CLASS\` row and delete it from \`CLASSES_STILL_OWING_A_CARD\` in crates/nvs-stdlib/src/registry.rs`;
    for (const member of klass.members ?? []) {
      const mname = member.name;
      const key = `${cname}::${mname}`;
      out.push({
        id: key,
        kind: "member",
        group: cname,
        path: `core/${tail}/${mname}`,
        anchor: anchors.get(key) ?? "",
        twin: phpNames.get(key) ?? [],
        summary: member.doc?.short || member.signature || "",
        help: member.doc?.short ? classHelp : `\`${key}\` has no card, so \`nvs agent show\` prints nothing for it`,
      });
    }
  }

  for (const [kind, key] of [["exception", "exceptions"], ["enum", "enums"], ["interface", "interfaces"]] as const) {
    for (const item of doc[key] ?? []) {
      const name = isObject(item) ? String(item.name) : pyStr(item);
      const short = isObject(item) ? ((item.doc as Doc)?.short ?? "") : "";
      out.push({
        id: name,
        kind,
        group: `types:${kind}`,
        path: `types/${classTail(name)}`,
        anchor: tables[kind]!.get(name) ?? "",
        twin: [],
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
      twin: [],
      summary: isObject(d) ? `${pyStr(d.class)} directive, applied at ${pyStr(d.apply)}` : "",
      help: "",
    });
  }

  out.push(...chapterFeatures(LANG, "lang"), ...chapterFeatures(TOOLCHAPTERS, "tools"));
  return out;
}
