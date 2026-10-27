// `bun nv site`: the website's stale guard, its checks and its build.
//
//     bun nv site --stale                  every broken id, stale page and uncovered feature
//     bun nv site --stamp PAGE...          record what each page's covered features are now, after the
//                                          page was reread against them
//     bun nv site --check [PART...]        run the named parts, or all of them: `site <part>: ok` per
//                                          part that passes, its problems and exit 1 for one that fails
//     bun nv site --bless SNIPPET...       write each snippet's `.out`, `.http.out` or `.err` from what the
//                                          binary does, and its `.tainted` from what `nvs lsp` reports,
//                                          show them, and delete a file it no longer needs; a `.term`'s
//                                          `.term.out` from what its commands print
//     bun nv site --bless-apps [APP/FILE...]
//                                          write each example app's copies under `website/public/
//                                          example-apps/` from what its server returns now, and add each
//                                          named `APP/FILE`
//     bun nv site --shoot [SHOT...]        take each named `.shot`'s screenshot, or every one, and write
//                                          its picture and its `.shot.out`
//     bun nv site --build                  the Astro build, and `site: built` when it succeeds
//     bun nv site --roadmap                write README.md's roadmap list from website/src/data/roadmap.json
//     bun nv site --roadmap --retired      drop the retired goals from roadmap.json, then write and commit
//     bun nv site --questions --status     `questions <area>: <n> owed, <n> stale` per area, exit 0
//     bun nv site --questions --owed [AREA] [--limit N]
//                                          each owed or stale page, with what its questions are written
//                                          from
//     bun nv site --questions --stamp URL...
//                                          record each entry's feature and hash, after its questions were
//                                          written or reread
//
// `--nvs PATH` names the binary that reads the roster and runs the snippets and the apps. A relative path
// is resolved against the directory `nv` runs in, before any command runs in another one. Without it,
// the newest built `nvs` is used (`lib/nvs-binary.ts`).
//
// **An app's copies** are the files under `website/public/example-apps/<app>/`, which its page shows and
// links. Each is what the app's server returns for `GET /<file name>`. The files on disk are the list,
// so no second list names them: `--bless-apps paperwork/invoice.pdf` adds one, and deleting a file drops
// it. `--check apps` fails on a copy whose bytes differ or whose `GET` does not return a `200`.
//
// **A snippet** is a `.nvs` file under `website/snippets/` that a page shows with `<Snippet src="..."/>`,
// `src` first. Beside it is a `.err`, or one or both of `.out` and `.http.out`, and each is compared
// with the real binary after CRLF becomes LF and trailing white space is dropped:
//
// - A `.err` is the diagnostic for a snippet that must not compile: `nvs check <name>.nvs`, run in the
//   snippet's own directory with `NO_COLOR=1`, must exit 1 and write that file to standard error. The
//   bare file name keeps each diagnostic's path the same wherever the check runs, and `nvs check` runs
//   nothing, so a `.err` snippet that compiles by mistake has no effect.
// - A `.out` is what `nvs run` prints, run from the repository root with the directory's `nvs.toml` as
//   `--config` when there is one.
// - A `.http.out` is the response body a real server sends: `nvs serve <snippet> --port 0` from the
//   root, with the same `--config`, is sent one request on the port its `listening on` line names, and
//   is stopped. The request is the snippet's `.nvsr` (method, path, query, headers and body; a section a
//   real connection cannot carry is an error), or `PLAIN_GET` when it has none. With no `[server]`
//   block every path runs the snippet. Status and headers are not compared, because the body is what a
//   reader compares with the code. The root `nvs.toml` arms a queue on `tests/db/compose.yaml`'s
//   `postgres`, so before the first snippet served under it that service is brought up and
//   `nvs queue migrate` is run, once per process. Where Docker runs no Linux containers, does not
//   answer or is not there, that server is not started, and the proofs' one rule on every platform
//   (`dockerAbsent`) makes it a skip with the reason: a web snippet is skipped whole, and a
//   command-line snippet keeps its `.out` check and skips its `.http.out`.
//
// Which output files a snippet that compiles has is the divergence rule, `snippetOutputs`. A snippet
// with a `.nvsr` is a web snippet: it has a `.http.out`, and a `.out` too only when `nvs run` exits 0
// and prints something else. A snippet without one is a command-line snippet: `nvs run` must exit 0
// and it has a `.out`, and a `.http.out` too only when the server answers 2xx with something else. So
// two different outputs are always both shown, and two equal ones are one file. `<Snippet
// output="http">` shows only the `.http.out`, and the front page, `FRONT_PAGE`, uses it for every
// snippet that has one. Each snippet's two runs follow each other, and `SNIPPET_WIDTH` snippets run at
// once. `--bless` writes the `.err` of a snippet that has one, or that has no file and does not
// compile, and otherwise writes the output files the rule owes and deletes the one it does not.
//
// A `.tainted` beside a snippet lists each token `nvs lsp` gives the `tainted` semantic-token modifier,
// one `taintedRanges` line each, and a snippet has one only when there is at least one such token.
// `Snippet.astro` gives each range a dotted underline and a tooltip, through the Expressive Code plugin
// in `website/config/tainted-ranges.mjs`. The grammar that colours a snippet cannot know which values
// are tainted, so the ranges come from the server, one session per snippet with the snippet opened at
// its own path. `--bless` writes or deletes the file, and `--check snippets` names one that is missing,
// stale or no longer needed.
//
// A `.replay` beside a snippet is an editor session that ends on the snippet's text, and a page shows it
// with `<Replay src="..."/>`, `src` naming the snippet. It is hand-written, in two sections:
//
//     --START--
//     <?nvs
//     property<User> $key = '<|>' as property<User>;
//     --STEPS--
//     type na
//     accept name
//
// `--START--` is the text the session opens on, with `<|>` once where the cursor starts. `--STEPS--` has
// one step per line, run in order from the cursor:
//
// - `type TEXT` types the rest of the line one character at a time. `\n` in it is a line break and `\\`
//   a backslash.
// - `accept LABEL` accepts the selected row of the open list. LABEL is the row the page's narrowing
//   selects, so a list where it does not is a failure. The row's insert text, or its label when it has
//   none, replaces what was typed since the list opened, and the list closes.
// - `hover` shows the diagnostic under the cursor, from what the server last published. No diagnostic
//   there is a failure.
// - `card` shows the hover card at the cursor. No card there is a failure.
// - `rename NAME` renames the name under the cursor to NAME, and makes the edits the server returned
//   for the open file. A refused rename is a failure.
// - `action TITLE` asks for the code actions at the cursor and makes the edits of the one with that
//   title. No action with that title is a failure.
// - `lens` shows the open file's code lenses. A file with none is a failure.
// - `calls` shows the call hierarchy of the method at the cursor: its callers and the methods it calls.
//
// Every step but `type`, `accept` and `hover` closes the open list. A `rename` or an `action` changes the
// text, so the snippet is the text after its edits.
//
// A list is asked for where an editor asks for one: after a trigger character the server's `initialize`
// result names, and after a letter typed where the character before it cannot be part of a name. It
// stays open while letters, digits and `_` are typed, and closes on any other character or an `accept`.
// Nothing records the narrowing as letters are typed, because it is the editor's work and not the
// server's, so the page imitates it.
//
// The `.replay.out` beside it is what `nvs lsp` returned, one block per answer, in step order. The
// session is one stream, as the `.tainted` one is: the start text opened at the snippet's own path, and
// a change and a request at each point above. A block opens on a header line, positions count from 1,
// and the replay above records one list:
//
//     completion 1 2:25
//     age	property	int
//     name	property	string
//
// `completion STEP L:C` is a list asked for in step STEP with the cursor at L:C. Its rows are in the
// server's order, one per line, five columns joined by one tab each: the label, the kind, the text at
// the row's right, the text written straight after the label, and the insert text. Each is what a
// `tests/lsp/` case reads from the item, and the insert text is empty when it is the label. Empty
// columns at the end of a row are left off. `diagnostic STEP L:C+LENGTH CODE` is the diagnostic a
// `hover` shows, its range on one line, and the message follows it, each line indented two spaces. The
// other steps' blocks:
//
// - `card STEP L:C`, and the card's Markdown, each line indented two spaces and a blank line left empty.
// - `rename STEP L:C OTHERS`, OTHERS the number of other files the rename changed, and an `edit` line
//   per edit in the open file.
// - `action STEP L:C`, each action's title indented two spaces in the server's order, and an `edit` line
//   per edit the chosen action makes.
// - `lens STEP`, and per lens `L:C TITLE` indented two spaces, L:C where its range starts.
// - `calls STEP L:C NAME`, NAME the method, then `in CALLER COUNT` per caller and `out CALLEE COUNT` per
//   method it calls, indented two spaces, COUNT the number of call sites.
//
// `edit L1:C1-L2:C2 "TEXT"` replaces that range of the text before the step with TEXT, a JSON string. A
// blank line separates two blocks. `--bless` writes the file from a new session.
// `--check snippets` fails on a `.replay` whose steps do not end on the snippet's text, a `.replay.out`
// that is missing or is not what the server returns now, and a `.replay.out` with no `.replay`. A page
// that shows a replay shows its snippet too, because the replay ends on the snippet's text.
//
// A `.dap` beside a snippet is a debug session over it, and `<Replay src="..."/>` shows it as a debugger
// replay when the snippet has no `.replay`. It is hand-written: `break LINE` once per breakpoint, lines
// counted from 1, and then one step per line, each a request sent once the program has stopped:
//
//     break 9
//     in
//     next
//     out
//
// `continue` runs to the next breakpoint, `next` steps over a line, `in` steps into a call and `out`
// steps out of a method. `nvs dap` runs the snippet from the repository root, with the directory's
// `nvs.toml` as `--config` when there is one, and the session starts the program and waits for its
// first stop (`lib/dap-replay.ts`). The `.dap.out` beside it is one block per stop, a blank line
// between two:
//
//     stop 1 step
//       frame main.nvs:4 Shop::total
//       frame main.nvs:9 main
//       var $price	int	4
//
// `stop STEP REASON` is where the program stopped after step STEP, 0 for the start, and REASON is the
// `stopped` event's. A `frame FILE:LINE NAME` line follows per frame of the call stack, innermost
// first, FILE the file's name alone, and then a `var` line per variable of the innermost frame's first
// scope: its name, its type and its value, joined by one tab each, with a line break in a value written
// `\n`. `end STEP EXIT` is the program's end with its exit code, and a step after it is a failure. A
// frame id and a variables reference only name an object for one session, so neither is written.
// `--bless` writes the file from a new session. `--check snippets` fails on a `.dap` that does not
// parse or run, a `.dap.out` that is missing or is not what `nvs dap` returns now, a `.dap.out` with no
// `.dap`, and a snippet with both a `.dap` and a `.replay`.
//
// **A terminal block** is a `.term` under `website/snippets/` that a page shows with `<Terminal
// src="tools/cli/check.term"/>`. It lists commands, one per non-blank line, each starting with `nvs`; a
// word is a run without white space, or a run inside `"` or `'`. Its fixture is the directory beside it
// with its name and no extension (`tools/cli/check/`), and a `.term` with no such directory runs in an
// empty one. A `.nvs` inside a fixture is what the commands run on, and is not a snippet. Each check and
// each bless copies the fixture under `.cache/nv-term/`, runs every command in that copy in order with
// `NO_COLOR=1`, and deletes it (`lib/term.ts`). The `.term.out` beside the `.term` is one block per
// command, a blank line between two:
//
//     $ nvs check main.nvs
//     main.nvs:3:5: error: ...
//     exit 1
//
// The block is `$ ` and the command's line, what it printed to standard output and then to standard
// error, and `exit` with its exit code. The copy's own path is written as `~/project` and the machine's
// temporary directory as `/tmp`, each with `/` after it, because the real paths name the machine. The
// file has every other value as `nvs` printed it on the blessed run. The comparison masks the values
// that differ from run to run on both sides: a path under `/tmp`, a date, a time, a 16 or 32 digit hex
// id, a port and a duration (`MASKS` in `lib/term.ts`). `--bless` writes the file from a new run, and
// `--check snippets` fails on a `.term` whose commands do not parse or end, a `.term.out` that is
// missing or that a new run does not match, a `.term.out` with no `.term`, and a `.term` no page
// shows. `Terminal.astro` shows the block, and its copy button copies the commands alone.
//
// **A screenshot** is a `.shot` under `website/snippets/`, and only `--shoot` takes its picture
// (`rule:testing/a-screenshot-is-taken-from-a-real-run`). It is hand-written, one step per line. A
// viewer shot names an app under `apps/`, the requests sent to it, and the steps taken in the viewer:
//
//     app notes
//     request GET /api/notes
//     request POST /notes title=First
//     open /
//     wait .requests .route
//     click .requests .route
//     type input[type=search] home
//
// `request METHOD PATH [BODY]` sends BODY as a form. The first step is `open PATH`, a path of the
// viewer; `wait`, `click` and `type` take a CSS selector, and `type` the text after it. `app .` serves
// the directory beside the `.shot` with its name instead, for a project no app under `apps/` is, such as
// one whose entry file throws an error no gate catches. The app is copied without its database, served in development with `[debug] store` on, so the server starts
// `nvs debug`, and the steps are taken once the viewer lists every request (`lib/shoot.ts`). A profile
// shot is one line, `profile main.nvs`: the program in the directory beside the `.shot` with its name
// and no extension, run with `--profile=speedscope` in a copy of it and opened in the speedscope build
// the VS Code extension bundles. A `.nvs` in that directory is what the shot runs, and is not a
// snippet. Every shot is taken in a 1280 by 800 window.
//
// The picture is the `.png` beside the `.shot` with its name. The `.shot.out` is `assets HASH`, a blank
// line and what the picture shows as text. HASH is over the files the page draws with: `viewer/dist/`
// or the speedscope release. A viewer shot's text is the page's visible text with `lib/term.ts`'s masks,
// a spaced duration, an age and the trace id digits in a made test's class name masked too, and each
// path inside the run's copy written from the project's root with `/`. A profile shot's text is one `timeline NAME` block per
// timeline of the export, with each frame and its number of calls and no time. `--check shots` takes
// every shot again and fails on a `.shot.out` that is missing or differs, a missing picture, a
// `.shot.out` or `.png` with no `.shot`, a `.shot` no page shows, and a `<Shot src="..."/>` that names no
// `.shot`. `Shot.astro` shows the picture. The pixels are never compared. With no Chromium installed the
// part prints a `skip` line, and with `NV_BROWSER` set that is a failure, as the browser suites do.
//
// **A handwritten page** is every page under `website/src/content/docs/` that no renderer writes, which
// is every page outside `reference/core/`. Its front matter carries `covers:`, a list of the proofs
// roster's feature ids (`Core\Str::length`, `lang:statements/match`, `directive:server.dispatch`), and
// `covers: []` on a page that explains no one feature. A page is named by its path under that directory
// (`install/hello-world.mdx`).
//
// **The lock**, `website/site.lock.json`, records per page a hash of each feature it covers: the
// feature's card in `nvs meta --json` or its section in its reference chapter, its `about.md`, and every
// file in its example directory. A language feature has no card: `bun nv render` writes its chapter
// section from the files in its directory. A covered id the roster no longer has is **broken**. A covered feature
// whose hash differs from the page's stamp, or that the page never stamped, makes the page **stale**.
// A language or type feature no page under `syntax/` covers is **uncovered**. `--stamp` is what clears
// a stale page, and it is run by whoever reread the page, never to quiet the check.
//
// **The questions**, `website/src/data/questions.json`, are four to six search questions per built page,
// keyed by its URL without the site's base, in the words a visitor types before they know a name.
// `Banner.astro` writes them into an attribute Pagefind indexes. Each entry is `{ feature, hash,
// questions }`, and `questionSources` says what each page's feature and hash are now: a Core member or
// CLI page is its roster id and `featureHash`, a Core class page its card and members' names, a
// configuration block page every feature it shows, and a handwritten page its key and its file. An
// entry whose feature or hash moved, or that was never stamped, is **stale**. An entry no built page
// has is **broken**. A built page with no entry is **owed**, and `--check questions` fails on all
// three, so a new feature's page fails CI until it has questions. The same check holds each entry to
// four to six questions of at most 15 words, none using a word from AGENTS.md § *Text an end user
// reads* (`QUESTION_WORDS`), and no question two entries share. A session writes them from what
// `--questions --owed` prints, and no tool or build calls a language-model service for them. They name
// no function from another language (`strlen`), and each belongs to the page that answers it best, so
// two close pages (`trim` and `trimStart`) split the phrasing. They live in an indexed attribute, never
// in hidden page text. `/in-depth/questions/` (`website/src/components/QuestionIndex.astro`) lists them
// all for a reader, outside the search index. The questions are not a feature proof.
//
// The parts of `--check`: `structure` (the nav is the five areas from `website/config/site.mjs`, then
// Install and Sponsoring, each area is one sidebar group, no retired page or generator is on disk, the
// footer has no prev/next, the Astro config has no redirect, no page or sidebar entry links to a URL
// `builtUrls` does not list, no style rule pairs `animation` with `animation-timeline`, which the
// minified build drops, and no animation loops outside a region that pauses it off screen:
// `lib/site-css.ts`), `snippets` (no handwritten page has an
// inline Novis fence, every `<Snippet src="..."/>` names a file under `website/snippets/`, every snippet
// is used, keeps the comment bounds of `bun nv proofs --comments`, and has the files § *A snippet*
// names, each matching), `stale` (no page lacks `covers:`, no id is broken and no page is stale), `questions` (see `questionProblems`),`reference` (see
// `referenceProblems`), `syntax` (see `syntaxProblems`), `guides` (see `guidesProblems`), `tools` (see
// `toolsProblems`), `in-depth`
// (see `inDepthProblems`), `apps` (see `appsProblems`), `shots` (§ *A screenshot*), `roadmap` (the
// roadmap is in step with the goal chain and the README: `lib/site-roadmap.ts`) and `prose` (the
// countable bounds of AGENTS.md
// § *Text an end user reads* over every handwritten page's prose: no sentence over 25 words, no dash
// joining two sentences, and no paragraph over six sentences). The legal pages are kept out of `prose`,
// since their wording is the law's.

