// Two checks over the website's styles, which `bun nv site --check structure` runs.
//
// **A loop runs only while a reader can see it.** An animation that repeats forever runs on every
// frame, on screen or off it, and one that repaints, such as a gradient clipped to text, took a fifth
// of every page's main thread. So a loop lives in a `data-motion` region: custom.css pauses every
// animation inside one that `src/scripts/on-screen.ts` has not marked `data-on-screen`. A file with a
// loop names `data-motion`, on the root it renders or in the comment that says where its root gets
// it. A loop that runs only in a state the reader started needs no region: a rule whose selector has
// `:hover`, `:focus`, `:focus-within`, `:focus-visible`, `:active`, or a script's
// `[data-...-state=...]`. A region that is always on screen, such as the header, gets no loop at all:
// it would run for as long as the page is open.
//
// **A timeline survives the minifier.** The build minifies CSS with lightningcss, which folds an
// `animation-timeline` into the `animation` shorthand of the same rule:
// `animation: grow linear both; animation-timeline: view()` is built as
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

/** A selector that only matches while the reader hovers, focuses or presses, or while a script's state lasts. */
const READER_STATE = /:(?:hover|focus|focus-within|focus-visible|active)\b|\[data-[\w-]+-state\s*=/;

/**
 * The rules in `text` that repeat an animation forever outside a state the reader started, as 1-based
 * lines, or none when the file names `data-motion`, so its loops pause off screen.
 */
export function endlessLoops(text: string): number[] {
  const lines: number[] = [];
  if (text.includes("data-motion")) return lines;
  for (const m of text.matchAll(/([^{};]*)\{([^{}]*)\}/g)) {
    const [, selector = "", block = ""] = m;
    if (!/animation(?:-iteration-count)?\s*:[^;}]*\binfinite\b/.test(block) || READER_STATE.test(selector)) continue;
    const at = m.index + m[0].indexOf("{");
    lines.push(text.slice(0, at).split("\n").length);
  }
  return lines;
}

/** Every `.css` and `.astro` file under the website's sources, repo-relative, with its text. */
function styleSources(root: string): { path: string; text: string }[] {
  const dir = join(root, STYLE_ROOT);
  if (!existsSync(dir)) return [];
  return readdirSync(dir, { recursive: true, withFileTypes: true })
    .filter((e) => e.isFile() && /\.(?:css|astro)$/.test(e.name))
    .map((e) => join(e.parentPath, e.name))
    .sort()
    .map((full) => ({ path: full.slice(root.length + 1).replaceAll("\\", "/"), text: readFileSync(full, "utf8") }));
}

/** Every rule in the website's styles that would lose its timeline or loop forever, as `path:line: what`. */
export function styleProblems(root: string): string[] {
  return styleSources(root).flatMap(({ path, text }) => [
    ...foldedTimelines(text).map(
      (line) => `${path}:${line}: a rule sets \`animation\` and \`animation-timeline\`; the minified build drops it, so name the animation with longhands`,
    ),
    ...endlessLoops(text).map(
      (line) => `${path}:${line}: an animation repeats forever, on screen or off; put data-motion on the root that holds it, so it pauses off screen`,
    ),
  ]);
}
