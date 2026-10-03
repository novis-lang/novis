import { afterEach, describe, expect, test } from "bun:test";
import { DOCS, paragraphs, proseProblems, snippetShape, stamp, staleness, wrapProblems, type World } from "../cmd/site.ts";
import type { Entry } from "../proofs/roster.ts";
import { scratch, type Scratch } from "./scratch.ts";

const entry = (id: string, kind: Entry["kind"], path: string, anchor = "", group = ""): Entry => ({ id, kind, group, path, anchor, twin: [], summary: "", help: "" });

const LENGTH = entry("Core\\Str::length", "member", "core/Str/length", "", "Core\\Str");
const MATCH = entry("lang:statements/match", "lang", "lang/statements/match", "docs/reference/lang/statements.md:1");
const WHILE = entry("lang:statements/while", "lang", "lang/statements/while", "docs/reference/lang/statements.md:5");

const world = (entries: Entry[] = [LENGTH, MATCH, WHILE]): World => ({
  entries,
  meta: { classes: [{ name: "Core\\Str", members: [{ name: "length", doc: { short: "Counts the characters." } }] }] },
});

let s: Scratch;
afterEach(() => s?.cleanup());

function site(): Scratch {
  s = scratch();
  s.put("docs/reference/lang/statements.md", "# match\n\nA match.\n\n# while\n\nA loop.\n");
  s.put("docs/examples/core/Str/length/about.md", "Counts characters.\n");
  s.put("docs/examples/core/Str/length/01-basic.nvs", "<?nvs\necho 1;\n");
  s.put(`${DOCS}/syntax/match.mdx`, "---\ntitle: match\ncovers: ['lang:statements/match']\n---\n\nText.\n");
  s.put(`${DOCS}/reference/strings.mdx`, "---\ntitle: Strings\ncovers:\n  - Core\\Str::length\n---\n\nText.\n");
  return s;
}

describe("nv site --stale", () => {
  test("a page never stamped is stale, and a stamp clears it", () => {
    const root = site().root;
    expect(staleness(world(), root).stale).toEqual(["reference/strings.mdx: Core\\Str::length (never stamped)", "syntax/match.mdx: lang:statements/match (never stamped)"]);
    expect(stamp(world(), ["syntax/match.mdx", `${DOCS}/reference/strings.mdx`], root)).toEqual([]);
    expect(staleness(world(), root).stale).toEqual([]);
  });

  test("a changed about.md makes its page stale, and a re-stamp clears it", () => {
    const root = site().root;
    stamp(world(), ["reference/strings.mdx", "syntax/match.mdx"], root);
    s.put("docs/examples/core/Str/length/about.md", "Counts the characters of a string.\n");
    expect(staleness(world(), root).stale).toEqual(["reference/strings.mdx: Core\\Str::length (changed since its stamp)"]);
    stamp(world(), ["reference/strings.mdx"], root);
    expect(staleness(world(), root).stale).toEqual([]);
  });

  test("a changed card or chapter section makes its page stale", () => {
    const root = site().root;
    stamp(world(), ["reference/strings.mdx", "syntax/match.mdx"], root);
    s.put("docs/reference/lang/statements.md", "# match\n\nA match, changed.\n\n# while\n\nA loop.\n");
    const changed = world();
    changed.meta.classes![0]!.members![0]!.doc = { short: "Counts graphemes." };
    expect(staleness(changed, root).stale).toEqual(["reference/strings.mdx: Core\\Str::length (changed since its stamp)", "syntax/match.mdx: lang:statements/match (changed since its stamp)"]);
  });

  test("a renamed id is broken, and the page cannot be stamped", () => {
    const root = site().root;
    const renamed = world([entry("Core\\Str::count", "member", "core/Str/count", "", "Core\\Str"), MATCH, WHILE]);
    expect(staleness(renamed, root).broken).toEqual(["reference/strings.mdx: Core\\Str::length"]);
    expect(stamp(renamed, ["reference/strings.mdx"], root)).toEqual(["reference/strings.mdx: covers Core\\Str::length, which the roster does not have"]);
  });

  test("a lang feature no Syntax page covers is uncovered, and a page without covers is named", () => {
    const root = site().root;
    s.put(`${DOCS}/guides/index.mdx`, "---\ntitle: Guides\n---\n\nText.\n");
    const found = staleness(world(), root);
    expect(found.uncovered).toEqual(["lang:statements/while"]);
    expect(found.unlisted).toEqual(["guides/index.mdx"]);
  });

  test("the wrap gate names every broken id, and only the stale pages the session's changes reach", () => {
    const root = site().root;
    const renamed = world([LENGTH, entry("lang:statements/matches", "lang", "lang/statements/matches", "docs/reference/lang/statements.md:1"), WHILE]);
    expect(wrapProblems(renamed, [], root)).toEqual(["broken: syntax/match.mdx: lang:statements/match"]);
    expect(wrapProblems(world(), ["docs/examples/core/Str/other/about.md"], root)).toEqual([]);
    expect(wrapProblems(world(), ["docs/examples/core/Str/length/about.md"], root)).toEqual(["stale: reference/strings.mdx: Core\\Str::length (never stamped)"]);
    expect(wrapProblems(world(), ["docs/reference/lang/statements.md", `${DOCS}/reference/strings.mdx`], root)).toEqual([
      "stale: reference/strings.mdx: Core\\Str::length (never stamped)",
      "stale: syntax/match.mdx: lang:statements/match (never stamped)",
    ]);
  });

  test("a generated Reference page is not a handwritten one", () => {
    const root = site().root;
    s.put(`${DOCS}/reference/core/str/length.mdx`, "---\ntitle: length\n---\n");
    expect(staleness(world(), root).unlisted).toEqual([]);
  });
});

