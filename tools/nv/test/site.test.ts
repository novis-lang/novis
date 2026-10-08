import { afterEach, describe, expect, test } from "bun:test";
import { join } from "node:path";
import { builtUrls, DOCS, errVerdict, FRONT_PAGE, GUIDE_SECTIONS, guidesProblems, IN_DEPTH_SECTIONS, inDepthProblems, paragraphs, parseRequest, proseProblems, serveRun, snippetOutputs, snippetRun, snippetShape, stamp, staleness, syntaxProblems, wrapProblems, type Served, type World } from "../cmd/site.ts";
import type { Entry } from "../proofs/roster.ts";
import { scratch, type Scratch } from "./scratch.ts";

const entry = (id: string, kind: Entry["kind"], path: string, anchor = "", group = ""): Entry => ({ id, kind, group, path, anchor, summary: "", help: "" });

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
      "website/snippets/guides/b/01-unused.nvs: no .out, .http.out or .err beside it",
    ]);
  });

  test("output=\"http\" needs a .http.out, takes no other value, and the front page needs it for every snippet with one", () => {
    const root = site().root;
    s.put(`${DOCS}/guides/a.mdx`, '---\ncovers: []\n---\n\n<Snippet src="guides/a/01-web.nvs" output="http" />\n<Snippet src="guides/a/02-cli.nvs" output="http" />\n<Snippet src="guides/a/02-cli.nvs" output="cli" />\n');
    s.put(`${DOCS}/${FRONT_PAGE}`, '---\ncovers: []\n---\n\n<Snippet src="guides/a/01-web.nvs" slot="x" />\n<Snippet src="guides/a/02-cli.nvs" />\n');
    s.put("website/snippets/guides/a/01-web.nvs", "<?nvs\necho 1;\n");
    s.put("website/snippets/guides/a/01-web.http.out", "1\n");
    s.put("website/snippets/guides/a/02-cli.nvs", "<?nvs\necho 2;\n");
    s.put("website/snippets/guides/a/02-cli.out", "2\n");
    expect(snippetShape(root).problems).toEqual([
      `${DOCS}/guides/a.mdx: <Snippet src="guides/a/02-cli.nvs"/> has output="http", and the snippet has no .http.out`,
      `${DOCS}/guides/a.mdx: <Snippet src="guides/a/02-cli.nvs"/> has output="cli", and the only value is "http"`,
      `${DOCS}/${FRONT_PAGE}: <Snippet src="guides/a/01-web.nvs"/> needs output="http": the front page shows only what the server sends`,
    ]);
  });

  test("a snippet with a .err passes, and one with both a .out and a .err is named", () => {
    const root = site().root;
    s.put(`${DOCS}/guides/a.mdx`, '---\ncovers: []\n---\n\n<Snippet src="guides/a/01-bad.nvs" />\n<Snippet src="guides/a/02-both.nvs" />\n');
    s.put("website/snippets/guides/a/01-bad.nvs", "<?nvs\necho $nope;\n");
    s.put("website/snippets/guides/a/01-bad.err", "error[E0402]: no variable `$nope`\n");
    s.put("website/snippets/guides/a/02-both.nvs", "<?nvs\necho 1;\n");
    s.put("website/snippets/guides/a/02-both.out", "1\n");
    s.put("website/snippets/guides/a/02-both.err", "error\n");
    expect(snippetShape(root).problems).toEqual(["website/snippets/guides/a/02-both.nvs: both a .out and a .err beside it; keep one"]);
  });

  test("a .err snippet is checked in its own directory, and a snippet is run and served from the root", () => {
    const root = site().root;
    s.put("website/snippets/guides/a/nvs.toml", "");
    expect(snippetRun("website/snippets/guides/a/01-bad.nvs", "err", "nvs", root)).toEqual({ argv: ["nvs", "check", "01-bad.nvs"], cwd: join(root, "website/snippets/guides/a"), env: { NO_COLOR: "1" } });
    expect(snippetRun("website/snippets/guides/a/02-ok.nvs", "out", "nvs", root)).toEqual({
      argv: ["nvs", "run", "--config", "website/snippets/guides/a/nvs.toml", "website/snippets/guides/a/02-ok.nvs"],
      cwd: root,
    });
    expect(serveRun("website/snippets/guides/b/03-web.nvs", "nvs", 0, root)).toEqual({ argv: ["nvs", "serve", "website/snippets/guides/b/03-web.nvs", "--port", "0"], cwd: root });
  });

  test("a .nvsr is the request the server is sent, and a section a connection cannot carry is refused", () => {
    const text = "--METHOD--\nPOST\n--PATH--\n/upload\n--QUERY--\na=1\n--HEADERS--\nHost: example.com\ncontent-type: multipart/form-data; boundary=b\n--BODY_CRLF--\n--b\n\nx\n--b--\n";
    expect(parseRequest(text)).toEqual({ method: "POST", path: "/upload", query: "a=1", headers: [["host", "example.com"], ["content-type", "multipart/form-data; boundary=b"]], body: "--b\r\n\r\nx\r\n--b--\r\n" });
    expect(parseRequest("--METHOD--\nGET\n--PATH--\n/\n--BODY--\n--PATH--\n")).toMatchObject({ path: "/", body: "--PATH--\n" });
    expect(parseRequest("--METHOD--\nGET\n--PATH--\n/\n--CLIENT_IP--\n203.0.113.7\n")).toBe("`--CLIENT_IP--` is not something a real connection can send");
    expect(parseRequest("--METHOD--\nGET\n")).toBe("a request states its `--METHOD--` and its `--PATH--`");
  });

  test("a .err matches the diagnostic, and a different one, a snippet that compiles and a crash are named", () => {
    const p = "website/snippets/guides/a/01-bad.nvs";
    const want = "error[E0402]: no variable `$nope`\n  --> 01-bad.nvs:2:6\n";
    const failed = (stderr: string, code = 1) => ({ code, stdout: "", stderr });
    expect(errVerdict(p, failed(want.replace(/\n/g, "\r\n") + "\n\n"), want)).toBeNull();
    expect(errVerdict(p, failed("error[E0402]: no variable `$other`\n"), want)).toBe(`${p}: \`nvs check\` reports something other than its .err`);
    expect(errVerdict(p, { code: 0, stdout: "no errors\n", stderr: "" }, want)).toBe(`${p}: compiles, and a snippet with a .err must not`);
    expect(errVerdict(p, failed("thread 'main' panicked\n", 101), want)).toBe(`${p}: \`nvs check\` exit 101: thread 'main' panicked`);
  });
});

