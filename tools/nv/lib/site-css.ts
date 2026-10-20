// A check over the website's styles, which `bun nv site --check structure` runs.
//
// The build minifies CSS with lightningcss, which folds an `animation-timeline` into the `animation`
// shorthand of the same rule: `animation: grow linear both; animation-timeline: view()` is built as
// `animation: linear both grow view()`. Browsers do not accept a timeline in the shorthand, so they
// drop the whole declaration and the scroll-driven animation never runs, while the dev server, which
// does not minify, shows it working. A rule with a timeline therefore names its animation with
// longhands only (`animation-name`, `animation-timing-function`, `animation-fill-mode`), which the
// minifier leaves as they are.

import { existsSync, readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";

export const STYLE_ROOT = "website/src";

/** The rules in `text` that set both the `animation` shorthand and `animation-timeline`, as 1-based lines. */
export function foldedTimelines(text: string): number[] {
  const lines: number[] = [];
  for (const m of text.matchAll(/\{[^{}]*\}/g)) {
    const block = m[0];
    if (/(?:^|[\s;{])animation\s*:/.test(block) && /animation-timeline\s*:/.test(block)) {
      lines.push(text.slice(0, m.index).split("\n").length);
    }
  }
  return lines;
}

/** Every `.css` and `.astro` file under the website's sources whose rule would lose its timeline, as `path:line: what`. */
export function foldedTimelineProblems(root: string): string[] {
  const dir = join(root, STYLE_ROOT);
  if (!existsSync(dir)) return [];
  return readdirSync(dir, { recursive: true, withFileTypes: true })
    .filter((e) => e.isFile() && /\.(?:css|astro)$/.test(e.name))
    .map((e) => join(e.parentPath, e.name))
    .sort()
    .flatMap((full) => {
      const path = full.slice(root.length + 1).replaceAll("\\", "/");
      return foldedTimelines(readFileSync(full, "utf8")).map(
        (line) => `${path}:${line}: a rule sets \`animation\` and \`animation-timeline\`; the minified build drops it, so name the animation with longhands`,
      );
    });
}
