// The language chapters under `docs/reference/lang/`, each written from its source
// `docs/examples/lang/<chapter>/chapter.md` (`proofs/chapters.ts`) and its features' files, so
// `nvs agent`, `/llms-full.txt` and the website read the same words. A chapter is the source's front
// matter, then one section per feature in the source's order:
//
//     # <heading>                 the source's line, with `<!-- primer -->` above it when marked
//     <about.md>
//     <details.md>                when the feature has one
//     ## Example 01: <its name>   then, for each example in file order, its sibling files as `file=`
//     ```nvs ...```               fences, the program, and its `.out` as an `output` fence
//     <!-- generated: <table> --> each table the source places after the feature
//
// The fence is the one `docs/reference/README.md` gives for what the example declares:
// `proof: compile-error` is `nvs error`, `proof: exit N` is `nvs exit=N` and `proof: test` is
// `nvs test`, so `bun nv reference` runs each one the way the examples tree does. The diagnostic and
// the test report name the example's file, and here they name `main.nvs`, the file `bun nv reference`
// writes. An example run with a request file (`.nvsr`) is `nvs skip`, because `bun nv reference` sends
// no request, and its output follows as a `text` fence. A sibling file is every file of the feature's
// directory that is not an example, its output, its request or its texts, and it is written before an
// example only when the example's source or its part of the feature's `nvs.toml` names its first path
// segment.
// The feature's `nvs.toml` is written before each example, without the blocks for another example.
//
// It spends the examples' size in each chapter: the chapters are about three times the hand-written
// ones, and `nvs agent` embeds them, once per binary.

import { readdirSync } from "node:fs";
import { MARKER, type Output, type Renderer } from "../lib/render.ts";
import { abs } from "../lib/paths.ts";
import { comparePaths } from "../lib/py.ts";
import { chapterFile, chapterSources, LANG_CHAPTERS, LANG_EXAMPLES, type ChapterSource } from "../proofs/chapters.ts";
import { ABOUT, DETAILS, read } from "../proofs/roster.ts";

