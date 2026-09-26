// `bun nv reference`: generate `docs/novis.md`, the one-file reference to everything Novis has, and
// prove what the two generated documents claim: this one, and the primer the binary renders.
//
//     bun nv reference                  regenerate docs/novis.md, then run every example in it
//     bun nv reference --check          regenerate in memory; exit 1 if docs/novis.md is stale
//     bun nv reference --no-examples    regenerate only
//     bun nv reference --examples-only [--only <substring>]   run the examples, write nothing
//     bun nv reference --primer --check prove `nvs agent primer` instead, writing nothing
//     bun nv reference --agent-walk     walk the surface cold: primer, find, show, check
//     bun nv reference --keep           leave the example directories under .agent-tmp/ behind
//
// `docs/novis.md` is written for a reader who has never seen this repository, and is generated, never
// edited. Its inputs are `nvs meta --json` from the built binary (every `Core` class, member, card,
// constant and enum, the exception tree, the global interfaces, the attributes and the `nvs.toml`
// directives), so the reference cannot describe a member that does not run; one hand-written chapter
// per topic under `docs/reference/lang/` and `docs/reference/tools/`, in filename order, in the format
// `docs/reference/README.md` gives; and an optional introduction per class under
// `docs/reference/core/`, named for the class after `Core\` with `\` written `-`.
//
// Every fenced `nvs` block in a chapter is a program this command runs against the binary, and the
// `output` fence after it is what the program must print, so an example that stops being true fails
// here rather than misleading the next reader.
//
// `nvs agent primer` is assembled by the binary from `<!-- primer -->`-marked sections of the same
// chapters (`rule:tooling/an-agent-asks-the-binary`), so this command writes no part of it: `--primer`
// proves the assembled document instead. Its examples run through the same harness, and each `E0xxx`
// it names must be declared in the diagnostic registry, because a refusal's PHP cell is a fragment and
// the code beside it is what can be executed about it (`rule:tooling/a-primer-claim-is-executed`).
// `--agent-walk` proves the surface rather than either document: each step is handed only what the
// step before it printed, so an answer that is not an address the next command resolves fails here.

import { existsSync, mkdirSync, readdirSync, readFileSync, rmSync, statSync, writeFileSync } from "node:fs";
import { availableParallelism } from "node:os";
import { basename, dirname, join, relative, sep } from "node:path";
import { covwsNvs } from "../lib/covws.ts";
import { ROOT, rel } from "../lib/paths.ts";
import { run as runProc } from "../lib/proc.ts";
import { ArgError, comparePaths, parseArgs, pyInt, pyRepr, splitlines } from "../lib/py.ts";
import { record } from "../lib/written.ts";

export const summary = "generate docs/novis.md and prove its examples: nv reference [--check | --no-examples | --examples-only [--only S] | --primer | --agent-walk] [--keep]";

const OUT = join(ROOT, "docs", "novis.md");
const SOURCES = join(ROOT, "docs", "reference");
const MIGRATION = join(ROOT, "docs", "spec", "02-php-migration.md");
/** The whole diagnostic registry: a code declared anywhere else does not exist. */
const DIAGNOSTICS = join(ROOT, "crates", "nvs-diagnostics", "src", "lib.rs");
const TMP = join(ROOT, ".agent-tmp", "reference-examples");
/** Where `--primer` parks what `nvs agent primer` printed, so a failure's `file:line` names a file still on disk. */
const PRIMER = join(ROOT, ".agent-tmp", "primer", "primer.md");
/** Where `--agent-walk` assembles the program it hands `nvs check`, for the same reason. */
const WALK = join(ROOT, ".agent-tmp", "agent-walk");
let binary = "";

/** The `nvs` this command runs: `NVS_BIN` when it is set, as `nv verify` and the acceptance sweep set
 * it, and otherwise the pipeline's own `covws` build. */
function nvsBinary(): string {
  return (binary ||= process.env.NVS_BIN || covwsNvs());
}

/** Seconds per example; a hung example is a bug in the example. */
const TIMEOUT = 60;