describe("nv site --check snippets: the command line and the server", () => {
  const p = "website/snippets/guides/a/02-ok.nvs";
  const ran = (stdout: string, code = 0, stderr = "") => ({ code, stdout, stderr });
  const served = (body: string, status = 200): Served => ({ status, body, failure: null, started: true });
  const files = (out: string | null, http: string | null) => ({ out, http });

  test("a command-line snippet matches its .out, and a different output or a failed run is named", () => {
    expect(snippetOutputs(p, false, ran("1\r\n"), served("1\n"), files("1\n", null))).toEqual({ owed: { out: "1\r\n", http: null }, problems: [] });
    expect(snippetOutputs(p, false, ran("2\n"), served("2\n"), files("1\n", null)).problems).toEqual([`${p}: prints something other than its .out`]);
    expect(snippetOutputs(p, false, ran("", 1, "error[E0402]: no variable\n"), null, files("1\n", null))).toEqual({ owed: null, problems: [`${p}: exit 1: error[E0402]: no variable`] });
  });

  test("a web snippet is checked by the server's body, and a missing .http.out or a 5xx is named", () => {
    const failing = ran("", 1, "error: no request\n");
    expect(snippetOutputs(p, true, failing, served("product 7\n", 422), files(null, "product 7\n")).problems).toEqual([]);
    expect(snippetOutputs(p, true, failing, served("product 7\n"), files(null, "product 8\n")).problems).toEqual([`${p}: \`nvs serve\` sends something other than its .http.out`]);
    expect(snippetOutputs(p, true, failing, served("product 7\n"), files("product 7\n", null)).problems).toEqual([
      `${p}: no .http.out beside it, and a snippet with a .nvsr shows the body \`nvs serve\` sends`,
      `${p}: delete its .out: \`nvs run\` exits 1 without a request`,
    ]);
    expect(snippetOutputs(p, true, failing, served("oops", 500), files(null, "x")).problems).toEqual([`${p}: \`nvs serve\` answered 500: oops`]);
  });

  test("two different outputs need both files, in either kind of snippet", () => {
    const cli = ran('{"a":1}\n');
    const http = served("{&quot;a&quot;:1}\n");
    expect(snippetOutputs(p, false, cli, http, files('{"a":1}\n', null)).problems).toEqual([`${p}: \`nvs run\` and \`nvs serve\` print different things, so the page shows both: write its .http.out`]);
    expect(snippetOutputs(p, true, cli, http, files(null, "{&quot;a&quot;:1}\n")).problems).toEqual([`${p}: \`nvs run\` and \`nvs serve\` print different things, so the page shows both: write its .out`]);
    expect(snippetOutputs(p, false, cli, http, files('{"a":1}\n', "{&quot;a&quot;:1}\n"))).toEqual({ owed: { out: '{"a":1}\n', http: "{&quot;a&quot;:1}\n" }, problems: [] });
  });

  test("two equal outputs are one file, and the second is named for deletion", () => {
    expect(snippetOutputs(p, false, ran("1\n"), served("1\n"), files("1\n", "1\n")).problems).toEqual([`${p}: delete its .http.out: \`nvs serve\` sends the same as its .out`]);
    expect(snippetOutputs(p, true, ran("1\n"), served("1\n"), files("1\n", "1\n")).problems).toEqual([`${p}: delete its .out: \`nvs run\` prints the same as its .http.out`]);
  });

  test("a server that does not apply to a command-line snippet needs no .http.out, and one that started and sent nothing is named", () => {
    expect(snippetOutputs(p, false, ran("1\n"), served("error page", 500), files("1\n", null)).problems).toEqual([]);
    expect(snippetOutputs(p, false, ran("1\n"), { status: 0, body: "", failure: "the server exited 1: error", started: false }, files("1\n", "x")).problems).toEqual([`${p}: delete its .http.out: \`nvs serve\` does not start with it`]);
    expect(snippetOutputs(p, false, ran("1\n"), { status: 0, body: "", failure: "socket closed", started: true }, files("1\n", null)).problems).toEqual([`${p}: \`nvs serve\` started and sent no response: socket closed`]);
  });
});

