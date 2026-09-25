// The website's Core reference: `website/src/data/core.json`, and a stub page per published class and
// member under `website/src/content/docs/docs/core/`.
//
// Three sources, each read once. The member tables are the `spec_core_members` record, which
// `nv import` reads out of `docs/spec/01-core-library.md`. The prose the record does not carry — each
// class's summary, its `Enums:` and `Constants:` lines, the `- \`Core\X\`: …` bullet rosters, and the
// headings that say which class a table belongs to — is read from that chapter, which is where it is
// written. The registry is `nvs meta --json`: which members are implemented, and the card of each
// member, enum case and constant. `website/config/spec-overrides.mjs` corrects what the chapter states
// only in prose.
//
// Only an implemented member is published, and a class with none is not. Where the registry carries a
// card field it wins; where it lacks one, the website's `Method*` components fall back to the spec at
// build time, so this file only attaches what the registry reported.
//
// A stub page is this renderer's while its front matter says `draft: true`, and is rewritten on every
// render. Removing that line gives the page to a person, and the renderer never writes it again. A
// draft page whose member is no longer published is deleted; a person's page is left alone.
//
// The registry comes from a built binary, so this renderer is in `--website` alone. CI's docs job runs
// `nv render --check` without building one.

import { spawnSync } from "node:child_process";
import { existsSync, readFileSync, statSync } from "node:fs";
import { join, posix } from "node:path";
import type { Output, Renderer } from "../lib/render.ts";
import { load } from "../lib/store.ts";
import { specCoreMembers } from "../schema/spec.ts";

export const SPEC = "docs/spec/01-core-library.md";
export const PAGES_DIR = "website/src/content/docs/docs/core";
export const DATA_FILE = "website/src/data/core.json";

/** The handwritten landing page of the Core reference. */
export const HANDWRITTEN = ["index.mdx"];

/** The spec's own table separator, as `nv import` reads it. */
const SEPARATOR = /^\|(\s*:?-+:?\s*\|)+\s*$/;

// ---------------------------------------------------------------- the binary's registry

interface MemberCard {
  short?: string;
  params?: { name?: string; desc?: string; shape?: { key?: string; type?: string; desc?: string }[] }[];
  return?: string;
  errors?: { error?: string; desc?: string }[];
}

interface Registry {
  /** Class name to the names of its registered members. */
  implemented: Map<string, string[]>;
  /** `Core\Str::length` to that member's card. */
  docs: Map<string, MemberCard>;
  /** `Core\Order` to its short card and each case's description. */
  enumDocs: Map<string, { short: string; cases: Map<string, string> }>;
  /** `Core\Math::PI` to its one sentence. */
  constDocs: Map<string, string>;
}

/** `NVS_BIN` when it is set, otherwise the most recently built of `target/debug` and `target/release`. */
function binary(root: string): string {
  const env = process.env.NVS_BIN;
  if (env) {
    if (!existsSync(env)) throw new Error(`NVS_BIN names ${env}, which does not exist`);
    return env;
  }
  const exe = process.platform === "win32" ? "nvs.exe" : "nvs";
  const built = ["debug", "release"]
    .map((profile) => join(root, "target", profile, exe))
    .filter((p) => existsSync(p))
    .sort((a, b) => statSync(b).mtimeMs - statSync(a).mtimeMs);
  if (built.length === 0) throw new Error("the Core reference reads `nvs meta --json`, and no nvs binary is built: run `cargo build` first");
  return built[0]!;
}

function registry(root: string): Registry {
  const nvs = binary(root);
  const run = spawnSync(nvs, ["meta", "--json"], { encoding: "utf8", timeout: 60_000, maxBuffer: 256 * 1024 * 1024 });
  if (run.error || run.status !== 0) throw new Error(`${nvs} meta --json failed: ${run.error?.message ?? run.stderr}`);
  const parsed = JSON.parse(run.stdout);
  const out: Registry = { implemented: new Map(), docs: new Map(), enumDocs: new Map(), constDocs: new Map() };
  for (const cls of parsed.classes ?? []) {
    out.implemented.set(cls.name, (cls.members ?? []).map((m: { name: string }) => m.name));
    for (const m of cls.members ?? []) if (m.doc) out.docs.set(`${cls.name}::${m.name}`, m.doc);
    for (const c of cls.constants ?? []) if (typeof c.doc === "string" && c.doc) out.constDocs.set(`${cls.name}::${c.name}`, c.doc);
  }
  for (const e of parsed.enums ?? []) {
    if (!e.doc) continue;
    const cases = new Map<string, string>();
    for (const c of e.doc.cases ?? []) if (typeof c.desc === "string" && c.desc) cases.set(c.name, c.desc);
    out.enumDocs.set(e.name, { short: typeof e.doc.short === "string" ? e.doc.short : "", cases });
  }
  return out;
}

