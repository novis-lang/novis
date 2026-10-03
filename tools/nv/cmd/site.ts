// `bun nv site`: the website's stale guard, its checks and its build.
//
//     bun nv site --stale                  every broken id, stale page and uncovered feature
//     bun nv site --stamp PAGE...          record what each page's covered features are now, after the
//                                          page was reread against them
//     bun nv site --check [PART...]        run the named parts, or all of them: `site <part>: ok` per
//                                          part that passes, its problems and exit 1 for one that fails
//     bun nv site --build                  the Astro build, and `site: built` when it succeeds
//
// `--nvs PATH` names the binary that reads the roster and runs the snippets. Without it, the first built
// of `target/release` and the `covws` debug build is used.
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
// The parts of `--check`: `structure` (the nav is the four areas from `website/config/site.mjs`, then
// Install and Sponsoring, each area is one sidebar group, no retired page or generator is on disk, the
// footer has no prev/next and the Astro config has no redirect), `snippets` (no handwritten page has an
// inline Novis fence, every `<Snippet src="..."/>` names a file under `website/snippets/`, every snippet
// is used, keeps the comment bounds of `bun nv proofs --comments`, and prints exactly its `.out` with
// exit 0), `stale` (no page lacks `covers:`, no id is broken and no page is stale) and `prose` (the
// countable bounds of AGENTS.md § *Text an end user reads* over every handwritten page's prose: no
// sentence over 25 words, no dash joining two sentences, and no paragraph over six sentences). The
// legal pages are kept out of `prose`, since their wording is the law's. `reference`, `syntax`,
// `guides`, `in-depth` and `apps` belong to the later stages of goal `website-overhaul` and fail
// until those stages write them.

import { createHash } from "node:crypto";
import { existsSync, readdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { COVWS_TARGET, hostTriple } from "../lib/covws.ts";
import { ROOT } from "../lib/paths.ts";
import { run as runProc } from "../lib/proc.ts";
import { comparePaths } from "../lib/py.ts";
import { commentProblems } from "../proofs/collect.ts";
import { ABOUT, examplesDir, metaJson, roster, RosterError, type Entry, type Meta } from "../proofs/roster.ts";

export const summary = "the website's stale guard, checks and build: nv site --stale | --stamp PAGE... | --check [PART...] | --build";

export const DOCS = "website/src/content/docs";
export const SNIPPETS = "website/snippets";
export const LOCK = "website/site.lock.json";
/** The page directories a renderer writes, under `DOCS`. */
const GENERATED = ["reference/core/"];
/** Pages whose wording is fixed by law, and which `prose` does not judge. */
const LEGAL = ["impressum.md", "datenschutz.md"];

export const PARTS = ["structure", "snippets", "stale", "reference", "syntax", "guides", "in-depth", "apps", "prose"] as const;
type Part = (typeof PARTS)[number];
/** The stage of goal `website-overhaul` that writes each part not written yet. */
const LATER: Partial<Record<Part, number>> = { reference: 3, syntax: 4, guides: 5, apps: 5, "in-depth": 6 };

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
  /** Language and type features no Syntax page covers. */
  uncovered: string[];
}

const isSyntaxFeature = (e: Entry) => e.kind === "lang" || e.path.startsWith("types/");

export function staleness(world: World, root: string = ROOT): Staleness {
  const byId = new Map(world.entries.map((e) => [e.id, e]));
  const lock = readLock(root);
  const out: Staleness = { unlisted: [], broken: [], stale: [], uncovered: [] };
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
      if (stamped === undefined) out.stale.push(`${page.key}: ${id} (never stamped)`);
      else if (stamped !== featureHash(root, entry, world.meta)) out.stale.push(`${page.key}: ${id} (changed since its stamp)`);
    }
  }
  out.uncovered = world.entries.filter((e) => isSyntaxFeature(e) && !syntaxCovers.has(e.id)).map((e) => e.id);
  return out;
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
    if (!existsSync(join(root, p.replace(/\.nvs$/, ".out")))) problems.push(`${p}: no .out beside it`);
  }
  return { problems, snippets };
}

async function snippetProblems(nvs: string): Promise<string[]> {
  const { problems, snippets } = snippetShape();
  for (const p of snippets) {
    for (const c of commentProblems(p)) problems.push(`${p}:${c}`);
    const out = await runProc([nvs, "run", p], { cwd: ROOT, timeoutMs: 60_000 });
    const want = readAt(ROOT, p.replace(/\.nvs$/, ".out"));
    const norm = (s: string) => s.replace(/\r\n?/g, "\n").trimEnd();
    if (out.code !== 0) problems.push(`${p}: exit ${out.code}: ${out.stderr.trim().split("\n")[0] ?? ""}`);
    else if (norm(out.stdout) !== norm(want)) problems.push(`${p}: prints something other than its .out`);
  }
  return problems;
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
    if (LATER[part] !== undefined) problems = [`not written yet: stage ${LATER[part]} of goal \`website-overhaul\` writes it`];
    else if (part === "structure") problems = structureProblems();
    else if (part === "prose") {
      problems = pages()
        .filter((p) => !LEGAL.includes(p.key))
        .flatMap((p) => {
          const offset = p.text.split("\n").length - p.body.split("\n").length;
          return proseProblems(p.body).map((x) => `${DOCS}/${p.key}:${Number(x.split(":")[0]) + offset}:${x.slice(x.indexOf(":") + 1)}`);
        });
    } else if (nvs === null) problems = ["no `nvs` binary: build one, or pass --nvs"];
    else if (part === "snippets") problems = await snippetProblems(nvs);
    else {
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
