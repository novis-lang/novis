// `bun nv site`: the website's stale guard, its checks and its build.
//
//     bun nv site --stale                  every broken id, stale page and uncovered feature
//     bun nv site --stamp PAGE...          record what each page's covered features are now, after the
//                                          page was reread against them
//     bun nv site --check [PART...]        run the named parts, or all of them: `site <part>: ok` per
//                                          part that passes, its problems and exit 1 for one that fails
//     bun nv site --bless SNIPPET...       write each snippet's `.out` or `.err` from what the binary does,
//                                          and show it
//     bun nv site --build                  the Astro build, and `site: built` when it succeeds
//
// `--nvs PATH` names the binary that reads the roster and runs the snippets. Without it, the first built
// of `target/release` and the `covws` debug build is used.
//
// **A snippet** is a `.nvs` file under `website/snippets/` that a page shows with `<Snippet src="..."/>`,
// and beside it is exactly one of two files. A `.out` is what the snippet prints: `nvs run` from the
// repository root, with the directory's `nvs.toml` as `--config` and the snippet's `.nvsr` as
// `--request` when they exist, must exit 0 and print that file. A `.err` is the compiler's diagnostic for
// a snippet that must not compile: `nvs check <name>.nvs`, run in the snippet's own directory with
// `NO_COLOR=1`, must exit 1 and write that file to standard error. The bare file name keeps the path in
// each diagnostic the same wherever the check runs, and `nvs check` runs nothing, so a `.err` snippet
// that compiles by mistake has no effect. Both comparisons turn CRLF into LF and drop trailing white
// space. `--bless` rewrites whichever of the two files is there, and for a new snippet writes a `.err`
// when it does not compile and a `.out` when it does.
//
// **A handwritten page** is every page under `website/src/content/docs/` that no renderer writes, which
// is every page outside `reference/core/`. Its front matter carries `covers:`, a list of the proofs
// roster's feature ids (`Core\Str::length`, `lang:statements/match`, `directive:server.dispatch`), and
// `covers: []` on a page that explains no one feature. A page is named by its path under that directory
// (`install/hello-world.mdx`).
//
// **The lock**, `website/site.lock.json`, records per page a hash of each feature it covers: the
// feature's card in `nvs meta --json` or its section in its reference chapter, its `about.md`, and every
// file in its example directory. A covered id the roster no longer has is **broken**. A covered feature
// whose hash differs from the page's stamp, or that the page never stamped, makes the page **stale**.
// A language or type feature no page under `syntax/` covers is **uncovered**. `--stamp` is what clears
// a stale page, and it is run by whoever reread the page, never to quiet the check.
//
// **The wrap gate**, `wrapGate`, is what `bun nv session --wrap` calls: it refuses every broken id, and
// every stale page the session made stale itself, by changing the page or one of its covered features.
//
// The parts of `--check`: `structure` (the nav is the four areas from `website/config/site.mjs`, then
// Install and Sponsoring, each area is one sidebar group, no retired page or generator is on disk, the
// footer has no prev/next, the Astro config has no redirect, and no page or sidebar entry links to a URL
// `builtUrls` does not list), `snippets` (no handwritten page has an
// inline Novis fence, every `<Snippet src="..."/>` names a file under `website/snippets/`, every snippet
// is used, keeps the comment bounds of `bun nv proofs --comments`, has one `.out` or one `.err` and
// matches it), `stale` (no page lacks `covers:`, no id is broken and no page is stale), `reference` (see
// `referenceProblems`), `syntax` (see `syntaxProblems`), `guides` (see `guidesProblems`), `in-depth`
// (see `inDepthProblems`), `apps` (see `appsProblems`) and `prose` (the countable bounds of AGENTS.md
// § *Text an end user reads* over every handwritten page's prose: no sentence over 25 words, no dash
// joining two sentences, and no paragraph over six sentences). The legal pages are kept out of `prose`,
// since their wording is the law's.

import { createHash } from "node:crypto";
import { existsSync, readdirSync, readFileSync, rmSync, statSync, writeFileSync } from "node:fs";
import { createServer } from "node:net";
import { join, resolve } from "node:path";
import { COVWS_TARGET, hostTriple } from "../lib/covws.ts";
import { rel, ROOT } from "../lib/paths.ts";
import { run as runProc } from "../lib/proc.ts";
import { comparePaths } from "../lib/py.ts";
import { commentProblems } from "../proofs/collect.ts";
import { ABOUT, EXAMPLES, examplesDir, implFile, metaJson, roster, RosterError, type Entry, type Meta } from "../proofs/roster.ts";
import { DATA_FILE, renderWebsiteCore } from "../renderers/website-core.ts";
import { REFERENCE_FILE, type Page as RefPage } from "../renderers/website-reference.ts";

export const summary = "the website's stale guard, checks and build: nv site --stale | --stamp PAGE... | --check [PART...] | --bless SNIPPET... | --build";

export const DOCS = "website/src/content/docs";
export const SNIPPETS = "website/snippets";
export const LOCK = "website/site.lock.json";
/** The page directories a renderer writes, under `DOCS`. */
const GENERATED = ["reference/core/"];
/** Pages whose wording is fixed by law, and which `prose` does not judge. */
const LEGAL = ["impressum.md", "datenschutz.md"];

export const PARTS = ["structure", "snippets", "stale", "reference", "syntax", "guides", "in-depth", "apps", "prose"] as const;
type Part = (typeof PARTS)[number];
/** Where `--build` writes the site, which `reference` reads. */
const DIST = "website/dist";

const SENTENCE_WORDS = 25;
const PARAGRAPH_SENTENCES = 6;

const readAt = (root: string, path: string) => {
  try {
    return readFileSync(join(root, path), "utf8").replace(/\r\n?/g, "\n");
  } catch {
    return "";
  }
};

