// The source of each language chapter: `docs/examples/lang/<chapter>/chapter.md`, beside the chapter's
// features. The directory name is the chapter's id. The file holds what no feature's `about.md` or
// `details.md` says: the chapter's front matter and the order of its features.
//
//     ---
//     order: 20
//     title: Types, declarations and conversions
//     summary: every type, how a variable declares one, ...
//     keywords: bool, int, ...
//     ---
//
//     # Every binding has a type
//     <!-- generated: interfaces -->
//     <!-- primer -->
//     # A complete program, annotated
//
// Each `# ` line is one feature, and its text is the feature's id: the slug of the heading is the name
// of the feature's directory, so a heading is never reworded. `<!-- generated: <table> -->` places one
// of `nvs meta --json`'s tables after the feature above it, and `<!-- primer -->` marks the feature
// below it for `nvs agent primer`, the same markers `docs/reference/README.md` gives a chapter. Nothing
// else is allowed: a sentence a chapter needs belongs in a feature's `about.md` or `details.md`.
//
// The chapter `docs/reference/lang/<order>-<chapter>.md` is the generated form of this file, which
// `bun nv render` writes (`renderers/lang-chapters.ts`).

import { existsSync } from "node:fs";
import { abs } from "../lib/paths.ts";
// roster.ts imports this file, so only functions are taken from it: they are read at call time.
import { namesIn, read, slugify } from "./roster.ts";

export const LANG_EXAMPLES = "docs/examples/lang";
export const CHAPTER_SOURCE = "chapter.md";
/** The chapters `bun nv reference` and `nvs agent` read, which the sources generate. */
export const LANG_CHAPTERS = "docs/reference/lang";

/** One feature of a chapter, in the chapter's order. */
export interface ChapterFeature {
  heading: string;
  slug: string;
  /** The source's own line, `docs/examples/lang/<chapter>/chapter.md:NN`. */
  line: number;
  primer: boolean;
  /** The tables placed after this feature's text. */
  generated: string[];
}

export interface ChapterSource {
  id: string;
  path: string;
  order: number;
  title: string;
  summary: string;
  keywords: string;
  features: ChapterFeature[];
  /** What is wrong with the file, one finding each. A source with a finding is still read. */
  problems: string[];
}

/** The chapter file the source generates, `docs/reference/lang/20-types.md`. */
export const chapterFile = (c: ChapterSource) => `${LANG_CHAPTERS}/${c.order}-${c.id}.md`;

/** Parses one source. `id` is its directory's name. */
export function parseChapterSource(id: string, path: string, text: string): ChapterSource {
  const out: ChapterSource = { id, path, order: 0, title: "", summary: "", keywords: "", features: [], problems: [] };
  const m = /^---\n([\s\S]*?)\n---\n/.exec(text);
  if (!m) out.problems.push(`${path}: no front matter`);
  for (const line of (m ? m[1]! : "").split("\n")) {
    const kv = /^(\w+):\s*(.*)$/.exec(line);
    if (!kv) continue;
    const [, key, value] = kv as unknown as [string, string, string];
    if (key === "order") out.order = Number(value);
    else if (key === "title") out.title = value;
    else if (key === "summary") out.summary = value;
    else if (key === "keywords") out.keywords = value;
    else out.problems.push(`${path}: unknown front matter key \`${key}\``);
  }
  for (const key of ["title", "summary", "keywords"] as const) if (!out[key]) out.problems.push(`${path}: no \`${key}\``);
  if (!Number.isInteger(out.order) || out.order <= 0) out.problems.push(`${path}: \`order\` is not a positive whole number`);

  let lineNo = m ? m[0].split("\n").length - 1 : 0;
  let primer = false;
  for (const line of (m ? text.slice(m[0].length) : text).split("\n")) {
    lineNo++;
    const t = line.trim();
    if (t === "") continue;
    if (line.startsWith("# ")) {
      const heading = line.slice(2).trim();
      out.features.push({ heading, slug: slugify(heading), line: lineNo, primer, generated: [] });
      primer = false;
    } else if (t === "<!-- primer -->") {
      primer = true;
    } else if (/^<!-- generated: [a-z]+ -->$/.test(t)) {
      const last = out.features.at(-1);
      if (last) last.generated.push(t.slice("<!-- generated: ".length, -" -->".length));
      else out.problems.push(`${path}:${lineNo}: a generated table before the first feature`);
    } else {
      out.problems.push(`${path}:${lineNo}: only a \`# \` heading or a marker belongs here; a sentence goes in a feature's about.md or details.md`);
    }
  }
  if (primer) out.problems.push(`${path}: \`<!-- primer -->\` with no feature below it`);
  for (const f of out.features) {
    if (!existsSync(abs(`${LANG_EXAMPLES}/${id}/${f.slug}`))) {
      out.problems.push(`${path}:${f.line}: \`# ${f.heading}\` has no directory ${LANG_EXAMPLES}/${id}/${f.slug}`);
    }
  }
  return out;
}

/** Every chapter source, in the chapters' order. */
export function chapterSources(): ChapterSource[] {
  const out: ChapterSource[] = [];
  for (const id of namesIn(LANG_EXAMPLES, "")) {
    const path = `${LANG_EXAMPLES}/${id}/${CHAPTER_SOURCE}`;
    if (!existsSync(abs(path))) continue;
    out.push(parseChapterSource(id, path, read(path)));
  }
  return out.sort((a, b) => a.order - b.order || (a.id < b.id ? -1 : 1));
}
