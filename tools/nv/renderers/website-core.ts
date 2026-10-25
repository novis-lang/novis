// The website's Core reference data: `website/src/data/core.json`. One route,
// `website/src/pages/reference/core/[...slug].astro`, renders a page per published class and member
// from it, and nothing under `website/src/content/` is written by this file.
//
// Two sources. The registry is `nvs meta --json`: every class, member, enum, shape type and constant,
// with its card. A class's summary is its card's one sentence followed by its introduction, the
// hand-written file under `docs/reference/core/` (named for the class after `Core\`, with `\` written
// `-`) or under `docs/reference/novis/` for a built-in component's class. `website-llms.ts` writes the
// same introductions into `/llms-full.txt`, and `bun nv reference` runs every example in them.
//
// Every registered class and member is published, and nothing else. A class, enum or type alias the
// registry calls `internal`, a built-in component's engine and what only it uses, is not published,
// since a program never calls it. An enum named under a class (`Core\Arr::Order`) is listed on that
// class's page, and every enum also has a Types page (`website-reference.ts`), which is the only page
// of an enum named under a namespace (`Core\Charset`).
//
// Each member carries `examples`, its feature path in the proofs roster (`core/Time-Date/at`, and
// `novis/Pdf-Document/standard` for a built-in component), and nothing else from `docs/examples/`: the
// route reads the member's `about.md` and examples in place under `docs/examples/<examples>/`
// (`rule:testing/examples-live-in-the-repository`).
//
// The registry comes from a built binary, so this renderer is in `--website` alone. CI's docs job runs
// `nv render --check` without building one, and its `reference` job builds one and runs
// `nv render --website --check`. It also writes the Configuration, CLI and Types reference data,
// `website-reference.ts`, from the proofs roster over the same `nvs meta --json`.

import { spawnSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { BUILT_NVS, newestNvs } from "../lib/nvs-binary.ts";
import type { Output, Renderer } from "../lib/render.ts";
import { memberPath, roster, type Meta } from "../proofs/roster.ts";
import { REFERENCE_FILE, referenceData } from "./website-reference.ts";

export const DATA_FILE = "website/src/data/core.json";

// ---------------------------------------------------------------- the binary's registry

interface MemberCard {
  short?: string;
  params?: { name?: string; desc?: string; shape?: { key?: string; type?: string; desc?: string }[] }[];
  return?: string;
  errors?: { error?: string; desc?: string }[];
}

interface RegistryMember {
  name: string;
  kind: string;
  /** `length(string $s): uint`, with no receiver and no class prefix. */
  signature: string;
}

interface Registry {
  /** Every registered class in the binary's order, with its class card's one sentence, its members and
   * the shape types it declares (`Info` for `Novis\Image\Image::Info`). */
  classes: { name: string; short: string; members: RegistryMember[]; types: ShapeType[]; constants: string[] }[];
  /** Every registered enum by its full name (`Novis\Pdf\Document::Standard`, `Core\Charset`), with the
   * cases the binary lists, which may be none. */
  enums: { name: string; cases: string[] }[];
  /** `Core\Str::length` to that member's card. */
  docs: Map<string, MemberCard>;
  /** `Core\Arr::Order` to its short card and each case's description. */
  enumDocs: Map<string, { short: string; cases: Map<string, string> }>;
  /** `Core\Math::PI` to its one sentence. */
  constDocs: Map<string, string>;
  /** The whole of `nvs meta --json`, which the proofs roster reads. */
  meta: Meta & { directives?: { key: string; class: string; apply: string }[] };
}

/** `NVS_BIN` when it is set, otherwise the most recently built of `target/debug` and `target/release`
 * (`lib/nvs-binary.ts`). */
export function binary(root: string): string {
  const env = process.env.NVS_BIN;
  if (env) {
    if (!existsSync(env)) throw new Error(`NVS_BIN names ${env}, which does not exist`);
    return env;
  }
  const built = newestNvs(root, BUILT_NVS.slice(0, 2));
  if (built === null) throw new Error("the Core reference reads `nvs meta --json`, and no nvs binary is built: run `cargo build` first");
  return built;
}

function registry(root: string): Registry {
  const nvs = binary(root);
  const run = spawnSync(nvs, ["meta", "--json"], { encoding: "utf8", timeout: 60_000, maxBuffer: 256 * 1024 * 1024 });
  if (run.error || run.status !== 0) throw new Error(`${nvs} meta --json failed: ${run.error?.message ?? run.stderr}`);
  const parsed = JSON.parse(run.stdout);
  const out: Registry = { classes: [], enums: [], docs: new Map(), enumDocs: new Map(), constDocs: new Map(), meta: parsed };
  for (const cls of parsed.classes ?? []) {
    if (cls.internal) continue;
    out.classes.push({
      name: cls.name,
      short: typeof cls.doc?.short === "string" ? cls.doc.short : "",
      members: (cls.members ?? []).map((m: RegistryMember) => ({ name: m.name, kind: m.kind, signature: m.signature })),
      types: (cls.types ?? []).filter((t: { internal?: boolean }) => !t.internal).map((t: { name: string; type: string }) => ({ name: t.name.slice(t.name.lastIndexOf("::") + 2), type: t.type })),
      constants: (cls.constants ?? []).map((c: { name: string }) => c.name),
    });
    for (const m of cls.members ?? []) if (m.doc) out.docs.set(`${cls.name}::${m.name}`, m.doc);
    for (const c of cls.constants ?? []) if (typeof c.doc === "string" && c.doc) out.constDocs.set(`${cls.name}::${c.name}`, c.doc);
  }
  for (const e of parsed.enums ?? []) {
    if (e.internal) continue;
    out.enums.push({ name: e.name, cases: (e.cases ?? []).map((c: { name: string }) => c.name) });
    if (!e.doc) continue;
    const cases = new Map<string, string>();
    for (const c of e.doc.cases ?? []) if (typeof c.desc === "string" && c.desc) cases.set(c.name, c.desc);
    out.enumDocs.set(e.name, { short: typeof e.doc.short === "string" ? e.doc.short : "", cases });
  }
  return out;
}


// ---------------------------------------------------------------- Markdown to HTML

const escapeHtml = (s: string) => s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");

/** Code, links, bold and italics. A link to an anchor of `/llms-full.txt` (`#lang-testing`) is its text alone. */
function inlineHtml(md: string): string {
  return md
    .split(/(`[^`]+`)/)
    .map((part) =>
      /^`[^`]+`$/.test(part)
        ? `<code>${escapeHtml(part.slice(1, -1))}</code>`
        : escapeHtml(part)
            .replace(/\[([^\]]*)\]\(([^)]+)\)/g, (_, text: string, target: string) => (target.startsWith("#") ? text : `<a href="${target}">${text}</a>`))
            .replace(/\*\*([^*]+)\*\*/g, "<strong>$1</strong>")
            .replace(/(^|[^\w*])\*([^*\s][^*]*)\*/g, "$1<em>$2</em>"),
    )
    .join("");
}

const tableRow = (line: string) =>
  line
    .trim()
    .replace(/^\|/, "")
    .replace(/\|$/, "")
    .split("|")
    .map((c) => c.trim());

/**
 * Paragraphs, bullet lists, tables and fenced code, which is all a class introduction uses. A fence's
 * language is its class (`language-nvs`), and an `output` fence is what the example before it prints.
 */