const EXAMPLE_RE = /^[0-9]{2}-.*\.nvs$/;
/** The files of a feature's directory that are not a sibling an example reads. */
const OWN_RE = /^[0-9]{2}-.*\.(?:nvs|out|nvsr|nvsp|nvse|in)$/;
const COMPILE_ERROR_RE = /(?:\/\/|#)\s*proof:\s*compile-error\b/;
const EXIT_RE = /(?:\/\/|#)\s*proof:\s*exit\s+([0-9]+)/;
const TEST_RE = /(?:\/\/|#)\s*proof:\s*test\b/;

/** Every file under `dir`, repo-relative to it, sorted as paths. */
function filesIn(dir: string, sub = ""): string[] {
  const out: string[] = [];
  for (const e of readdirSync(abs(sub ? `${dir}/${sub}` : dir), { withFileTypes: true })) {
    const rel = sub ? `${sub}/${e.name}` : e.name;
    if (e.isDirectory()) out.push(...filesIn(dir, rel));
    else out.push(rel);
  }
  return out.sort(comparePaths);
}

/** The fence's info string for a file, by its extension. */
function fenceLang(name: string): string {
  const ext = name.slice(name.lastIndexOf(".") + 1);
  return ext === "nvs" || ext === "toml" || ext === "json" || ext === "csv" ? ext : "text";
}

/** A fenced block whose fence is longer than any run of backticks in `body`. */
function fence(info: string, body: string): string {
  const longest = Math.max(2, ...[...body.matchAll(/`+/g)].map((m) => m[0].length));
  const ticks = "`".repeat(longest + 1);
  return `${ticks}${info}\n${body.replace(/\n+$/, "")}\n${ticks}`;
}

/** `01-the-steps-of-an-order.nvs` -> `## Example 01: the steps of an order`. A heading, so `nvs agent
 * find` reaches an example by the words of its name, and `nvs agent show` prints it alone. */
function exampleTitle(name: string): string {
  const m = /^([0-9]{2})-(.*)\.nvs$/.exec(name)!;
  return `## Example ${m[1]}: ${m[2]!.replaceAll("-", " ")}`;
}

/** A feature's `nvs.toml` without the `[[app]]` blocks, and the comment lines above each, whose
 * `entry` is another example: `nvs` stops on an entry that names no file, and the chapter writes one
 * example to a folder. */
function ownBlocks(config: string, example: string): string {
  const blocks: string[][] = [[]];
  for (const line of config.split("\n")) {
    if (line.trim() === "[[app]]") {
      const above = blocks.at(-1)!;
      let start = above.length;
      while (start > 0 && above[start - 1]!.startsWith("#")) start--;
      blocks.push([...above.splice(start), line]);
    } else blocks.at(-1)!.push(line);
  }
  return blocks
    .filter((block) => {
      const entry = /^entry\s*=\s*"([^"]*)"/m.exec(block.join("\n"));
      return !entry || entry[1] === example;
    })
    .flat()
    .join("\n")
    .replace(/^\n+/, "");
}

/** One feature's examples, each with the sibling files it reads. */
function examples(dir: string): string[] {
  const files = filesIn(dir);
  const siblings = files.filter((f) => !OWN_RE.test(f) && f !== ABOUT && f !== DETAILS);
  const config = siblings.includes("nvs.toml") ? read(`${dir}/nvs.toml`) : "";
  const blocks: string[] = [];
  for (const name of files.filter((f) => EXAMPLE_RE.test(f))) {
    const source = read(`${dir}/${name}`);
    const asMain = (text: string) => text.replaceAll(`${dir}/${name}`, "main.nvs").replaceAll(name, "main.nvs");
    const out = asMain(read(`${dir}/${name.replace(/\.nvs$/, ".out")}`));
    const parts = [exampleTitle(name)];
    for (const s of siblings) {
      const head = s.split("/")[0]!;
      if (s === "nvs.toml") {
        const own = ownBlocks(config, name);
        if (own.trim()) parts.push(fence(`toml file=${s}`, asMain(own)));
      } else if (source.includes(head) || ownBlocks(config, name).includes(head)) parts.push(fence(`${fenceLang(s)} file=${s}`, asMain(read(`${dir}/${s}`))));
    }
    const exit = EXIT_RE.exec(source);
    const status = exit ? ` exit=${exit[1]}` : "";
    if (files.includes(name.replace(/\.nvs$/, ".nvsr"))) {
      parts.push(fence("nvs skip", source), fence("text", out));
      blocks.push(parts.join("\n\n"));
      continue;
    }
    const info = COMPILE_ERROR_RE.test(source) ? "nvs error" : TEST_RE.test(source) ? `nvs test${status}` : `nvs${status}`;
    parts.push(fence(info, source));
    if (out.trim()) parts.push(fence("output", out));
    blocks.push(parts.join("\n\n"));
  }
  return blocks;
}

/** One chapter's text from its source. */
export function renderChapter(source: ChapterSource): string {
  const lines = [
    "---",
    `id: ${source.id}`,
    `title: ${source.title}`,
    `summary: ${source.summary}`,
    `keywords: ${source.keywords}`,
    `generated: ${MARKER}; the source is ${source.path}`,
    "---",
  ];
  for (const f of source.features) {
    const dir = `${LANG_EXAMPLES}/${source.id}/${f.slug}`;
    lines.push("");
    if (f.primer) lines.push("<!-- primer -->");
    lines.push(`# ${f.heading}`);
    for (const text of [read(`${dir}/${ABOUT}`), read(`${dir}/${DETAILS}`)]) if (text.trim()) lines.push("", text.trim());
    for (const block of examples(dir)) lines.push("", block);
    for (const table of f.generated) lines.push("", `<!-- generated: ${table} -->`);
  }
  return lines.join("\n") + "\n";
}

/** Every chapter. A source with a problem ends the render, because its chapter would be wrong. */
export function renderLangChapters(): Output[] {
  const sources = chapterSources();
  const problems = sources.flatMap((s) => s.problems);
  if (problems.length) throw new Error(`the chapter sources have problems:\n  ${problems.join("\n  ")}`);
  return sources.map((s) => ({ path: chapterFile(s), text: renderChapter(s) }));
}

export const langChapters: Renderer = {
  name: "lang-chapters",
  render: () => renderLangChapters(),
  owns: { dir: LANG_CHAPTERS, keep: [] },
};