// ---------------------------------------------------------------- Markdown to HTML

interface Site {
  githubFile(path: string): string;
  decisionRecord(number: string): string;
}

const escapeHtml = (s: string) => s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");

/** A spec link target on the site: a repo-relative one goes to GitHub, and so does a decision record. */
function rewriteTarget(site: Site, target: string): string {
  const record = /(?:^|\/)(\d{4})-[^/]*\.md(?:#.*)?$/.exec(target);
  if (record && target.includes("adr")) return site.decisionRecord(record[1]!);
  if (/^[a-z]+:/i.test(target) || target.startsWith("/")) return target;
  const repoPath = posix.normalize(posix.join("docs/spec", target)).split("#")[0]!;
  if (repoPath.startsWith("..")) return target;
  return site.githubFile(repoPath);
}

function inlineHtml(site: Site, md: string): string {
  return escapeHtml(md)
    .replace(/\[([^\]]*)\]\(([^)]+)\)/g, (_, text: string, target: string) => `<a href="${rewriteTarget(site, target)}">${text}</a>`)
    .replace(/`([^`]+)`/g, "<code>$1</code>")
    .replace(/\*\*([^*]+)\*\*/g, "<strong>$1</strong>")
    .replace(/\*([^*]+)\*/g, "<em>$1</em>");
}

/** Paragraphs and bullet lists, which is all a class summary uses. */
function blocksHtml(site: Site, md: string): string {
  const out: string[] = [];
  let paragraph: string[] = [];
  let list: string[] | null = null;
  const flushParagraph = () => {
    if (paragraph.length > 0) out.push(`<p>${inlineHtml(site, paragraph.join(" "))}</p>`);
    paragraph = [];
  };
  const flushList = () => {
    if (list) out.push(`<ul>${list.map((item) => `<li>${inlineHtml(site, item)}</li>`).join("")}</ul>`);
    list = null;
  };
  for (const line of md.split(/\r?\n/)) {
    const trimmed = line.trim();
    if (trimmed === "") {
      flushParagraph();
      flushList();
    } else if (/^-\s+/.test(trimmed)) {
      flushParagraph();
      if (!list) list = [];
      list.push(trimmed.replace(/^-\s+/, ""));
    } else if (list && /^\s+\S/.test(line)) {
      list[list.length - 1] += " " + trimmed;
    } else {
      flushList();
      paragraph.push(trimmed);
    }
  }
  flushParagraph();
  flushList();
  return out.join("\n");
}

// ---------------------------------------------------------------- signatures

interface Param {
  kind: "param";
  name: string;
  type: string;
  default: string | null;
  optional: boolean;
  variadic: boolean;
}

interface Option {
  name: string;
  type: string;
}

interface Signature {
  name: string;
  classPrefix: string | null;
  receiver: string | null;
  generics: string | null;
  params: Param[];
  options: Option[];
  returnType: string;
  signature: string;
}

const unescapeCell = (s: string) => s.replace(/\\\|/g, "|").trim();

/** `s` split at the commas outside any `()`, `{}`, `<>` or `[]`. */
function splitTopLevel(s: string): string[] {
  const parts: string[] = [];
  let depth = 0;
  let cur = "";
  for (const ch of s) {
    if ("({<[".includes(ch)) depth++;
    else if (")}>]".includes(ch)) depth--;
    if (ch === "," && depth === 0) {
      parts.push(cur);
      cur = "";
    } else {
      cur += ch;
    }
  }
  if (cur.trim() !== "") parts.push(cur);
  return parts.map((p) => p.trim());
}

function matchParen(s: string, open: number): number {
  let depth = 0;
  for (let i = open; i < s.length; i++) {
    if (s[i] === "(") depth++;
    else if (s[i] === ")" && --depth === 0) return i;
  }
  return -1;
}

/** An options bag's inside, `a?: int, b?: bool` or `hour?, minute?`. */
function parseOptions(inner: string): Option[] {
  if (inner.trim() === "") return [];
  return splitTopLevel(inner).map((part) => {
    const m = /^([A-Za-z_][A-Za-z0-9_]*)\??\s*(?::\s*(.+))?$/.exec(part.trim());
    return m ? { name: m[1]!, type: (m[2] ?? "").trim() } : { name: part.trim(), type: "" };
  });
}

/** `type $name`, `type $name = default`, `T ...$values`, or an options bag `{…}`. */
function parseParam(raw: string): Param | { kind: "options"; options: Option[] } | null {
  const s = raw.trim();
  // A bare `{…}` is an options bag; `{…} $name` is a shape-typed parameter.
  if (s.startsWith("{") && !/\}\s*\$/.test(s)) return { kind: "options", options: parseOptions(s.slice(1, s.lastIndexOf("}"))) };
  let rest = s;
  let def: string | null = null;
  const eq = rest.indexOf("=");
  if (eq !== -1) {
    def = rest.slice(eq + 1).trim();
    rest = rest.slice(0, eq).trim();
  }
  const variadic = rest.includes("...");
  rest = rest.replace("...", " ").replace(/\s+/g, " ").trim();
  const dollar = rest.lastIndexOf("$");
  if (dollar === -1) return null;
  const name = rest.slice(dollar + 1).trim();
  if (!/^[A-Za-z_][A-Za-z0-9_]*$/.test(name)) return null;
  return { kind: "param", name, type: rest.slice(0, dollar).trim(), default: def, optional: def !== null, variadic };
}

/** `length(string $s): uint`, `$i->in(Zone $zone): DateTime`, `Uri::parse(string $uri): Uri`, `decodeAs<T>(…): T`. */
export function parseSignature(raw: string): Signature | null {
  let s = unescapeCell(raw).replace(/`/g, "").trim();
  let receiver: string | null = null;
  let classPrefix: string | null = null;
  let m = /^\$([A-Za-z_][A-Za-z0-9_]*)->/.exec(s);
  if (m) {
    receiver = m[1]!;
    s = s.slice(m[0].length);
  } else {
    m = /^([A-Za-z_][A-Za-z0-9_\\]*)::/.exec(s);
    if (m) {
      classPrefix = m[1]!;
      s = s.slice(m[0].length);
    }
  }
  const head = /^([A-Za-z_][A-Za-z0-9_]*)\s*(<[^>]*>)?\s*\(/.exec(s);
  if (!head) return null;
  const name = head[1]!;
  const generics = head[2] ?? null;
  const open = s.indexOf("(", name.length);
  const close = matchParen(s, open);
  if (close === -1) return null;
  const paramsRaw = s.slice(open + 1, close);
  const after = s.slice(close + 1).trim();
  const ret = /^:\s*([^—]+?)(?:\s*[—(].*)?$/.exec(after);
  const returnType = ret ? ret[1]!.trim() : after === "" ? "void" : null;
  if (returnType === null) return null;
  const params: Param[] = [];
  let options: Option[] = [];
  for (const part of splitTopLevel(paramsRaw)) {
    if (part === "") continue;
    const p = parseParam(part);
    if (p === null) return null;
    if (p.kind === "options") options = p.options;
    else params.push(p);
  }
  const sigParams = splitTopLevel(paramsRaw).join(", ");
  return { name, classPrefix, receiver, generics, params, options, returnType, signature: `${name}${generics ?? ""}(${sigParams}): ${returnType}` };
}

// ---------------------------------------------------------------- the spec, read into classes

interface Overrides {
  receivers?: Record<string, string>;
  rows?: Record<string, { skip?: boolean; silent?: boolean; class?: string; cloneAlso?: string[] }>;
  members?: { class: string; signature: string; static?: boolean; replaces?: string; notes?: string; qualifier?: string; section?: string }[];
  enumOwners?: Record<string, string>;
  classes?: Record<string, { summary?: string; hide?: boolean; constants?: string[]; enums?: SpecEnum[] }>;
}

interface SpecEnum {
  name: string;
  cases: string[];
  raw: string;
  doc?: { shortHtml?: string; cases?: { name: string; descHtml: string }[] };
}

// Key order is the order `core.json` writes, which the website's history diffs against.
interface Member {
  id: string;
  name: string;
  static: boolean;
  receiver: string | null;
  generics: string | null;
  signature: string;
  params: Param[];
  options: Option[];
  returnType: string;
  replaces: string;
  notes: string;
  qualifier: string;
  section: string;
  fromOverride?: true;
  implemented?: boolean;
  notesHtml?: string;
  doc?: Record<string, unknown>;
  slug?: string;
  url?: string;
}

interface Class {
  id: string;
  name: string;
  section: string;
  summary: string;
  surface: string;
  adrs: string[];
  enums: SpecEnum[];
  constants: (string | { name: string; descHtml: string })[];
  members: Member[];
  unparsed: { memberCell: string; signatureCell: string; section: string }[];
  hidden?: true;
  implemented?: boolean;
  slug?: string;
  url?: string;
  summaryHtml?: string;
  surfaceHtml?: string;
}

/** Every `Core\X[\Y]` in a line. */
const coreNamesIn = (line: string) => [...line.matchAll(/Core\\[A-Za-z][A-Za-z0-9_]*(?:\\[A-Za-z][A-Za-z0-9_]*)?/g)].map((m) => m[0]);

const classIdOf = (name: string) => name.replace(/^Core\\/, "").replace(/\\/g, ".");

type SpecTable = { section: string[]; columns: string[]; rows: string[][] };

/**
 * The classes the spec names, with their members. `text` is the chapter, walked for its headings and
 * prose; `tables` are its tables, in the order they appear, and each takes the place of the table
 * lines the walk skips.
 */
export function parseSpec(text: string, tables: SpecTable[], overrides: Overrides): { classes: Class[]; warnings: string[] } {
  const warnings: string[] = [];
  const classes = new Map<string, Class>();

  function classRecord(name: string): Class {
    const clean = name.replace(/<[^>]*>/, "");
    const full = clean.startsWith("Core\\") ? clean : `Core\\${clean}`;
    let rec = classes.get(full);
    if (!rec) {
      rec = { id: classIdOf(full), name: full, section: "", summary: "", surface: "", adrs: [], enums: [], constants: [], members: [], unparsed: [] };
      classes.set(full, rec);
    }
    return rec;
  }

  /** A bare prefix like `Uri` or `Zone`, against the namespaces the headings above it opened. */
  function resolvePrefix(prefix: string, nsStack: string[]): string {
    if (prefix.startsWith("Core\\")) return prefix;
    for (const ns of nsStack) if (classes.has(`${ns}\\${prefix}`)) return `${ns}\\${prefix}`;
    if (classes.has(`Core\\${prefix}`)) return `Core\\${prefix}`;
    // A class new to its namespace resolves against the section's namespace, never a sub-heading's class.
    const sectionNs = nsStack.length >= 2 ? nsStack[nsStack.length - 2] : nsStack[0];
    if (sectionNs && sectionNs !== "Core") return `${sectionNs}\\${prefix}`;
    return `Core\\${prefix}`;
  }

  const lines = text.split(/\r?\n/);
  let currentClasses: Class[] = [];
  let currentSection = "";
  let nsStack: string[] = [];
  let summaryTarget: Class | null = null;
  let collectingSummary = false;
  let fence = false;
  let nextTable = 0;
  const rowOverrides = overrides.rows ?? {};

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i]!;
    const opensTable = !fence && line.startsWith("|") && SEPARATOR.test(lines[i + 1] ?? "");
    if (/^\s*(```|~~~)/.test(line)) fence = !fence;

    const h = /^(#{1,3})\s+(.*)$/.exec(line);
    if (h) {
      const [, hashes, headingText] = h;
      const named = coreNamesIn(headingText!);
      if (hashes === "##") {
        currentSection = headingText!.trim();
        currentClasses = named.map((n) => classRecord(n));
        for (const c of currentClasses) if (!c.section) c.section = currentSection;
        nsStack = named.length > 0 ? [named[0]!, "Core"] : ["Core"];
        // A summary is collected for the section's first class; a section naming several shares none.
        summaryTarget = currentClasses[0] ?? null;
        collectingSummary = summaryTarget !== null && summaryTarget.summary === "";
      } else if (hashes === "###") {
        if (named.length > 0) {
          currentClasses = named.map((n) => classRecord(n));
          for (const c of currentClasses) if (!c.section) c.section = currentSection;
          nsStack = [named[0]!, ...nsStack.filter((n) => n !== named[0])];
        }
        collectingSummary = false;
      } else {
        collectingSummary = false;
      }
      continue;
    }

    // The summary is the prose between a class's `##` heading and its first table or code block.
    if (collectingSummary && summaryTarget) {
      if (line.startsWith("|") || line.startsWith("```")) collectingSummary = false;
      else summaryTarget.summary += line + "\n";
    }

    const enumLine = /^Enums:\s*(.*)$/.exec(line);
    if (enumLine && currentClasses[0]) {
      let acc = enumLine[1]!;
      while (!/\.\s*$/.test(acc) && i + 1 < lines.length && lines[i + 1]!.trim() !== "") acc += " " + lines[++i]!.trim();
      for (const em of acc.matchAll(/`([A-Za-z\\]+)\s*\{\s*([^}]*)\}`/g)) {
        const cases = em[2]!.split(",").map((c) => c.trim().replace(/….*$/, "…"));
        const ownerName = overrides.enumOwners?.[em[1]!];
        const owner = ownerName ? classRecord(ownerName) : currentClasses[0];
        owner.enums.push({ name: em[1]!, cases, raw: em[0].replace(/`/g, "") });
      }
      continue;
    }
    const constLine = /^Constants?:\s*(.*)$/.exec(line);
    if (constLine && currentClasses[0]) {
      let acc = constLine[1]!;
      while (!/[.—]\s*$/.test(acc) && i + 1 < lines.length && lines[i + 1]!.trim() !== "" && !lines[i + 1]!.startsWith("#")) {
        acc += " " + lines[++i]!.trim();
      }
      const stop = acc.search(/—|\breplacing\b/);
      const rosterText = stop === -1 ? acc : acc.slice(0, stop);
      for (const cm of rosterText.matchAll(/`([A-Za-z_][A-Za-z0-9_:\\]*)`/g)) currentClasses[0].constants.push(cm[1]!.replace(/^.*::/, ""));
      continue;
    }

    // A bullet roster: `- \`Core\Request\`: everything the class covers …`.
    const bullet = /^-\s+`(Core\\[A-Za-z][A-Za-z0-9_\\]*)`:\s*(.*)$/.exec(line);
    if (bullet) {
      const rec = classRecord(bullet[1]!);
      if (!rec.section) rec.section = currentSection;
      let surface = bullet[2]!;
      while (i + 1 < lines.length && /^\s+\S/.test(lines[i + 1]!)) surface += " " + lines[++i]!.trim();
      if (!rec.surface) rec.surface = surface.trim();
      continue;
    }

    if (!opensTable) continue;
    const table = tables[nextTable++];
    if (!table) throw new Error(`${SPEC}:${i + 1} opens a table the spec_core_members record does not have: run \`bun nv import --write\``);
    let j = i + 2;
    while (j < lines.length && lines[j]!.startsWith("|")) j++;
    if (j - i - 2 !== table.rows.length) {
      throw new Error(`${SPEC}:${i + 1}: the table has ${j - i - 2} row(s) and the spec_core_members record's has ${table.rows.length}: run \`bun nv import --write\``);
    }
    i = j - 1;
    readTable(table);
  }
  if (nextTable !== tables.length) throw new Error(`the spec_core_members record has ${tables.length} table(s) and ${SPEC} ${nextTable}: run \`bun nv import --write\``);

  function readTable(table: SpecTable) {
    const header = table.columns.map((c) => c.replace(/\*/g, "").trim());
    const rows = table.rows.map((r) => r.map((c) => c.replace(/\\\|/g, "|")));
    const kind0 = header[0]?.toLowerCase();
    if (kind0 === "class" || kind0 === "type") {
      for (const row of rows) {
        const cellNames = coreNamesIn(row[0]!);
        const names = cellNames.length > 0 ? cellNames : [resolvePrefix(row[0]!.replace(/`/g, "").replace(/<[^>]*>/g, "").trim(), nsStack)];
        for (const n of names) {
          const rec = classRecord(n);
          if (!rec.section) rec.section = currentSection;
          if (!rec.surface) rec.surface = unescapeCell(row[1] ?? "");
          for (const am of (row[2] ?? "").matchAll(/\b(\d{4})\b/g)) if (!rec.adrs.includes(am[1]!)) rec.adrs.push(am[1]!);
        }
      }
      return;
    }
    if (kind0 !== "member") return;

    // Columns: Member | Signature | (Replaces|Notes|Answers) | Q?
    const col2 = (header[2] ?? "").toLowerCase();
    for (const row of rows) {
      const memberCellRaw = row[0] ?? "";
      const sigCell = row[1] ?? "";
      const replaces = col2 === "replaces" ? unescapeCell(row[2] ?? "") : "";
      const notes = col2 !== "replaces" ? unescapeCell(row[2] ?? "") : "";
      const qualifier = unescapeCell(row[3] ?? (col2 === "q" ? row[2]! : "") ?? "").replace(/\*/g, "");

      const key = memberCellRaw.replace(/`/g, "").trim();
      const rowOv = rowOverrides[key];
      if (rowOv?.skip) continue;

      const names = [...memberCellRaw.matchAll(/`([^`]+)`/g)].map((m) => m[1]!);
      if (names.length === 0) names.push(key);

      // A combined row (`toBase64` / `fromBase64`) states one signature per name, each its own code span.
      const sigSpans = [...sigCell.matchAll(/`([^`]+)`/g)].map((m) => m[1]!);
      const parsedSpans = (sigSpans.length > 0 ? sigSpans : [sigCell]).map((s) => parseSignature(s));
      const parsed = parsedSpans[0];
      if (!parsed) {
        const target = currentClasses[0] ?? classRecord("Core\\Unknown");
        target.unparsed.push({ memberCell: key, signatureCell: unescapeCell(sigCell), section: currentSection });
        if (!rowOv?.silent) warnings.push(`unparsed row in "${currentSection}": ${key} | ${sigCell.slice(0, 80)}`);
        continue;
      }

      const firstName = names[0]!;
      const prefixed = /^([A-Za-z\\]+)::/.exec(firstName);
      let owner: Class;
      if (rowOv?.class) owner = classRecord(rowOv.class);
      else if (parsed.classPrefix) owner = classRecord(resolvePrefix(parsed.classPrefix, nsStack));
      else if (prefixed) owner = classRecord(resolvePrefix(prefixed[1]!, nsStack));
      else if (parsed.receiver && overrides.receivers?.[parsed.receiver]) owner = classRecord(overrides.receivers[parsed.receiver]!);
      else if (currentClasses.length === 1) owner = currentClasses[0]!;
      else {
        warnings.push(currentClasses.length > 1 ? `ambiguous class for "${firstName}" in "${currentSection}": add a row override` : `no class context for "${firstName}" in "${currentSection}"`);
        continue;
      }

      const isStatic = parsed.receiver === null;
      const push = (name: string, sig: Signature) => {
        if (owner.members.some((mm) => mm.name === name)) return;
        owner.members.push({
          id: `${owner.id}.${name}`,
          name,
          static: isStatic,
          receiver: sig.receiver,
          generics: sig.generics,
          signature: sig.signature.replace(new RegExp(`^${sig.name}`), name),
          params: sig.params,
          options: sig.options,
          returnType: sig.returnType,
          replaces,
          notes,
          qualifier,
          section: currentSection,
        });
      };

      const bare = (n: string) => n.replace(/^.*::/, "").replace(/^\$[A-Za-z0-9_]*->/, "");
      push(bare(firstName), parsed);

      // A further name takes its own stated signature, or the first one's when the cell is
      // space-separated (`sin` `cos` `tan`) or the override allows it.
      const spaceSeparated = !memberCellRaw.includes("/");
      for (const extra of names.slice(1)) {
        const extraBare = bare(extra);
        const own = parsedSpans.find((sig) => sig && sig.name === extraBare) ?? null;
        if (own) push(extraBare, own);
        else if (spaceSeparated || rowOv?.cloneAlso?.includes(extraBare)) push(extraBare, parsed);
        else if (!rowOv?.silent && !(overrides.members ?? []).some((om) => om.class === owner.name && parseSignature(om.signature)?.name === extraBare)) {
          warnings.push(`"${key}" names ${extraBare} but only ${bare(firstName)}'s signature is stated: add an override member or cloneAlso`);
        }
      }
    }
  }

  for (const om of overrides.members ?? []) {
    const owner = classRecord(om.class);
    const parsed = parseSignature(om.signature);
    if (!parsed) {
      warnings.push(`override member ${om.class}: signature does not parse: ${om.signature}`);
      continue;
    }
    if (owner.members.some((mm) => mm.name === parsed.name)) continue;
    owner.members.push({
      id: `${owner.id}.${parsed.name}`,
      name: parsed.name,
      static: om.static ?? parsed.receiver === null,
      receiver: parsed.receiver,
      generics: parsed.generics,
      signature: parsed.signature,
      params: parsed.params,
      options: parsed.options,
      returnType: parsed.returnType,
      replaces: om.replaces ?? "",
      notes: om.notes ?? "",
      qualifier: om.qualifier ?? "",
      section: om.section ?? owner.section,
      fromOverride: true,
    });
  }

  for (const [name, extra] of Object.entries(overrides.classes ?? {})) {
    const rec = classRecord(name);
    if (extra.summary && !rec.summary.trim()) rec.summary = extra.summary;
    if (extra.hide) rec.hidden = true;
    for (const c of extra.constants ?? []) if (!rec.constants.includes(c)) rec.constants.push(c);
    for (const e of extra.enums ?? []) if (!rec.enums.some((x) => x.name === e.name)) rec.enums.push(e);
  }

  for (const rec of classes.values()) rec.summary = rec.summary.trim();
  const list = [...classes.values()].filter((c) => !c.hidden && c.name !== "Core\\Unknown");
  list.sort((a, b) => a.id.localeCompare(b.id));
  return { classes: list, warnings };
}

// ---------------------------------------------------------------- the registry's cards, attached

function attachCards(classes: Class[], reg: Registry, site: Site): void {
  const asHtml = (s: unknown) => inlineHtml(site, String(s));
  // The spec's `Enums:` line names an enum by its last segment (`Order`), and the registry in full.
  const enumDocFor = (shortName: string) =>
    reg.enumDocs.get(`Core\\${shortName}`) ?? [...reg.enumDocs.entries()].find(([full]) => full.endsWith(`\\${shortName}`))?.[1];

  for (const cls of classes) {
    const registered = reg.implemented.get(cls.name) ?? [];
    cls.implemented = reg.implemented.has(cls.name);
    for (const m of cls.members) m.implemented = registered.includes(m.name);

    for (const e of cls.enums) {
      const doc = enumDocFor(e.name);
      if (!doc) continue;
      e.doc = {
        ...(doc.short ? { shortHtml: asHtml(doc.short) } : {}),
        ...(doc.cases.size > 0 ? { cases: e.cases.map((name) => ({ name, descHtml: doc.cases.has(name) ? asHtml(doc.cases.get(name)) : "" })) } : {}),
      };
    }
    cls.constants = cls.constants.map((c) => {
      const name = typeof c === "string" ? c : c.name;
      const desc = reg.constDocs.get(`${cls.name}::${name}`);
      return { name, descHtml: desc ? asHtml(desc) : "" };
    });

    for (const m of cls.members) {
      m.notesHtml = m.notes ? asHtml(m.notes) : "";
      const doc = reg.docs.get(`${cls.name}::${m.name}`);
      if (!doc) continue;
      m.doc = {
        ...(doc.short ? { shortHtml: asHtml(doc.short) } : {}),
        ...(Array.isArray(doc.params)
          ? {
              params: doc.params.map((p) => ({
                name: String(p.name ?? ""),
                descHtml: p.desc ? asHtml(p.desc) : "",
                ...(Array.isArray(p.shape)
                  ? { shape: p.shape.map((k) => ({ key: String(k.key ?? ""), type: String(k.type ?? ""), descHtml: k.desc ? asHtml(k.desc) : "" })) }
                  : {}),
              })),
            }
          : {}),
        ...(doc.return ? { returnHtml: asHtml(doc.return) } : {}),
        ...(Array.isArray(doc.errors) ? { errors: doc.errors.map((e) => ({ error: String(e.error ?? ""), descHtml: e.desc ? asHtml(e.desc) : "" })) } : {}),
      };
    }
  }
}

// ---------------------------------------------------------------- the stub pages

/** Whether a page's front matter still says `draft: true`, which makes it this renderer's. */
export function isDraft(text: string): boolean {
  const fm = /^---\r?\n([\s\S]*?)\r?\n---/.exec(text);
  return fm ? /\bdraft:\s*true\b/.test(fm[1]!) : false;
}

const yaml = (s: string) => JSON.stringify(s);

/** Markdown as plain text, for a front-matter `description`. */
const plain = (s: string) =>
  s
    .replace(/\[([^\]]*)\]\([^)]*\)/g, "$1")
    .replace(/`/g, "")
    .replace(/\*\*?/g, "")
    .trim();

/** The Replaces cell as a phrase, or ''. Its backticks stay: they are MDX-safe and protect `<=>`. */
function replacesPhrase(replaces: string): string {
  const clean = replaces.replace(/\s*\(([^)]*)\)/g, "").trim();
  return !clean || /^nothing\b/i.test(clean) ? "" : clean;
}

const OWNERSHIP = `{/* OWNERSHIP: while \`draft: true\` stands above, this page is TOOL-OWNED and
    \`bun nv render --website\` REGENERATES it on every run — edits here will be lost.
    To take ownership, remove \`draft: true\`; the tool then never touches this
    file again. Keep a <Method...> component wherever its default (rendered
    from the repository's own data) is good enough; replace one with your own
    prose where it is not. */}`;

function classPage(cls: Class): string {
  return `---
title: ${yaml(cls.name)}
description: ${yaml(`The ${cls.name} class of the Novis Core library — every member, with signatures and status.`)}
sidebar:
  label: ${yaml(cls.id.replace(/\./g, "\\"))}
  order: 0
novis:
  kind: class
  id: ${yaml(cls.id)}
  draft: true
---

import ClassOverview from '@components/ClassOverview.astro'

${OWNERSHIP}

{/* Group note: if this class needs an important note at the top (like the
    UTF-8 note on Str), write it here as a normal Starlight aside. */}

<ClassOverview id=${yaml(cls.id)} />
`;
}

function memberPage(cls: Class, m: Member, reg: Registry): string {
  const repl = replacesPhrase(m.replaces);
  const lead = m.doc?.shortHtml
    ? plain(String(reg.docs.get(`${cls.name}::${m.name}`)?.short ?? ""))
    : repl
      ? `Novis's replacement for PHP's ${plain(repl)}.`
      : `A member of ${cls.name}.`;
  const hasParams = m.params.length > 0;
  const hasOptions = m.options.length > 0;
  const imports = [
    `import MethodLead from '@components/MethodLead.astro'`,
    `import MethodSignature from '@components/MethodSignature.astro'`,
    `import MethodDescription from '@components/MethodDescription.astro'`,
    ...(hasParams || hasOptions ? [`import ParamDocs from '@components/ParamDocs.astro'`] : []),
    `import MethodReturn from '@components/MethodReturn.astro'`,
    `import MethodErrors from '@components/MethodErrors.astro'`,
    `import MethodChangelog from '@components/MethodChangelog.astro'`,
    `import MethodExamples from '@components/MethodExamples.astro'`,
    `import SeeAlso from '@components/SeeAlso.astro'`,
  ].join("\n");
  const id = yaml(m.id);
  const params = hasParams ? `\n### Parameters\n\n<ParamDocs id=${id} kind="params" />\n` : "";
  const options = hasOptions ? `\n### Options\n\n<ParamDocs id=${id} kind="options" />\n` : "";
  return `---
title: ${yaml(`${cls.name}::${m.name}`)}
description: ${yaml(lead)}
sidebar:
  label: ${yaml(m.name)}
novis:
  kind: method
  id: ${id}
  draft: true
---

${imports}

${OWNERSHIP}

<MethodLead id=${id} />

<MethodSignature id=${id} />

## Description

<MethodDescription id=${id} />
${params}${options}
## Return value

<MethodReturn id=${id} />

## Errors

<MethodErrors id=${id} />

<MethodChangelog id=${id} />

{/* Tips & tricks: add a "## Tips" section here when there is something worth
    saying. The section is simply absent until then. */}

<MethodExamples id=${id} />

{/* Related members: list ids like "Str.isEmpty". Renders nothing while empty. */}
<SeeAlso ids={[]} />
`;
}

// ---------------------------------------------------------------- the renderer

export async function renderWebsiteCore(root: string): Promise<Output[]> {
  const site: Site = await import(join(root, "website", "config", "site.mjs"));
  const { overrides } = (await import(join(root, "website", "config", "spec-overrides.mjs"))) as { overrides: Overrides };
  const record = load(specCoreMembers, root)[0];
  if (!record || record.issues.length > 0) throw new Error("the spec_core_members record is missing or does not match its schema: run `bun nv check`");

  const { classes, warnings } = parseSpec(readFileSync(join(root, SPEC), "utf8"), record.value.tables, overrides);
  for (const w of warnings) console.warn(`nv render: website-core: ${w}`);
  const reg = registry(root);
  attachCards(classes, reg, site);

  for (const cls of classes) {
    cls.members = cls.members.filter((m) => m.implemented);
    cls.unparsed = [];
  }
  const published = classes.filter((c) => c.members.length > 0);
  for (const cls of published) {
    cls.slug = cls.id.toLowerCase().replace(/\./g, "-");
    cls.url = `/docs/core/${cls.slug}/`;
    cls.summaryHtml = cls.summary ? blocksHtml(site, cls.summary) : "";
    cls.surfaceHtml = cls.surface ? inlineHtml(site, cls.surface) : "";
    for (const m of cls.members) {
      m.slug = m.name.toLowerCase();
      m.url = `/docs/core/${cls.slug}/${m.slug}/`;
    }
  }

  const outputs: Output[] = [{ path: DATA_FILE, text: `${JSON.stringify({ classes: published }, null, 2)}\n` }];
  /** A page is written when it is missing or still a draft; a person's page is left as it is. */
  const stub = (path: string, text: string) => {
    const full = join(root, path);
    if (!existsSync(full) || isDraft(readFileSync(full, "utf8"))) outputs.push({ path, text });
  };
  for (const cls of published) {
    stub(`${PAGES_DIR}/${cls.slug}/index.mdx`, classPage(cls));
    for (const m of cls.members) stub(`${PAGES_DIR}/${cls.slug}/${m.slug}.mdx`, memberPage(cls, m, reg));
  }
  return outputs;
}

export const websiteCore: Renderer = {
  name: "website-core",
  render: renderWebsiteCore,
  owns: {
    dir: PAGES_DIR,
    keep: HANDWRITTEN,
    mine: (path, root) => path.endsWith(".mdx") && isDraft(readFileSync(join(root, path), "utf8")),
  },
};