export function blocksHtml(md: string): string {
  const out: string[] = [];
  let paragraph: string[] = [];
  let list: string[] | null = null;
  let table: string[][] | null = null;
  let fence: { lang: string; lines: string[] } | null = null;
  const flush = () => {
    if (paragraph.length > 0) out.push(`<p>${inlineHtml(paragraph.join(" "))}</p>`);
    if (list) out.push(`<ul>${list.map((item) => `<li>${inlineHtml(item)}</li>`).join("")}</ul>`);
    if (table) {
      const [head, ...body] = table;
      const cells = (row: string[], tag: string) => `<tr>${row.map((c) => `<${tag}>${inlineHtml(c)}</${tag}>`).join("")}</tr>`;
      out.push(`<table><thead>${cells(head!, "th")}</thead><tbody>${body.map((r) => cells(r, "td")).join("")}</tbody></table>`);
    }
    paragraph = [];
    list = null;
    table = null;
  };
  for (const line of md.split(/\r?\n/)) {
    const trimmed = line.trim();
    if (fence) {
      if (trimmed === "```") {
        const cls = fence.lang === "output" ? ' class="output"' : "";
        out.push(`<pre${cls}><code${fence.lang && fence.lang !== "output" ? ` class="language-${fence.lang}"` : ""}>${escapeHtml(fence.lines.join("\n"))}</code></pre>`);
        fence = null;
      } else fence.lines.push(line);
    } else if (trimmed.startsWith("```")) {
      flush();
      fence = { lang: trimmed.slice(3).split(/\s/)[0] ?? "", lines: [] };
    } else if (trimmed === "") {
      flush();
    } else if (trimmed.startsWith("|")) {
      if (!table) flush();
      table ??= [];
      if (!/^\|(\s*:?-+:?\s*\|)+$/.test(trimmed)) table.push(tableRow(trimmed));
    } else if (/^-\s+/.test(trimmed)) {
      if (paragraph.length > 0 || table) flush();
      list ??= [];
      list.push(trimmed.replace(/^-\s+/, ""));
    } else if (list && /^\s+\S/.test(line)) {
      list[list.length - 1] += ` ${trimmed}`;
    } else {
      if (list || table) flush();
      paragraph.push(trimmed);
    }
  }
  if (fence) throw new Error(`a class introduction opens a \`\`\`${fence.lang} fence it never closes`);
  flush();
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
  let s = raw.trim();
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

// ---------------------------------------------------------------- the registry, read into classes

/** A named shape a class declares: `Info` and `{format: Format, width: uint, …}`. */
interface ShapeType {
  name: string;
  type: string;
}

interface ClassEnum {
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
  doc?: Record<string, unknown>;
  slug?: string;
  url?: string;
  examples?: string;
}

interface Class {
  id: string;
  name: string;
  summary: string;
  enums: ClassEnum[];
  /** The shape types the class declares, by the name its signatures use. */
  types: ShapeType[];
  constants: { name: string; descHtml: string }[];
  members: Member[];
  slug?: string;
  url?: string;
  summaryHtml?: string;
}

const classIdOf = (name: string) => name.replace(/^Core\\/, "").replace(/\\/g, ".");

/**
 * Every registered class, sorted by id, with its members built from the registry's own signatures. An
 * instance member's receiver is its class's last segment, `$bigInt`. A class takes the shape types and
 * constants it declares, and every enum named under it (`Novis\Pdf\Document::Standard`), with the cases
 * the registry lists. `summary` is the class card's one sentence and nothing else: the renderer adds the
 * introduction.
 */
export function registryClasses(reg: Pick<Registry, "classes" | "enums">): Class[] {
  const byName = new Map<string, Class>();
  for (const rc of reg.classes) {
    if (rc.name === "Core\\Unknown") continue;
    const cls: Class = { id: classIdOf(rc.name), name: rc.name, summary: rc.short, enums: [], types: rc.types, constants: rc.constants.map((name) => ({ name, descHtml: "" })), members: [] };
    byName.set(rc.name, cls);
    const last = rc.name.slice(rc.name.lastIndexOf("\\") + 1);
    const receiver = last.charAt(0).toLowerCase() + last.slice(1);
    for (const rm of rc.members) {
      const sig = parseSignature(rm.signature);
      if (!sig) throw new Error(`the registry's signature for ${rc.name}::${rm.name} does not parse: ${rm.signature}`);
      const isStatic = rm.kind === "static";
      cls.members.push({
        id: `${cls.id}.${rm.name}`,
        name: rm.name,
        static: isStatic,
        receiver: isStatic ? null : receiver,
        generics: sig.generics,
        signature: sig.signature,
        params: sig.params,
        options: sig.options,
        returnType: sig.returnType,
      });
    }
  }
  for (const e of reg.enums) {
    const at = e.name.lastIndexOf("::");
    const owner = at < 0 ? undefined : byName.get(e.name.slice(0, at));
    if (!owner) continue;
    const name = e.name.slice(at + 2);
    owner.enums.push({ name, cases: e.cases, raw: e.cases.length > 0 ? `${name} { ${e.cases.join(", ")} }` : name });
  }
  return [...byName.values()].sort((a, b) => a.id.localeCompare(b.id));
}

/** The body of a class's introduction under `docs/reference/`, without its front matter, or `""`. */
function introOf(root: string, name: string): string {
  const novis = name.startsWith("Novis\\");
  const stem = (name.startsWith("Core\\") ? name.slice(5) : novis ? name.slice(6) : name).replaceAll("\\", "-");
  const path = join(root, "docs", "reference", novis ? "novis" : "core", `${stem}.md`);
  if (!existsSync(path)) return "";
  return readFileSync(path, "utf8").replace(/^---\r?\n[\s\S]*?\r?\n---\r?\n/, "").trim();
}

// ---------------------------------------------------------------- the registry's cards, attached

function attachCards(classes: Class[], reg: Registry): void {
  for (const cls of classes) {
    for (const e of cls.enums) {
      const doc = reg.enumDocs.get(`${cls.name}::${e.name}`);
      if (!doc) continue;
      e.doc = {
        ...(doc.short ? { shortHtml: inlineHtml(doc.short) } : {}),
        ...(doc.cases.size > 0 ? { cases: e.cases.map((name) => ({ name, descHtml: doc.cases.has(name) ? inlineHtml(doc.cases.get(name)!) : "" })) } : {}),
      };
    }
    for (const c of cls.constants) {
      const desc = reg.constDocs.get(`${cls.name}::${c.name}`);
      c.descHtml = desc ? inlineHtml(desc) : "";
    }

    for (const m of cls.members) {
      const doc = reg.docs.get(`${cls.name}::${m.name}`);
      if (!doc) continue;
      m.doc = {
        ...(doc.short ? { shortHtml: inlineHtml(doc.short) } : {}),
        ...(Array.isArray(doc.params)
          ? {
              params: doc.params.map((p) => ({
                name: String(p.name ?? ""),
                descHtml: p.desc ? inlineHtml(p.desc) : "",
                ...(Array.isArray(p.shape)
                  ? { shape: p.shape.map((k) => ({ key: String(k.key ?? ""), type: String(k.type ?? ""), descHtml: k.desc ? inlineHtml(k.desc) : "" })) }
                  : {}),
              })),
            }
          : {}),
        ...(doc.return ? { returnHtml: inlineHtml(doc.return) } : {}),
        ...(Array.isArray(doc.errors) ? { errors: doc.errors.map((e) => ({ error: String(e.error ?? ""), descHtml: e.desc ? inlineHtml(e.desc) : "" })) } : {}),
      };
    }
  }
}

// ---------------------------------------------------------------- the renderer

export async function renderWebsiteCore(root: string): Promise<Output[]> {
  const reg = registry(root);
  const classes = registryClasses(reg);
  attachCards(classes, reg);

  for (const cls of classes) {
    cls.slug = cls.id.toLowerCase().replace(/\./g, "-");
    cls.url = `/reference/core/${cls.slug}/`;
    cls.summaryHtml = blocksHtml([cls.summary, introOf(root, cls.name)].filter(Boolean).join("\n\n"));
    for (const m of cls.members) {
      m.slug = m.name.toLowerCase();
      m.url = `/reference/core/${cls.slug}/${m.slug}/`;
      m.examples = memberPath(cls.name, m.name);
    }
  }

  const pages = referenceData(await roster("", reg.meta), reg.meta.directives ?? []);
  return [
    { path: DATA_FILE, text: `${JSON.stringify({ classes }, null, 2)}\n` },
    { path: REFERENCE_FILE, text: `${JSON.stringify({ pages }, null, 2)}\n` },
  ];
}

export const websiteCore: Renderer = {
  name: "website-core",
  render: renderWebsiteCore,
};