/** A chapter's leading `---` block: `key: value` lines, three of which mean something. */
const FRONT_RE = /^---\n([\s\S]*?)\n---\n/;
/** A fenced block: the info string, then the body up to the closing fence. */
const FENCE_RE = /^```([^\n]*)\n([\s\S]*?)^```[ \t]*$/gm;
/** A generated-table placeholder inside a chapter. */
const PLACEHOLDER_RE = /^<!-- generated: ([a-z-]+) -->$/gm;
/** A chapter's own-source note, stripped from the generated file. */
const SRC_RE = /^<!-- src:.*?-->[ \t]*\n?/gm;
/** A chapter section's primer marker, stripped from the generated file. */
const PRIMER_RE = /^<!-- primer -->[ \t]*\n?/gm;
/** A markdown heading, for demotion. */
const HEADING_RE = /^(#{1,6}) (.*)$/gm;
/** One row of the migration table. */
const ROW_RE = /^\| `([^`]+)` \| (member|language|dropped|open) \| (.*) \|$/;
/** A markdown link whose target is a path rather than a URL or an in-page anchor. */
const REL_LINK_RE = /(?<=\]\()(?![\p{L}\p{N}_]+:|[#/])([^)]+)(?=\))/gu;
/** One declared diagnostic code, in the registry. */
const CODE_DECL_RE = /Code::new\("(E\d{4})"\)/g;
/** One diagnostic code cited in prose or in a table cell. */
const CODE_CITED_RE = /\bE\d{4}\b/g;
/** A parenthesised decision citation inside a registry card, which names a file the reference's readers do not have. */
const ADR_PAREN_RE = /\s*\(ADR \d{4}(?: §+ [\p{L}\p{N}_.\-]+)?\)/gu;
/** A `Core` member as a program calls it: the qualified name, and the parenthesis that makes it a call. */
const CALLED_RE = /(Core\\[A-Za-z0-9_\\]*[A-Za-z0-9_]::[A-Za-z_][A-Za-z0-9_]*)\(/;

/**
 * A constant whose value the binary answers per platform, and the one spelling the generated file uses
 * for it everywhere. Keyed by the constant rather than by the value: `Core\Env::OS` has several values
 * and `Core\Env::EOL`'s two differ only by a `\r` that is easy to miss in a diff.
 */
const PLATFORM_VALUES: Record<string, string> = {
  "Core\\Path::SEPARATOR": '"/" ("\\" on Windows)',
  "Core\\Env::EOL": '"\\n" ("\\r\\n" on Windows)',
  "Core\\Env::OS": '"Linux" ("Windows" or "Darwin" per platform)',
};

/** A refusal that stops the command: printed alone on stderr, exit 1. */
class Fatal extends Error {}

// ------------------------------------------------------------------ Python's string handling

/** Python's `str.strip(chars)` for a set of characters. */
function stripChars(s: string, chars: string): string {
  let a = 0;
  let b = s.length;
  while (a < b && chars.includes(s[a]!)) a++;
  while (b > a && chars.includes(s[b - 1]!)) b--;
  return s.slice(a, b);
}

/** Python's `str.strip()` with no argument. */
const strip = (s: string) => s.replace(/^\s+|\s+$/gu, "");

/** How many `\n` come before `offset`. */
function lineAt(text: string, offset: number): number {
  let n = 1;
  for (let i = text.indexOf("\n"); i >= 0 && i < offset; i = text.indexOf("\n", i + 1)) n++;
  return n;
}

/** A directory's `*.md` files in the order `sorted(Path.glob)` gives them on Windows. */
function markdownIn(dir: string): string[] {
  if (!existsSync(dir)) return [];
  return readdirSync(dir)
    .filter((n) => n.endsWith(".md") && statSync(join(dir, n)).isFile())
    .sort(comparePaths)
    .map((n) => join(dir, n));
}

const readText = (path: string) => readFileSync(path, "utf8");

/** Writes `body` exactly, creating the directories above it. */
function writeText(path: string, body: string): void {
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, body, "utf8");
}

// ------------------------------------------------------------------ sources

interface Chapter {
  path: string;
  id: string;
  title: string;
  keywords: string;
  summary: string;
  body: string;
}

function chapterAnchor(ch: Chapter): string {
  return `${basename(dirname(ch.path)) === "tools" ? "tools" : "lang"}-${ch.id}`;
}

function parseFront(raw: string, path: string): [Map<string, string>, string] {
  const text = raw.replace(/\r\n/g, "\n");
  const m = FRONT_RE.exec(text);
  if (!m) throw new Fatal(`nv reference: ${rel(path)} has no leading \`---\` block`);
  const fields = new Map<string, string>();
  for (const line of splitlines(m[1]!)) {
    const colon = line.indexOf(":");
    const key = colon >= 0 ? line.slice(0, colon) : line;
    let value = strip(colon >= 0 ? line.slice(colon + 1) : "");
    // A title holding a `:` is quoted to survive the split above; the quotes are the front matter's.
    if (value.length >= 2 && value[0] === value[value.length - 1] && "\"'".includes(value[0]!)) value = value.slice(1, -1);
    fields.set(strip(key), value);
  }
  return [fields, text.slice(m[0].length)];
}

function loadChapters(part: string): Chapter[] {
  return markdownIn(join(SOURCES, part)).map((path) => {
    const [fields, body] = parseFront(readText(path), path);
    for (const need of ["id", "title"]) {
      if (!fields.has(need)) throw new Fatal(`nv reference: ${rel(path)} lacks \`${need}:\``);
    }
    return {
      path,
      id: fields.get("id")!,
      title: fields.get("title")!,
      keywords: fields.get("keywords") ?? "",
      summary: fields.get("summary") ?? "",
      body: stripChars(body, "\n"),
    };
  });
}

/** The hand-written introduction for a `Core` class, or null when nobody wrote one. */
function classIntro(name: string): [Map<string, string>, string] | null {
  const stem = (name.startsWith("Core\\") ? name.slice(5) : name).replaceAll("\\", "-");
  const path = join(SOURCES, "core", `${stem}.md`);
  if (!existsSync(path) || !statSync(path).isFile()) return null;
  return parseFront(readText(path), path);
}

/** The registry as `nvs meta --json` prints it, read field by field below. */
type Json = any;

async function registry(): Promise<Json> {
  if (!existsSync(nvsBinary())) throw new Fatal(`nv reference: no binary at ${rel(nvsBinary())} -- \`bun nv verify\` builds it`);
  const p = await runProc([nvsBinary(),"meta", "--json"]);
  if (p.code !== 0) throw new Fatal(`nv reference: \`nvs meta --json\` failed:\n${p.stderr}`);
  return JSON.parse(p.stdout);
}

/**
 * `text`'s relative links rewritten from `source`'s directory to `OUT`'s. A cell copied out of
 * `docs/spec/` keeps the links it was written with, which resolve from there and not from `docs/`.
 */
