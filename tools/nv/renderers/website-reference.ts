// The website's Configuration and CLI reference data: `website/src/data/reference.json`. One route,
// `website/src/pages/reference/[area]/[...slug].astro`, renders a page per entry from it, and reads each
// feature's `about.md`, examples and `nvs.toml` in place under `docs/examples/<examples>/`.
//
// Both areas are the proofs roster's own features, grouped and never listed by hand. Configuration is
// one page per `nvs.toml` block: every `directive:<key>` whose key starts with the block's name, and,
// as the page's introduction, the `tools:config` or `tools:server` chapter section whose title opens
// with that block in brackets (`` `[limits]` and `[limits.hard]` ``). A `tools:config` section that
// names no block is a page of its own. CLI is one page per section of the `cli`, `server` (`nvs …`
// sections only), `editor` and `agents` chapters. Types is one page per enum, exception and
// interface, grouped by kind: every name a signature uses that is not a class has a page here.
// `website-core.ts` calls `referenceData` with the roster it has already read, so this file runs no
// binary.

import type { Entry } from "../proofs/roster.ts";

export const REFERENCE_FILE = "website/src/data/reference.json";

/** One feature shown on a page: its roster id, its heading and its directory under `docs/examples/`. */
export interface Feature {
  id: string;
  title: string;
  examples: string;
}

/** A `directive:<key>` feature, with who may change it and what a change to it takes. */
export interface Directive extends Feature {
  key: string;
  class: string;
  apply: string;
}

export interface Page {
  area: "config" | "cli" | "types";
  slug: string;
  url: string;
  title: string;
  /** The sidebar group the page is listed under. */
  group: string;
  /** The chapter section that introduces the page, when there is one. */
  intro: Feature | null;
  /** A block's keys, the block's own directive first. Empty for every other page. */
  keys: Directive[];
}

/** The sidebar group of each chapter whose sections are CLI pages, in the order the sidebar lists them. */
const CLI_CHAPTERS: [string, string][] = [
  ["cli", "Commands"],
  ["server", "Server"],
  ["editor", "Editor"],
  ["agents", "Coding agents"],
];

/** The sidebar group of each roster kind that is a Types page, in the order the sidebar lists them. */
const TYPE_KINDS: [Entry["kind"], string][] = [
  ["enum", "Enums"],
  ["exception", "Exceptions"],
  ["interface", "Interfaces"],
];

/** The chapter id and section slug of a `tools:<chapter>/<slug>` id. */
function chapterOf(id: string): { chapter: string; slug: string } | null {
  const m = /^tools:([^/]+)\/(.+)$/.exec(id);
  return m ? { chapter: m[1]!, slug: m[2]! } : null;
}

/** `limits` for `` `[limits]` and `[limits.hard]` ``, `app` for `` `[[app]]` — per-application blocks ``. */
function blockNamed(title: string): { block: string; written: string } | null {
  const m = /`(\[\[?)([a-z_]+)(?:\.[a-z_.]+)?(\]\]?)`/.exec(title);
  return m ? { block: m[2]!, written: `${m[1]}${m[2]}${m[3]}` } : null;
}

const feature = (e: Entry, title: string): Feature => ({ id: e.id, title, examples: e.path });

/** Every Configuration, CLI and Types page, from the roster and the binary's directive list. A slug
 * two pages share throws. */
export function referenceData(entries: Entry[], directives: { key: string; class: string; apply: string }[]): Page[] {
  const meta = new Map(directives.map((d) => [d.key, d]));
  const blocks = new Map<string, Page>();
  const topics: Page[] = [];
  const cli = new Map<string, Page[]>(CLI_CHAPTERS.map(([c]) => [c, []]));
  const types = new Map<string, Page[]>(TYPE_KINDS.map(([k]) => [k, []]));

  const blockPage = (block: string): Page => {
    let page = blocks.get(block);
    if (!page) {
      page = { area: "config", slug: block, url: `/reference/config/${block}/`, title: `[${block}]`, group: "Blocks", intro: null, keys: [] };
      blocks.set(block, page);
    }
    return page;
  };

  for (const e of entries) {
    if (e.kind === "directive") {
      const key = e.id.slice("directive:".length);
      const d = meta.get(key);
      blockPage(key.split(".")[0]!).keys.push({ ...feature(e, key), key, class: d?.class ?? "", apply: d?.apply ?? "" });
      continue;
    }
    const kind = TYPE_KINDS.find(([k]) => k === e.kind);
    if (kind) {
      // `types/Novis-Pdf-Document-Standard` -> `novis-pdf-document-standard`.
      const slug = e.path.slice("types/".length).toLowerCase();
      types.get(kind[0])!.push({ area: "types", slug, url: `/reference/types/${slug}/`, title: e.id, group: kind[1], intro: feature(e, e.id), keys: [] });
      continue;
    }
    const at = e.kind === "tool" ? chapterOf(e.id) : null;
    if (!at) continue;
    const named = at.chapter === "config" || at.chapter === "server" ? blockNamed(e.summary) : null;
    if (named) {
      const page = blockPage(named.block);
      page.intro = feature(e, e.summary);
      page.title = named.written;
    } else if (at.chapter === "config") {
      topics.push({ area: "config", slug: at.slug, url: `/reference/config/${at.slug}/`, title: e.summary, group: "Topics", intro: feature(e, e.summary), keys: [] });
    } else if (cli.has(at.chapter) && (at.chapter !== "server" || at.slug.startsWith("nvs-"))) {
      const group = CLI_CHAPTERS.find(([c]) => c === at.chapter)![1];
      cli.get(at.chapter)!.push({ area: "cli", slug: at.slug, url: `/reference/cli/${at.slug}/`, title: e.summary, group, intro: feature(e, e.summary), keys: [] });
    }
  }

  for (const page of blocks.values()) page.keys.sort((a, b) => (a.key === page.slug ? -1 : b.key === page.slug ? 1 : 0));
  const typePages = [...types.values()].flatMap((list) => list.sort((a, b) => a.title.localeCompare(b.title)));
  const pages = [...[...blocks.values()].sort((a, b) => a.slug.localeCompare(b.slug)), ...topics, ...[...cli.values()].flat(), ...typePages];
  const seen = new Set<string>();
  for (const p of pages) {
    if (seen.has(p.url)) throw new Error(`two reference pages share the URL ${p.url}`);
    seen.add(p.url);
  }
  return pages;
}