describe("nv site --check syntax", () => {
  test("a Syntax page with no snippet or no Do and don't list is named, and so is each uncovered feature", () => {
    const root = site().root;
    s.put(`${DOCS}/syntax/index.mdx`, "---\ntitle: Syntax\ncovers: []\n---\n\nText.\n");
    s.put(`${DOCS}/syntax/empty.mdx`, "---\ntitle: Empty\ncovers: []\n---\n\n## Do and don't\n\nText.\n");
    expect(syntaxProblems(world(), root)).toEqual([
      `${DOCS}/syntax/empty.mdx: covers no feature`,
      `${DOCS}/syntax/empty.mdx: shows no <Snippet>`,
      `${DOCS}/syntax/empty.mdx: has no \`## Do and don't\` heading with a list under it`,
      `${DOCS}/syntax/match.mdx: shows no <Snippet>`,
      `${DOCS}/syntax/match.mdx: has no \`## Do and don't\` heading with a list under it`,
      "uncovered: lang:statements/while",
    ]);
  });

  test("a page with covers, a snippet and a Do and don't list passes", () => {
    const root = site().root;
    s.put(`${DOCS}/syntax/match.mdx`, "---\ntitle: match\ncovers: ['lang:statements/match', 'lang:statements/while']\n---\n\n<Snippet src=\"syntax/match/01-basic.nvs\" />\n\n## Do and don't\n\n- Do this.\n");
    expect(syntaxProblems(world(), root)).toEqual([]);
  });
});