function rerootLinks(text: string, source: string): string {
  return text.replace(REL_LINK_RE, (link: string) => {
    const hash = link.indexOf("#");
    const target = hash >= 0 ? link.slice(0, hash) : link;
    const anchor = hash >= 0 ? link.slice(hash + 1) : "";
    if (!target) return link;
    const moved = relative(dirname(OUT), join(dirname(source), target)).split(sep).join("/") || ".";
    return moved + (anchor ? `#${anchor}` : "");
  });
}

/** [section, php, outcome, novis] for every row of docs/spec/02-php-migration.md. */
function migrationRows(): [string, string, string, string][] {
  const rows: [string, string, string, string][] = [];
  let section = "";
  for (const line of splitlines(readText(MIGRATION))) {
    if (line.startsWith("## ")) section = strip(line.slice(3));
    const m = ROW_RE.exec(line);
    if (m) rows.push([section, m[1]!, m[2]!, m[3]!]);
  }
  return rows;
}

// ------------------------------------------------------------------ rendering helpers

function anchorId(...parts: string[]): string {
  const text = parts.join("-").toLowerCase().replaceAll("\\", "-").replaceAll("::", "-").replace(/[^a-z0-9]+/g, "-");
  return stripChars(text, "-");
}

const anchor = (id: string) => `<a id="${id}"></a>`;

/** Every heading in a chapter body pushed down `by` levels, so a chapter's own `##` nests. */
function demote(body: string, by: number): string {
  return body.replace(HEADING_RE, (_m, hashes: string, rest: string) => `${"#".repeat(Math.min(6, hashes.length + by))} ${rest}`);
}

const code = (text: string) => `\`${text}\``;

/** A `|` escaped, so a cell holding one does not split the table row. */
const esc = (text: string) => text.replaceAll("|", "\\|");

/** A card's prose with its parenthesised decision citations removed; an inline one stays. */
const card = (text: string) => text.replace(ADR_PAREN_RE, "");

// ------------------------------------------------------------------ Part B: the Core library

function memberHeading(className: string, member: Json): string {
  return member.kind === "instance" ? `${className}->${member.name}` : `${className}::${member.name}`;
}

function classVar(className: string): string {
  const last = className.slice(className.lastIndexOf("\\") + 1);
  return last[0]!.toLowerCase() + last.slice(1);
}

/** How a call is written: `Core\Str::length(string $s): uint`, `$dt->format(string $pattern)`. */
function spelledSignature(className: string, member: Json): string {
  const sig: string = member.signature;
  if (member.kind === "instance") return `$${classVar(className)}->${sig}`;
  if (member.kind === "constructor") return `new ${className}(${sig.slice("constructor(".length)}`;
  return `${className}::${sig}`;
}

function qualifier(q: string | undefined): string | null {
  return q && q !== "contagious" ? q : null;
}

function renderMember(className: string, member: Json, lines: string[]): void {
  const heading = memberHeading(className, member);
  lines.push(anchor(anchorId("core", heading)), `#### \`${heading}\``, "", "```nvs skip", spelledSignature(className, member), "```", "");
  const doc = member.doc ?? {};
  if (doc.short) lines.push(card(doc.short), "");
  const docsByName = new Map<string, Json>((doc.params ?? []).map((p: Json) => [p.name, p]));
  const params: Json[] = member.params ?? [];
  const options: Json[] = member.options ?? [];
  if (params.length > 0 || options.length > 0) {
    lines.push("| Parameter | Type | Meaning |", "|---|---|---|");
    for (const p of params) {
      const name = p.variadic ? `...$${p.name}` : `$${p.name}`;
      const extras: string[] = [];
      if ("default" in p) extras.push(`default ${code(p.default)}`);
      const q = qualifier(p.qualifier);
      if (q) extras.push(q);
      const pdoc = docsByName.get(p.name) ?? {};
      let desc = card(pdoc.desc ?? "");
      const shape: Json[] = pdoc.shape ?? [];
      if (shape.length > 0) desc += ` Keys: ${shape.map((k) => `\`${k.key}\` (${k.type}) ${card(k.desc)}`).join("; ")}`;
      const meta = extras.length > 0 ? ` (${extras.join(", ")})` : "";
      lines.push(`| \`${name}\` | \`${esc(p.type)}\`${meta} | ${esc(desc)} |`);
    }
    for (const o of options) {
      const extras = [`default ${code(o.default)}`];
      const q = qualifier(o.qualifier);
      if (q) extras.push(q);
      const desc = card(docsByName.get(o.name)?.desc ?? "");
      lines.push(`| \`{${o.name}: …}\` | \`${esc(o.type)}\` (${extras.join(", ")}) | ${esc(desc)} |`);
    }
    lines.push("");
  }
  const ret = card(doc.return ?? "");
  if (member.kind !== "constructor") lines.push(`**Returns** \`${member.returns}\`${ret ? ` — ${ret}` : ""}`, "");
  if (doc.errors?.length) lines.push(`**Throws** ${doc.errors.map((e: Json) => `\`${e.error}\` — ${card(e.desc)}`).join("; ")}`, "");
}

function renderClass(cls: Json, lines: string[], index: string[]): void {
  const name: string = cls.name;
  const display = name + (cls.typeParams?.length ? `<${cls.typeParams.join(", ")}>` : "");
  lines.push(anchor(anchorId("core", name)), `### \`${display}\``, "");
  const intro = classIntro(name);
  let summary = "";
  let keywords = cls.members.map((m: Json) => m.name).join(", ");
  if (intro) {
    summary = intro[0].get("summary") ?? "";
    if (intro[0].get("keywords")) keywords = `${intro[0].get("keywords")}, ${keywords}`;
  }
  lines.push(`Keywords: ${keywords}`, "");
  if (intro) lines.push(demote(stripChars(intro[1], "\n"), 3), "");
  index.push(`| [\`${display}\`](#${anchorId("core", name)}) | ${esc(summary)} |`);
  // The quick index of members, one line each, before the cards.
  lines.push("| Member | Signature |", "|---|---|");
  // `constructor` is read as an own field: every object inherits one of that name.
  const ctor = Object.hasOwn(cls, "constructor") ? cls.constructor : null;
  if (ctor) lines.push(`| \`new ${name}\` | \`${esc(spelledSignature(name, ctor))}\` |`);
  for (const m of cls.members) {
    lines.push(`| [\`${memberHeading(name, m)}\`](#${anchorId("core", memberHeading(name, m))}) | \`${esc(m.signature)}\` |`);
  }
  for (const k of cls.constants ?? []) {
    // The binary answers the platform it was built on; the reference is read everywhere.
    const value = PLATFORM_VALUES[`${name}::${k.name}`] ?? k.value;
    lines.push(`| \`${name}::${k.name}\` | \`${k.type}\` = \`${esc(value)}\` — ${esc(card(k.doc ?? ""))} |`);
  }
  lines.push("");
  if (ctor) renderMember(name, { ...ctor, name: "constructor" }, lines);
  for (const m of cls.members) renderMember(name, m, lines);
}

