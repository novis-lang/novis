// The links a Core method signature gives a type built into the language, such as `string` or
// `callable`: each row of `LANGUAGE_TYPES` in website/src/lib/core.ts names a page the site builds,
// and its `#anchor` is an id on that page.

import { expect, test } from "bun:test";
import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { ROOT } from "../lib/paths.ts";

const CORE = join(ROOT, "website", "src", "lib", "core.ts");
const DOCS = join(ROOT, "website", "src", "content", "docs");

/** Each `['name', '/path/#anchor']` row of the `LANGUAGE_TYPES` table. */
function rows(): [string, string][] {
  const source = readFileSync(CORE, "utf8");
  const table = /const LANGUAGE_TYPES = new Map\(\[([\s\S]*?)\]\)/.exec(source);
  if (!table) throw new Error(`${CORE} has no LANGUAGE_TYPES table`);
  return [...table[1]!.matchAll(/\['([^']+)', '([^']+)'\]/g)].map((m) => [m[1]!, m[2]!]);
}

/** The ids a page has: each `FeatureSection` id and each `##` heading's slug. */
function ids(page: string): Set<string> {
  const text = readFileSync(page, "utf8");
  const out = new Set([...text.matchAll(/\bid="([^"]+)"/g)].map((m) => m[1]!));
  for (const m of text.matchAll(/^#{2,6} (.+)$/gm)) {
    out.add(m[1]!.trim().toLowerCase().replace(/[^\w\s-]/g, "").replace(/\s+/g, "-"));
  }
  return out;
}

test("every built-in type in a signature links to a page and an anchor that exist", () => {
  const found = rows();
  expect(found.map(([name]) => name)).toContain("callable");
  const problems: string[] = [];
  for (const [name, href] of found) {
    const [path, anchor] = href.split("#");
    const base = join(DOCS, path!.replace(/^\/|\/$/g, ""));
    const page = [`${base}.mdx`, `${base}.md`, join(base, "index.mdx")].find(existsSync);
    if (!page) problems.push(`${name}: ${path} is not a page`);
    else if (anchor && !ids(page).has(anchor)) problems.push(`${name}: ${path} has no #${anchor}`);
  }
  expect(problems).toEqual([]);
});
