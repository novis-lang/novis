// The checks over the website's diagrams, which `bun nv site --check prose` runs with the pages'.
//
// A diagram's data is a JSON file under `website/src/data/diagrams/`, drawn by a component under
// `website/src/components/diagrams/`. Its words are text an end user reads, so every string in it is
// held to the same countable bounds as a page's paragraphs, except the strings under a key that is
// not prose: a tone, an icon or a link. A link to `#anchor` must also land: every page that imports
// the data file has an element with that id, so a renamed card cannot leave a dead branch on a map.

import { existsSync, readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";

export const DIAGRAM_DATA = "website/src/data/diagrams";

/** The keys whose strings are not prose. */
const NOT_PROSE = new Set(["tone", "icon", "href"]);

/** Every prose string in a diagram's data, with the path that leads to it, such as `branches[2].label`. */
export function diagramStrings(data: unknown, at = ""): { at: string; text: string }[] {
  if (typeof data === "string") return [{ at, text: data }];
  if (Array.isArray(data)) return data.flatMap((v, i) => diagramStrings(v, `${at}[${i}]`));
  if (data && typeof data === "object") {
    return Object.entries(data).flatMap(([k, v]) => (NOT_PROSE.has(k) ? [] : diagramStrings(v, at ? `${at}.${k}` : k)));
  }
  return [];
}

/** Every `#anchor` a diagram's data links to. */
export function diagramAnchors(data: unknown): string[] {
  if (Array.isArray(data)) return data.flatMap(diagramAnchors);
  if (data && typeof data === "object") {
    return Object.entries(data).flatMap(([k, v]) =>
      k === "href" && typeof v === "string" && v.startsWith("#") ? [v.slice(1)] : diagramAnchors(v),
    );
  }
  return [];
}

/** The `.mdx` and `.md` files under `dir` of `root`, repo-relative. */
function pagesUnder(root: string, dir: string): string[] {
  const full = join(root, dir);
  if (!existsSync(full)) return [];
  return readdirSync(full, { recursive: true, withFileTypes: true })
    .filter((e) => e.isFile() && /\.mdx?$/.test(e.name))
    .map((e) => join(e.parentPath, e.name).slice(root.length + 1).replaceAll("\\", "/"));
}

/**
 * The problems in the diagrams' data, as `path: what`: a string over a prose bound, judged by
 * `prose` (a page body's checker, which `site.ts` passes in), and a `#anchor` that a page drawing the
 * diagram has no id for.
 */
export function diagramProblems(root: string, docs: string, prose: (body: string) => string[]): string[] {
  const dir = join(root, DIAGRAM_DATA);
  if (!existsSync(dir)) return [];
  const pages = pagesUnder(root, docs).map((p) => ({ p, text: readFileSync(join(root, p), "utf8") }));
  const problems: string[] = [];
  for (const name of readdirSync(dir).filter((n) => n.endsWith(".json")).sort()) {
    const file = `${DIAGRAM_DATA}/${name}`;
    let data: unknown;
    try {
      data = JSON.parse(readFileSync(join(dir, name), "utf8"));
    } catch (e) {
      problems.push(`${file}: not JSON: ${(e as Error).message}`);
      continue;
    }
    for (const { at, text } of diagramStrings(data)) {
      for (const p of prose(text)) problems.push(`${file}: ${at}:${p.slice(p.indexOf(":") + 1)}`);
    }
    const users = pages.filter(({ text }) => text.includes(`diagrams/${name}'`) || text.includes(`diagrams/${name}"`));
    for (const anchor of diagramAnchors(data)) {
      for (const { p, text } of users) {
        if (!text.includes(`id="${anchor}"`)) problems.push(`${file}: links to #${anchor}, and ${p} has no element with that id`);
      }
    }
  }
  return problems;
}