function renderEnums(enums: Json[], lines: string[]): void {
  lines.push(
    anchor("core-enums"),
    "### `Core` enums",
    "",
    "Every enum `Core` declares. A case is written `Core\\Order::Asc` and is an `int` underneath (see the enums chapter); a member's signature names the enum it takes.",
    "",
  );
  for (const e of enums) {
    const doc = e.doc ?? {};
    lines.push(anchor(anchorId("enum", e.name)), `#### \`${e.name}\``, "");
    if (doc.short) lines.push(card(doc.short), "");
    lines.push("| Case | Meaning |", "|---|---|");
    for (const c of doc.cases ?? []) lines.push(`| \`${e.name}::${c.name}\` | ${esc(card(c.desc))} |`);
    lines.push("");
  }
}

const TABLES: Record<string, (reg: Json) => string> = {
  exceptions: (reg) => {
    const lines = ["| Class | Extends | Own properties |", "|---|---|---|"];
    for (const x of reg.exceptions) {
      const props = (x.properties ?? []).map((p: string) => code(`$${p}`)).join(", ") || "—";
      lines.push(`| \`${x.name}\` | ${x.parent ? code(x.parent) : "— (the root)"} | ${props} |`);
    }
    return lines.join("\n");
  },
  interfaces: (reg) => {
    const lines = ["| Interface | Type parameters |", "|---|---|"];
    for (const i of reg.interfaces) lines.push(`| \`${i.name}\` | ${(i.typeParams ?? []).join(", ") || "—"} |`);
    return lines.join("\n");
  },
  attributes: (reg) => reg.attributes.map((a: string) => `- \`#[${a}]\``).join("\n"),
  directives: (reg) => {
    const meaning: Record<string, string> = {
      System: "operator only — a request cannot change it",
      Runtime: "a request may retune it, up to the `[limits.hard]` ceiling",
      RuntimeTighten: "a request may only narrow it",
    };
    const lines = ["| Key | Class | Applies |", "|---|---|---|"];
    for (const d of reg.directives) {
      lines.push(`| \`${d.key}\` | ${meaning[d.class] ?? d.class} | ${d.apply === "Reload" ? "at reload" : "at boot only"} |`);
    }
    return lines.join("\n");
  },
  "php-migration": (reg) => {
    const known = new Set<string>();
    for (const c of reg.classes) {
      for (const m of c.members) known.add(`${c.name}::${m.name}`).add(`${c.name}->${m.name}`);
    }
    const lines = ["| PHP | Outcome | Novis |", "|---|---|---|"];
    for (const [, php, outcome, novis] of migrationRows()) {
      if (outcome === "open") continue;
      if (outcome === "member") {
        const names = [...novis.matchAll(/`(Core\\[A-Za-z\\]+(?:::|->)[a-zA-Z]+)`/g)].map((m) => m[1]!);
        if (names.length === 0 || !names.every((n) => known.has(n))) continue;
      }
      lines.push(`| \`${php}\` | ${outcome} | ${rerootLinks(novis, MIGRATION)} |`);
    }
    return lines.join("\n");
  },
};

// ------------------------------------------------------------------ the document