/** Every file under a repo-relative directory, repo-relative and sorted. */
function filesUnder(root: string, dir: string): string[] {
  const out: string[] = [];
  const walk = (d: string) => {
    if (!existsSync(join(root, d))) return;
    for (const name of readdirSync(join(root, d))) {
      const p = `${d}/${name}`;
      if (statSync(join(root, p)).isDirectory()) walk(p);
      else out.push(p);
    }
  };
  walk(dir);
  return out.sort(comparePaths);
}

export interface Page {
  /** The path under `DOCS`. */
  key: string;
  text: string;
  /** The text after the front matter. */
  body: string;
  /** `covers:`, or null when the front matter has none. */
  covers: string[] | null;
}

function frontMatter(text: string): { data: Record<string, unknown>; body: string } {
  const m = /^---\n([\s\S]*?)\n---\n?/.exec(text);
  if (!m) return { data: {}, body: text };
  let data: unknown;
  try {
    data = Bun.YAML.parse(m[1]!);
  } catch {
    data = {};
  }
  return { data: typeof data === "object" && data !== null ? (data as Record<string, unknown>) : {}, body: text.slice(m[0].length) };
}

/** Every handwritten page. */
export function pages(root: string = ROOT): Page[] {
  return filesUnder(root, DOCS)
    .filter((p) => /\.mdx?$/.test(p))
    .map((p) => p.slice(DOCS.length + 1))
    .filter((key) => !GENERATED.some((g) => key.startsWith(g)))
    .map((key) => {
      const text = readAt(root, `${DOCS}/${key}`);
      const { data, body } = frontMatter(text);
      const covers = Array.isArray(data.covers) ? data.covers.map(String) : null;
      return { key, text, body, covers };
    });
}

/** The section of a reference chapter a language or tool feature is, from its `# ` heading to the next. */
function section(root: string, anchor: string): string {
  const at = anchor.lastIndexOf(":");
  const lines = readAt(root, anchor.slice(0, at)).split("\n");
  const out: string[] = [];
  let fence = "";
  for (const line of lines.slice(Number(anchor.slice(at + 1)) - 1)) {
    const marker = line.trim().slice(0, 3);
    if (fence) {
      if (marker === fence) fence = "";
    } else if (marker === "```" || marker === "~~~") {
      fence = marker;
    } else if (out.length > 0 && line.startsWith("# ")) {
      break;
    }
    out.push(line);
  }
  return out.join("\n");
}

const named = (items: unknown[] | undefined, key: string, name: string) =>
  (items ?? []).find((x) => (typeof x === "object" && x !== null ? String((x as Record<string, unknown>)[key]) : String(x)) === name);

/** What the binary's help says of a feature: its card in `nvs meta --json`, or its chapter section. */
export function card(root: string, entry: Entry, meta: Meta): string {
  let found: unknown;
  switch (entry.kind) {
    case "member": {
      const member = entry.id.slice(entry.id.lastIndexOf("::") + 2);
      found = meta.classes?.find((c) => c.name === entry.group)?.members?.find((m) => m.name === member);
      break;
    }
    case "exception":
      found = named(meta.exceptions, "name", entry.id);
      break;
    case "enum":
      found = named(meta.enums, "name", entry.id);
      break;
    case "interface":
      found = named(meta.interfaces, "name", entry.id);
      break;
    case "directive":
      found = named(meta.directives, "key", entry.id.slice("directive:".length));
      break;
    default:
      return entry.anchor ? section(root, entry.anchor) : "";
  }
  return JSON.stringify(found ?? null);
}

/** The hash a page's stamp records for a feature: its card, its `about.md` and its examples. */
export function featureHash(root: string, entry: Entry, meta: Meta): string {
  const h = createHash("sha256");
  h.update(card(root, entry, meta));
  const dir = examplesDir(entry);
  h.update(`\0${readAt(root, `${dir}/${ABOUT}`)}`);
  for (const p of filesUnder(root, dir)) if (!p.endsWith(`/${ABOUT}`)) h.update(`\0${p}\0${readAt(root, p)}`);
  return h.digest("hex").slice(0, 16);
}

export type Lock = Record<string, Record<string, string>>;

export function readLock(root: string = ROOT): Lock {
  const text = readAt(root, LOCK);
  return text ? (JSON.parse(text) as Lock) : {};
}

function writeLock(root: string, lock: Lock): void {
  const sorted: Lock = {};
  for (const page of Object.keys(lock).sort(comparePaths)) {
    if (Object.keys(lock[page]!).length === 0) continue;
    sorted[page] = Object.fromEntries(Object.entries(lock[page]!).sort(([a], [b]) => comparePaths(a, b)));
  }
  writeFileSync(join(root, LOCK), `${JSON.stringify(sorted, null, 2)}\n`);
}

/** What the roster and `nvs meta --json` say ships, which every verdict here is judged against. */
export interface World {
  entries: Entry[];
  meta: Meta;
}

export interface Staleness {
  /** Pages with no `covers:` line. */
  unlisted: string[];
  /** `page: id` for a covered id the roster does not have. */
  broken: string[];
  /** `page: id (why)` for a covered feature changed since the page's stamp, or never stamped. */
  stale: string[];
  /** The page and the feature of each `stale` line, in the same order. */
  staleAt: { page: string; entry: Entry }[];
  /** Language and type features no Syntax page covers. */
  uncovered: string[];
}

const isSyntaxFeature = (e: Entry) => e.kind === "lang" || e.path.startsWith("types/");

export function staleness(world: World, root: string = ROOT): Staleness {
  const byId = new Map(world.entries.map((e) => [e.id, e]));
  const lock = readLock(root);
  const out: Staleness = { unlisted: [], broken: [], stale: [], staleAt: [], uncovered: [] };
  const syntaxCovers = new Set<string>();
  for (const page of pages(root)) {
    if (page.covers === null) {
      out.unlisted.push(page.key);
      continue;
    }
    for (const id of page.covers) {
      if (page.key.startsWith("syntax/")) syntaxCovers.add(id);
      const entry = byId.get(id);
      if (!entry) {
        out.broken.push(`${page.key}: ${id}`);
        continue;
      }
      const stamped = lock[page.key]?.[id];
      const why = stamped === undefined ? "never stamped" : stamped !== featureHash(root, entry, world.meta) ? "changed since its stamp" : "";
      if (!why) continue;
      out.stale.push(`${page.key}: ${id} (${why})`);
      out.staleAt.push({ page: page.key, entry });
    }
  }
  out.uncovered = world.entries.filter((e) => isSyntaxFeature(e) && !syntaxCovers.has(e.id)).map((e) => e.id);
  return out;
}

