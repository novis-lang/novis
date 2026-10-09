import { afterEach, describe, expect, test } from "bun:test";
import { DOCS, pageUrl, QUESTIONS, questionCounts, questionProblems, questionSources, readQuestions, stampQuestions, wrapProblems, type World } from "../cmd/site.ts";
import type { Entry } from "../proofs/roster.ts";
import { DATA_FILE } from "../renderers/website-core.ts";
import { REFERENCE_FILE } from "../renderers/website-reference.ts";
import { scratch, type Scratch } from "./scratch.ts";

const entry = (id: string, kind: Entry["kind"], path: string, anchor = "", group = ""): Entry => ({ id, kind, group, path, anchor, summary: "", help: "" });

const LENGTH = entry("Core\\Str::length", "member", "core/Str/length", "crates/nvs-stdlib/src/str.rs:10", "Core\\Str");
const LIMIT = entry("directive:limits.memory", "directive", "config/limits-memory");

const world = (): World => ({
  entries: [LENGTH, LIMIT],
  meta: {
    classes: [{ name: "Core\\Str", doc: { short: "Strings." }, members: [{ name: "length", doc: { short: "Counts the characters." } }] }],
    directives: [{ key: "limits.memory" }],
  },
});

const STR = "/reference/core/str/";
const LEN = "/reference/core/str/length/";
const LIMITS = "/reference/config/limits/";
const HELLO = "/install/hello-world/";

let s: Scratch;
afterEach(() => s?.cleanup());

function site(questions: Record<string, unknown>): string {
  s = scratch();
  s.put("docs/examples/core/Str/length/about.md", "Counts characters.\n");
  s.put(DATA_FILE, JSON.stringify({ classes: [{ name: "Core\\Str", url: STR, members: [{ name: "length", url: LEN }] }] }));
  s.put(REFERENCE_FILE, JSON.stringify({ pages: [{ area: "config", slug: "limits", url: LIMITS, title: "[limits]", group: "Blocks", intro: null, keys: [{ id: LIMIT.id, title: "limits.memory", examples: "config/limits-memory", key: "limits.memory", class: "System", apply: "Reload" }] }] }));
  s.put(`${DOCS}/install/hello-world.mdx`, "---\ntitle: Hello\ncovers: []\n---\n\nText.\n");
  s.put(QUESTIONS, JSON.stringify(questions));
  return s.root;
}

const asked = ["How do I count the characters in a string?"];

describe("nv site --questions", () => {
  test("every built page has a source, and a handwritten page's URL drops its extension and index", () => {
    const root = site({});
    expect(questionSources(world(), root).map((x) => `${x.area} ${x.url} ${x.feature}`)).toEqual([
      `core ${STR} Core\\Str`,
      `core ${LEN} Core\\Str::length`,
      `config ${LIMITS} directive:limits.memory`,
      `pages ${HELLO} install/hello-world.mdx`,
    ]);
    expect(pageUrl("index.mdx")).toBe("/");
    expect(pageUrl("guides/Index.mdx")).toBe("/guides/");
  });

  test("a bare list is never stamped, and a stamp records the feature and the hash", () => {
    const root = site({ [LEN]: asked });
    expect(readQuestions(root)[LEN]).toEqual({ feature: "", hash: "", questions: asked });
    expect(questionProblems(world(), root)).toEqual([`stale: ${LEN} (Core\\Str::length): reread the page, rewrite its questions, then \`bun nv site --questions --stamp ${LEN}\``]);
    expect(stampQuestions(world(), [LEN], root)).toEqual([]);
    expect(readQuestions(root)[LEN]!.feature).toBe("Core\\Str::length");
    expect(questionProblems(world(), root)).toEqual([]);
  });

  test("a changed about.md or page file makes its questions stale", () => {
    const root = site({ [LEN]: asked, [HELLO]: asked });
    stampQuestions(world(), [LEN, HELLO], root);
    s.put("docs/examples/core/Str/length/about.md", "Counts the characters of a string.\n");
    s.put(`${DOCS}/install/hello-world.mdx`, "---\ntitle: Hello\ncovers: []\n---\n\nNew text.\n");
    expect(questionProblems(world(), root).map((p) => p.split(" (")[0])).toEqual([`stale: ${LEN}`, `stale: ${HELLO}`]);
  });

  test("a URL no page has is broken and cannot be stamped, and the status counts owed pages per area", () => {
    const root = site({ "/reference/core/str/gone/": asked, [LEN]: asked });
    expect(questionProblems(world(), root)[0]).toBe("broken: /reference/core/str/gone/: no page has this URL");
    expect(stampQuestions(world(), ["/reference/core/str/gone/"], root)).toEqual(["/reference/core/str/gone/: no built page has this URL"]);
    expect(stampQuestions(world(), [STR], root)).toEqual([`${STR}: has no questions to stamp`]);
    expect(questionCounts(world(), root)).toEqual(["questions core: 1 owed, 1 stale", "questions config: 1 owed, 0 stale", "questions cli: 0 owed, 0 stale", "questions pages: 1 owed, 0 stale"]);
  });

  test("the wrap gate names only the stale questions the session's changes reach", () => {
    const root = site({ [LEN]: asked, [HELLO]: asked });
    stampQuestions(world(), [LEN, HELLO], root);
    s.put("docs/examples/core/Str/length/about.md", "Counts the characters of a string.\n");
    expect(wrapProblems(world(), [], root)).toEqual([]);
    expect(wrapProblems(world(), ["docs/examples/core/Str/length/about.md"], root)).toEqual([`stale questions: ${LEN} (Core\\Str::length)`]);
    expect(wrapProblems(world(), ["crates/nvs-stdlib/src/str.rs"], root)).toEqual([`stale questions: ${LEN} (Core\\Str::length)`]);
  });
});