const HOW_TO_READ = `## How to read this file

This is the complete reference to the Novis language and its \`Core\` library, in one file, generated
from the compiler's own registry and from one hand-written chapter per topic. **Do not read it top to
bottom.** Find what you need through the index below, then read one section:

- **Every section starts with an HTML anchor** — \`<a id="lang-types"></a>\` — directly above its
  heading, and every heading names its subject in full (\`#### Core\\Str::length\`), so a search for the
  anchor id, the heading text, or a keyword lands on the section. A \`Keywords:\` line under each
  chapter and class heading lists what it covers for searching.
- **Part A is the language**: syntax and semantics, one chapter per topic, each with runnable
  examples. Read A.1 first if you have never seen Novis; it is short. Read a chapter in full when you
  need its topic — they are written to be complete rather than introductory.
- **Part B is the \`Core\` library**: one section per class, opening with a short introduction and one
  worked example, then a member index, then **one card per member** with its signature, parameters,
  return value and what it throws. Every member exists in the shipped binary; nothing planned is here.
- **Part C is the toolchain**: the \`nvs\` command, \`nvs.toml\`, and testing.
- **Part D is the PHP crosswalk**: for someone who knows PHP, what each built-in became.

Conventions the whole file uses:

- A signature reads \`Core\\Str::length(string $s): uint\` for a static member — every \`Core\` member is
  static unless written \`$x->name(...)\`, which marks an instance method reached through a value that
  some other member returned. The \`$name\` of every parameter is callable by name: \`Core\\Str::length(s:
  $x)\`. A trailing \`{a?: T, b?: U}\` is an *options bag* — one optional shape argument, written
  \`{a: value}\`, addressable as \`options:\`.
- \`?T\` means "\`T\` or \`null\`", and is how every member spells *absent*. Failure is a thrown
  \`Throwable\` subclass named on the card; nothing returns \`false\` to mean failure.
- Every code block marked \`nvs\` is a complete program that was run against the binary while this
  file was generated, and the \`output\` block after it is exactly what it printed. Blocks marked
  \`nvs skip\` are fragments. Run a program with \`nvs run file.nvs\`.
`;

function build(reg: Json): string {
  const lang = loadChapters("lang");
  const tools = loadChapters("tools");
  const lines: string[] = [];
  const indexLine = (part: string, n: number, ch: Chapter) =>
    `- ${part}.${n} [${ch.title}](#${chapterAnchor(ch)}) — ${ch.summary} *(${ch.keywords})*`;
  lines.push("# Novis — the complete reference", "");
  // The `GENERATED FILE` marker is the repository's convention and the link checker reads it: without
  // it this file's chapter-relative links are checked again here, where they have become anchors.
  lines.push(
    "<!-- GENERATED FILE — do not edit by hand. Written by `bun nv reference` from `nvs meta --json` and docs/reference/: edit the chapter under docs/reference/ or the registry in crates/nvs-stdlib, then regenerate. -->",
    "",
    HOW_TO_READ,
  );
  lines.push("## Index", "", "### Part A — The language", "");
  lang.forEach((ch, i) => lines.push(indexLine("A", i + 1, ch)));
  lines.push("", "### Part B — The `Core` library", "");
  const classIndex: string[] = [];
  const bodyB: string[] = [];
  for (const cls of reg.classes) renderClass(cls, bodyB, classIndex);
  renderEnums(reg.enums, bodyB);
  lines.push("| Class | What it is for |", "|---|---|", ...classIndex);
  lines.push("| [`Core` enums](#core-enums) | every enum a member takes, with its cases |", "");
  lines.push("### Part C — The toolchain", "");
  tools.forEach((ch, i) => lines.push(indexLine("C", i + 1, ch)));
  lines.push("", "### Part D — Coming from PHP", "", "- D.1 [PHP built-ins and what each became](#php-migration)", "");

  const used = new Set<string>();
  const expand = (body: string): string => {
    const out = body.replace(PLACEHOLDER_RE, (_m, name: string) => {
      const table = TABLES[name];
      if (!table) throw new Fatal(`nv reference: unknown placeholder \`${name}\``);
      used.add(name);
      return table(reg);
    });
    // A chapter's `<!-- src: ... -->` lines say which decision owns a paragraph, and a `<!-- primer -->`
    // marker selects what `nvs agent primer` lifts; neither says anything to this file's reader.
    return out.replace(SRC_RE, "").replace(PRIMER_RE, "");
  };
  const chapters = (part: string, list: Chapter[]) => {
    list.forEach((ch, i) => {
      lines.push(anchor(chapterAnchor(ch)), `## ${part}.${i + 1} ${ch.title}`, "", `Keywords: ${ch.keywords}`, "", expand(demote(ch.body, 2)), "");
    });
  };
  lines.push("# Part A — The language", "");
  chapters("A", lang);
  lines.push(
    "# Part B — The `Core` library",
    "",
    "Every function and constant in Novis is a member of a class under the reserved `Core` namespace; there are no free functions. The sections below are in the registry's own order. A class with type parameters (`Core\\ObjectMap<K, V>`) is written with them at `new`.",
    "",
    ...bodyB,
  );
  lines.push("# Part C — The toolchain", "");
  chapters("C", tools);
  lines.push(
    "# Part D — Coming from PHP",
    "",
    anchor("php-migration"),
    "## D.1 PHP built-ins and what each became",
    "",
    "Keywords: PHP, migration, replaces, equivalent, what happened to",
    "",
    "One row per PHP built-in. *member*: a `Core` member in Part B does the job. *language*: an operator or keyword does it. *dropped*: nothing does, and the cell says why and what to write instead — a `Core\\Name` in a *dropped* row's text that has no section in Part B is a description of the rewrite, not a member that exists today. Built-ins still undecided are not listed.",
    "",
    TABLES["php-migration"]!(reg),
    "",
  );
  used.add("php-migration");
  // A roster no chapter placed is appended, so nothing the binary declares is lost.
  for (const [name, table] of Object.entries(TABLES)) {
    if (!used.has(name)) lines.push(anchor(anchorId("roster", name)), `## ${name}`, "", table(reg), "");
  }
  const text = lines.join("\n").replace(/\n{3,}/g, "\n\n");
  return `${text.replace(/\n+$/, "")}\n`;
}

// ------------------------------------------------------------------ examples