import { createHash } from "node:crypto";
import { existsSync, mkdirSync, readdirSync, readFileSync, rmSync, statSync, writeFileSync } from "node:fs";
import { request as httpRequest } from "node:http";
import { createServer } from "node:net";
import { availableParallelism } from "node:os";
import { dirname, join, resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { newestNvs } from "../lib/nvs-binary.ts";
import { lspFrame, lspMessages } from "../lib/lsp.ts";
import { rel, ROOT } from "../lib/paths.ts";
import { parseDap, recordDap } from "../lib/dap-replay.ts";
import { parseReplay, recordReplay } from "../lib/replay.ts";
import { recordTerm, sameTerm, TERM, TERM_OUT } from "../lib/term.ts";
import { canShoot, recordShot, SHOT, SHOT_OUT, SHOT_PNG, shotFixture, shotPng } from "../lib/shoot.ts";
import { run as runProc } from "../lib/proc.ts";
import { comparePaths } from "../lib/py.ts";
import { styleProblems } from "../lib/site-css.ts";
import { diagramProblems } from "../lib/site-diagrams.ts";
import { pruneRetired, roadmapProblems, writeRoadmap } from "../lib/site-roadmap.ts";
import { commentProblems } from "../proofs/collect.ts";
import { dockerAbsent, servicesFor } from "../proofs/run.ts";
import { ABOUT, EXAMPLES, examplesDir, metaJson, roster, RosterError, type Entry, type Meta } from "../proofs/roster.ts";
import { DATA_FILE, renderWebsiteCore } from "../renderers/website-core.ts";
import { REFERENCE_FILE, type Page as RefPage } from "../renderers/website-reference.ts";

export const summary = "the website's stale guard, checks and build: nv site --stale | --stamp PAGE... | --check [PART...] | --bless SNIPPET... | --bless-apps [APP/FILE...] | --shoot [SHOT...] | --build | --roadmap | --questions --status | --owed [AREA] | --stamp URL...";

export const DOCS = "website/src/content/docs";
export const SNIPPETS = "website/snippets";
export const LOCK = "website/site.lock.json";
/** The page directories a renderer writes, under `DOCS`. */
const GENERATED = ["reference/core/"];
/** Pages whose wording is fixed by law, and which `prose` does not judge. */
const LEGAL = ["impressum.mdx", "datenschutz.mdx"];

export const PARTS = ["structure", "snippets", "stale", "questions", "reference", "syntax", "guides", "tools", "in-depth", "apps", "shots", "roadmap", "prose"] as const;
type Part = (typeof PARTS)[number];
/** Where `--build` writes the site, which `reference` reads. */
const DIST = "website/dist";

const SENTENCE_WORDS = 25;
const PARAGRAPH_SENTENCES = 6;

const readAt = (root: string, path: string) => {
  try {
    return readFileSync(join(root, path), "utf8").replace(/\r\n?/g, "\n");
  } catch {
    return "";
  }
};

/** Every file under a repo-relative directory, repo-relative and sorted. */
function filesUnder(root: string, dir: string): string[] {
  const out: string[] = [];
  const walk = (d: string) => {
    if (!existsSync(join(root, d))) return;
    for (const name of readdirSync(join(root, d))) {
      const p = `${d}/${name}`;
      if (statSync(join(root, p)).isDirectory()) walk(p);
      else out.push(p);
    }
  };
  walk(dir);
  return out.sort(comparePaths);
}

export interface Page {
  /** The path under `DOCS`. */
  key: string;
  text: string;
  /** The text after the front matter. */
  body: string;
  /** `covers:`, or null when the front matter has none. */
  covers: string[] | null;
}

function frontMatter(text: string): { data: Record<string, unknown>; body: string } {
  const m = /^---\n([\s\S]*?)\n---\n?/.exec(text);
  if (!m) return { data: {}, body: text };
  let data: unknown;
  try {
    data = Bun.YAML.parse(m[1]!);
  } catch {
    data = {};
  }
  return { data: typeof data === "object" && data !== null ? (data as Record<string, unknown>) : {}, body: text.slice(m[0].length) };
}

/** Every handwritten page. */
export function pages(root: string = ROOT): Page[] {
  return filesUnder(root, DOCS)
    .filter((p) => /\.mdx?$/.test(p))
    .map((p) => p.slice(DOCS.length + 1))
    .filter((key) => !GENERATED.some((g) => key.startsWith(g)))
    .map((key) => {
      const text = readAt(root, `${DOCS}/${key}`);
      const { data, body } = frontMatter(text);
      const covers = Array.isArray(data.covers) ? data.covers.map(String) : null;
      return { key, text, body, covers };
    });
}

/** The section of a reference chapter a language or tool feature is, from its `# ` heading to the next. */
function section(root: string, anchor: string): string {
  const at = anchor.lastIndexOf(":");
  const lines = readAt(root, anchor.slice(0, at)).split("\n");
  const out: string[] = [];
  let fence = "";
  for (const line of lines.slice(Number(anchor.slice(at + 1)) - 1)) {
    const marker = line.trim().slice(0, 3);
    if (fence) {
      if (marker === fence) fence = "";
    } else if (marker === "```" || marker === "~~~") {
      fence = marker;
    } else if (out.length > 0 && line.startsWith("# ")) {
      break;
    }
    out.push(line);
  }
  return out.join("\n");
}

const named = (items: unknown[] | undefined, key: string, name: string) =>
  (items ?? []).find((x) => (typeof x === "object" && x !== null ? String((x as Record<string, unknown>)[key]) : String(x)) === name);

/** What the binary's help says of a feature: its card in `nvs meta --json`, or its chapter section.
 * A language feature's is empty, because `bun nv render` writes its section from its own files. */
export function card(root: string, entry: Entry, meta: Meta): string {
  let found: unknown;
  switch (entry.kind) {
    case "member": {
      const member = entry.id.slice(entry.id.lastIndexOf("::") + 2);
      found = meta.classes?.find((c) => c.name === entry.group)?.members?.find((m) => m.name === member);
      break;
    }
    case "exception":
      found = named(meta.exceptions, "name", entry.id);
      break;
    case "enum":
      found = named(meta.enums, "name", entry.id);
      break;
    case "interface":
      found = named(meta.interfaces, "name", entry.id);
      break;
    case "directive":
      found = named(meta.directives, "key", entry.id.slice("directive:".length));
      break;
    case "lang":
      // Its chapter section is generated from the files `featureHash` already hashes.
      return "";
    default:
      return entry.anchor ? section(root, entry.anchor) : "";
  }
  return JSON.stringify(found ?? null);
}

/** The hash a page's stamp records for a feature: its card, its `about.md`, and every other file in its
 * directory, which is its examples, their outputs and a language feature's `details.md`. */
export function featureHash(root: string, entry: Entry, meta: Meta): string {
  const h = createHash("sha256");
  h.update(card(root, entry, meta));
  const dir = examplesDir(entry);
  h.update(`\0${readAt(root, `${dir}/${ABOUT}`)}`);
  for (const p of filesUnder(root, dir)) if (!p.endsWith(`/${ABOUT}`)) h.update(`\0${p}\0${readAt(root, p)}`);
  return h.digest("hex").slice(0, 16);
}

export type Lock = Record<string, Record<string, string>>;

export function readLock(root: string = ROOT): Lock {
  const text = readAt(root, LOCK);
  return text ? (JSON.parse(text) as Lock) : {};
}

function writeLock(root: string, lock: Lock): void {
  const sorted: Lock = {};
  for (const page of Object.keys(lock).sort(comparePaths)) {
    if (Object.keys(lock[page]!).length === 0) continue;
    sorted[page] = Object.fromEntries(Object.entries(lock[page]!).sort(([a], [b]) => comparePaths(a, b)));
  }
  writeFileSync(join(root, LOCK), `${JSON.stringify(sorted, null, 2)}\n`);
}

/** What the roster and `nvs meta --json` say ships, which every verdict here is judged against. */
export interface World {
  entries: Entry[];
  meta: Meta;
}

export interface Staleness {
  /** Pages with no `covers:` line. */
  unlisted: string[];
  /** `page: id` for a covered id the roster does not have. */
  broken: string[];
  /** `page: id (why)` for a covered feature changed since the page's stamp, or never stamped. */
  stale: string[];
  /** Language and type features no Syntax page covers. */
  uncovered: string[];
}

const isSyntaxFeature = (e: Entry) => e.kind === "lang" || e.path.startsWith("types/");

export function staleness(world: World, root: string = ROOT): Staleness {
  const byId = new Map(world.entries.map((e) => [e.id, e]));
  const lock = readLock(root);
  const out: Staleness = { unlisted: [], broken: [], stale: [], uncovered: [] };
  const syntaxCovers = new Set<string>();
  for (const page of pages(root)) {
    if (page.covers === null) {
      out.unlisted.push(page.key);
      continue;
    }
    for (const id of page.covers) {
      if (page.key.startsWith("syntax/")) syntaxCovers.add(id);
      const entry = byId.get(id);
      if (!entry) {
        out.broken.push(`${page.key}: ${id}`);
        continue;
      }
      const stamped = lock[page.key]?.[id];
      const why = stamped === undefined ? "never stamped" : stamped !== featureHash(root, entry, world.meta) ? "changed since its stamp" : "";
      if (!why) continue;
      out.stale.push(`${page.key}: ${id} (${why})`);
    }
  }
  out.uncovered = world.entries.filter((e) => isSyntaxFeature(e) && !syntaxCovers.has(e.id)).map((e) => e.id);
  return out;
}

/** The page key a `--stamp` argument names: a key, or a path under `DOCS` from the repository root. */
const pageKey = (arg: string) => arg.replaceAll("\\", "/").replace(new RegExp(`^(?:\\./)?${DOCS}/`), "");

/** Records the current hash of every feature each named page covers. Returns what stopped it, if anything. */
export function stamp(world: World, keys: string[], root: string = ROOT): string[] {
  const all = new Map(pages(root).map((p) => [p.key, p]));
  const byId = new Map(world.entries.map((e) => [e.id, e]));
  const lock = readLock(root);
  const problems: string[] = [];
  for (const key of keys.map(pageKey)) {
    const page = all.get(key);
    if (!page) problems.push(`${key}: no handwritten page has this name`);
    else if (page.covers === null) problems.push(`${key}: has no \`covers:\` line`);
    else {
      const missing = page.covers.filter((id) => !byId.has(id));
      if (missing.length) problems.push(`${key}: covers ${missing.join(", ")}, which the roster does not have`);
      else lock[key] = Object.fromEntries(page.covers.map((id) => [id, featureHash(root, byId.get(id)!, world.meta)]));
    }
  }
  if (problems.length === 0) writeLock(root, lock);
  return problems;
}

/** Every page's search questions, keyed by the page's URL without the site's base. */
export const QUESTIONS = "website/src/data/questions.json";

/** One page's search questions, and the feature and hash they were last stamped against. */
export interface QuestionEntry {
  feature: string;
  hash: string;
  questions: string[];
}

export type Questions = Record<string, QuestionEntry>;

/** The areas `--questions --status` counts. */
export const QUESTION_AREAS = ["core", "config", "cli", "types", "pages"] as const;
export type QuestionArea = (typeof QUESTION_AREAS)[number];

/** What one built page's questions are written from: its feature, and that feature's hash now. */
export interface QuestionSource {
  url: string;
  area: QuestionArea;
  feature: string;
  hash: string;
  /** The roster features the page shows, which its questions are written from. Empty for a class page
   * and a handwritten page. */
  ids: string[];
}

const digest = (parts: string[]) => {
  const h = createHash("sha256");
  for (const p of parts) h.update(`\0${p}`);
  return h.digest("hex").slice(0, 16);
};

/** A handwritten page's URL from its key: no extension, no trailing `index`, lower case. */
export const pageUrl = (key: string) => `/${key.toLowerCase().replace(/\.mdx?$/, "").replace(/(^|\/)index$/, "")}/`.replace(/\/{2,}/g, "/");

/** The questions on disk. An entry that is a bare list of questions has no feature and no hash yet. */
export function readQuestions(root: string = ROOT): Questions {
  const text = readAt(root, QUESTIONS);
  const raw = text ? (JSON.parse(text) as Record<string, QuestionEntry | string[]>) : {};
  return Object.fromEntries(Object.entries(raw).map(([url, e]) => [url, Array.isArray(e) ? { feature: "", hash: "", questions: e } : e]));
}

function writeQuestions(root: string, questions: Questions): void {
  const sorted = Object.fromEntries(Object.keys(questions).sort(comparePaths).map((url) => [url, questions[url]!]));
  writeFileSync(join(root, QUESTIONS), `${JSON.stringify(sorted, null, 2)}\n`);
}

/**
 * Every built page whose questions can be stamped, and what each is written from. A Core member page,
 * and a CLI page, is its roster feature and `featureHash`. A Core class page is the class, hashed over
 * its card and its members' names. A configuration block page is its first feature, hashed over every
 * feature it shows. A handwritten page is its key under `DOCS`, hashed over the file.
 */
export function questionSources(world: World, root: string = ROOT): QuestionSource[] {
  const byId = new Map(world.entries.map((e) => [e.id, e]));
  const hashOf = (id: string) => {
    const e = byId.get(id);
    return e ? featureHash(root, e, world.meta) : "";
  };
  const out: QuestionSource[] = [];
  const core = JSON.parse(readAt(root, DATA_FILE) || "{}") as { classes?: { name: string; url: string; members: { name: string; url: string }[] }[] };
  for (const cls of core.classes ?? []) {
    const known = world.meta.classes?.find((c) => c.name === cls.name);
    out.push({ url: cls.url, area: "core", feature: cls.name, hash: digest([JSON.stringify(known?.doc ?? null), ...(known?.members ?? []).map((m) => m.name)]), ids: [] });
    for (const m of cls.members) {
      const id = `${cls.name}::${m.name}`;
      if (byId.has(id)) out.push({ url: m.url, area: "core", feature: id, hash: hashOf(id), ids: [id] });
    }
  }
  const reference = JSON.parse(readAt(root, REFERENCE_FILE) || "{}") as { pages?: RefPage[] };
  for (const page of reference.pages ?? []) {
    const ids = [...(page.intro ? [page.intro.id] : []), ...page.keys.map((k) => k.id)];
    if (ids.length === 0) continue;
    const hash = page.area === "cli" && ids.length === 1 ? hashOf(ids[0]!) : digest(ids.flatMap((id) => [id, hashOf(id)]));
    out.push({ url: page.url, area: page.area, feature: ids[0]!, hash, ids });
  }
  for (const page of pages(root)) out.push({ url: pageUrl(page.key), area: "pages", feature: page.key, hash: digest([page.text]), ids: [] });
  return out;
}

export interface QuestionStatus {
  /** An entry whose feature or hash differs from its page's now, or that was never stamped. */
  stale: QuestionSource[];
  /** An entry's URL that no built page has. */
  broken: string[];
  /** A built page with no entry. */
  owed: QuestionSource[];
}

export function questionStatus(world: World, root: string = ROOT): QuestionStatus {
  const questions = readQuestions(root);
  const sources = questionSources(world, root);
  const urls = new Set(sources.map((s) => s.url));
  return {
    stale: sources.filter((s) => questions[s.url] && (questions[s.url]!.feature !== s.feature || questions[s.url]!.hash !== s.hash)),
    broken: Object.keys(questions).filter((url) => !urls.has(url)),
    owed: sources.filter((s) => !questions[s.url]),
  };
}

/**
 * The words AGENTS.md § *Text an end user reads* keeps out of text a user reads, matched whole. `name`
 * and `mark` are not here: a visitor's question uses them in their plain sense ("a file's name").
 */
const QUESTION_WORDS = ["spelling", "answers", "hands back", "hands you", "asks", "asking", "stands in for", "lets go of", "drops", "refuses", "refused", "refusal", "member", "binding", "holds", "shard", "tier", "slot", "refcount", "lowering", "the registry"];

/** How one entry's questions break the bounds: four to six, at most 15 words each, no word from `QUESTION_WORDS`. */
function questionShape(url: string, questions: string[]): string[] {
  const out: string[] = [];
  if (questions.length < 4 || questions.length > 6) out.push(`shape: ${url}: has ${questions.length} questions, not four to six`);
  for (const q of questions) {
    const words = q.trim().split(/\s+/).length;
    if (words > 15) out.push(`shape: ${url}: "${q}" has ${words} words, more than 15`);
    const banned = QUESTION_WORDS.filter((w) => new RegExp(`\\b${w}\\b`, "i").test(q));
    if (banned.length > 0) out.push(`shape: ${url}: "${q}" uses ${banned.map((w) => `"${w}"`).join(", ")}`);
  }
  return out;
}

/**
 * What `--check questions` fails on: each broken URL, each stale entry, each owed page with the command
 * that prints what its questions are written from, each entry outside `questionShape`'s bounds, and each
 * question another entry already has, compared in lower case without punctuation.
 */
export function questionProblems(world: World, root: string = ROOT): string[] {
  const s = questionStatus(world, root);
  const questions = readQuestions(root);
  const seen = new Map<string, string>();
  const repeats: string[] = [];
  for (const [url, e] of Object.entries(questions)) {
    for (const q of e.questions) {
      const key = q.toLowerCase().replace(/[^\p{L}\p{N}\s]/gu, "").replace(/\s+/g, " ").trim();
      const first = seen.get(key);
      if (first === undefined) seen.set(key, url);
      else repeats.push(`repeat: ${url}: "${q}" is also a question of ${first}`);
    }
  }
  return [
    ...s.broken.map((u) => `broken: ${u}: no page has this URL`),
    ...s.stale.map((x) => `stale: ${x.url} (${x.feature}): reread the page, rewrite its questions, then \`bun nv site --questions --stamp ${x.url}\``),
    ...s.owed.map((x) => `owed: ${x.url} (${x.feature}): \`bun nv site --questions --owed ${x.area}\` prints what to write its questions from, then add them and \`bun nv site --questions --stamp ${x.url}\``),
    ...Object.entries(questions).flatMap(([url, e]) => questionShape(url, e.questions)),
    ...repeats,
  ];
}

/** `--questions --status`: one line per area, owed and stale counted. */
export function questionCounts(world: World, root: string = ROOT): string[] {
  const s = questionStatus(world, root);
  const count = (list: QuestionSource[], area: QuestionArea) => list.filter((x) => x.area === area).length;
  return QUESTION_AREAS.map((a) => `questions ${a}: ${count(s.owed, a)} owed, ${count(s.stale, a)} stale`);
}

/** What one page's questions are written from, as `--questions --owed` prints it. */
export function questionMaterial(world: World, source: QuestionSource, root: string = ROOT): string {
  const byId = new Map(world.entries.map((e) => [e.id, e]));
  const lines = [`== ${source.url}  ${source.feature}`];
  if (source.area === "pages") lines.push(readAt(root, `${DOCS}/${source.feature}`).trim());
  else if (source.ids.length === 0) {
    const known = world.meta.classes?.find((c) => c.name === source.feature);
    lines.push(known?.doc?.short ?? "", `methods: ${(known?.members ?? []).map((m) => m.name).join(", ")}`);
  }
  for (const id of source.ids) {
    const e = byId.get(id);
    if (!e) continue;
    lines.push(`-- ${id}`, card(root, e, world.meta), readAt(root, `${examplesDir(e)}/${ABOUT}`).trim());
  }
  return lines.join("\n");
}

/** Records each named entry's current feature and hash. Returns what stopped it, if anything. */
export function stampQuestions(world: World, urls: string[], root: string = ROOT): string[] {
  const questions = readQuestions(root);
  const sources = new Map(questionSources(world, root).map((s) => [s.url, s]));
  const problems: string[] = [];
  for (const url of urls) {
    const source = sources.get(url);
    if (!source) problems.push(`${url}: no built page has this URL`);
    else if (!questions[url]) problems.push(`${url}: has no questions to stamp`);
    else questions[url] = { feature: source.feature, hash: source.hash, questions: questions[url]!.questions };
  }
  if (problems.length === 0) writeQuestions(root, questions);
  return problems;
}

/** What was retired and must stay gone, repo-relative. */
const RETIRED = [
  `${DOCS}/docs`,
  `${DOCS}/why-novis.mdx`,
  `${DOCS}/site-index.mdx`,
  "website/src/content/claims",
  "website/src/data/rules.json",
  "website/src/data/decisions.json",
  "website/src/components/DecisionSummary.astro",
  "website/src/components/RuleBookIndex.astro",
  "website/src/components/SiteIndex.astro",
  "website/src/components/WhyClaims.astro",
  "website/config/rule-sections.mjs",
  "website/scripts",
  "tools/nv/renderers/website-rules.ts",
];
const AREAS = [
  ["Guides", "/guides/"],
  ["Syntax", "/syntax/"],
  ["Reference", "/reference/"],
  ["Tools", "/tools/"],
  ["In-Depth", "/in-depth/"],
];

export function structureProblems(root: string = ROOT): string[] {
  const problems: string[] = [];
  const site = readAt(root, "website/config/site.mjs");
  const block = /export const AREAS = \[([\s\S]*?)\n\]/.exec(site)?.[1] ?? "";
  const areas = [...block.matchAll(/label:\s*'([^']+)'[^\n]*href:\s*'([^']+)'/g)].map((m) => [m[1]!, m[2]!]);
  if (JSON.stringify(areas) !== JSON.stringify(AREAS)) problems.push(`website/config/site.mjs: AREAS is ${JSON.stringify(areas)}, and the goal names ${JSON.stringify(AREAS)}`);
  if (!/export const INSTALL = \{ label: 'Install', href: '\/install\/' \}/.test(site)) problems.push("website/config/site.mjs: INSTALL is not `{ label: 'Install', href: '/install/' }`");

  const header = readAt(root, "website/src/components/Header.astro");
  const order = ["areas.map(", "install.href", "sponsoring.href"].map((s) => header.indexOf(s));
  if (order.some((i) => i < 0) || order[0]! > order[1]! || order[1]! > order[2]!) {
    problems.push("website/src/components/Header.astro: the nav is not the areas, then Install, then Sponsoring");
  }
  const footer = readAt(root, "website/src/components/Footer.astro");
  if (footer.includes("Pagination")) problems.push("website/src/components/Footer.astro: still shows prev/next (`Pagination`)");

  const astro = readAt(root, "website/astro.config.ts");
  if (/^\s*redirects\s*:/m.test(astro)) problems.push("website/astro.config.ts: has a `redirects` entry, and removed pages get none");
  const sidebar = astro.slice(astro.indexOf("sidebar: ["));
  const groups = [...sidebar.matchAll(/^ {10}label: ([^,\n]+),/gm)].map((m) => m[1]!.replace(/^'|'$/g, ""));
  const want = [...AREAS.map(([label]) => label!), "INSTALL.label"];
  if (JSON.stringify(groups) !== JSON.stringify(want)) problems.push(`website/astro.config.ts: the sidebar groups are ${groups.join(", ")}, and the goal names ${want.join(", ")}`);

  for (const path of RETIRED) if (existsSync(join(root, path))) problems.push(`${path}: retired, and still on disk`);

  const built = builtUrls(root);
  const target = (href: string) => href.replace(/[#?].*$/, "").replace(/\/?$/, "/");
  const asset = (href: string) => /\.[a-z0-9]+$/i.test(href.replace(/[#?].*$/, ""));
  for (const page of pages(root)) {
    for (const m of page.body.matchAll(LINK_RE)) {
      const href = m[1] ?? m[2]!;
      if (!asset(href) && !built.has(target(href))) problems.push(`${DOCS}/${page.key}: links to ${href}, which the site does not build`);
    }
  }
  for (const m of sidebar.matchAll(/link: '(\/[^']*)'/g)) {
    if (!built.has(target(m[1]!))) problems.push(`website/astro.config.ts: the sidebar links to ${m[1]}, which the site does not build`);
  }
  return problems;
}

/** A site-relative link in a page: a markdown link's target, or an `href` attribute's value. */
const LINK_RE = /\]\((\/(?!\/)[^)\s]*)\)|href=["'](\/(?!\/)[^"']*)["']/g;

/**
 * Every URL the site builds, each ending in `/`: one per handwritten page, the Core reference's class and
 * method pages, and the Configuration, CLI and Types reference's landing and entry pages.
 */
export function builtUrls(root: string = ROOT): Set<string> {
  const urls = new Set<string>(["/"]);
  for (const path of filesUnder(root, DOCS).filter((p) => /\.mdx?$/.test(p))) {
    const key = path.slice(DOCS.length + 1).replace(/\.mdx?$/, "");
    urls.add(`/${key.replace(/(^|\/)index$/, "")}/`.replace(/\/+/g, "/"));
  }
  const core = JSON.parse(readAt(root, DATA_FILE) || '{"classes":[]}') as { classes: { members: { url: string }[] }[] };
  for (const cls of core.classes) {
    for (const m of cls.members) {
      urls.add(m.url);
      urls.add(m.url.replace(/[^/]+\/$/, ""));
    }
  }
  const refPages = (JSON.parse(readAt(root, REFERENCE_FILE) || '{"pages":[]}') as { pages: RefPage[] }).pages;
  for (const area of ["config", "cli", "types"]) urls.add(`/reference/${area}/`);
  for (const p of refPages) urls.add(p.url);
  return urls;
}

/**
 * The sections of In-Depth, in sidebar order: the page or directory under `in-depth/`, and the fewest
 * pages a directory section has beside its `index.mdx`. A section with `0` is one page, `<slug>.mdx`.
 */
export const IN_DEPTH_SECTIONS: [slug: string, least: number][] = [
  ["what-for", 0],
  ["never", 0],
  ["how-we-decide", 0],
  ["how-novis-is-written", 0],
  ["how-novis-is-tested", 0],
  ["design-principles", 0],
  ["concepts", 5],
  ["new", 0],
  ["questions", 0],
  ["roadmap", 0],
  ["what-changed", 0],
];

/**
 * The `in-depth` part: each of `IN_DEPTH_SECTIONS` has its page, or its directory with at least its count
 * of pages beside the landing page, and the In-Depth sidebar group links the sections in that order.
 */
export function inDepthProblems(root: string = ROOT): string[] {
  const problems: string[] = [];
  const keys = new Set(pages(root).map((p) => p.key));
  for (const [slug, least] of IN_DEPTH_SECTIONS) {
    const prefix = `in-depth/${slug}/`;
    if (least === 0) {
      if (!keys.has(`in-depth/${slug}.mdx`) && !keys.has(`in-depth/${slug}.md`)) problems.push(`${DOCS}/in-depth/${slug}.mdx: no page`);
      continue;
    }
    const steps = [...keys].filter((k) => k.startsWith(prefix) && !/\/index\.mdx?$/.test(k));
    if (!keys.has(`${prefix}index.mdx`)) problems.push(`${DOCS}/${prefix}index.mdx: no landing page`);
    if (steps.length < least) problems.push(`${DOCS}/${prefix}: ${steps.length} page(s) beside its index, and the goal names ${least} at least`);
  }
  problems.push(...sidebarOrder(root, "In-Depth", "in-depth", IN_DEPTH_SECTIONS.map(([slug]) => slug)));
  return problems;
}

/** The tools of the Tools area, in sidebar order: each is a directory under `tools/` whose page is its `index.mdx`. */
export const TOOL_SECTIONS = ["editor", "formatter", "test-runner", "cli", "debugger", "debug-viewer", "profiler", "api-document"];

/** A picture a tool page may show, each one checked against a real run. */
const PICTURE_RE = /<(Terminal|Replay|Shot)\s+src=/;

/**
 * The `tools` part (`rule:testing/the-tools-area-has-a-page-for-each-tool`): each of `TOOL_SECTIONS` has its
 * page, and each page covers a feature and shows a checked picture. The overview has a section for each,
 * whose `## ` heading links the page and which shows a checked picture of its own, and the Tools sidebar
 * links each page in that order. A tool with no page yet is named as owed, and nothing else about it is.
 */
export function toolsProblems(root: string = ROOT): string[] {
  const problems: string[] = [];
  const all = new Map(pages(root).map((p) => [p.key, p]));
  const overview = all.get("tools/index.mdx");
  if (!overview) problems.push(`${DOCS}/tools/index.mdx: no overview page`);
  const present: string[] = [];
  for (const dir of TOOL_SECTIONS) {
    const key = `tools/${dir}/index.mdx`;
    const page = all.get(key);
    if (!page) {
      problems.push(`${DOCS}/${key}: owed, no page yet`);
      continue;
    }
    present.push(dir);
    if (!page.covers?.length) problems.push(`${DOCS}/${key}: covers no feature`);
    if (!PICTURE_RE.test(page.body)) problems.push(`${DOCS}/${key}: shows no <Terminal>, <Replay> or <Shot>`);
    if (!overview) continue;
    const section = overview.body.split(/^## /m).find((s) => s.startsWith(`[`) && s.includes(`](/tools/${dir}/)`));
    if (!section) problems.push(`${DOCS}/tools/index.mdx: has no \`## [...](/tools/${dir}/)\` section`);
    else if (!PICTURE_RE.test(section)) problems.push(`${DOCS}/tools/index.mdx: the /tools/${dir}/ section shows no <Terminal>, <Replay> or <Shot>`);
  }
  problems.push(...sidebarOrder(root, "Tools", "tools", present));
  return problems;
}

/** The problems with one sidebar group's links into `/<area>/<section>/`, which must all exist and be in order. */
function sidebarOrder(root: string, label: string, area: string, sections: string[]): string[] {
  const problems: string[] = [];
  const astro = readAt(root, "website/astro.config.ts");
  const from = astro.indexOf(`label: '${label}'`);
  const rest = from < 0 ? "" : astro.slice(from + 1);
  const next = rest.search(/^ {10}label: /m);
  const group = next < 0 ? rest : rest.slice(0, next);
  let last = -1;
  for (const section of sections) {
    const at = group.indexOf(`link: '/${area}/${section}/`);
    if (at < 0) problems.push(`website/astro.config.ts: the ${label} sidebar has no link into /${area}/${section}/`);
    else if (at < last) problems.push(`website/astro.config.ts: the ${label} sidebar lists /${area}/${section}/ out of the goal's order`);
    else last = at;
  }
  return problems;
}

const FENCE_RE = /^\s*(?:```|~~~)\s*(?:novis|nvs)\b/;
/** A `<Snippet>` tag: its `src`, which is always its first attribute, and the rest of the tag. */
const SNIPPET_RE = /<Snippet\s+src=["']([^"']+)["']([^>]*)>/g;
const REPLAY_RE = /<Replay\s+src=["']([^"']+)["']/g;
const TERMINAL_RE = /<Terminal\s+src=["']([^"']+)["']/g;
const SHOT_RE = /<Shot\s+src=["']([^"']+)["']/g;
/** The front page, under `DOCS`, which shows only what the server sends. */
export const FRONT_PAGE = "index.mdx";

/** What a snippet is compared with: `out` is what `nvs run` prints, `http` is the body `nvs serve`
 * sends, and `err` is what `nvs check` reports. */
export type SnippetKind = "out" | "http" | "err";
const EXT: Record<SnippetKind, string> = { out: "out", http: "http.out", err: "err" };

/** The file beside a snippet with the extension `ext`. */
const beside = (p: string, ext: string) => p.replace(/\.nvs$/, `.${ext}`);
/** The file beside a snippet that `kind` is compared with. */
export const snippetFile = (p: string, kind: SnippetKind) => beside(p, EXT[kind]);
const hasFile = (root: string, p: string, kind: SnippetKind) => existsSync(join(root, snippetFile(p, kind)));

/** The problems `snippets` finds without running anything. */
export function snippetShape(root: string = ROOT): { problems: string[]; snippets: string[]; terms: string[] } {
  const problems: string[] = [];
  const used = new Set<string>();
  const files = filesUnder(root, SNIPPETS);
  const terms = files.filter((p) => p.endsWith(`.${TERM}`));
  for (const page of pages(root)) {
    page.text.split("\n").forEach((line, i) => {
      if (FENCE_RE.test(line)) problems.push(`${DOCS}/${page.key}:${i + 1}: an inline Novis code fence; use <Snippet src="..."/>`);
    });
    for (const m of page.text.matchAll(SNIPPET_RE)) {
      const src = m[1]!;
      used.add(src);
      const at = `${DOCS}/${page.key}: <Snippet src="${src}"/>`;
      if (!existsSync(join(root, SNIPPETS, src))) {
        problems.push(`${at} names no file under ${SNIPPETS}/`);
        continue;
      }
      const output = /\boutput=["']([^"']*)["']/.exec(m[2] ?? "")?.[1];
      const http = hasFile(root, `${SNIPPETS}/${src}`, "http");
      if (output !== undefined && output !== "http") problems.push(`${at} has output="${output}", and the only value is "http"`);
      else if (output === "http" && !http) problems.push(`${at} has output="http", and the snippet has no .http.out`);
      else if (page.key === FRONT_PAGE && http && output === undefined) problems.push(`${at} needs output="http": the front page shows only what the server sends`);
    }
    // A replay ends on its snippet's text, so a page that shows the replay shows the snippet.
    for (const m of page.text.matchAll(REPLAY_RE)) {
      const src = m[1]!;
      used.add(src);
      const at = `${DOCS}/${page.key}: <Replay src="${src}"/>`;
      if (!existsSync(join(root, SNIPPETS, src))) problems.push(`${at} names no file under ${SNIPPETS}/`);
      else if (!existsSync(join(root, SNIPPETS, beside(src, "replay"))) && !existsSync(join(root, SNIPPETS, beside(src, "dap")))) problems.push(`${at}: the snippet has no .replay or .dap beside it`);
    }
    for (const m of page.text.matchAll(TERMINAL_RE)) {
      const src = m[1]!;
      used.add(src);
      const at = `${DOCS}/${page.key}: <Terminal src="${src}"/>`;
      if (!src.endsWith(`.${TERM}`)) problems.push(`${at} names no .${TERM} file`);
      else if (!existsSync(join(root, SNIPPETS, src))) problems.push(`${at} names no file under ${SNIPPETS}/`);
    }
  }
  for (const t of terms) if (!used.has(t.slice(SNIPPETS.length + 1))) problems.push(`${t}: no page shows it`);
  for (const p of files.filter((f) => f.endsWith(`.${TERM_OUT}`))) {
    if (!terms.includes(p.slice(0, -".out".length))) problems.push(`${p}: a .${TERM_OUT} and no .${TERM} beside it`);
  }
  // A program inside a terminal block's or a shot's fixture is what it runs, not a snippet.
  const fixtures = [...terms.map((t) => `${termFixture(t)}/`), ...files.filter((p) => p.endsWith(`.${SHOT}`)).map((s) => `${shotFixture(s)}/`)];
  const snippets = files.filter((p) => p.endsWith(".nvs") && !fixtures.some((f) => p.startsWith(f)));
  for (const p of snippets) {
    if (!used.has(p.slice(SNIPPETS.length + 1))) problems.push(`${p}: no page shows it`);
    const [out, http, err] = (["out", "http", "err"] as const).map((k) => hasFile(root, p, k));
    if (err && out) problems.push(`${p}: both a .out and a .err beside it; keep one`);
    else if (err && http) problems.push(`${p}: both a .http.out and a .err beside it; keep one`);
    else if (!out && !http && !err) problems.push(`${p}: no .out, .http.out or .err beside it`);
  }
  return { problems, snippets, terms };
}

/** The fixture directory of a terminal block: its `.term`'s path without the extension. */
export const termFixture = (t: string) => t.slice(0, -`.${TERM}`.length);

/** How one snippet is run on the command line, `out`, or checked, `err`: the header's § *A snippet*. */
export function snippetRun(p: string, kind: "out" | "err", nvs: string, root: string = ROOT): { argv: string[]; cwd: string; env?: Record<string, string> } {
  const dir = p.slice(0, p.lastIndexOf("/"));
  if (kind === "err") return { argv: [nvs, "check", p.slice(dir.length + 1)], cwd: join(root, dir), env: { NO_COLOR: "1" } };
  const config = `${dir}/nvs.toml`;
  return { argv: [nvs, "run", ...(existsSync(join(root, config)) ? ["--config", config] : []), p], cwd: root };
}

/** How one snippet is served on `port`: the header's § *A snippet*. */
export function serveRun(p: string, nvs: string, port: number, root: string = ROOT): { argv: string[]; cwd: string } {
  const config = `${p.slice(0, p.lastIndexOf("/"))}/nvs.toml`;
  return { argv: [nvs, "serve", ...(existsSync(join(root, config)) ? ["--config", config] : []), p, "--port", String(port)], cwd: root };
}

/** One HTTP request, as a snippet's `.nvsr` describes it. */
export interface SnippetRequest {
  method: string;
  path: string;
  query: string;
  /** Lower-case names, in the file's order. */
  headers: [string, string][];
  /** Null when the file has no body section. */
  body: string | null;
}

/** The request the server is sent for a snippet with no `.nvsr`: what a browser sends for
 * `http://localhost:8000/`, the address `nvs serve` listens on when nothing else is set. */
export const PLAIN_GET: SnippetRequest = { method: "GET", path: "/", query: "", headers: [["host", "localhost:8000"]], body: null };

/**
 * A `.nvsr` read the way `nvs-test`'s `request::read` reads it: `--NAME--` sections, and a body that is
 * the rest of the file, with CRLF line endings under `--BODY_CRLF--`. A section a real connection cannot
 * carry (`--CLIENT_IP--`, `--SCHEME--`, `--MOUNT--`) is an error, since the server decides those itself.
 */
export function parseRequest(text: string): SnippetRequest | string {
  const lines = text.replace(/\r\n?/g, "\n").split(/(?<=\n)/);
  const sections = new Map<string, string>();
  let body: string | null = null;
  let name: string | null = null;
  for (let i = 0; i < lines.length; i++) {
    const head = /^--([A-Z_]+)--\s*$/.exec(lines[i]!);
    if (head && (head[1] === "BODY" || head[1] === "BODY_CRLF")) {
      const rest = lines.slice(i + 1).join("");
      body = head[1] === "BODY" ? rest : rest.replace(/\n/g, "\r\n");
      break;
    }
    if (head) {
      name = head[1]!;
      if (!["METHOD", "PATH", "QUERY", "HEADERS"].includes(name)) return `\`--${name}--\` is not something a real connection can send`;
      if (sections.has(name)) return `\`--${name}--\` appears twice`;
      sections.set(name, "");
    } else if (name === null) {
      if (lines[i]!.trim() !== "") return `line ${i + 1}: text before \`--METHOD--\``;
    } else sections.set(name, sections.get(name)! + lines[i]);
  }
  const method = sections.get("METHOD")?.trim();
  const path = sections.get("PATH")?.trim();
  if (!method || !path) return "a request states its `--METHOD--` and its `--PATH--`";
  const headers: [string, string][] = [];
  for (const line of (sections.get("HEADERS") ?? "").split("\n").map((l) => l.trim())) {
    if (line === "") continue;
    const colon = line.indexOf(":");
    if (colon <= 0) return `\`${line}\` is not a header field`;
    headers.push([line.slice(0, colon).trim().toLowerCase(), line.slice(colon + 1).trim()]);
  }
  return { method, path, query: sections.get("QUERY")?.trim() ?? "", headers, body };
}

/** What `nvs run` did. */
export interface Ran {
  code: number;
  stdout: string;
  stderr: string;
}

/** What `nvs serve` sent: its status and body, or `failure`, why it sent nothing. `started` is whether
 * the server accepted connections at all. `skipped` is why this host does not start it, a skip and
 * not a failure. */
export interface Served {
  status: number;
  body: string;
  failure: string | null;
  started: boolean;
  skipped?: string;
}

const same = (a: string, b: string) => a.replace(/\r\n?/g, "\n").trimEnd() === b.replace(/\r\n?/g, "\n").trimEnd();
const firstLine = (s: string) => s.trim().split("\n")[0] ?? "";

/**
 * The divergence rule for a snippet that compiles. `owed` is the text its `.out` and its `.http.out` must
 * have, and null for a file that must not exist; it is null as a whole when a context the snippet must run
 * in failed. `problems` compares `files`, the text of each file or null where there is none, with it.
 *
 * A snippet with a `.nvsr` is a web snippet. Its `.http.out` is the body `nvs serve` sends, under any
 * status below 500. Its `.out` is what `nvs run` prints, only when that run exits 0 and prints something
 * else. A snippet without one is a command-line snippet. Its `.out` is what `nvs run` prints, which must
 * exit 0. Its `.http.out` is the body for `PLAIN_GET`, only when the server starts, the status is 2xx
 * and the body is something else; a server that started and then sent nothing is a problem. Two equal
 * outputs are one file, so a page never shows the same text twice.
 *
 * A server this host does not start is `skipped`, with `owed` null: a web snippet is not checked, and
 * a command-line snippet's `.out` is checked and its `.http.out` is left as it is.
 */
export function snippetOutputs(p: string, web: boolean, cli: Ran, served: Served | null, files: { out: string | null; http: string | null }): { owed: { out: string | null; http: string | null } | null; problems: string[]; skipped?: string } {
  let out: string | null = null;
  let http: string | null = null;
  let noOut = "";
  let noHttp = "";
  const skipped = served?.skipped;
  if (skipped !== undefined) {
    if (web) return { owed: null, problems: [], skipped };
    if (cli.code !== 0) return { owed: null, problems: [`${p}: exit ${cli.code}: ${firstLine(cli.stderr)}`], skipped };
    if (files.out === null) return { owed: null, problems: [`${p}: no .out beside it`], skipped };
    return { owed: null, problems: same(cli.stdout, files.out) ? [] : [`${p}: prints something other than its .out`], skipped };
  }
  if (web) {
    if (served === null || served.failure !== null) return { owed: null, problems: [`${p}: \`nvs serve\` sent no response: ${served?.failure ?? "it was not started"}`] };
    if (served.status >= 500) return { owed: null, problems: [`${p}: \`nvs serve\` answered ${served.status}: ${firstLine(served.body)}`] };
    http = served.body;
    if (cli.code !== 0) noOut = `\`nvs run\` exits ${cli.code} without a request`;
    else if (same(cli.stdout, served.body)) noOut = "`nvs run` prints the same as its .http.out";
    else out = cli.stdout;
  } else {
    if (cli.code !== 0) return { owed: null, problems: [`${p}: exit ${cli.code}: ${firstLine(cli.stderr)}`] };
    out = cli.stdout;
    if (served !== null && served.started && served.failure !== null) return { owed: null, problems: [`${p}: \`nvs serve\` started and sent no response: ${served.failure}`] };
    if (served === null || served.failure !== null) noHttp = "`nvs serve` does not start with it";
    else if (served.status < 200 || served.status > 299) noHttp = `\`nvs serve\` answered ${served.status}`;
    else if (same(served.body, cli.stdout)) noHttp = "`nvs serve` sends the same as its .out";
    else http = served.body;
  }
  const problems: string[] = [];
  const differ = "`nvs run` and `nvs serve` print different things, so the page shows both:";
  if (http !== null && files.http === null) problems.push(web ? `${p}: no .http.out beside it, and a snippet with a .nvsr shows the body \`nvs serve\` sends` : `${p}: ${differ} write its .http.out`);
  if (out !== null && files.out === null) problems.push(web ? `${p}: ${differ} write its .out` : `${p}: no .out beside it`);
  if (out === null && files.out !== null) problems.push(`${p}: delete its .out: ${noOut}`);
  if (http === null && files.http !== null) problems.push(`${p}: delete its .http.out: ${noHttp}`);
  if (out !== null && files.out !== null && !same(out, files.out)) problems.push(`${p}: prints something other than its .out`);
  if (http !== null && files.http !== null && !same(http, files.http)) problems.push(`${p}: \`nvs serve\` sends something other than its .http.out`);
  return { owed: { out, http }, problems };
}

/** The problem with a `.err` snippet's `nvs check`, or null when it matches `want`, the text of its `.err`. */
export function errVerdict(p: string, result: Ran, want: string): string | null {
  if (result.code === 0) return `${p}: compiles, and a snippet with a .err must not`;
  if (result.code !== 1) return `${p}: \`nvs check\` exit ${result.code}: ${firstLine(result.stderr)}`;
  return same(result.stderr, want) ? null : `${p}: \`nvs check\` reports something other than its .err`;
}

/** The extension of the file beside a snippet that lists its tainted variables: the header's § *A snippet*. */
export const TAINTED = "tainted";

/** The session `nvs lsp` is sent for one snippet, as one stream on its stdin: the handshake, the snippet
 * opened at `uri`, its semantic tokens as request 2, then shutdown and exit. */
export function taintScript(uri: string, text: string): string {
  return [
    { id: 1, method: "initialize", params: { processId: null, rootUri: null, capabilities: {} } },
    { method: "initialized", params: {} },
    { method: "textDocument/didOpen", params: { textDocument: { uri, languageId: "nvs", version: 1, text } } },
    { id: 2, method: "textDocument/semanticTokens/full", params: { textDocument: { uri } } },
    { id: 3, method: "shutdown", params: null },
    { method: "exit", params: null },
  ]
    .map(lspFrame)
    .join("");
}

/** The names a server's semantic tokens index, as its `initialize` result declares them. */
export interface Legend {
  tokenTypes: string[];
  tokenModifiers: string[];
}

/**
 * A snippet's `.tainted` text, from the semantic tokens `nvs lsp` sent for `text`, or null when no token
 * carries the `tainted` modifier. `data` is LSP's relative encoding, five numbers a token, and the bit is
 * found by name in `legend`, so a reordered legend changes nothing here. Each line is one token as
 * `line:column+length type text`: line and column from 1 and counted in UTF-16 units, as a
 * `tests/lsp/semantic/` case writes them, the token's type by its legend name (`variable`, `parameter`,
 * `property`), and the text the range covers, so a reader can check it.
 */
export function taintedRanges(text: string, legend: Legend, data: number[]): string | null {
  const bit = legend.tokenModifiers.indexOf(TAINTED);
  if (bit < 0) throw new Error(`the server declares no \`${TAINTED}\` modifier: ${legend.tokenModifiers.join(", ")}`);
  const lines = text.split("\n");
  const found: string[] = [];
  let line = 0;
  let column = 0;
  for (let i = 0; i + 4 < data.length; i += 5) {
    const [dLine, dColumn, length, type, modifiers] = data.slice(i, i + 5) as [number, number, number, number, number];
    column = dLine === 0 ? column + dColumn : dColumn;
    line += dLine;
    if ((modifiers >> bit) & 1) found.push(`${line + 1}:${column + 1}+${length} ${legend.tokenTypes[type] ?? type} ${(lines[line] ?? "").slice(column, column + length)}`);
  }
  return found.length ? `${found.join("\n")}\n` : null;
}

/** The problem with a snippet's `.tainted`, given what it owes, `owed`, and the text of the file, each
 * null where there is none. */
export function taintVerdict(p: string, owed: string | null, file: string | null): string | null {
  if (owed === null) return file === null ? null : `${p}: delete its .${TAINTED}: \`nvs lsp\` marks no variable in it as tainted`;
  if (file === null) return `${p}: no .${TAINTED} beside it, and \`nvs lsp\` marks a variable in it as tainted`;
  return same(owed, file) ? null : `${p}: its .${TAINTED} is not where \`nvs lsp\` marks a tainted variable now`;
}

/** How long one snippet's run, or its server's start and response, may take. */
const SNIPPET_MS = 60_000;

const runSnippet = (p: string, kind: "out" | "err", nvs: string) => {
  const { argv, cwd, env } = snippetRun(p, kind, nvs);
  return runProc(argv, { cwd, timeoutMs: SNIPPET_MS, ...(env ? { env } : {}) });
};

/** Sends `request` once to the server on `port`, and gives the status and body it answers with. */
function send(port: number, request: SnippetRequest, timeoutMs: number): Promise<Omit<Served, "started">> {
  const headers: Record<string, string> = {};
  for (const [name, value] of request.headers) headers[name] = name in headers ? `${headers[name]}, ${value}` : value;
  if (request.body !== null && !("content-length" in headers)) headers["content-length"] = String(Buffer.byteLength(request.body));
  const path = request.query === "" ? request.path : `${request.path}?${request.query}`;
  return new Promise((resolve) => {
    const req = httpRequest({ host: "127.0.0.1", port, method: request.method, path, headers, timeout: timeoutMs }, (res) => {
      const chunks: Buffer[] = [];
      res.on("data", (c: Buffer) => chunks.push(c));
      res.on("end", () => resolve({ status: res.statusCode ?? 0, body: Buffer.concat(chunks).toString("utf8"), failure: null }));
      res.on("error", (e) => resolve({ status: 0, body: "", failure: e.message }));
    });
    req.on("timeout", () => req.destroy(new Error(`no response within ${timeoutMs / 1000} s`)));
    req.on("error", (e) => resolve({ status: 0, body: "", failure: e.message }));
    if (request.body !== null) req.write(request.body);
    req.end();
  });
}

/** The services a snippet served under the root `nvs.toml` needs, as `requires:` lines. That file's
 * `[queue]` arms on `[db.main]`, the `postgres` service, and `nvs serve` will not boot until that
 * database holds the queue's tables. A snippet with its own `nvs.toml` never touches Docker. */
const ROOT_CONFIG_SERVICES = "// requires: postgres";
/** `rootConfigReady`, started once per process by the first snippet served under the root `nvs.toml`. */
let rootConfigUp: Promise<{ why: string; skip: boolean } | null> | null = null;

/** Brings up what the root `nvs.toml` serves on, and runs `nvs queue migrate` over it: the compose
 * service keeps no data volume, so each new container starts without the queue's tables. Null when
 * the root config can serve. Otherwise `why` it cannot, and `skip` when the reason is that this host
 * runs no compose containers (`dockerAbsent`), the one rule the proofs follow too. */
async function rootConfigReady(nvs: string): Promise<{ why: string; skip: boolean } | null> {
  const absent = await dockerAbsent();
  if (absent) return { why: `the root \`nvs.toml\` needs \`postgres\`, and ${absent}`, skip: true };
  const down = await servicesFor(ROOT_CONFIG_SERVICES);
  if (down) return { why: `the root \`nvs.toml\` ${down}`, skip: false };
  const migrated = await runProc([nvs, "queue", "migrate"], { cwd: ROOT, timeoutMs: SNIPPET_MS, env: { NOVIS_NO_INIT: "1" } });
  if (migrated.code === 0) return null;
  return { why: `the root \`nvs.toml\` needs the queue's tables, and \`nvs queue migrate\` failed (exit ${migrated.code}): ${migrated.stderr.trim().split(/\r?\n/).at(-1) ?? ""}`, skip: false };
}

/**
 * `nvs serve` started with the snippet as its one file and port 0, `request` sent once to the port its
 * `listening on` line names, and the server stopped. The system picks the port as the server binds it,
 * so servers started in parallel never meet on one, and the line is printed only after every file
 * compiled and the socket is bound. With no `[server]` block the mode is production, so every path runs
 * the snippet. A server that exits before that line is a failure with the end of its standard error.
 */
async function serveSnippet(p: string, request: SnippetRequest, nvs: string): Promise<Served> {
  const { argv, cwd } = serveRun(p, nvs, 0);
  if (!argv.includes("--config")) {
    rootConfigUp ??= rootConfigReady(nvs);
    const down = await rootConfigUp;
    if (down?.skip) return { status: 0, body: "", failure: null, started: false, skipped: down.why };
    if (down) return { status: 0, body: "", failure: down.why, started: false };
  }
  const stop = new AbortController();
  let bound: (port: number) => void = () => {};
  const port = new Promise<number>((resolve) => (bound = resolve));
  const onLine = (line: string, stream: "stdout" | "stderr") => {
    const m = stream === "stdout" ? /^listening on http:\/\/127\.0\.0\.1:(\d+) /.exec(line) : null;
    if (m) bound(Number(m[1]));
  };
  const server = runProc(argv, { cwd, signal: stop.signal, timeoutMs: SNIPPET_MS, onLine, env: { NOVIS_NO_INIT: "1" } });
  const started = await Promise.race([port, server.then(() => null)]);
  const served: Served = started === null ? { status: 0, body: "", failure: "", started: false } : { ...(await send(started, request, SNIPPET_MS)), started: true };
  stop.abort();
  const ran = await server;
  const tail = ran.stderr.trim().split("\n").slice(-3).join(" / ");
  if (served.started) return served.failure === null ? served : { ...served, failure: `${served.failure}; the server wrote: ${tail}` };
  served.failure = ran.timedOut ? `it did not start within ${SNIPPET_MS / 1000} s` : `the server exited ${ran.code}: ${tail}`;
  return served;
}

/** The text of each of a snippet's two output files, null where it has none. */
const outputFiles = (p: string) => ({ out: hasFile(ROOT, p, "out") ? readAt(ROOT, snippetFile(p, "out")) : null, http: hasFile(ROOT, p, "http") ? readAt(ROOT, snippetFile(p, "http")) : null });

/** The `.tainted` text a snippet owes, from one `nvs lsp` session opened at the snippet's own path, or
 * null when it owes none. A string inside `failure` is why the session gave no tokens. */
async function snippetTaint(p: string, nvs: string): Promise<{ owed: string | null } | { failure: string }> {
  const text = readAt(ROOT, p);
  const ran = await runProc([nvs, "lsp"], { cwd: ROOT, input: taintScript(pathToFileURL(join(ROOT, p)).href, text), timeoutMs: SNIPPET_MS });
  const messages = lspMessages(ran.stdout);
  const answer = (id: number) => messages.find((m) => m.id === id && !("method" in m));
  const legend = (answer(1)?.result as { capabilities?: { semanticTokensProvider?: { legend?: Legend } } } | undefined)?.capabilities?.semanticTokensProvider?.legend;
  const data = (answer(2)?.result as { data?: number[] } | null | undefined)?.data;
  if (legend === undefined || data === undefined) return { failure: `${p}: \`nvs lsp\` sent no semantic tokens (exit ${ran.code}): ${firstLine(ran.stderr)}` };
  try {
    return { owed: taintedRanges(text, legend, data) };
  } catch (e) {
    return { failure: `${p}: ${(e as Error).message}` };
  }
}

/** The text of a snippet's `.tainted`, null where it has none. */
const taintFile = (p: string) => (existsSync(join(ROOT, beside(p, TAINTED))) ? readAt(ROOT, beside(p, TAINTED)) : null);

/** The extensions of a snippet's replay and of its recording: the header's § *A snippet*. */
export const REPLAY = "replay";
export const RECORDING = "replay.out";

/** What a snippet's `.replay` records now and the text its last step ends on, from `nvs lsp` with the
 * snippet's own path as the document; null when the snippet has no `.replay`. */
export async function snippetReplay(p: string, nvs: string, root: string): Promise<{ recording: string; last: string } | { failure: string } | null> {
  if (!existsSync(join(root, beside(p, REPLAY)))) return null;
  const replay = parseReplay(readAt(root, beside(p, REPLAY)));
  if (typeof replay === "string") return { failure: `${beside(p, REPLAY)}: ${replay}` };
  const play = (input: string) => runProc([nvs, "lsp"], { cwd: root, input, timeoutMs: SNIPPET_MS });
  const got = await recordReplay(pathToFileURL(join(root, p)).href, replay, play);
  return "failure" in got ? { failure: `${beside(p, REPLAY)}: ${got.failure}` } : got;
}

/** The problems with a snippet's `.replay` and `.replay.out` under `root`: a replay that does not run or
 * does not end on the snippet's text, and a recording that is missing, stale or has no replay. */
export async function replayProblems(p: string, nvs: string, root: string = ROOT): Promise<string[]> {
  const file = existsSync(join(root, beside(p, RECORDING))) ? readAt(root, beside(p, RECORDING)) : null;
  const replayed = await snippetReplay(p, nvs, root);
  if (replayed === null) return file === null ? [] : [`${p}: a .${RECORDING} beside it and no .${REPLAY}`];
  if ("failure" in replayed) return [replayed.failure];
  const problems: string[] = [];
  if (!same(replayed.last, readAt(root, p))) problems.push(`${beside(p, REPLAY)}: its steps end on other text than the snippet`);
  if (file === null) problems.push(`${p}: no .${RECORDING} beside its .${REPLAY}`);
  else if (!same(replayed.recording, file)) problems.push(`${p}: its .${RECORDING} is not what \`nvs lsp\` returns for its .${REPLAY} now`);
  return problems;
}

/** The extensions of a snippet's debug session and of its recording: the header's § *A snippet*. */
export const DAP = "dap";
export const DAP_OUT = "dap.out";

/** What a snippet's `.dap` records now, from `nvs dap` over the snippet; null when it has no `.dap`. */
export async function snippetDap(p: string, nvs: string, root: string): Promise<{ recording: string } | { failure: string } | null> {
  if (!existsSync(join(root, beside(p, DAP)))) return null;
  const dap = parseDap(readAt(root, beside(p, DAP)));
  if (typeof dap === "string") return { failure: `${beside(p, DAP)}: ${dap}` };
  const config = `${p.slice(0, p.lastIndexOf("/"))}/nvs.toml`;
  const argv = [nvs, ...(existsSync(join(root, config)) ? ["--config", config] : []), "dap"];
  const got = await recordDap(join(root, p), dap, (opts) => runProc(argv, { cwd: root, timeoutMs: SNIPPET_MS, ...opts }));
  return "failure" in got ? { failure: `${beside(p, DAP)}: ${got.failure}` } : got;
}

/** The problems with a snippet's `.dap` and `.dap.out` under `root`: a session that does not run, and a
 * recording that is missing, stale or has no `.dap`. */
export async function dapProblems(p: string, nvs: string, root: string = ROOT): Promise<string[]> {
  const file = existsSync(join(root, beside(p, DAP_OUT))) ? readAt(root, beside(p, DAP_OUT)) : null;
  const recorded = await snippetDap(p, nvs, root);
  if (recorded === null) return file === null ? [] : [`${p}: a .${DAP_OUT} beside it and no .${DAP}`];
  if ("failure" in recorded) return [recorded.failure];
  const problems: string[] = [];
  if (existsSync(join(root, beside(p, REPLAY)))) problems.push(`${p}: both a .${REPLAY} and a .${DAP} beside it; a page shows one replay per snippet`);
  if (file === null) problems.push(`${p}: no .${DAP_OUT} beside its .${DAP}`);
  else if (!same(recorded.recording, file)) problems.push(`${p}: its .${DAP_OUT} is not what \`nvs dap\` returns for its .${DAP} now`);
  return problems;
}

/** What a `.term`'s commands record now, run with `nvs` in a copy of its fixture with `NO_COLOR=1`. */
export async function termRecording(t: string, nvs: string, root: string = ROOT): Promise<{ recording: string } | { failure: string }> {
  const play = async (argv: string[], cwd: string) => {
    const ran = await runProc([nvs, ...argv], { cwd, env: { NO_COLOR: "1" }, timeoutMs: SNIPPET_MS });
    if (ran.timedOut) throw new Error(`\`nvs ${argv.join(" ")}\` did not end within ${SNIPPET_MS / 1000}s${ran.hung ? " (hung)" : ""}`);
    return ran;
  };
  try {
    const got = await recordTerm(readAt(root, t), join(root, termFixture(t)), play);
    return "failure" in got ? { failure: `${t}: ${got.failure}` } : got;
  } catch (e) {
    return { failure: `${t}: ${(e as Error).message}` };
  }
}

/** The problems with a `.term` under `root`: commands that do not run, and a `.term.out` that is missing
 * or is not what `nvs` prints now, once both are masked. */
export async function termProblems(t: string, nvs: string, root: string = ROOT): Promise<string[]> {
  const got = await termRecording(t, nvs, root);
  if ("failure" in got) return [got.failure];
  const out = `${t}.out`;
  if (!existsSync(join(root, out))) return [`${t}: no .${TERM_OUT} beside it; run \`bun nv site --bless ${t}\``];
  return sameTerm(got.recording, readAt(root, out)) ? [] : [`${t}: its .${TERM_OUT} is not what \`nvs\` prints for its commands now`];
}

/** A snippet that compiles, run in both contexts: `nvs run`, and `nvs serve` sent its `.nvsr` or
 * `PLAIN_GET`. A command-line snippet whose `nvs run` fails is not served. */
async function bothContexts(p: string, nvs: string): Promise<{ web: boolean; cli: Ran; served: Served | null } | string> {
  const web = existsSync(join(ROOT, beside(p, "nvsr")));
  const request = web ? parseRequest(readAt(ROOT, beside(p, "nvsr"))) : PLAIN_GET;
  if (typeof request === "string") return `${beside(p, "nvsr")}: ${request}`;
  const cli = await runSnippet(p, "out", nvs);
  const served = web || cli.code === 0 ? await serveSnippet(p, request, nvs) : null;
  return { web, cli, served };
}

/** Runs `job` over `items`, `width` at a time, and gives the results in the order of `items`. */
async function inParallel<T, R>(items: T[], width: number, job: (item: T) => Promise<R>): Promise<R[]> {
  const results: R[] = new Array(items.length);
  let next = 0;
  const worker = async () => {
    while (next < items.length) {
      const i = next++;
      results[i] = await job(items[i]!);
    }
  };
  await Promise.all(Array.from({ length: Math.min(width, items.length) }, worker));
  return results;
}

/** How many snippets run at once: half the cores, at most eight. Each one's command-line run and server
 * run follow each other. */
const SNIPPET_WIDTH = Math.max(1, Math.min(8, availableParallelism() >> 1));

/** Every snippet's problems. A snippet whose server this host does not start prints a `skip` line
 * with the reason, the way the proofs print one. */
async function snippetProblems(nvs: string): Promise<string[]> {
  const { problems, snippets, terms } = snippetShape();
  const perTerm = await inParallel(terms, SNIPPET_WIDTH, (t) => termProblems(t, nvs));
  const skips: string[] = [];
  const perSnippet = await inParallel(snippets, SNIPPET_WIDTH, async (p): Promise<string[]> => {
    const found = commentProblems(p).map((c) => `${p}:${c}`);
    const taint = await snippetTaint(p, nvs);
    const taintProblem = "failure" in taint ? taint.failure : taintVerdict(p, taint.owed, taintFile(p));
    if (taintProblem) found.push(taintProblem);
    found.push(...(await replayProblems(p, nvs)));
    found.push(...(await dapProblems(p, nvs)));
    const [out, http, err] = (["out", "http", "err"] as const).map((k) => hasFile(ROOT, p, k));
    // A snippet `snippetShape` already named has no one file to match.
    if ((err && (out || http)) || (!out && !http && !err)) return found;
    if (err) {
      const verdict = errVerdict(p, await runSnippet(p, "err", nvs), readAt(ROOT, snippetFile(p, "err")));
      return verdict ? [...found, verdict] : found;
    }
    const ran = await bothContexts(p, nvs);
    if (typeof ran === "string") return [...found, ran];
    const verdict = snippetOutputs(p, ran.web, ran.cli, ran.served, outputFiles(p));
    if (verdict.skipped !== undefined) skips.push(`${p}: ${ran.web ? "not checked" : "its .http.out is not checked"}, because ${verdict.skipped}`);
    return [...found, ...verdict.problems];
  });
  for (const skip of skips.sort()) console.log(`  skip  ${skip}`);
  return [...problems, ...perTerm.flat(), ...perSnippet.flat()];
}

/** Writes `text` to `dest` and shows it. */
function blessWrite(dest: string, text: string): void {
  const existed = existsSync(join(ROOT, dest));
  writeFileSync(join(ROOT, dest), text);
  console.log(`  ${existed ? "rewrote" : "wrote"}  ${dest}`);
  for (const line of text.trimEnd().split("\n")) console.log(`      | ${line}`);
}

/** `--bless`: each snippet's `.out`, `.http.out` or `.err` written from what the binary does and shown,
 * and an output file the divergence rule no longer needs deleted. */
async function blessSnippets(nvs: string, paths: string[]): Promise<number> {
  let failed = false;
  const fail = (why: string) => {
    console.log(`  FAIL  ${why}`);
    failed = true;
  };
  for (const p of paths) {
    if (p.startsWith(`${SNIPPETS}/`) && p.endsWith(`.${TERM}`) && existsSync(join(ROOT, p))) {
      const got = await termRecording(p, nvs);
      if ("failure" in got) fail(got.failure);
      else blessWrite(`${p}.out`, got.recording);
      continue;
    }
    if (!p.startsWith(`${SNIPPETS}/`) || !p.endsWith(".nvs") || !existsSync(join(ROOT, p))) {
      fail(`${p}: not a .nvs or .${TERM} file under ${SNIPPETS}/`);
      continue;
    }
    const taint = await snippetTaint(p, nvs);
    if ("failure" in taint) fail(taint.failure);
    else if (taint.owed !== null) blessWrite(beside(p, TAINTED), taint.owed);
    else if (taintFile(p) !== null) {
      rmSync(join(ROOT, beside(p, TAINTED)));
      console.log(`  deleted  ${beside(p, TAINTED)}`);
    }
    const replayed = await snippetReplay(p, nvs, ROOT);
    if (replayed === null && existsSync(join(ROOT, beside(p, RECORDING)))) {
      rmSync(join(ROOT, beside(p, RECORDING)));
      console.log(`  deleted  ${beside(p, RECORDING)}`);
    } else if (replayed !== null && "failure" in replayed) fail(replayed.failure);
    else if (replayed !== null && !same(replayed.last, readAt(ROOT, p))) fail(`${beside(p, REPLAY)}: its steps end on other text than the snippet`);
    else if (replayed !== null) blessWrite(beside(p, RECORDING), replayed.recording);
    const debugged = await snippetDap(p, nvs, ROOT);
    if (debugged === null && existsSync(join(ROOT, beside(p, DAP_OUT)))) {
      rmSync(join(ROOT, beside(p, DAP_OUT)));
      console.log(`  deleted  ${beside(p, DAP_OUT)}`);
    } else if (debugged !== null && "failure" in debugged) fail(debugged.failure);
    else if (debugged !== null) blessWrite(beside(p, DAP_OUT), debugged.recording);
    const [out, http, err] = (["out", "http", "err"] as const).map((k) => hasFile(ROOT, p, k));
    if (err && (out || http)) {
      fail(`${p}: both a .err and an output file beside it; delete the one that is wrong`);
      continue;
    }
    const lf = (s: string) => s.replace(/\r\n?/g, "\n");
    if (err || (!out && !http && (await runSnippet(p, "err", nvs)).code !== 0)) {
      const result = await runSnippet(p, "err", nvs);
      const verdict = errVerdict(p, result, result.stderr);
      if (verdict) fail(verdict);
      else blessWrite(snippetFile(p, "err"), lf(result.stderr));
      continue;
    }
    const ran = await bothContexts(p, nvs);
    if (typeof ran === "string") {
      fail(ran);
      continue;
    }
    const { owed, problems, skipped } = snippetOutputs(p, ran.web, ran.cli, ran.served, { out: null, http: null });
    if (skipped !== undefined) {
      fail(`${p}: not blessed, because ${skipped}`);
      continue;
    }
    if (owed === null) {
      fail(problems[0]!);
      continue;
    }
    for (const kind of ["out", "http"] as const) {
      const want = owed[kind];
      if (want !== null) blessWrite(snippetFile(p, kind), lf(want));
      else if (hasFile(ROOT, p, kind)) {
        rmSync(join(ROOT, snippetFile(p, kind)));
        console.log(`  deleted  ${snippetFile(p, kind)}`);
      }
    }
  }
  if (!failed) console.log("nv site: read what was written; a blessed file is a claim, not a formality.");
  return failed ? 1 : 0;
}

/** Every `.shot` under `root`'s snippets, and the problems that need no run: a `<Shot src>` that names no
 * `.shot`, a `.shot` no page shows, and a `.shot.out` or a `.png` with no `.shot`. */
export function shotShape(root: string = ROOT): { shots: string[]; problems: string[] } {
  const files = filesUnder(root, SNIPPETS);
  const shots = files.filter((p) => p.endsWith(`.${SHOT}`));
  const problems: string[] = [];
  const used = new Set<string>();
  for (const page of pages(root)) {
    for (const m of page.text.matchAll(SHOT_RE)) {
      const src = m[1]!;
      used.add(src);
      const at = `${DOCS}/${page.key}: <Shot src="${src}"/>`;
      if (!src.endsWith(`.${SHOT}`)) problems.push(`${at} names no .${SHOT} file`);
      else if (!existsSync(join(root, SNIPPETS, src))) problems.push(`${at} names no file under ${SNIPPETS}/`);
    }
  }
  for (const p of shots) if (!used.has(p.slice(SNIPPETS.length + 1))) problems.push(`${p}: no page shows it`);
  for (const p of files.filter((f) => f.endsWith(`.${SHOT_OUT}`))) {
    if (!shots.includes(p.slice(0, -".out".length))) problems.push(`${p}: a .${SHOT_OUT} and no .${SHOT} beside it`);
  }
  for (const p of files.filter((f) => f.endsWith(`.${SHOT_PNG}`))) {
    if (!shots.some((s) => shotPng(s) === p)) problems.push(`${p}: a picture and no .${SHOT} beside it`);
  }
  return { shots, problems };
}

/** The problems with one shot, given the `.shot.out` a new run records, the one on disk or null, and
 * whether its picture exists. */
export function shotVerdict(p: string, recording: string, onDisk: string | null, picture: boolean): string[] {
  const shoot = `run \`bun nv site --shoot ${p}\``;
  if (onDisk === null) return [`${p}: no .${SHOT_OUT} beside it; ${shoot}`];
  const problems: string[] = [];
  if (!same(recording, onDisk)) problems.push(`${p}: its .${SHOT_OUT} is not what the run shows now; ${shoot}`);
  if (!picture) problems.push(`${p}: no ${shotPng(p).slice(p.lastIndexOf("/") + 1)} beside it; ${shoot}`);
  return problems;
}

/** The `shots` part: § *A screenshot*. The shots run one after another, since each starts a server and a
 * browser. */
async function shotsProblems(nvs: string, root: string = ROOT): Promise<string[]> {
  const { shots, problems } = shotShape(root);
  if (shots.length && !canShoot()) {
    console.log(`  skip  shots: no Chromium headless shell is installed, so ${shots.length} shot(s) are not taken`);
    return problems;
  }
  for (const p of shots) {
    const got = await recordShot(p, nvs, root);
    if ("failure" in got) problems.push(got.failure);
    else {
      const out = `${p}.out`;
      problems.push(...shotVerdict(p, got.recording, existsSync(join(root, out)) ? readAt(root, out) : null, existsSync(join(root, shotPng(p)))));
    }
  }
  return problems;
}

/** `--shoot [SHOT...]`: each named shot, or every one, taken again, with its picture and its `.shot.out`
 * written. */
async function shoot(nvs: string, named: string[]): Promise<number> {
  const { shots } = shotShape();
  const bad = named.filter((p) => !shots.includes(p));
  if (bad.length) {
    console.error(`nv site: ${bad.join(", ")}: not a .${SHOT} under ${SNIPPETS}/`);
    return 2;
  }
  if (!canShoot()) {
    console.error("nv site: no Chromium headless shell is installed; tools/nv/test/chromium.ts says how to install it");
    return 1;
  }
  let failed = 0;
  for (const p of named.length ? named : shots) {
    const got = await recordShot(p, nvs);
    if ("failure" in got) {
      console.log(`  FAIL  ${got.failure}`);
      failed++;
      continue;
    }
    writeFileSync(join(ROOT, shotPng(p)), got.png);
    console.log(`  wrote  ${shotPng(p)} (${got.png.length} bytes)`);
    blessWrite(`${p}.out`, got.recording);
  }
  if (!failed) console.log("nv site: look at each picture; a shot is a claim about what the tool shows.");
  return failed ? 1 : 0;
}

/** The paragraphs of a page's prose: no front matter, code, imports, components, comments, tables or headings. */
export function paragraphs(body: string): { line: number; text: string }[] {
  const out: { line: number; text: string }[] = [];
  let current: { line: number; text: string } | null = null;
  let fence = "";
  let comment = false;
  const end = () => {
    if (current) out.push(current);
    current = null;
  };
  body.split("\n").forEach((raw, i) => {
    const line = raw.trim();
    if (fence) {
      if (line.startsWith(fence)) fence = "";
      return;
    }
    if (comment) {
      if (line.includes("*/}")) comment = false;
      return;
    }
    if (line.startsWith("```") || line.startsWith("~~~")) {
      end();
      fence = line.slice(0, 3);
      return;
    }
    if (line.startsWith("{/*")) {
      end();
      comment = !line.includes("*/}");
      return;
    }
    const skip = !line || /^(?:import|export)\s/.test(line) || /^[<#|:]/.test(line);
    const item = /^(?:[-*+]|\d+\.)\s+/.test(line);
    if (skip || item) end();
    if (skip) return;
    const text = line.replace(/^(?:[-*+]|\d+\.)\s+/, "");
    if (current) current.text += ` ${text}`;
    else current = { line: i + 1, text };
  });
  end();
  return out;
}

/** The countable bounds over one page's prose, as `line: what`. Line numbers count from the body's first line. */
export function proseProblems(body: string): string[] {
  const problems: string[] = [];
  for (const { line, text } of paragraphs(body)) {
    const plain = text.replace(/`[^`]*`/g, "code").replace(/\[([^\]]*)\]\([^)]*\)/g, "$1");
    if (/—|\s–\s|\s--\s/.test(plain)) problems.push(`${line}: a dash joins two sentences; write two`);
    const sentences = plain.split(/(?<=[.!?])\s+/).filter((s) => s.trim());
    if (sentences.length > PARAGRAPH_SENTENCES) problems.push(`${line}: a paragraph of ${sentences.length} sentences; ${PARAGRAPH_SENTENCES} is the bound`);
    for (const s of sentences) {
      const words = s.split(/\s+/).filter(Boolean);
      if (words.length > SENTENCE_WORDS) problems.push(`${line}: a ${words.length}-word sentence opening \`${words.slice(0, 5).join(" ")} ...\`; ${SENTENCE_WORDS} is the bound`);
    }
  }
  return problems;
}

/** The `nvs` binary as an absolute path, since the apps and the snippets run it from other directories:
 * the one named, or the newest built (`lib/nvs-binary.ts`). */
export function binary(explicit: string | undefined): string | null {
  if (explicit !== undefined) return existsSync(explicit) ? resolve(explicit) : null;
  return newestNvs();
}

/** A roster feature Reference owes a page: a class member, an enum, exception or interface, a
 * directive, a `tools:config` section, or a section of the `cli`, `editor` or `agents` chapter or an
 * `nvs …` section of `server`. */
function referenceOwes(e: Entry): boolean {
  if (["member", "directive", "enum", "exception", "interface"].includes(e.kind)) return true;
  const m = /^tools:([^/]+)\/(.+)$/.exec(e.id);
  if (!m) return false;
  return ["config", "cli", "editor", "agents"].includes(m[1]!) || (m[1] === "server" && m[2]!.startsWith("nvs-"));
}

/**
 * The `reference` part. `core.json` and `reference.json` are what `bun nv render --website` writes from
 * this binary today. Every roster feature `referenceOwes` has a page, through the feature path both
 * files carry. Every name on an `about.md` file's `related:` line is a Core member with a page. And in
 * the built site, no page says "Not documented yet", and every Reference page shows one example block
 * per example file of every feature on it.
 */
async function referenceProblems(w: World, nvs: string, root: string = ROOT): Promise<string[]> {
  const problems: string[] = [];
  const saved = process.env.NVS_BIN;
  process.env.NVS_BIN = nvs;
  try {
    for (const out of await renderWebsiteCore(root)) {
      if (readAt(root, out.path) !== out.text) problems.push(`${out.path}: not current against this binary: run \`bun nv render --website\``);
    }
  } finally {
    if (saved === undefined) delete process.env.NVS_BIN;
    else process.env.NVS_BIN = saved;
  }

  const core = JSON.parse(readAt(root, DATA_FILE) || '{"classes":[]}') as { classes: { name: string; members: { name: string; url: string; examples: string }[] }[] };
  const refPages = (JSON.parse(readAt(root, REFERENCE_FILE) || '{"pages":[]}') as { pages: RefPage[] }).pages;
  /** Each page's URL, with the feature paths it shows. */
  const shown = new Map<string, string[]>();
  for (const cls of core.classes) for (const m of cls.members) shown.set(m.url, [m.examples]);
  for (const p of refPages) shown.set(p.url, [...(p.intro ? [p.intro.examples] : []), ...p.keys.map((k) => k.examples)]);
  const paged = new Set([...shown.values()].flat());
  for (const e of w.entries) if (referenceOwes(e) && !paged.has(e.path)) problems.push(`${e.id}: has no Reference page`);

  const names = new Set(core.classes.flatMap((c) => c.members.map((m) => `${c.name}::${m.name}`)));
  for (const path of filesUnder(root, EXAMPLES).filter((p) => p.endsWith(`/${ABOUT}`))) {
    const last = readAt(root, path).trimEnd().split("\n").pop() ?? "";
    const m = /^related:\s*(.*)$/.exec(last);
    if (!m) continue;
    for (const name of m[1]!.split(",").map((n) => n.trim()).filter(Boolean)) {
      if (!names.has(name)) problems.push(`${path}: the related: line names ${name}, which has no Core page`);
    }
  }

  if (!existsSync(join(root, DIST))) return [...problems, `${DIST}: no built site: run \`bun nv site --build\``];
  for (const path of filesUnder(root, DIST).filter((p) => p.endsWith(".html"))) {
    if (readAt(root, path).includes("Not documented yet")) problems.push(`${path}: says "Not documented yet"`);
  }
  for (const [url, features] of shown) {
    const html = readAt(root, `${DIST}${url}index.html`);
    if (!html) {
      problems.push(`${url}: not in the built site`);
      continue;
    }
    const want = features.reduce((n, f) => n + filesUnder(root, `${EXAMPLES}/${f}`).filter((p) => /\.(nvs|cmd)$/.test(p) && p.split("/").length === `${EXAMPLES}/${f}`.split("/").length + 1).length, 0);
    const got = html.split('class="method-example"').length - 1;
    if (got !== want) problems.push(`${url}: shows ${got} example(s), and its features have ${want}`);
  }
  return problems;
}

/** The landing page of Syntax, which explains no one feature. */
const SYNTAX_INDEX = "syntax/index.mdx";
/** The heading a Syntax page's best practice is under, with the list it needs below it. */
const DO_DONT_RE = /^## Do and don['’]t[ \t]*\n+- /m;

/**
 * The `syntax` part: every Syntax page but the landing page covers at least one feature, shows at least
 * one `<Snippet>` or `<FeatureSection>` (a feature's `about.md`, examples and `details.md`) and has a `## Do and don't` heading with a list under it, and no language or type
 * feature is uncovered.
 */
export function syntaxProblems(world: World, root: string = ROOT): string[] {
  const problems: string[] = [];
  for (const page of pages(root)) {
    if (!page.key.startsWith("syntax/") || page.key === SYNTAX_INDEX) continue;
    const at = `${DOCS}/${page.key}`;
    if (!page.covers?.length) problems.push(`${at}: covers no feature`);
    if (!/<(Snippet\s+src|FeatureSection\s+examples)=/.test(page.body)) problems.push(`${at}: shows no <Snippet> or <FeatureSection>`);
    if (!DO_DONT_RE.test(page.body)) problems.push(`${at}: has no \`## Do and don't\` heading with a list under it`);
  }
  for (const id of staleness(world, root).uncovered) problems.push(`uncovered: ${id}`);
  return problems;
}

/**
 * The sections of Guides, in sidebar order: the directory under `guides/`, the fewest pages it has beside
 * its `index.mdx`, and whether every one of those pages shows a `<Snippet>`.
 */
export const GUIDE_SECTIONS: [dir: string, least: number, snippets: boolean][] = [
  ["why-novis", 1, false],
  ["tour", 1, false],
  ["simple-programs", 6, true],
  ["how-to-use", 1, false],
  ["cookbook", 10, true],
  ["testing", 1, false],
  ["production", 1, false],
  ["example-apps", 1, false],
  ["extensions", 1, false],
];

/**
 * The `guides` part: each of `GUIDE_SECTIONS` is a directory under `guides/` with at least its count of
 * pages beside its landing page, every such page of a snippet section shows a `<Snippet>`, and the
 * Guides sidebar group links the sections in that order.
 */
export function guidesProblems(root: string = ROOT): string[] {
  const problems: string[] = [];
  const all = pages(root);
  for (const [dir, least, snippets] of GUIDE_SECTIONS) {
    const prefix = `guides/${dir}/`;
    const inside = all.filter((p) => p.key.startsWith(prefix));
    const steps = inside.filter((p) => p.key !== `${prefix}index.mdx` && p.key !== `${prefix}index.md`);
    if (inside.length === 0) problems.push(`${DOCS}/${prefix}: no page`);
    else if (steps.length < least) problems.push(`${DOCS}/${prefix}: ${steps.length} page(s) beside its index, and the goal names ${least} at least`);
    if (snippets) for (const p of steps) if (!/<Snippet\s+src=/.test(p.body)) problems.push(`${DOCS}/${p.key}: shows no <Snippet>`);
  }
  problems.push(...sidebarOrder(root, "Guides", "guides", GUIDE_SECTIONS.map(([dir]) => dir)));
  return problems;
}

/** The example apps' directory: each directory in it is one app, with `nvs.toml`, `public/index.nvs` and
 * its classes and their tests under `src/`. */
export const APPS = "apps";
/** How long an app's server has to answer `/` with a `200` once started. The debug build compiles every
 * file before it binds the socket. */
const BOOT_MS = 60_000;

/** A TCP port nothing listens on now, which the operating system picked. */
function freePort(): Promise<number> {
  return new Promise((resolve, reject) => {
    const server = createServer();
    server.once("error", reject);
    server.listen(0, "127.0.0.1", () => {
      const address = server.address();
      const port = typeof address === "object" && address !== null ? address.port : 0;
      server.close(() => resolve(port));
    });
  });
}

/** Where an app's copies are, one directory per app: see § *An app's copies* in the head comment. */
export const APP_FILES = "website/public/example-apps";

/** Each app's directory name under `APPS`, sorted. */
function appDirs(root: string): string[] {
  const dirs = existsSync(join(root, APPS)) ? readdirSync(join(root, APPS)).filter((d) => statSync(join(root, APPS, d)).isDirectory()) : [];
  return dirs.sort(comparePaths);
}

/** The names of an app's copies, the files directly in `APP_FILES/<app>/`, sorted. */
export function appFiles(app: string, root: string = ROOT): string[] {
  const dir = join(root, APP_FILES, app);
  return existsSync(dir) ? readdirSync(dir).filter((f) => statSync(join(dir, f)).isFile()).sort(comparePaths) : [];
}

/** The problem with one copy, given the status and body the app's `GET /<file>` returned and the bytes on
 * disk, or null when they are the same. */
export function appFileProblem(app: string, file: string, status: number, body: Uint8Array, stored: Uint8Array): string | null {
  const at = `${APP_FILES}/${app}/${file}`;
  if (status !== 200) return `\`${at}\`: \`GET /${file}\` returned ${status || "no response"}`;
  if (Buffer.compare(body, stored) !== 0) return `\`${at}\` is not what \`GET /${file}\` returns now: run \`bun nv site --bless-apps\``;
  return null;
}

/** Runs `body` with the app's directory, and then deletes every file the run left there that was not
 * there before (the app's database). */
async function inApp<T>(root: string, app: string, body: (cwd: string) => Promise<T>): Promise<T> {
  const before = new Set(filesUnder(root, app));
  try {
    return await body(join(root, app));
  } finally {
    for (const f of filesUnder(root, app)) if (!before.has(f)) rmSync(join(root, f), { force: true });
  }
}

/**
 * Starts `nvs serve public/index.nvs` in `cwd` on a free port, waits until it answers `GET /` with a `200`
 * within `BOOT_MS`, runs `use` with the port, and stops the server. Returns why the server did not
 * answer, or null when it did and `use` ran.
 */
async function withServer(nvs: string, cwd: string, use: (port: number) => Promise<void>): Promise<string | null> {
  const port = await freePort();
  const stop = new AbortController();
  const serve = runProc([nvs, "serve", "public/index.nvs", "--port", String(port)], { cwd, signal: stop.signal, env: { NOVIS_NO_INIT: "1" } });
  let status = 0;
  let exited = false;
  void serve.then(() => (exited = true));
  const until = Date.now() + BOOT_MS;
  while (status !== 200 && !exited && Date.now() < until) {
    try {
      status = (await fetch(`http://127.0.0.1:${port}/`, { redirect: "manual" })).status;
    } catch {
      await Bun.sleep(250);
    }
  }
  try {
    if (status === 200) await use(port);
  } finally {
    stop.abort();
  }
  const served = await serve;
  if (status === 200) return null;
  return exited && !served.aborted ? `the server exited ${served.code}: ${served.stderr.trim().split("\n").slice(-5).join(" / ")}` : status ? `\`/\` returned ${status}` : `no answer within ${BOOT_MS / 1000} s`;
}

/** The status and body of `GET /<file>` on a running app, and status 0 when the connection failed. */
async function getFile(port: number, file: string): Promise<{ status: number; body: Uint8Array }> {
  try {
    const res = await fetch(`http://127.0.0.1:${port}/${encodeURIComponent(file)}`, { redirect: "manual" });
    return { status: res.status, body: new Uint8Array(await res.arrayBuffer()) };
  } catch {
    return { status: 0, body: new Uint8Array() };
  }
}

const serveFailed = (app: string, why: string) => `${app}: \`nvs serve public/index.nvs\` did not answer \`GET /\` with 200: ${why}`;

/**
 * The `apps` part: every app under `APPS` passes `nvs test .`, `nvs serve public/index.nvs` answers
 * `GET /` with a `200` within `BOOT_MS`, and each of its copies is what the server returns now
 * (`appFileProblem`). Both run in the app's own directory, as its page tells a reader to.
 */
export async function appsProblems(nvs: string, root: string = ROOT): Promise<string[]> {
  const problems: string[] = [];
  const dirs = appDirs(root);
  if (dirs.length === 0) return [`${APPS}/: no app`];
  for (const dir of dirs) {
    const app = `${APPS}/${dir}`;
    await inApp(root, app, async (cwd) => {
      const test = await runProc([nvs, "test", "."], { cwd, timeoutMs: 10 * 60_000, env: { NOVIS_NO_INIT: "1" } });
      if (test.code !== 0) {
        const tail = (test.stdout + test.stderr).trim().split("\n").slice(-15);
        problems.push(`${app}: \`nvs test .\` exited ${test.code}`, ...tail.map((l) => `  ${l}`));
      }
      const why = await withServer(nvs, cwd, async (port) => {
        for (const file of appFiles(dir, root)) {
          const got = await getFile(port, file);
          const problem = appFileProblem(dir, file, got.status, got.body, readFileSync(join(root, APP_FILES, dir, file)));
          if (problem) problems.push(problem);
        }
      });
      if (why) problems.push(serveFailed(app, why));
    });
  }
  return problems;
}

/**
 * `--bless-apps [APP/FILE...]`: writes each app's copies, and each named `APP/FILE` beside them, from
 * what the app's server returns now. A file whose `GET` does not return a `200` is named and not written.
 */
async function blessApps(nvs: string, add: string[], root: string = ROOT): Promise<number> {
  const wanted = new Map(appDirs(root).map((dir) => [dir, new Set(appFiles(dir, root))]));
  for (const a of add) {
    const [dir = "", file, ...more] = a.split("/");
    const files = wanted.get(dir);
    if (files === undefined || !file || more.length) {
      console.error(`nv site: ${a} is not APP/FILE for an app under ${APPS}/`);
      return 2;
    }
    files.add(file);
  }
  let failed = 0;
  for (const [dir, files] of wanted) {
    if (files.size === 0) continue;
    const app = `${APPS}/${dir}`;
    const why = await inApp(root, app, (cwd) =>
      withServer(nvs, cwd, async (port) => {
        for (const file of [...files].sort(comparePaths)) {
          const at = `${APP_FILES}/${dir}/${file}`;
          const got = await getFile(port, file);
          if (got.status !== 200) {
            failed++;
            console.log(`nv site: \`GET /${file}\` on ${app} returned ${got.status || "no response"}, so ${at} is not written`);
            continue;
          }
          mkdirSync(dirname(join(root, at)), { recursive: true });
          writeFileSync(join(root, at), got.body);
          console.log(`nv site: wrote ${at} (${got.body.length} bytes)`);
        }
      }),
    );
    if (why) {
      failed++;
      console.log(`nv site: ${serveFailed(app, why)}`);
    }
  }
  return failed ? 1 : 0;
}

async function world(nvs: string): Promise<World> {
  const meta = await metaJson(nvs);
  return { entries: await roster(nvs, meta), meta };
}

function report(title: string, items: string[]): void {
  if (items.length === 0) return;
  console.log(`${title} (${items.length}):`);
  for (const item of items) console.log(`  ${item}`);
}

async function check(parts: Part[], nvs: string | null): Promise<number> {
  let failed = 0;
  let w: World | null = null;
  for (const part of parts) {
    let problems: string[];
    if (part === "structure") problems = structureProblems().concat(styleProblems(ROOT));
    else if (part === "guides") problems = guidesProblems();
    else if (part === "tools") problems = toolsProblems();
    else if (part === "in-depth") problems = inDepthProblems();
    else if (part === "roadmap") problems = roadmapProblems(ROOT);
    else if (part === "prose") {
      problems = pages()
        .filter((p) => !LEGAL.includes(p.key))
        .flatMap((p) => {
          const offset = p.text.split("\n").length - p.body.split("\n").length;
          return proseProblems(p.body).map((x) => `${DOCS}/${p.key}:${Number(x.split(":")[0]) + offset}:${x.slice(x.indexOf(":") + 1)}`);
        })
        .concat(diagramProblems(ROOT, DOCS, proseProblems));
    } else if (nvs === null) problems = ["no `nvs` binary: build one, or pass --nvs"];
    else if (part === "snippets") problems = await snippetProblems(nvs);
    else if (part === "apps") problems = await appsProblems(nvs);
    else if (part === "shots") problems = await shotsProblems(nvs);
    else if (part === "reference") {
      w ??= await world(nvs);
      problems = await referenceProblems(w, nvs);
    } else if (part === "syntax") {
      w ??= await world(nvs);
      problems = syntaxProblems(w);
    } else if (part === "questions") {
      w ??= await world(nvs);
      problems = questionProblems(w);
    } else {
      w ??= await world(nvs);
      const s = staleness(w);
      problems = [...s.unlisted.map((p) => `${p}: has no \`covers:\` line`), ...s.broken.map((b) => `broken: ${b}`), ...s.stale.map((x) => `stale: ${x}`)];
    }
    if (problems.length === 0) console.log(`site ${part}: ok`);
    else {
      failed++;
      report(`site ${part}: failed`, problems);
    }
  }
  return failed ? 1 : 0;
}

/** `--questions --status | --owed [AREA] [--limit N] | --stamp URL...`. */
function questionsMode(w: World, operands: string[]): number {
  const [sub, ...args] = operands;
  if (sub === "--status") {
    for (const line of questionCounts(w)) console.log(line);
    return 0;
  }
  if (sub === "--owed") {
    const at = args.indexOf("--limit");
    const limit = at >= 0 ? Number(args[at + 1]) : Infinity;
    const area = (at >= 0 ? [...args.slice(0, at), ...args.slice(at + 2)] : args)[0];
    if (area !== undefined && !(QUESTION_AREAS as readonly string[]).includes(area)) {
      console.error(`nv site: no area ${area}; the areas are ${QUESTION_AREAS.join(", ")}`);
      return 2;
    }
    const s = questionStatus(w);
    const due = [...s.stale, ...s.owed].filter((x) => area === undefined || x.area === area).sort((a, b) => comparePaths(a.url, b.url));
    for (const x of due.slice(0, limit)) console.log(`${questionMaterial(w, x)}\n`);
    console.log(`nv site: ${Math.min(due.length, limit)} of ${due.length} page(s) owe questions`);
    return 0;
  }
  if (sub === "--stamp") {
    if (args.length === 0) {
      console.error("nv site: --questions --stamp needs at least one URL");
      return 2;
    }
    const problems = stampQuestions(w, args);
    for (const p of problems) console.log(`nv site: ${p}`);
    if (problems.length === 0) console.log(`nv site: stamped ${args.length} page(s) in ${QUESTIONS}`);
    return problems.length ? 1 : 0;
  }
  console.error("nv site: --questions needs --status, --owed or --stamp");
  return 2;
}

async function build(): Promise<number> {
  const out = await runProc(["node", "node_modules/astro/bin/astro.mjs", "build"], { cwd: join(ROOT, "website"), timeoutMs: 30 * 60_000 });
  if (out.code !== 0) {
    console.log(out.stdout.trim().split("\n").slice(-30).join("\n"));
    console.log(out.stderr.trim());
    console.log(`site: the build failed with exit ${out.code}`);
    return 1;
  }
  console.log("site: built");
  return 0;
}

export async function run(args: string[]): Promise<number> {
  const at = args.indexOf("--nvs");
  const explicit = at >= 0 ? args[at + 1] : undefined;
  const rest = at >= 0 ? [...args.slice(0, at), ...args.slice(at + 2)] : args;
  const [mode, ...operands] = rest;
  if (mode === undefined || mode === "-h" || mode === "--help") {
    console.log(summary);
    return mode === undefined ? 2 : 0;
  }
  if (mode === "--build") return build();
  if (mode === "--roadmap" && operands[0] === "--retired") {
    const { said, ok } = pruneRetired(ROOT);
    console.log(`nv site: ${said}`);
    return ok ? 0 : 1;
  }
  if (mode === "--roadmap") {
    const problem = writeRoadmap(ROOT);
    console.log(problem === null ? "nv site: wrote the roadmap into README.md" : `nv site: ${problem}`);
    return problem === null ? 0 : 1;
  }
  const nvs = binary(explicit);
  try {
    if (mode === "--bless") {
      if (operands.length === 0) {
        console.error("nv site: --bless needs at least one snippet");
        return 2;
      }
      if (nvs === null) {
        console.error("nv site: no `nvs` binary: build one, or pass --nvs");
        return 1;
      }
      return await blessSnippets(nvs, operands.map((p) => rel(resolve(p))));
    }
    if (mode === "--bless-apps") {
      if (nvs === null) {
        console.error("nv site: no `nvs` binary: build one, or pass --nvs");
        return 1;
      }
      return await blessApps(nvs, operands);
    }
    if (mode === "--shoot") {
      if (nvs === null) {
        console.error("nv site: no `nvs` binary: build one, or pass --nvs");
        return 1;
      }
      return await shoot(nvs, operands.map((p) => rel(resolve(p))));
    }
    if (mode === "--check") {
      const unknown = operands.filter((p) => !(PARTS as readonly string[]).includes(p));
      if (unknown.length) {
        console.error(`nv site: no part ${unknown.join(", ")}; the parts are ${PARTS.join(", ")}`);
        return 2;
      }
      return await check((operands.length ? operands : [...PARTS]) as Part[], nvs);
    }
    if (mode === "--questions") {
      if (nvs === null) {
        console.error("nv site: no `nvs` binary: build one, or pass --nvs");
        return 1;
      }
      return questionsMode(await world(nvs), operands);
    }
    if (mode !== "--stale" && mode !== "--stamp") {
      console.error(`nv site: no mode ${mode}\n${summary}`);
      return 2;
    }
    if (nvs === null) {
      console.error("nv site: no `nvs` binary: build one, or pass --nvs");
      return 1;
    }
    const w = await world(nvs);
    if (mode === "--stamp") {
      if (operands.length === 0) {
        console.error("nv site: --stamp needs at least one page");
        return 2;
      }
      const problems = stamp(w, operands);
      for (const p of problems) console.log(`nv site: ${p}`);
      if (problems.length === 0) console.log(`nv site: stamped ${operands.length} page(s) in ${LOCK}`);
      return problems.length ? 1 : 0;
    }
    const s = staleness(w);
    report("pages with no `covers:` line", s.unlisted);
    report("broken: a covered id the roster does not have", s.broken);
    report("stale: reread the page, then `bun nv site --stamp <page>`", s.stale);
    report("uncovered: a language or type feature no Syntax page covers", s.uncovered);
    if (!s.unlisted.length && !s.broken.length && !s.stale.length && !s.uncovered.length) console.log("nv site: nothing is stale");
    return s.unlisted.length || s.broken.length || s.stale.length ? 1 : 0;
  } catch (e) {
    if (!(e instanceof RosterError)) throw e;
    console.error(`nv site: ${e.message}`);
    return 1;
  }
}