describe("nv site --check guides", () => {
  const page = (snippet: string) => `---\ntitle: A page\ncovers: []\n---\n\n${snippet}\n`;
  const sidebar = (dirs: string[]) =>
    ["sidebar: [", "        {", "          label: 'Guides',", "          items: [", ...dirs.map((d) => `            { label: '${d}', link: '/guides/${d}/' },`), "          ],", "        },", "        {", "          label: INSTALL.label,", "          items: [{ label: 'x', link: '/guides/tour/' }],", "        },"].join("\n");

  /** Every section at its count, but the pages `leave` names. */
  function guides(leave: string[] = []): Scratch {
    s = site();
    const put = (key: string, text: string) => leave.some((l) => key.startsWith(l)) || s.put(`${DOCS}/${key}`, text);
    for (const [dir, least, snippets] of GUIDE_SECTIONS) {
      put(`guides/${dir}/index.mdx`, page(""));
      for (let i = 1; i <= least; i++) put(`guides/${dir}/${String(i).padStart(2, "0")}.mdx`, page(snippets ? `<Snippet src="guides/${dir}/${i}.nvs" />` : ""));
    }
    s.put("website/astro.config.ts", sidebar(GUIDE_SECTIONS.map(([d]) => d)));
    return s;
  }

  test("the eight sections with their counts, snippets and sidebar order pass", () => {
    expect(guidesProblems(guides().root)).toEqual([]);
  });

  test("a missing section, a short count, a page with no snippet and a sidebar out of order are named", () => {
    const root = guides(["guides/testing/", "guides/simple-programs/06.mdx"]).root;
    s.put(`${DOCS}/guides/cookbook/01.mdx`, page("No snippet."));
    s.put("website/astro.config.ts", sidebar(["tour", "why-novis", "simple-programs", "how-to-use", "cookbook", "production", "example-apps"]));
    expect(guidesProblems(root)).toEqual([
      `${DOCS}/guides/simple-programs/: 5 page(s) beside its index, and the goal names 6 at least`,
      `${DOCS}/guides/cookbook/01.mdx: shows no <Snippet>`,
      `${DOCS}/guides/testing/: no page`,
      "website/astro.config.ts: the Guides sidebar lists /guides/tour/ out of the goal's order",
      "website/astro.config.ts: the Guides sidebar has no link into /guides/testing/",
    ]);
  });
});

describe("nv site --check in-depth", () => {
  const page = `---\ntitle: A page\ncovers: []\n---\n\nText.\n`;
  const sidebar = (slugs: string[]) =>
    ["sidebar: [", "        {", "          label: 'In-Depth',", "          items: [", ...slugs.map((d) => `            { label: '${d}', link: '/in-depth/${d}/' },`), "          ],", "        },"].join("\n");

  /** Every section with its page or its directory at its count, but the pages `leave` names. */
  function inDepth(leave: string[] = []): Scratch {
    s = site();
    const put = (key: string) => leave.some((l) => key.startsWith(l)) || s.put(`${DOCS}/${key}`, page);
    for (const [slug, least] of IN_DEPTH_SECTIONS) {
      if (least === 0) put(`in-depth/${slug}.mdx`);
      else {
        put(`in-depth/${slug}/index.mdx`);
        for (let i = 1; i <= least; i++) put(`in-depth/${slug}/${i}.mdx`);
      }
    }
    s.put("website/astro.config.ts", sidebar(IN_DEPTH_SECTIONS.map(([d]) => d)));
    return s;
  }

  test("the eight sections with their pages and sidebar order pass", () => {
    expect(inDepthProblems(inDepth().root)).toEqual([]);
  });

  test("a missing page, a short directory and a sidebar out of order are named", () => {
    const root = inDepth(["in-depth/roadmap.mdx", "in-depth/concepts/5.mdx"]).root;
    s.put("website/astro.config.ts", sidebar(["never", "what-for", "how-we-decide", "design-principles", "concepts", "falls-behind", "what-changed"]));
    expect(inDepthProblems(root)).toEqual([
      `${DOCS}/in-depth/concepts/: 4 page(s) beside its index, and the goal names 5 at least`,
      `${DOCS}/in-depth/roadmap.mdx: no page`,
      "website/astro.config.ts: the In-Depth sidebar lists /in-depth/never/ out of the goal's order",
      "website/astro.config.ts: the In-Depth sidebar has no link into /in-depth/roadmap/",
    ]);
  });

  test("the built URLs are each page, the Core pages and the Configuration and CLI pages", () => {
    s = scratch();
    const root = s.root;
    s.put(`${DOCS}/in-depth/index.mdx`, page);
    s.put(`${DOCS}/in-depth/roadmap.mdx`, page);
    s.put("website/src/data/core.json", JSON.stringify({ classes: [{ members: [{ url: "/reference/core/str/length/" }] }] }));
    s.put("website/src/data/reference.json", JSON.stringify({ pages: [{ url: "/reference/config/app/" }] }));
    expect([...builtUrls(root)].sort()).toEqual(["/", "/in-depth/", "/in-depth/roadmap/", "/reference/cli/", "/reference/config/", "/reference/config/app/", "/reference/core/str/", "/reference/core/str/length/"]);
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