interface Example {
  chapter: string;
  index: number;
  line: number;
  mode: "run" | "error" | "test";
  /** The entry program's source. */
  entry: string;
  files: Map<string, string>;
  expected: string | null;
  exitCode: number;
}

function parseInfo(info: string): [string, Map<string, string>] {
  const parts = strip(info).split(/\s+/).filter(Boolean);
  const attrs = new Map<string, string>();
  for (const p of parts.slice(1)) {
    const eq = p.indexOf("=");
    attrs.set(eq >= 0 ? p.slice(0, eq) : p, eq >= 0 ? p.slice(eq + 1) : "");
  }
  return [parts[0] ?? "", attrs];
}

function examplesIn(path: string): Example[] {
  const text = readText(path).replace(/\r\n/g, "\n");
  const out: Example[] = [];
  let pending = new Map<string, string>();
  let last: Example | null = null;
  for (const m of text.matchAll(FENCE_RE)) {
    const [lang, attrs] = parseInfo(m[1]!);
    const body = m[2]!;
    const line = lineAt(text, m.index!);
    if (lang === "output") {
      if (last === null) throw new Fatal(`nv reference: ${basename(path)}:${line}: an \`output\` block with no program before it`);
      last.expected = body;
      last = null;
      continue;
    }
    if (attrs.has("file")) {
      pending.set(attrs.get("file")!, body);
      continue;
    }
    if (lang !== "nvs" || attrs.has("skip")) continue;
    const mode = attrs.has("error") ? "error" : attrs.has("test") ? "test" : "run";
    const exitCode = pyInt(attrs.get("exit") ?? "0");
    if (exitCode === null) throw new Fatal(`nv reference: ${basename(path)}:${line}: \`exit=${attrs.get("exit")}\` is not a number`);
    const ex: Example = { chapter: path, index: out.length + 1, line, mode, entry: body, files: pending, expected: null, exitCode };
    pending = new Map();
    out.push(ex);
    last = ex;
  }
  return out;
}

function normalize(s: string): string {
  return stripChars(
    s
      .replace(/\r\n/g, "\n")
      .split("\n")
      .map((line) => line.replace(/\s+$/u, ""))
      .join("\n"),
    "\n",
  );
}

/** The binary run once, with its output's newlines normalized. */
async function ask(args: string[], cwd?: string) {
  const p = await runProc([nvsBinary(),...args], { cwd: cwd ?? ROOT, timeoutMs: TIMEOUT * 1000 });
  return { ...p, stdout: p.stdout.replace(/\r\n/g, "\n"), stderr: p.stderr.replace(/\r\n/g, "\n") };
}

/** A fresh directory holding an example's side files and its entry as `main.nvs`; returns the entry. */
function lay(work: string, ex: Example): string {
  rmSync(work, { recursive: true, force: true });
  mkdirSync(work, { recursive: true });
  for (const [name, body] of ex.files) writeText(join(work, name), body);
  const entry = join(work, "main.nvs");
  writeText(entry, ex.entry);
  return entry;
}

/** Null when the example holds; otherwise one paragraph saying how it failed. */
async function runExample(ex: Example, keep: boolean): Promise<string | null> {
  const where = `${rel(ex.chapter)}:${ex.line}`;
  const stem = basename(ex.chapter).replace(/\.[^.]*$/, "");
  const work = join(TMP, basename(dirname(ex.chapter)), `${stem}-${String(ex.index).padStart(2, "0")}`);
  lay(work, ex);
  const verb = ex.mode === "test" ? "test" : ex.mode === "error" ? "check" : "run";
  const p = await runProc([nvsBinary(),verb, "main.nvs"], { cwd: work, timeoutMs: TIMEOUT * 1000 });
  if (p.timedOut) return `${where}: timed out after ${TIMEOUT}s`;
  const { stdout, stderr } = p;
  let problem: string | null = null;
  if (ex.mode === "error") {
    if (p.code === 0) problem = "was expected to fail `nvs check`, but it compiled";
    else if (ex.expected !== null && !normalize(stderr).includes(normalize(ex.expected))) {
      problem = `failed as expected, but the diagnostic does not contain the \`output\` block:\n${strip(stderr)}`;
    }
  } else if (ex.mode === "test") {
    if (p.code !== ex.exitCode) problem = `\`nvs test\` exited ${p.code}, expected ${ex.exitCode}:\n${strip(stdout)}\n${strip(stderr)}`;
    else if (ex.expected !== null) {
      const missing = normalize(ex.expected)
        .split("\n")
        .filter((l) => l && !stdout.includes(l));
      if (missing.length > 0) problem = `\`nvs test\` output lacks [${missing.map(pyRepr).join(", ")}]:\n${strip(stdout)}`;
    }
  } else if (p.code !== ex.exitCode) {
    problem = `exited ${p.code}, expected ${ex.exitCode}:\n${strip(stdout)}\n${strip(stderr)}`;
  } else if (ex.expected !== null && normalize(stdout) !== normalize(ex.expected)) {
    problem = `printed something else.\n--- expected\n${normalize(ex.expected)}\n--- got\n${normalize(stdout)}`;
    if (strip(stderr)) problem += `\n--- stderr\n${strip(stderr)}`;
  }
  if (!keep && problem === null) rmSync(work, { recursive: true, force: true });
  return problem ? `${where}: ${problem}` : null;
}