describe("nv site --check snippets", () => {
  test("an inline Novis fence, a missing snippet and an unused one are named", () => {
    const root = site().root;
    s.put(`${DOCS}/guides/a.mdx`, '---\ncovers: []\n---\n\n```novis\necho 1;\n```\n\n<Snippet src="guides/a/01-one.nvs" />\n<Snippet src="guides/a/02-gone.nvs" />\n');
    s.put("website/snippets/guides/a/01-one.nvs", "<?nvs\necho 1;\n");
    s.put("website/snippets/guides/a/01-one.out", "1\n");
    s.put("website/snippets/guides/b/01-unused.nvs", "<?nvs\necho 2;\n");
    expect(snippetShape(root).problems).toEqual([
      `${DOCS}/guides/a.mdx:5: an inline Novis code fence; use <Snippet src="..."/>`,
      `${DOCS}/guides/a.mdx: <Snippet src="guides/a/02-gone.nvs"/> names no file under website/snippets/`,
      "website/snippets/guides/b/01-unused.nvs: no page shows it",
      "website/snippets/guides/b/01-unused.nvs: no .out beside it",
    ]);
  });
});

describe("nv site --check prose", () => {
  test("code, components, comments and headings are not prose, and list items are paragraphs of their own", () => {
    const body = ["import X from 'y'", "", "# Title", "", "One line", "goes on.", "", "```sh", "a — b", "```", "{/* a", "comment */}", "<X />", "- first", "- second"].join("\n");
    expect(paragraphs(body)).toEqual([
      { line: 5, text: "One line goes on." },
      { line: 14, text: "first" },
      { line: 15, text: "second" },
    ]);
  });

  test("a long sentence, a joining dash and a long paragraph are named", () => {
    const long = `${"word ".repeat(26).trim()}.`;
    expect(proseProblems(`${long}\n\nOne — two.\n\nA. B. C. D. E. F. G.`)).toEqual([
      "1: a 26-word sentence opening `word word word word word ...`; 25 is the bound",
      "3: a dash joins two sentences; write two",
      "5: a paragraph of 7 sentences; 6 is the bound",
    ]);
  });

  test("a code span counts as one word", () => {
    expect(proseProblems(`\`${"a ".repeat(40)}\` is fine.`)).toEqual([]);
  });
});
