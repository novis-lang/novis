import { afterEach, describe, expect, test } from "bun:test";
import { join, relative } from "node:path";
import { APP_FILES, appFileProblem, appFiles, binary, builtUrls, codeBlocks, codeProblems, DOCS, fenceInfo, errVerdict, FRONT_PAGE, GUIDE_SECTIONS, guidesProblems, IN_DEPTH_SECTIONS, jsonProblems, inDepthProblems, paragraphs, parseRequest, proseProblems, serveRun, shellCommands, shellProblems, snippetOutputs, snippetRun, snippetShape, stamp, staleness, syntaxProblems, TOOL_SECTIONS, tomlProblems, toolsProblems, yamlProblems, type Served, type World } from "../cmd/site.ts";
import { ROOT } from "../lib/paths.ts";
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

  test("a lang feature's changed details.md or example makes its page stale", () => {
    const root = site().root;
    s.put("docs/examples/lang/statements/match/about.md", "A match.\n");
    s.put("docs/examples/lang/statements/match/details.md", "Arms run in order.\n");
    s.put("docs/examples/lang/statements/match/01-basic.nvs", "<?nvs\necho 1;\n");
    stamp(world(), ["syntax/match.mdx"], root);
    s.put("docs/examples/lang/statements/match/details.md", "Arms run in order, and the first match wins.\n");
    expect(staleness(world(), root).stale).toEqual(["reference/strings.mdx: Core\\Str::length (never stamped)", "syntax/match.mdx: lang:statements/match (changed since its stamp)"]);
    stamp(world(), ["syntax/match.mdx"], root);
    s.put("docs/examples/lang/statements/match/01-basic.nvs", "<?nvs\necho 2;\n");
    expect(staleness(world(), root).stale).toContain("syntax/match.mdx: lang:statements/match (changed since its stamp)");
  });

  test("a changed card makes its page stale, and a language feature's generated chapter section does not", () => {
    const root = site().root;
    stamp(world(), ["reference/strings.mdx", "syntax/match.mdx"], root);
    s.put("docs/reference/lang/statements.md", "# match\n\nA match, changed.\n\n# while\n\nA loop.\n");
    const changed = world();
    changed.meta.classes![0]!.members![0]!.doc = { short: "Counts graphemes." };
    expect(staleness(changed, root).stale).toEqual(["reference/strings.mdx: Core\\Str::length (changed since its stamp)"]);
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

  test("a replay shows its snippet, and one that names no snippet or no .replay or .dap is named", () => {
    const root = site().root;
    s.put(`${DOCS}/guides/a.mdx`, '---\ncovers: []\n---\n\n<Replay src="guides/a/01-key.nvs" />\n<Replay src="guides/a/02-plain.nvs" />\n<Replay src="guides/a/03-gone.nvs" />\n');
    s.put("website/snippets/guides/a/01-key.nvs", "<?nvs\necho 1;\n");
    s.put("website/snippets/guides/a/01-key.out", "1\n");
    s.put("website/snippets/guides/a/01-key.replay", "--START--\n<?nvs\necho <|>;\n--STEPS--\ntype 1\n");
    s.put("website/snippets/guides/a/02-plain.nvs", "<?nvs\necho 2;\n");
    s.put("website/snippets/guides/a/02-plain.out", "2\n");
    expect(snippetShape(root).problems).toEqual([
      `${DOCS}/guides/a.mdx: <Replay src="guides/a/02-plain.nvs"/>: the snippet has no .replay or .dap beside it`,
      `${DOCS}/guides/a.mdx: <Replay src="guides/a/03-gone.nvs"/> names no file under website/snippets/`,
    ]);
  });

  test("a walkthrough scene shown on a page shows its editor's snippet, and one no page shows does not", () => {
    const root = site().root;
    const scenes = "website/src/components/walkthrough/scenes";
    s.put(`${DOCS}/in-depth/a.mdx`, "---\ncovers: []\n---\n\n<Shown />\n");
    s.put(`${scenes}/Shown.astro`, '<Stage id="x" label="x">\n  <Editor name="editor" src="w/01-shown.nvs" place={{ wide: [0, 0, 1, 1] }} />\n  <Editor name="gone" src="w/03-gone.nvs" place={{ wide: [0, 0, 1, 1] }} />\n</Stage>\n');
    s.put(`${scenes}/Hidden.astro`, '<Editor src="w/02-hidden.nvs" />\n');
    s.put("website/snippets/w/01-shown.nvs", "<?nvs\necho 1;\n");
    s.put("website/snippets/w/01-shown.out", "1\n");
    s.put("website/snippets/w/02-hidden.nvs", "<?nvs\necho 2;\n");
    s.put("website/snippets/w/02-hidden.out", "2\n");
    expect(snippetShape(root).problems).toEqual([
      `${scenes}/Shown.astro: <Editor src="w/03-gone.nvs"/> names no file under website/snippets/`,
      "website/snippets/w/02-hidden.nvs: no page shows it",
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

  test("a server this host does not start is a skip: a web snippet is not checked, and a command-line one keeps its .out check", () => {
    const skipped: Served = { status: 0, body: "", failure: null, started: false, skipped: "this host has no Docker" };
    expect(snippetOutputs(p, true, ran("", 1), skipped, files(null, "x"))).toEqual({ owed: null, problems: [], skipped: "this host has no Docker" });
    expect(snippetOutputs(p, false, ran("1\n"), skipped, files("1\n", "x"))).toEqual({ owed: null, problems: [], skipped: "this host has no Docker" });
    expect(snippetOutputs(p, false, ran("2\n"), skipped, files("1\n", null)).problems).toEqual([`${p}: prints something other than its .out`]);
    expect(snippetOutputs(p, false, ran("1\n"), skipped, files(null, null)).problems).toEqual([`${p}: no .out beside it`]);
  });
});

describe("nv site --check syntax", () => {
  test("a Syntax page with no snippet or no Do and don't list is named, and so is each uncovered feature", () => {
    const root = site().root;
    s.put(`${DOCS}/syntax/index.mdx`, "---\ntitle: Syntax\ncovers: []\n---\n\nText.\n");
    s.put(`${DOCS}/syntax/empty.mdx`, "---\ntitle: Empty\ncovers: []\n---\n\n## Do and don't\n\nText.\n");
    expect(syntaxProblems(world(), root)).toEqual([
      `${DOCS}/syntax/empty.mdx: covers no feature`,
      `${DOCS}/syntax/empty.mdx: shows no <Snippet> or <FeatureSection>`,
      `${DOCS}/syntax/empty.mdx: has no \`## Do and don't\` heading with a list under it`,
      `${DOCS}/syntax/match.mdx: shows no <Snippet> or <FeatureSection>`,
      `${DOCS}/syntax/match.mdx: has no \`## Do and don't\` heading with a list under it`,
      "uncovered: lang:statements/while",
    ]);
  });

  test("a page with covers, a snippet and a Do and don't list passes", () => {
    const root = site().root;
    s.put(`${DOCS}/syntax/match.mdx`, "---\ntitle: match\ncovers: ['lang:statements/match', 'lang:statements/while']\n---\n\n<Snippet src=\"syntax/match/01-basic.nvs\" />\n\n## Do and don't\n\n- Do this.\n");
    expect(syntaxProblems(world(), root)).toEqual([]);
  });

  test("a page that shows its features with <FeatureSection> passes", () => {
    const root = site().root;
    s.put(`${DOCS}/syntax/match.mdx`, "---\ntitle: match\ncovers: ['lang:statements/match', 'lang:statements/while']\n---\n\n<FeatureSection examples=\"lang/statements/match\" heading=\"match\" id=\"match\" />\n\n## Do and don't\n\n- Do this.\n");
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

  test("the nine sections with their counts, snippets and sidebar order pass", () => {
    expect(guidesProblems(guides().root)).toEqual([]);
  });

  test("a missing section, a short count, a page with no snippet and a sidebar out of order are named", () => {
    const root = guides(["guides/testing/", "guides/simple-programs/06.mdx"]).root;
    s.put(`${DOCS}/guides/cookbook/01.mdx`, page("No snippet."));
    s.put("website/astro.config.ts", sidebar(["tour", "why-novis", "walkthroughs", "simple-programs", "how-to-use", "cookbook", "production", "example-apps", "extensions"]));
    expect(guidesProblems(root)).toEqual([
      `${DOCS}/guides/simple-programs/: 5 page(s) beside its index, and the goal names 6 at least`,
      `${DOCS}/guides/cookbook/01.mdx: shows no <Snippet>`,
      `${DOCS}/guides/testing/: no page`,
      "website/astro.config.ts: the Guides sidebar lists /guides/tour/ out of the goal's order",
      "website/astro.config.ts: the Guides sidebar has no link into /guides/testing/",
    ]);
  });
});

describe("nv site --check tools", () => {
  const page = (covers: string, body: string) => `---\ntitle: A tool\ncovers: ${covers}\n---\n\n${body}\n`;
  const sidebar = (dirs: string[]) =>
    ["sidebar: [", "        {", "          label: 'Tools',", "          items: [", ...dirs.map((d) => `            { label: '${d}', link: '/tools/${d}/' },`), "          ],", "        },"].join("\n");

  /** An overview with a linked section and a picture for every tool, and a page for each tool but the ones `leave` names. */
  function tools(leave: string[] = []): Scratch {
    s = site();
    s.put(`${DOCS}/tools/index.mdx`, page("[]", TOOL_SECTIONS.map((d) => `## [${d}](/tools/${d}/)\n\n<Shot src="tools/${d}/a.shot" alt="A picture." />\n`).join("\n")));
    for (const dir of TOOL_SECTIONS) if (!leave.includes(dir)) s.put(`${DOCS}/tools/${dir}/index.mdx`, page("['lang:statements/match']", `<Terminal src="tools/${dir}/a.term" />`));
    s.put("website/astro.config.ts", sidebar(TOOL_SECTIONS.filter((d) => !leave.includes(d))));
    return s;
  }

  test("an overview, every page with a picture and features, and the sidebar in order pass", () => {
    expect(toolsProblems(tools().root)).toEqual([]);
  });

  test("an owed page, a page with no picture or features, a missing overview section, an overview section with no picture and a sidebar out of order are named", () => {
    const root = tools(["profiler"]).root;
    s.put(`${DOCS}/tools/cli/index.mdx`, page("[]", "Text only."));
    s.put(`${DOCS}/tools/index.mdx`, page("[]", "## [editor](/tools/editor/)\n\nText only."));
    s.put("website/astro.config.ts", sidebar(["formatter", "editor", ...TOOL_SECTIONS.slice(2).filter((d) => d !== "profiler")]));
    expect(toolsProblems(root)).toEqual([
        `${DOCS}/tools/index.mdx: the /tools/editor/ section shows no <Terminal>, <Replay> or <Shot>`,
      `${DOCS}/tools/index.mdx: has no \`## [...](/tools/formatter/)\` section`,
      `${DOCS}/tools/index.mdx: has no \`## [...](/tools/test-runner/)\` section`,
      `${DOCS}/tools/cli/index.mdx: covers no feature`,
      `${DOCS}/tools/cli/index.mdx: shows no <Terminal>, <Replay> or <Shot>`,
      `${DOCS}/tools/index.mdx: has no \`## [...](/tools/cli/)\` section`,
      `${DOCS}/tools/index.mdx: has no \`## [...](/tools/debugger/)\` section`,
      `${DOCS}/tools/index.mdx: has no \`## [...](/tools/debug-viewer/)\` section`,
      `${DOCS}/tools/profiler/index.mdx: owed, no page yet`,
      `${DOCS}/tools/index.mdx: has no \`## [...](/tools/api-document/)\` section`,
      "website/astro.config.ts: the Tools sidebar lists /tools/formatter/ out of the goal's order",
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

  test("every section with its page and sidebar order passes", () => {
    expect(inDepthProblems(inDepth().root)).toEqual([]);
  });

  test("a missing page, a short directory and a sidebar out of order are named", () => {
    const root = inDepth(["in-depth/roadmap.mdx", "in-depth/concepts/5.mdx"]).root;
    s.put("website/astro.config.ts", sidebar(["never", "what-for", "how-we-decide", "how-novis-is-written", "how-novis-is-tested", "design-principles", "concepts", "new", "personal-opinions", "questions", "what-changed"]));
    expect(inDepthProblems(root)).toEqual([
      `${DOCS}/in-depth/concepts/: 4 page(s) beside its index, and the goal names 5 at least`,
      `${DOCS}/in-depth/roadmap.mdx: no page`,
      "website/astro.config.ts: the In-Depth sidebar lists /in-depth/never/ out of the goal's order",
      "website/astro.config.ts: the In-Depth sidebar has no link into /in-depth/roadmap/",
    ]);
  });

  test("the built URLs are each page, the Core pages and the Configuration, CLI and Types pages", () => {
    s = scratch();
    const root = s.root;
    s.put(`${DOCS}/in-depth/index.mdx`, page);
    s.put(`${DOCS}/in-depth/roadmap.mdx`, page);
    s.put("website/src/data/core.json", JSON.stringify({ classes: [{ members: [{ url: "/reference/core/str/length/" }] }] }));
    s.put("website/src/data/reference.json", JSON.stringify({ pages: [{ url: "/reference/config/app/" }] }));
    expect([...builtUrls(root)].sort()).toEqual(["/", "/in-depth/", "/in-depth/roadmap/", "/reference/cli/", "/reference/config/", "/reference/config/app/", "/reference/core/str/", "/reference/core/str/length/", "/reference/types/"]);
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

  test("the text inside an Opinion is its author's and not judged", () => {
    const long = `${"word ".repeat(30).trim()}.`;
    const body = ["Before.", "", '<Opinion author="A" role="b" date="c">', "", long, "", "One — two.", "", "</Opinion>", "", "After."].join("\n");
    expect(paragraphs(body)).toEqual([
      { line: 1, text: "Before." },
      { line: 11, text: "After." },
    ]);
  });

  test("a code span counts as one word", () => {
    expect(proseProblems(`\`${"a ".repeat(40)}\` is fine.`)).toEqual([]);
  });
});

describe("nv site --check code", () => {
  const PAGE = [
    "---",
    "title: Code",
    "covers: []",
    "---",
    "",
    "```toml",
    "[server]",
    "```",
    "",
    "<Tabs>",
    "  <TabItem label=\"Linux\">",
    "    ```bash",
    "    nvs serve",
    "    ```",
    "  </TabItem>",
    "</Tabs>",
    "",
    '```sh skip="installs a system service"',
    "sudo nvs service install",
    "```",
    "",
    "```txt skip",
    "a tree",
    "```",
    "",
    "```",
    "no language",
    "```",
    "",
    "````md",
    "```html",
    "<p>inner</p>",
    "```",
    "````",
    "",
    "```nvs",
    "echo 1;",
    "```",
  ].join("\n");

  test("a fence's language goes through the aliases, and a reason may be quoted or bare", () => {
    expect(fenceInfo("bash")).toEqual({ lang: "sh", attrs: new Map() });
    expect(fenceInfo(` toml fragment title="nvs.toml"`)).toEqual({ lang: "toml", attrs: new Map([["fragment", ""], ["title", "nvs.toml"]]) });
    expect(fenceInfo(`sh skip='a "service"' x=y`).attrs).toEqual(new Map([["skip", 'a "service"'], ["x", "y"]]));
  });

  test("an indented fence keeps its body without the indent, and a longer fence holds a shorter one", () => {
    const { blocks, problems } = codeBlocks("a.mdx", PAGE);
    expect(problems).toEqual([]);
    expect(blocks.map((b) => [b.line, b.lang])).toEqual([[6, "toml"], [12, "sh"], [18, "sh"], [22, "txt"], [26, ""], [30, "md"], [36, "nvs"]]);
    expect(blocks[1]!.text).toBe("nvs serve");
    expect(blocks[5]!.text).toBe("```html\n<p>inner</p>\n```");
  });

  test("a fence never closed is named", () => {
    expect(codeBlocks("a.mdx", "x\n```sh\nnvs run\n").problems).toEqual([`${DOCS}/a.mdx:2: a fence that is never closed`]);
  });

  test("each block is run by its language's check, or skipped with a reason, and the rest are named", async () => {
    site();
    s.put(`${DOCS}/guides/code.mdx`, PAGE);
    const ran: string[] = [];
    const checks = { toml: (b: { text: string }) => (ran.push(b.text), b.text === "[server]" ? ["`[server]` is wrong"] : []) };
    const at = (line: number) => `${DOCS}/guides/code.mdx:${line}`;
    expect(await codeProblems({ root: s.root, nvs: null }, checks)).toEqual([
      `${at(6)}: \`[server]\` is wrong`,
      `${at(12)}: a \`sh\` block that no check runs: check it, or write skip="<why>" in its fence`,
      `${at(22)}: \`skip\` with no reason: write skip="<why no check runs it>"`,
      `${at(26)}: a block with no language: name it after the fence`,
      `${at(30)}: a \`md\` block that no check runs: check it, or write skip="<why>" in its fence`,
    ]);
    expect(ran).toEqual(["[server]"]);
  });

  const block = (lang: string, text: string, attrs: [string, string][] = []) => ({ key: "guides/code.mdx", line: 6, lang, attrs: new Map(attrs), text });

  test("a json block parses whole, and a fragment parses as the members of an object", () => {
    expect(jsonProblems(block("json", `{"a": [1, 2]}`))).toEqual([]);
    expect(jsonProblems(block("json", `"a": 1`))[0]).toStartWith("not JSON: ");
    expect(jsonProblems(block("json", `"a": 1`, [["fragment", ""]]))).toEqual([]);
  });

  test("a yaml block parses, and a broken one is named", () => {
    expect(yamlProblems(block("yaml", "services:\n  app:\n    image: novis\n"))).toEqual([]);
    expect(yamlProblems(block("yaml", "a: [1, 2\n"))[0]).toStartWith("not YAML: ");
  });

  const NVS = binary(undefined);
  test.skipIf(!NVS)("a toml block is read by nvs config check, and a file it names that is not on this machine gets a stub", async () => {
    const ctx = { root: ROOT, nvs: NVS };
    const named = `[secrets]\napi_key_file = "/run/secrets/api_key"\n\n[[app]]\nentry = "jobs/nightly.nvs"\n\n[app.limits]\nmemory = "64M"`;
    expect(await tomlProblems(block("toml", named), ctx)).toEqual([]);
    const [wrong] = await tomlProblems(block("toml", `[server]\nbogus = 1`), ctx);
    expect(wrong).toContain("`nvs config check` fails:");
    expect(wrong).toContain("unknown field `bogus`");
    expect(await tomlProblems(block("toml", `memory = "64M"`, [["fragment", "limits"]]), ctx)).toEqual([]);
    expect(await tomlProblems(block("toml", `memory = "64M"`, [["fragment", ""]]), ctx)).toEqual(['`fragment` with no table: write fragment="<table>", the header the block sits under']);
  });

  test("a shell block's commands join continuations, split on && and |, and drop redirects, settings and sudo", () => {
    const words = (lang: string, text: string) => shellCommands(block(lang, text)).map((c) => [c.line, c.words.join(" ")]);
    expect(words("sh", `# set up\nNO_COLOR=1 sudo nvs serve \\\n  --port 8000\ncd app && nvs build --openapi shop.nvs > out.json 2>&1 | less`)).toEqual([[2, "nvs serve --port 8000"], [4, "cd app"], [4, "nvs build --openapi shop.nvs"], [4, "less"]]);
    expect(words("sh", `$ nvs test\nok 3 tests\n$ nvs run "a b.nvs"`)).toEqual([[1, "nvs test"], [3, "nvs run a b.nvs"]]);
    expect(words("bat", `REM a comment\nnvs.exe serve ^\n  app.nvs`)).toEqual([[2, "nvs.exe serve app.nvs"]]);
  });

  test.skipIf(!NVS)("a shell block's nvs lines are parsed by nvs, a bun nv line names a command, and any other program is named", async () => {
    const ctx = { root: ROOT, nvs: NVS };
    expect(await shellProblems(block("sh", "cd app\nnvs serve app.nvs --port 8000\nbun nv bench-compare"), ctx)).toEqual([]);
    const [bad] = await shellProblems(block("sh", "nvs serve --no-such-flag"), ctx);
    expect(bad).toStartWith("line 1: `nvs` does not accept `nvs serve --no-such-flag`:");
    expect(await shellProblems(block("sh", "bun nv no-such-command"), ctx)).toEqual(["line 1: `bun nv` has no command `no-such-command`"]);
    expect(await shellProblems(block("sh", "git clone x\ncurl y\ngit pull"), ctx)).toEqual(['`git`, `curl` are not `nvs` or `bun nv`, so no check runs them: write skip="<why>" in the fence']);
  });
});

describe("nv site --check apps: an app's copies", () => {
  const bytes = (text: string) => new TextEncoder().encode(text);

  test("the files in an app's directory are its copies, and a nested directory is not one", () => {
    site();
    s.put(`${APP_FILES}/shop/report.pdf`, "%PDF-2");
    s.put(`${APP_FILES}/shop/invoice.pdf`, "%PDF-1");
    s.put(`${APP_FILES}/shop/old/x.pdf`, "%PDF-0");
    expect(appFiles("shop", s.root)).toEqual(["invoice.pdf", "report.pdf"]);
    expect(appFiles("blog", s.root)).toEqual([]);
  });

  test("a copy matches byte for byte, and other bytes or a status other than 200 are named", () => {
    expect(appFileProblem("shop", "a.pdf", 200, bytes("%PDF"), bytes("%PDF"))).toBeNull();
    expect(appFileProblem("shop", "a.pdf", 200, bytes("%PDF2"), bytes("%PDF"))).toBe(`\`${APP_FILES}/shop/a.pdf\` is not what \`GET /a.pdf\` returns now: run \`bun nv site --bless-apps\``);
    expect(appFileProblem("shop", "a.pdf", 404, bytes(""), bytes("%PDF"))).toBe(`\`${APP_FILES}/shop/a.pdf\`: \`GET /a.pdf\` returned 404`);
    expect(appFileProblem("shop", "a.pdf", 0, bytes(""), bytes("%PDF"))).toBe(`\`${APP_FILES}/shop/a.pdf\`: \`GET /a.pdf\` returned no response`);
  });

  test("a relative --nvs becomes an absolute path, so an app's own directory does not change it", () => {
    site();
    s.put("bin/nvs.exe", "");
    const relativePath = relative(process.cwd(), join(s.root, "bin/nvs.exe"));
    expect(binary(relativePath)).toBe(join(s.root, "bin/nvs.exe"));
    expect(binary("no/such/nvs.exe")).toBeNull();
  });
});