/** `task` over every item, `width` at a time, with the results in the items' order. */
async function pool<T, R>(items: T[], width: number, task: (item: T) => Promise<R>): Promise<R[]> {
  const results: R[] = new Array(items.length);
  let next = 0;
  const worker = async () => {
    while (next < items.length) {
      const i = next++;
      results[i] = await task(items[i]!);
    }
  };
  await Promise.all(Array.from({ length: Math.min(width, items.length) }, worker));
  return results;
}

async function checkExamples(only: string | undefined, keep: boolean): Promise<number> {
  let paths = ["lang", "tools", "core"].flatMap((part) => markdownIn(join(SOURCES, part)));
  if (only) paths = paths.filter((p) => p.split(sep).join("/").includes(only));
  const examples = paths.flatMap(examplesIn);
  if (examples.length === 0) {
    console.log("nv reference: no examples to run");
    return 0;
  }
  mkdirSync(TMP, { recursive: true });
  const width = Math.max(2, Math.min(8, Math.floor(availableParallelism() / 2)));
  const failures = (await pool(examples, width, (ex) => runExample(ex, keep))).filter((r): r is string => r !== null);
  for (const f of failures) console.log(`FAIL ${f}\n`);
  console.log(`nv reference: ${examples.length - failures.length} of ${examples.length} examples hold${failures.length > 0 ? ` (${failures.length} failed)` : ""}`);
  return failures.length > 0 ? 1 : 0;
}

/**
 * How many diagnostic codes the primer names, and one line per code declared nowhere. A refusal the
 * primer states is a table row whose PHP cell is a fragment, so there is no program to hand `nvs
 * check`: what is executable about the row is the code in its third column, and a refusal that cannot
 * be raised is the one lie a generated document can still tell.
 */
function undeclaredCodes(text: string): [number, string[]] {
  const declared = new Set([...readText(DIAGNOSTICS).matchAll(CODE_DECL_RE)].map((m) => m[1]!));
  const firstLine = new Map<string, number>();
  for (const m of text.matchAll(CODE_CITED_RE)) {
    if (!firstLine.has(m[0])) firstLine.set(m[0], lineAt(text, m.index!));
  }
  const bad = [...firstLine.entries()]
    .sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0))
    .filter(([c]) => !declared.has(c))
    .map(([c, line]) => `${rel(PRIMER)}:${line}: \`${c}\` is stated as a refusal, and is declared nowhere in ${rel(DIAGNOSTICS)}`);
  return [firstLine.size, bad];
}

/** Proves the primer against the binary that printed it: its examples run, its refusals exist. */
async function checkPrimer(keep: boolean): Promise<number> {
  const p = await ask(["agent", "primer"]);
  if (p.code !== 0) {
    console.log(`nv reference: \`nvs agent primer\` exited ${p.code}\n${strip(p.stderr)}`);
    return 1;
  }
  writeText(PRIMER, p.stdout);
  // The assembled document is what is proven, not the chapters it lifted from: an example the primer
  // carries without the `file` block or `output` fence beside it in the chapter fails only here.
  const examples = examplesIn(PRIMER);
  const [cited, badCodes] = undeclaredCodes(p.stdout);
  if (examples.length === 0 || cited === 0) {
    console.log(`nv reference: the primer prints ${examples.length} example(s) and names ${cited} diagnostic code(s) -- a marked section has lost the claim this proves`);
    return 1;
  }
  const badExamples: string[] = [];
  for (const ex of examples) {
    const r = await runExample(ex, keep);
    if (r) badExamples.push(r);
  }
  for (const f of [...badCodes, ...badExamples]) console.log(`FAIL ${f}\n`);
  console.log(`nv reference: ${examples.length - badExamples.length} of ${examples.length} primer examples hold, ${cited - badCodes.length} of ${cited} refusal codes are declared`);
  return badCodes.length > 0 || badExamples.length > 0 ? 1 : 0;
}

// ------------------------------------------------------------------ the cold-start walk

/**
 * The symbol an index line opens with: the line up to its first `(`, `<` or space.
 * `crates/nvs-cli/src/agent.rs`'s header owns that shape, and reading a printed line back the way the
 * surface promises a consumer can is half of what the walk proves.
 */
function symbolOf(line: string): string {
  const cuts = ["(", "<", " "].map((c) => line.indexOf(c)).filter((at) => at >= 0);
  return line.slice(0, cuts.length > 0 ? Math.min(...cuts) : line.length);
}

/**
 * Walks the surface cold, the way an agent holding nothing but this binary walks it
 * (`rule:tooling/an-agent-asks-the-binary`). Every step is handed only what the step before it
 * printed: the members looked up are the ones the primer's own programs call, the symbol `show` is
 * asked for is the one `find`'s line opened with, and the program `nvs check` reads is the one the
 * primer prints. Each program is checked twice: as printed, which must pass, and with its first call
 * written as a free function, which must be refused by name.
 */