/**
 * What `bun nv session --wrap` refuses, given every path the session changed: each broken id, and each
 * stale page that is the session's own. A stale page is the session's when it changed the page, the
 * feature's example directory, or the file the feature's anchor names, which is its implementation or its
 * reference chapter. A page stale for a reason the session never touched is CI's `--check stale` to report.
 */
export function wrapProblems(world: World, changed: string[], root: string = ROOT): string[] {
  const s = staleness(world, root);
  const touched = (path: string) => changed.some((c) => c === path || c.startsWith(`${path}/`));
  const own = s.stale.filter((_, i) => {
    const { page, entry } = s.staleAt[i]!;
    return touched(`${DOCS}/${page}`) || touched(examplesDir(entry)) || (implFile(entry) !== "" && touched(implFile(entry)));
  });
  return [...s.broken.map((b) => `broken: ${b}`), ...own.map((x) => `stale: ${x}`)];
}

/** The paths whose change can make a page broken or stale; a session that changed none of them is not gated. */
export const FEEDS = [DOCS, EXAMPLES, "docs/reference", "crates"];

/** `wrapProblems` with the roster read from the built `nvs`, or the reason it could not be read. */
export async function wrapGate(changed: string[]): Promise<string[]> {
  if (!changed.some((c) => FEEDS.some((f) => c.startsWith(`${f}/`)))) return [];
  const nvs = binary(undefined);
  if (nvs === null) return ["no `nvs` binary to read the roster from: `bun nv verify` builds one"];
  try {
    return wrapProblems(await world(nvs), changed);
  } catch (e) {
    if (!(e instanceof RosterError)) throw e;
    return [`the roster does not load: ${e.message}`];
  }
}

/** The page key a `--stamp` argument names: a key, or a path under `DOCS` from the repository root. */
const pageKey = (arg: string) => arg.replaceAll("\\", "/").replace(new RegExp(`^(?:\\./)?${DOCS}/`), "");

/** Records the current hash of every feature each named page covers. Returns what stopped it, if anything. */
export function stamp(world: World, keys: string[], root: string = ROOT): string[] {
  const all = new Map(pages(root).map((p) => [p.key, p]));
  const byId = new Map(world.entries.map((e) => [e.id, e]));
  const lock = readLock(root);
  const problems: string[] = [];
  for (const key of keys.map(pageKey)) {
    const page = all.get(key);
    if (!page) problems.push(`${key}: no handwritten page has this name`);
    else if (page.covers === null) problems.push(`${key}: has no \`covers:\` line`);
    else {
      const missing = page.covers.filter((id) => !byId.has(id));
      if (missing.length) problems.push(`${key}: covers ${missing.join(", ")}, which the roster does not have`);
      else lock[key] = Object.fromEntries(page.covers.map((id) => [id, featureHash(root, byId.get(id)!, world.meta)]));
    }
  }
  if (problems.length === 0) writeLock(root, lock);
  return problems;
}

/** What was retired and must stay gone, repo-relative. */
const RETIRED = [
  `${DOCS}/docs`,
  `${DOCS}/why-novis.mdx`,
  `${DOCS}/site-index.mdx`,
  "website/src/content/claims",
  "website/src/data/rules.json",
  "website/src/data/decisions.json",
  "website/src/components/DecisionSummary.astro",
  "website/src/components/RuleBookIndex.astro",
  "website/src/components/SiteIndex.astro",
  "website/src/components/WhyClaims.astro",
  "website/config/rule-sections.mjs",
  "website/scripts",
  "tools/nv/renderers/website-rules.ts",
];
const AREAS = [
  ["Guides", "/guides/"],
  ["Syntax", "/syntax/"],
  ["Reference", "/reference/"],
  ["In-Depth", "/in-depth/"],
];

export function structureProblems(root: string = ROOT): string[] {
  const problems: string[] = [];
  const site = readAt(root, "website/config/site.mjs");
  const block = /export const AREAS = \[([\s\S]*?)\n\]/.exec(site)?.[1] ?? "";
  const areas = [...block.matchAll(/label:\s*'([^']+)'[^\n]*href:\s*'([^']+)'/g)].map((m) => [m[1]!, m[2]!]);
  if (JSON.stringify(areas) !== JSON.stringify(AREAS)) problems.push(`website/config/site.mjs: AREAS is ${JSON.stringify(areas)}, and the goal names ${JSON.stringify(AREAS)}`);
  if (!/export const INSTALL = \{ label: 'Install', href: '\/install\/' \}/.test(site)) problems.push("website/config/site.mjs: INSTALL is not `{ label: 'Install', href: '/install/' }`");

  const header = readAt(root, "website/src/components/Header.astro");
  const order = ["areas.map(", "install.href", "sponsoring.href"].map((s) => header.indexOf(s));
  if (order.some((i) => i < 0) || order[0]! > order[1]! || order[1]! > order[2]!) {
    problems.push("website/src/components/Header.astro: the nav is not the areas, then Install, then Sponsoring");
  }
  const footer = readAt(root, "website/src/components/Footer.astro");
  if (footer.includes("Pagination")) problems.push("website/src/components/Footer.astro: still shows prev/next (`Pagination`)");

  const astro = readAt(root, "website/astro.config.ts");
  if (/^\s*redirects\s*:/m.test(astro)) problems.push("website/astro.config.ts: has a `redirects` entry, and removed pages get none");
  const sidebar = astro.slice(astro.indexOf("sidebar: ["));
  const groups = [...sidebar.matchAll(/^ {10}label: ([^,\n]+),/gm)].map((m) => m[1]!.replace(/^'|'$/g, ""));
  const want = [...AREAS.map(([label]) => label!), "INSTALL.label"];
  if (JSON.stringify(groups) !== JSON.stringify(want)) problems.push(`website/astro.config.ts: the sidebar groups are ${groups.join(", ")}, and the goal names ${want.join(", ")}`);

  for (const path of RETIRED) if (existsSync(join(root, path))) problems.push(`${path}: retired, and still on disk`);

  const built = builtUrls(root);
  const target = (href: string) => href.replace(/[#?].*$/, "").replace(/\/?$/, "/");
  const asset = (href: string) => /\.[a-z0-9]+$/i.test(href.replace(/[#?].*$/, ""));
  for (const page of pages(root)) {
    for (const m of page.body.matchAll(LINK_RE)) {
      const href = m[1] ?? m[2]!;
      if (!asset(href) && !built.has(target(href))) problems.push(`${DOCS}/${page.key}: links to ${href}, which the site does not build`);
    }
  }
  for (const m of sidebar.matchAll(/link: '(\/[^']*)'/g)) {
    if (!built.has(target(m[1]!))) problems.push(`website/astro.config.ts: the sidebar links to ${m[1]}, which the site does not build`);
  }
  return problems;
}

/** A site-relative link in a page: a markdown link's target, or an `href` attribute's value. */
const LINK_RE = /\]\((\/(?!\/)[^)\s]*)\)|href=["'](\/(?!\/)[^"']*)["']/g;

/**
 * Every URL the site builds, each ending in `/`: one per handwritten page, the Core reference's class and
 * method pages, and the Configuration and CLI reference's landing and entry pages.
 */
export function builtUrls(root: string = ROOT): Set<string> {
  const urls = new Set<string>(["/"]);
  for (const path of filesUnder(root, DOCS).filter((p) => /\.mdx?$/.test(p))) {
    const key = path.slice(DOCS.length + 1).replace(/\.mdx?$/, "");
    urls.add(`/${key.replace(/(^|\/)index$/, "")}/`.replace(/\/+/g, "/"));
  }
  const core = JSON.parse(readAt(root, DATA_FILE) || '{"classes":[]}') as { classes: { members: { url: string }[] }[] };
  for (const cls of core.classes) {
    for (const m of cls.members) {
      urls.add(m.url);
      urls.add(m.url.replace(/[^/]+\/$/, ""));
    }
  }
  const refPages = (JSON.parse(readAt(root, REFERENCE_FILE) || '{"pages":[]}') as { pages: RefPage[] }).pages;
  for (const area of ["config", "cli"]) urls.add(`/reference/${area}/`);
  for (const p of refPages) urls.add(p.url);
  return urls;
}

/**
 * The sections of In-Depth, in sidebar order: the page or directory under `in-depth/`, and the fewest
 * pages a directory section has beside its `index.mdx`. A section with `0` is one page, `<slug>.mdx`.
 */
export const IN_DEPTH_SECTIONS: [slug: string, least: number][] = [
  ["what-for", 0],
  ["never", 0],
  ["how-we-decide", 0],
  ["design-principles", 0],
  ["concepts", 5],
  ["falls-behind", 0],
  ["roadmap", 0],
  ["what-changed", 0],
];

/**
 * The `in-depth` part: each of `IN_DEPTH_SECTIONS` has its page, or its directory with at least its count
 * of pages beside the landing page, and the In-Depth sidebar group links the sections in that order.
 */
export function inDepthProblems(root: string = ROOT): string[] {
  const problems: string[] = [];
  const keys = new Set(pages(root).map((p) => p.key));
  for (const [slug, least] of IN_DEPTH_SECTIONS) {
    const prefix = `in-depth/${slug}/`;
    if (least === 0) {
      if (!keys.has(`in-depth/${slug}.mdx`) && !keys.has(`in-depth/${slug}.md`)) problems.push(`${DOCS}/in-depth/${slug}.mdx: no page`);
      continue;
    }
    const steps = [...keys].filter((k) => k.startsWith(prefix) && !/\/index\.mdx?$/.test(k));
    if (!keys.has(`${prefix}index.mdx`)) problems.push(`${DOCS}/${prefix}index.mdx: no landing page`);
    if (steps.length < least) problems.push(`${DOCS}/${prefix}: ${steps.length} page(s) beside its index, and the goal names ${least} at least`);
  }
  problems.push(...sidebarOrder(root, "In-Depth", "in-depth", IN_DEPTH_SECTIONS.map(([slug]) => slug)));
  return problems;
}

/** The problems with one sidebar group's links into `/<area>/<section>/`, which must all exist and be in order. */
function sidebarOrder(root: string, label: string, area: string, sections: string[]): string[] {
  const problems: string[] = [];
  const astro = readAt(root, "website/astro.config.ts");
  const from = astro.indexOf(`label: '${label}'`);
  const rest = from < 0 ? "" : astro.slice(from + 1);
  const next = rest.search(/^ {10}label: /m);
  const group = next < 0 ? rest : rest.slice(0, next);
  let last = -1;
  for (const section of sections) {
    const at = group.indexOf(`link: '/${area}/${section}/`);
    if (at < 0) problems.push(`website/astro.config.ts: the ${label} sidebar has no link into /${area}/${section}/`);
    else if (at < last) problems.push(`website/astro.config.ts: the ${label} sidebar lists /${area}/${section}/ out of the goal's order`);
    else last = at;
  }
  return problems;
}

const FENCE_RE = /^\s*(?:```|~~~)\s*(?:novis|nvs)\b/;
const SNIPPET_RE = /<Snippet\s+src=["']([^"']+)["']/g;

/** The problems `snippets` finds without running anything. */
export function snippetShape(root: string = ROOT): { problems: string[]; snippets: string[] } {
  const problems: string[] = [];
  const used = new Set<string>();
  for (const page of pages(root)) {
    page.text.split("\n").forEach((line, i) => {
      if (FENCE_RE.test(line)) problems.push(`${DOCS}/${page.key}:${i + 1}: an inline Novis code fence; use <Snippet src="..."/>`);
    });
    for (const m of page.text.matchAll(SNIPPET_RE)) {
      used.add(m[1]!);
      if (!existsSync(join(root, SNIPPETS, m[1]!))) problems.push(`${DOCS}/${page.key}: <Snippet src="${m[1]}"/> names no file under ${SNIPPETS}/`);
    }
  }
  const snippets = filesUnder(root, SNIPPETS).filter((p) => p.endsWith(".nvs"));
  for (const p of snippets) {
    if (!used.has(p.slice(SNIPPETS.length + 1))) problems.push(`${p}: no page shows it`);
    const out = existsSync(join(root, beside(p, "out")));
    const err = existsSync(join(root, beside(p, "err")));
    if (out && err) problems.push(`${p}: both a .out and a .err beside it; keep one`);
    else if (!out && !err) problems.push(`${p}: no .out or .err beside it`);
  }
  return { problems, snippets };
}

/** What a snippet is compared with: `out` is what `nvs run` prints, `err` is what `nvs check` reports. */
export type SnippetKind = "out" | "err";

/** The file beside a snippet with the extension `ext`. */
const beside = (p: string, ext: string) => p.replace(/\.nvs$/, `.${ext}`);

/** How one snippet of `kind` is run: the header's § *A snippet*. */
export function snippetRun(p: string, kind: SnippetKind, nvs: string, root: string = ROOT): { argv: string[]; cwd: string; env?: Record<string, string> } {
  const dir = p.slice(0, p.lastIndexOf("/"));
  if (kind === "err") return { argv: [nvs, "check", p.slice(dir.length + 1)], cwd: join(root, dir), env: { NO_COLOR: "1" } };
  const config = `${dir}/nvs.toml`;
  const request = beside(p, "nvsr");
  return { argv: [nvs, "run", ...(existsSync(join(root, config)) ? ["--config", config] : []), ...(existsSync(join(root, request)) ? ["--request", request] : []), p], cwd: root };
}

/** The problem with one snippet's result, or null when it matches `want`, the text of its `.out` or `.err`. */
export function snippetVerdict(p: string, kind: SnippetKind, result: { code: number; stdout: string; stderr: string }, want: string): string | null {
  const norm = (s: string) => s.replace(/\r\n?/g, "\n").trimEnd();
  const first = (s: string) => s.trim().split("\n")[0] ?? "";
  if (kind === "out") {
    if (result.code !== 0) return `${p}: exit ${result.code}: ${first(result.stderr)}`;
    return norm(result.stdout) === norm(want) ? null : `${p}: prints something other than its .out`;
  }
  if (result.code === 0) return `${p}: compiles, and a snippet with a .err must not`;
  if (result.code !== 1) return `${p}: \`nvs check\` exit ${result.code}: ${first(result.stderr)}`;
  return norm(result.stderr) === norm(want) ? null : `${p}: \`nvs check\` reports something other than its .err`;
}

const runSnippet = (p: string, kind: SnippetKind, nvs: string) => {
  const { argv, cwd, env } = snippetRun(p, kind, nvs);
  return runProc(argv, { cwd, timeoutMs: 60_000, ...(env ? { env } : {}) });
};

async function snippetProblems(nvs: string): Promise<string[]> {
  const { problems, snippets } = snippetShape();
  for (const p of snippets) {
    for (const c of commentProblems(p)) problems.push(`${p}:${c}`);
    const out = existsSync(join(ROOT, beside(p, "out")));
    const err = existsSync(join(ROOT, beside(p, "err")));
    // A snippet with both files is already named by `snippetShape`, and neither file is the one to match.
    if (out && err) continue;
    const kind: SnippetKind = err ? "err" : "out";
    const verdict = snippetVerdict(p, kind, await runSnippet(p, kind, nvs), readAt(ROOT, beside(p, kind)));
    if (verdict) problems.push(verdict);
  }
  return problems;
}

/** `--bless`: each snippet's `.out` or `.err` written from what the binary does, and what was written. */
async function blessSnippets(nvs: string, paths: string[]): Promise<number> {
  let failed = false;
  for (const p of paths) {
    if (!p.startsWith(`${SNIPPETS}/`) || !p.endsWith(".nvs") || !existsSync(join(ROOT, p))) {
      console.log(`  FAIL  ${p}: not a .nvs file under ${SNIPPETS}/`);
      failed = true;
      continue;
    }
    const has = (kind: SnippetKind) => existsSync(join(ROOT, beside(p, kind)));
    if (has("out") && has("err")) {
      console.log(`  FAIL  ${p}: both a .out and a .err beside it; delete the one that is wrong`);
      failed = true;
      continue;
    }
    const kind: SnippetKind = has("err") ? "err" : has("out") ? "out" : (await runSnippet(p, "err", nvs)).code === 0 ? "out" : "err";
    const result = await runSnippet(p, kind, nvs);
    const text = (kind === "out" ? result.stdout : result.stderr).replace(/\r\n?/g, "\n");
    const verdict = snippetVerdict(p, kind, result, text);
    if (verdict) {
      console.log(`  FAIL  ${verdict}`);
      failed = true;
      continue;
    }
    const dest = beside(p, kind);
    const existed = has(kind);
    writeFileSync(join(ROOT, dest), text);
    console.log(`  ${existed ? "rewrote" : "wrote"}  ${dest}`);
    for (const line of text.trimEnd().split("\n")) console.log(`      | ${line}`);
  }
  if (!failed) console.log("nv site: read what was written; a blessed file is a claim, not a formality.");
  return failed ? 1 : 0;
}

/** The paragraphs of a page's prose: no front matter, code, imports, components, comments, tables or headings. */
export function paragraphs(body: string): { line: number; text: string }[] {
  const out: { line: number; text: string }[] = [];
  let current: { line: number; text: string } | null = null;
  let fence = "";
  let comment = false;
  const end = () => {
    if (current) out.push(current);
    current = null;
  };
  body.split("\n").forEach((raw, i) => {
    const line = raw.trim();
    if (fence) {
      if (line.startsWith(fence)) fence = "";
      return;
    }
    if (comment) {
      if (line.includes("*/}")) comment = false;
      return;
    }
    if (line.startsWith("```") || line.startsWith("~~~")) {
      end();
      fence = line.slice(0, 3);
      return;
    }
    if (line.startsWith("{/*")) {
      end();
      comment = !line.includes("*/}");
      return;
    }
    const skip = !line || /^(?:import|export)\s/.test(line) || /^[<#|:]/.test(line);
    const item = /^(?:[-*+]|\d+\.)\s+/.test(line);
    if (skip || item) end();
    if (skip) return;
    const text = line.replace(/^(?:[-*+]|\d+\.)\s+/, "");
    if (current) current.text += ` ${text}`;
    else current = { line: i + 1, text };
  });
  end();
  return out;
}

/** The countable bounds over one page's prose, as `line: what`. Line numbers count from the body's first line. */
export function proseProblems(body: string): string[] {
  const problems: string[] = [];
  for (const { line, text } of paragraphs(body)) {
    const plain = text.replace(/`[^`]*`/g, "code").replace(/\[([^\]]*)\]\([^)]*\)/g, "$1");
    if (/—|\s–\s|\s--\s/.test(plain)) problems.push(`${line}: a dash joins two sentences; write two`);
    const sentences = plain.split(/(?<=[.!?])\s+/).filter((s) => s.trim());
    if (sentences.length > PARAGRAPH_SENTENCES) problems.push(`${line}: a paragraph of ${sentences.length} sentences; ${PARAGRAPH_SENTENCES} is the bound`);
    for (const s of sentences) {
      const words = s.split(/\s+/).filter(Boolean);
      if (words.length > SENTENCE_WORDS) problems.push(`${line}: a ${words.length}-word sentence opening \`${words.slice(0, 5).join(" ")} ...\`; ${SENTENCE_WORDS} is the bound`);
    }
  }
  return problems;
}

function binary(explicit: string | undefined): string | null {
  if (explicit !== undefined) return existsSync(explicit) ? explicit : null;
  const exe = process.platform === "win32" ? "nvs.exe" : "nvs";
  for (const p of [`target/release/${exe}`, `${COVWS_TARGET}/${hostTriple()}/debug/${exe}`].map((x) => join(ROOT, x))) {
    if (existsSync(p)) return p;
  }
  return null;
}

/** A roster feature Reference owes a page: a Core member, a directive, a `tools:config` section, or a
 * section of the `cli`, `editor` or `agents` chapter or an `nvs …` section of `server`. */
function referenceOwes(e: Entry): boolean {
  if (e.kind === "member" || e.kind === "directive") return true;
  const m = /^tools:([^/]+)\/(.+)$/.exec(e.id);
  if (!m) return false;
  return ["config", "cli", "editor", "agents"].includes(m[1]!) || (m[1] === "server" && m[2]!.startsWith("nvs-"));
}

/**
 * The `reference` part. `core.json` and `reference.json` are what `bun nv render --website` writes from
 * this binary today. Every roster feature `referenceOwes` has a page, through the feature path both
 * files carry. Every name on an `about.md` file's `related:` line is a Core member with a page. And in
 * the built site, no page says "Not documented yet", and every Reference page shows one example block
 * per example file of every feature on it.
 */
async function referenceProblems(w: World, nvs: string, root: string = ROOT): Promise<string[]> {
  const problems: string[] = [];
  const saved = process.env.NVS_BIN;
  process.env.NVS_BIN = nvs;
  try {
    for (const out of await renderWebsiteCore(root)) {
      if (readAt(root, out.path) !== out.text) problems.push(`${out.path}: not current against this binary: run \`bun nv render --website\``);
    }
  } finally {
    if (saved === undefined) delete process.env.NVS_BIN;
    else process.env.NVS_BIN = saved;
  }

  const core = JSON.parse(readAt(root, DATA_FILE) || '{"classes":[]}') as { classes: { name: string; members: { name: string; url: string; examples: string }[] }[] };
  const refPages = (JSON.parse(readAt(root, REFERENCE_FILE) || '{"pages":[]}') as { pages: RefPage[] }).pages;
  /** Each page's URL, with the feature paths it shows. */
  const shown = new Map<string, string[]>();
  for (const cls of core.classes) for (const m of cls.members) shown.set(m.url, [m.examples]);
  for (const p of refPages) shown.set(p.url, [...(p.intro ? [p.intro.examples] : []), ...p.keys.map((k) => k.examples)]);
  const paged = new Set([...shown.values()].flat());
  for (const e of w.entries) if (referenceOwes(e) && !paged.has(e.path)) problems.push(`${e.id}: has no Reference page`);

  const names = new Set(core.classes.flatMap((c) => c.members.map((m) => `${c.name}::${m.name}`)));
  for (const path of filesUnder(root, EXAMPLES).filter((p) => p.endsWith(`/${ABOUT}`))) {
    const last = readAt(root, path).trimEnd().split("\n").pop() ?? "";
    const m = /^related:\s*(.*)$/.exec(last);
    if (!m) continue;
    for (const name of m[1]!.split(",").map((n) => n.trim()).filter(Boolean)) {
      if (!names.has(name)) problems.push(`${path}: the related: line names ${name}, which has no Core page`);
    }
  }

  if (!existsSync(join(root, DIST))) return [...problems, `${DIST}: no built site: run \`bun nv site --build\``];
  for (const path of filesUnder(root, DIST).filter((p) => p.endsWith(".html"))) {
    if (readAt(root, path).includes("Not documented yet")) problems.push(`${path}: says "Not documented yet"`);
  }
  for (const [url, features] of shown) {
    const html = readAt(root, `${DIST}${url}index.html`);
    if (!html) {
      problems.push(`${url}: not in the built site`);
      continue;
    }
    const want = features.reduce((n, f) => n + filesUnder(root, `${EXAMPLES}/${f}`).filter((p) => p.endsWith(".nvs") && p.split("/").length === `${EXAMPLES}/${f}`.split("/").length + 1).length, 0);
    const got = html.split('class="method-example"').length - 1;
    if (got !== want) problems.push(`${url}: shows ${got} example(s), and its features have ${want}`);
  }
  return problems;
}

/** The landing page of Syntax, which explains no one feature. */
const SYNTAX_INDEX = "syntax/index.mdx";
/** The heading a Syntax page's best practice is under, with the list it needs below it. */
const DO_DONT_RE = /^## Do and don['’]t[ \t]*\n+- /m;

/**
 * The `syntax` part: every Syntax page but the landing page covers at least one feature, shows at least
 * one `<Snippet>` and has a `## Do and don't` heading with a list under it, and no language or type
 * feature is uncovered.
 */
export function syntaxProblems(world: World, root: string = ROOT): string[] {
  const problems: string[] = [];
  for (const page of pages(root)) {
    if (!page.key.startsWith("syntax/") || page.key === SYNTAX_INDEX) continue;
    const at = `${DOCS}/${page.key}`;
    if (!page.covers?.length) problems.push(`${at}: covers no feature`);
    if (!/<Snippet\s+src=/.test(page.body)) problems.push(`${at}: shows no <Snippet>`);
    if (!DO_DONT_RE.test(page.body)) problems.push(`${at}: has no \`## Do and don't\` heading with a list under it`);
  }
  for (const id of staleness(world, root).uncovered) problems.push(`uncovered: ${id}`);
  return problems;
}

/**
 * The sections of Guides, in sidebar order: the directory under `guides/`, the fewest pages it has beside
 * its `index.mdx`, and whether every one of those pages shows a `<Snippet>`.
 */
export const GUIDE_SECTIONS: [dir: string, least: number, snippets: boolean][] = [
  ["why-novis", 1, false],
  ["tour", 1, false],
  ["simple-programs", 6, true],
  ["how-to-use", 1, false],
  ["cookbook", 10, true],
  ["testing", 1, false],
  ["production", 1, false],
  ["example-apps", 1, false],
];

/**
 * The `guides` part: each of `GUIDE_SECTIONS` is a directory under `guides/` with at least its count of
 * pages beside its landing page, every such page of a snippet section shows a `<Snippet>`, and the
 * Guides sidebar group links the sections in that order.
 */
export function guidesProblems(root: string = ROOT): string[] {
  const problems: string[] = [];
  const all = pages(root);
  for (const [dir, least, snippets] of GUIDE_SECTIONS) {
    const prefix = `guides/${dir}/`;
    const inside = all.filter((p) => p.key.startsWith(prefix));
    const steps = inside.filter((p) => p.key !== `${prefix}index.mdx` && p.key !== `${prefix}index.md`);
    if (inside.length === 0) problems.push(`${DOCS}/${prefix}: no page`);
    else if (steps.length < least) problems.push(`${DOCS}/${prefix}: ${steps.length} page(s) beside its index, and the goal names ${least} at least`);
    if (snippets) for (const p of steps) if (!/<Snippet\s+src=/.test(p.body)) problems.push(`${DOCS}/${p.key}: shows no <Snippet>`);
  }
  problems.push(...sidebarOrder(root, "Guides", "guides", GUIDE_SECTIONS.map(([dir]) => dir)));
  return problems;
}

/** The example apps' directory: each directory in it is one app, with `nvs.toml`, `public/index.nvs` and
 * `tests/`. */
export const APPS = "apps";
/** How long an app's server has to answer `/` with a `200` once started. The debug build compiles every
 * file before it binds the socket. */
const BOOT_MS = 60_000;

/** A TCP port nothing listens on now, which the operating system picked. */
function freePort(): Promise<number> {
  return new Promise((resolve, reject) => {
    const server = createServer();
    server.once("error", reject);
    server.listen(0, "127.0.0.1", () => {
      const address = server.address();
      const port = typeof address === "object" && address !== null ? address.port : 0;
      server.close(() => resolve(port));
    });
  });
}

/**
 * The `apps` part: every app under `APPS` passes `nvs test tests/`, and `nvs serve public/index.nvs`
 * answers `GET /` with a `200` within `BOOT_MS`. Both run in the app's own directory, as its page tells
 * a reader to, and every file the run left there that was not there before (the app's database) is
 * deleted afterwards.
 */
export async function appsProblems(nvs: string, root: string = ROOT): Promise<string[]> {
  const problems: string[] = [];
  const dirs = existsSync(join(root, APPS)) ? readdirSync(join(root, APPS)).filter((d) => statSync(join(root, APPS, d)).isDirectory()) : [];
  if (dirs.length === 0) return [`${APPS}/: no app`];
  for (const dir of dirs.sort(comparePaths)) {
    const app = `${APPS}/${dir}`;
    const cwd = join(root, app);
    const before = new Set(filesUnder(root, app));
    try {
      const test = await runProc([nvs, "test", "tests/"], { cwd, timeoutMs: 10 * 60_000, env: { NOVIS_NO_INIT: "1" } });
      if (test.code !== 0) {
        const tail = (test.stdout + test.stderr).trim().split("\n").slice(-15);
        problems.push(`${app}: \`nvs test tests/\` exited ${test.code}`, ...tail.map((l) => `  ${l}`));
      }
      const port = await freePort();
      const stop = new AbortController();
      const serve = runProc([nvs, "serve", "public/index.nvs", "--port", String(port)], { cwd, signal: stop.signal, env: { NOVIS_NO_INIT: "1" } });
      let status = 0;
      let exited = false;
      void serve.then(() => (exited = true));
      const until = Date.now() + BOOT_MS;
      while (status !== 200 && !exited && Date.now() < until) {
        try {
          status = (await fetch(`http://127.0.0.1:${port}/`, { redirect: "manual" })).status;
        } catch {
          await Bun.sleep(250);
        }
      }
      stop.abort();
      const served = await serve;
      if (status !== 200) {
        const why = exited && !served.aborted ? `the server exited ${served.code}: ${served.stderr.trim().split("\n").slice(-5).join(" / ")}` : status ? `\`/\` returned ${status}` : `no answer within ${BOOT_MS / 1000} s`;
        problems.push(`${app}: \`nvs serve public/index.nvs\` did not answer \`GET /\` with 200: ${why}`);
      }
    } finally {
      for (const f of filesUnder(root, app)) if (!before.has(f)) rmSync(join(root, f), { force: true });
    }
  }
  return problems;
}

async function world(nvs: string): Promise<World> {
  const meta = await metaJson(nvs);
  return { entries: await roster(nvs, meta), meta };
}

function report(title: string, items: string[]): void {
  if (items.length === 0) return;
  console.log(`${title} (${items.length}):`);
  for (const item of items) console.log(`  ${item}`);
}

async function check(parts: Part[], nvs: string | null): Promise<number> {
  let failed = 0;
  let w: World | null = null;
  for (const part of parts) {
    let problems: string[];
    if (part === "structure") problems = structureProblems();
    else if (part === "guides") problems = guidesProblems();
    else if (part === "in-depth") problems = inDepthProblems();
    else if (part === "prose") {
      problems = pages()
        .filter((p) => !LEGAL.includes(p.key))
        .flatMap((p) => {
          const offset = p.text.split("\n").length - p.body.split("\n").length;
          return proseProblems(p.body).map((x) => `${DOCS}/${p.key}:${Number(x.split(":")[0]) + offset}:${x.slice(x.indexOf(":") + 1)}`);
        });
    } else if (nvs === null) problems = ["no `nvs` binary: build one, or pass --nvs"];
    else if (part === "snippets") problems = await snippetProblems(nvs);
    else if (part === "apps") problems = await appsProblems(nvs);
    else if (part === "reference") {
      w ??= await world(nvs);
      problems = await referenceProblems(w, nvs);
    } else if (part === "syntax") {
      w ??= await world(nvs);
      problems = syntaxProblems(w);
    } else {
      w ??= await world(nvs);
      const s = staleness(w);
      problems = [...s.unlisted.map((p) => `${p}: has no \`covers:\` line`), ...s.broken.map((b) => `broken: ${b}`), ...s.stale.map((x) => `stale: ${x}`)];
    }
    if (problems.length === 0) console.log(`site ${part}: ok`);
    else {
      failed++;
      report(`site ${part}: failed`, problems);
    }
  }
  return failed ? 1 : 0;
}

async function build(): Promise<number> {
  const out = await runProc(["node", "node_modules/astro/bin/astro.mjs", "build"], { cwd: join(ROOT, "website"), timeoutMs: 30 * 60_000 });
  if (out.code !== 0) {
    console.log(out.stdout.trim().split("\n").slice(-30).join("\n"));
    console.log(out.stderr.trim());
    console.log(`site: the build failed with exit ${out.code}`);
    return 1;
  }
  console.log("site: built");
  return 0;
}

export async function run(args: string[]): Promise<number> {
  const at = args.indexOf("--nvs");
  const explicit = at >= 0 ? args[at + 1] : undefined;
  const rest = at >= 0 ? [...args.slice(0, at), ...args.slice(at + 2)] : args;
  const [mode, ...operands] = rest;
  if (mode === undefined || mode === "-h" || mode === "--help") {
    console.log(summary);
    return mode === undefined ? 2 : 0;
  }
  if (mode === "--build") return build();
  const nvs = binary(explicit);
  try {
    if (mode === "--bless") {
      if (operands.length === 0) {
        console.error("nv site: --bless needs at least one snippet");
        return 2;
      }
      if (nvs === null) {
        console.error("nv site: no `nvs` binary: build one, or pass --nvs");
        return 1;
      }
      return await blessSnippets(nvs, operands.map((p) => rel(resolve(p))));
    }
    if (mode === "--check") {
      const unknown = operands.filter((p) => !(PARTS as readonly string[]).includes(p));
      if (unknown.length) {
        console.error(`nv site: no part ${unknown.join(", ")}; the parts are ${PARTS.join(", ")}`);
        return 2;
      }
      return await check((operands.length ? operands : [...PARTS]) as Part[], nvs);
    }
    if (mode !== "--stale" && mode !== "--stamp") {
      console.error(`nv site: no mode ${mode}\n${summary}`);
      return 2;
    }
    if (nvs === null) {
      console.error("nv site: no `nvs` binary: build one, or pass --nvs");
      return 1;
    }
    const w = await world(nvs);
    if (mode === "--stamp") {
      if (operands.length === 0) {
        console.error("nv site: --stamp needs at least one page");
        return 2;
      }
      const problems = stamp(w, operands);
      for (const p of problems) console.log(`nv site: ${p}`);
      if (problems.length === 0) console.log(`nv site: stamped ${operands.length} page(s) in ${LOCK}`);
      return problems.length ? 1 : 0;
    }
    const s = staleness(w);
    report("pages with no `covers:` line", s.unlisted);
    report("broken: a covered id the roster does not have", s.broken);
    report("stale: reread the page, then `bun nv site --stamp <page>`", s.stale);
    report("uncovered: a language or type feature no Syntax page covers", s.uncovered);
    if (!s.unlisted.length && !s.broken.length && !s.stale.length && !s.uncovered.length) console.log("nv site: nothing is stale");
    return s.unlisted.length || s.broken.length || s.stale.length ? 1 : 0;
  } catch (e) {
    if (!(e instanceof RosterError)) throw e;
    console.error(`nv site: ${e.message}`);
    return 1;
  }
}