async function checkWalk(keep: boolean): Promise<number> {
  const primed = await ask(["agent", "primer"]);
  const primer = primed.stdout;
  if (primed.code !== 0 || !strip(primer)) {
    console.log(`nv reference: \`nvs agent primer\` exited ${primed.code} printing ${primer.length} bytes\n${strip(primed.stderr)}`);
    return 1;
  }
  writeText(PRIMER, primer);

  const failures: string[] = [];
  for (const named of ["nvs agent find", "nvs agent show", "nvs check"]) {
    if (!primer.includes(named)) failures.push(`the primer never names \`${named}\`, so an agent holding it alone does not reach the next step`);
  }

  const programs = examplesIn(PRIMER).filter((ex) => ex.mode === "run");
  const called = new Set<string>();
  for (const ex of programs) {
    for (const m of ex.entry.matchAll(new RegExp(CALLED_RE.source, "g"))) called.add(m[1]!);
  }
  const symbols = [...called].sort();
  if (programs.length === 0 || symbols.length === 0) {
    console.log(`nv reference: the primer prints ${programs.length} program(s) calling ${symbols.length} \`Core\` member(s) -- the walk has nothing to look up`);
    return 1;
  }

  for (const symbol of symbols) {
    const found = await ask(["agent", "find", symbol]);
    const lines = splitlines(found.stdout).filter((l) => strip(l));
    if (found.code !== 0 || lines.length === 0) {
      failures.push(`\`nvs agent find ${symbol}\` exited ${found.code} printing ${lines.length} line(s), and the primer's own program calls that member`);
      continue;
    }
    const hit = lines.find((l) => symbolOf(l) === symbol);
    if (hit === undefined) {
      failures.push(`\`nvs agent find ${symbol}\` printed ${lines.length} line(s), none of them opening with \`${symbol}\``);
      continue;
    }
    const shown = await ask(["agent", "show", symbolOf(hit)]);
    const rows = splitlines(shown.stdout).filter((l) => strip(l));
    if (shown.code !== 0) {
      failures.push(`\`nvs agent show ${symbol}\` exited ${shown.code} on the symbol \`nvs agent find\` printed:\n${strip(shown.stderr)}`);
    } else if (rows.length === 0 || rows[0] !== hit) {
      failures.push(`\`nvs agent show ${symbol}\` does not open with the line \`find\` printed`);
    } else if (rows.length === 1) {
      failures.push(`\`nvs agent show ${symbol}\` printed its index line and nothing else -- no description, no parameters, no return`);
    }
  }

  for (const ex of programs) {
    const work = join(WALK, `program-${String(ex.index).padStart(2, "0")}`);
    const entry = lay(work, ex);
    const checked = await ask(["check", "main.nvs"], work);
    if (checked.code !== 0) {
      failures.push(`\`nvs check\` refuses the program the primer prints:\n${strip(checked.stderr || checked.stdout)}`);
      continue;
    }
    const call = CALLED_RE.exec(ex.entry);
    if (call === null) continue;
    const member = call[1]!.slice(call[1]!.lastIndexOf("::") + 2);
    writeText(entry, ex.entry.replace(call[0], `${member}(`));
    const free = await ask(["check", "main.nvs"], work);
    const said = `${free.stdout}\n${free.stderr}`;
    if (free.code === 0) failures.push(`\`nvs check\` accepts \`${member}(\` as a free function, so the step the loop ends on answers nothing`);
    else if (!said.includes(member)) failures.push(`\`nvs check\` refuses \`${member}(\` without naming it:\n${strip(said)}`);
    else if (!keep) rmSync(work, { recursive: true, force: true });
  }

  for (const problem of failures) console.log(`FAIL ${problem}\n`);
  console.log(
    `nv reference: the walk resolved ${symbols.length} member(s) through \`find\` and \`show\` and checked ${programs.length} program(s)` +
      (failures.length > 0 ? `, ${failures.length} step(s) failed` : ""),
  );
  return failures.length > 0 ? 1 : 0;
}

// ------------------------------------------------------------------ main

const USAGE = [
  "usage: nv reference [-h] [--check] [--no-examples] [--examples-only]",
  "                    [--primer] [--agent-walk] [--only ONLY] [--keep]",
].join("\n");

export async function run(args: string[]): Promise<number> {
  let parsed;
  try {
    parsed = parseArgs(args, { flags: ["--check", "--no-examples", "--examples-only", "--primer", "--agent-walk", "--keep"], valued: ["--only"] });
  } catch (e) {
    if (!(e instanceof ArgError)) throw e;
    console.error(`${USAGE}\nnv reference: error: ${e.message}`);
    return 2;
  }
  const { flags, values } = parsed;
  if (flags.has("--help")) {
    console.log(`${USAGE}\n\nnv reference: ${summary}`);
    return 0;
  }
  const keep = flags.has("--keep");
  const only = values.get("--only");
  try {
    // The primer is rendered by the binary at the call and never written to the tree, so there is
    // nothing for `--check` to compare; the walk likewise only asks the binary questions.
    if (flags.has("--primer")) return await checkPrimer(keep);
    if (flags.has("--agent-walk")) return await checkWalk(keep);
    if (flags.has("--examples-only")) return await checkExamples(only, keep);

    const text = build(await registry());
    const current = existsSync(OUT) ? readText(OUT).replace(/\r\n/g, "\n") : "";
    if (flags.has("--check")) {
      if (text !== current) {
        console.log("nv reference: docs/novis.md is stale -- run `bun nv reference` and commit it");
        return 1;
      }
      console.log("nv reference: docs/novis.md is current");
    } else if (text !== current) {
      writeText(OUT, text);
      // Nothing a session does names this file, and verification runs this command in every session:
      // without the note, the loop's sweep reads it as somebody else's edit.
      record(OUT);
      const kb = Math.floor(Buffer.byteLength(text, "utf8") / 1024);
      console.log(`nv reference: wrote ${rel(OUT)} (${kb} KB, ${text.split("\n").length - 1} lines)`);
    } else {
      console.log("nv reference: docs/novis.md unchanged");
    }
    if (flags.has("--no-examples")) return 0;
    return await checkExamples(only, keep);
  } catch (e) {
    if (!(e instanceof Fatal)) throw e;
    console.error(e.message);
    return 1;
  }
}
