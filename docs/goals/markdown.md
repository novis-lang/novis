---
looper: 1
title: "Markdown in Core: safe, fast, both directions, free of the bugs others shipped"
summary: "Reached when Core\\Markdown converts Markdown to HTML whole, in chunks and as it arrives, and HTML to Markdown, every catalogued bug of the major Markdown libraries is a passing test, every CommonMark and GFM spec example passes, rendering in chunks gives the same HTML as the whole document, and work grows linearly over every catalogued slow input."
context: [crates/nvs-stdlib/src/html.rs, docs/rules/core-classes/html-sanitize.md, docs/rules/core-classes/html-parsing.md]
stages:
  - id: 1
    title: "the known-bug catalogue and the spec examples are in the tree"
    status: done
    context: [tests/hostile/README.md]
  - id: 2
    title: "the engine is chosen by the catalogue, and the rules are written"
    status: done
    context: [docs/rules/core-classes/json-whole-and-stream.md, docs/rules/core-api/one-paradigm-per-operation.md, docs/rules/packaging/search-sorts-by-the-readers-choice.md]
  - id: 3
    title: "Core\\Markdown::toHtml renders a whole document safely and in linear time"
    status: done
    context: [crates/nvs-stdlib/src/lib.rs, docs/rules/core-api/reference-card.md]
  - id: 4
    title: "a program changes links, images, headings and code blocks through callbacks"
    status: done
  - id: 5
    title: "Core\\Markdown::toHtmlEach renders a document of any size one block group at a time"
    status: done
    context: [crates/nvs-stdlib/src/byte_source.rs]
  - id: 6
    title: "Core\\Markdown\\Live renders text that arrives in pieces"
    status: done
  - id: 7
    title: "Core\\Markdown::fromHtml writes Markdown from HTML"
    status: done
    context: [crates/nvs-stdlib/src/xml.rs]
  - id: 8
    title: "fromHtml reads a large document without holding the whole tree"
    status: done
  - id: 9
    title: "two fuzz targets, and a differential fuzz against a second engine"
    status: done
    context: [fuzz/Cargo.toml]
  - id: 10
    title: "the four members carry their feature proofs"
    status: done
    context: [benches/members/README.md, docs/rules/testing/feature-proofs.md, docs/agent/prose.md]
  - id: 11
    title: "the extension guide shows a crate Core does not ship"
    status: done
    context: [website/guides/highlight-extension/README.md, website/src/content/docs/guides/extensions/rust.mdx]
  - id: 12
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: catalogue
    stage: 1
    name: the known-bug catalogue covers every library decision 3 lists, and every entry is complete
    run: [cargo, test, --test, markdown_known_bugs]
    timeout: 1800
    tests:
      - the_catalogue_covers_every_library_the_goal_lists
      - every_catalogue_entry_has_a_source_a_class_an_input_and_an_expectation
  - id: to-html
    stage: 3
    name: every spec example and every catalogued bug renders as expected, the output is a sanitize fixed point, and work is linear
    run: [cargo, test, --lib, "markdown::"]
    timeout: 1800
    tests:
      - every_commonmark_spec_example_renders_as_the_spec_says
      - every_gfm_spec_example_renders_as_the_spec_says
      - sanitizing_the_output_again_changes_nothing
      - work_grows_linearly_over_every_catalogued_slow_input
      - nesting_past_512_flattens_as_the_html_parser_does
      - added_output_past_its_budget_renders_as_source_text
  - id: known-bugs
    stage: 3
    name: every catalogued bug of another library is a passing test here
    run: [cargo, test, --test, markdown_known_bugs]
    timeout: 1800
    tests:
      - every_known_bug_renders_as_the_catalogue_expects
  - id: callbacks
    stage: 4
    name: a callback changes what it is given, and its result passes the same checks as the document
    run: [cargo, test, --lib, "markdown::"]
    timeout: 1800
    tests:
      - a_link_callback_cannot_produce_a_javascript_url
      - an_image_callback_returning_null_removes_the_image
      - a_heading_callback_sets_the_id
      - a_code_callback_replaces_a_fenced_block
  - id: chunks
    stage: 5
    name: rendering in chunks gives the same HTML as the whole document, and a file costs one chunk
    run: [cargo, test, --lib, "markdown::"]
    timeout: 1800
    tests:
      - rendering_in_chunks_gives_the_same_html_as_the_whole_document
      - to_html_each_holds_one_block_group_of_a_file_not_the_document
      - an_unclosed_raw_html_element_keeps_its_blocks_in_one_group
  - id: live
    stage: 6
    name: what Live finished plus finish() is the whole document's HTML
    run: [cargo, test, --lib, "markdown::"]
    timeout: 1800
    tests:
      - live_done_parts_and_finish_join_to_the_whole_document_html
      - live_never_changes_a_part_it_already_finished
  - id: from-html
    stage: 7
    name: HTML text never becomes Markdown syntax, and the round trips keep the meaning
    run: [cargo, test, --lib, "markdown::"]
    timeout: 1800
    tests:
      - from_html_text_never_becomes_markdown_syntax
      - markdown_to_html_to_markdown_keeps_the_meaning
      - from_html_twice_gives_the_same_markdown
      - a_table_markdown_cannot_hold_is_written_as_sanitized_html
      - a_code_block_with_backticks_gets_a_longer_fence
  - id: from-html-stream
    stage: 8
    name: fromHtml over a large document gives the same Markdown as over the whole tree
    run: [cargo, test, --lib, "markdown::"]
    timeout: 1800
    tests:
      - from_html_streamed_gives_the_same_markdown_as_the_whole_tree
  - id: guide
    stage: 11
    name: the extension guide's project builds and its tests pass
    run: [bun, nv, ext-templates, --example]
    timeout: 1800
---

# Markdown in Core: safe, fast, both directions, free of the bugs others shipped

## Why

The user decided on 2026-11-06 that Novis gets best-in-class Markdown support: fast, safe, reliable,
stable and flexible, and it handles huge files without performance problems. It converts in both
directions. Markdown → HTML is the common need: content sites, comments, READMEs, and every backend
that shows LLM output. HTML → Markdown is the newer need: agents ask for pages as Markdown
(`Accept: text/markdown`), and Novis already has a WHATWG HTML parser to build it on. Editing a
Markdown AST is out of scope for this goal.

The research of that day found that every ecosystem has Markdown libraries, and that the pain lies
elsewhere:

- **XSS by default.** Parsers pass raw HTML and `javascript:` links through, and maintainers dispute
  the CVEs as "the caller's job" (markdown-it CVE-2025-7969, cebe/markdown CVE-2018-1000874). Every
  Laravel guide says to set `html_input: strip` and `allow_unsafe_links: false` by hand.
- **Super-linear time on hostile input.** marked has several ReDoS CVEs, league/commonmark had a
  high-severity one fixed in 2.9.1, and cmark-gfm published quadratic-time advisories. marked's advice
  is a worker with a timeout.
- **Flavour drift.** The same text renders differently between tools. GitLab moved to CommonMark
  because of it.
- **Streaming LLM output.** Shopify, Spotify and Streak each wrote a buffering parser of their own.

The user's condition, also from 2026-11-06: **none of the bugs the other libraries already shipped,
from the first release.** So the goal starts with a catalogue of those bugs, and each one becomes a
passing test before any member ships. Whole classes of bug are removed by construction (decision 5),
not one input at a time.

The repository already waits for this goal. `rule:packaging/search-sorts-by-the-readers-choice`
declines rendered readmes because `Core` has no Markdown renderer
(`docs/rules/packaging/search-sorts-by-the-readers-choice.md:61`), and `docs/rules/security.md:2762`
names a Markdown renderer as a sink.

## Standing decisions

1. **The members.** One class, `Core\Markdown`, plus one object for live input:
   - `Core\Markdown::toHtml(string $markdown, ...callbacks): Core\Html\Markup` renders a whole document.
   - `Core\Markdown::toHtmlEach(Core\Bytes::Source|Core\Bytes::Chunks $markdown, ...callbacks):
     iterable<Core\Html\Markup>` renders one block group at a time, on the model of
     `Core\Json::decodeEach` (`rule:core-classes/json-whole-and-stream`).
   - `Core\Markdown\Live` is for text that arrives in pieces, such as LLM output. `push(string $piece)`
     returns an object with `done: Markup` (blocks that are finished; append them) and `open: Markup`
     (the unfinished last block; replace it). `finish(): Markup` closes the document. It is an object
     because it has a lifetime (`rule:core-api/a-lifetime-is-an-object`).
     **Built (stage 6) as** `Core\Markdown::live(...callbacks): Core\Markdown\Live`, whose `push`
     returns a `Core\Markdown\Update` with members `done()` and `open()`. A `Core` instance has no
     property a program can read and a `Core` member cannot return a shape, so the two parts are
     members; a static factory creates the object, as `Core\Hash::stream` does. `open` is everything
     not yet in a finished group, rendered as if the text ended there, so it may hold a raw-HTML group
     still open as well as the last block.
   - `Core\Markdown::fromHtml(string|Core\Html\Markup|Core\Xml\Node $html): string` writes Markdown.
   - No member takes an options bag. Stage 2 writes these names into the rules. A rename the rule
     review forces is allowed, and it is recorded there.
2. **One flavour: CommonMark plus GFM**, which means tables, task lists, strikethrough, extended
   autolinks and footnotes. GFM's tag filter is not needed, because raw HTML goes through the sanitizer
   (decision 5). Front matter, math and admonitions are not parsed: CommonMark's own result is the
   output, and a fenced block with a language reaches the `code` callback (decision 6).
3. **The known-bug catalogue** is `tests/data/markdown/known-bugs.toml`. It covers these libraries:
   cmark, cmark-gfm, commonmark.js, markdown-it, marked, pulldown-cmark, comrak, md4c,
   league/commonmark, Parsedown, Python-Markdown, mistune, and for the other direction Turndown and
   html-to-markdown. Its sources are each library's published security advisories, its issues labelled
   security, crash, performance or regression, and its own regression test files. Each entry has the
   library, the public advisory or issue URL, the bug class, a minimal input, the direction, and the
   expected result: HTML, Markdown, or a bound on work. A public advisory is not a private report, so
   naming the library and the URL is allowed. Stage 1 collects it with `gh api` and `git clone` into
   `.agent-tmp/`. If the session cannot reach the network, it says so in the handoff and the catalogue
   is filled in the next interactive session. That is the one case this goal stops for. The Rust test
   `crates/nvs-stdlib/tests/markdown_known_bugs.rs` reads the file, so the catalogue is the source of
   truth and the test fails naming each entry that does not pass.
4. **The engine** is chosen in stage 2 by measurement over the catalogue and the spec examples, in this
   order: (a) catalogue entries it fails that cannot be fixed by a clean patch, (b) linear work on every
   catalogued slow input, (c) it can resolve a reference defined outside the text it parses, which
   decision 7 needs, (d) speed, (e) simplicity. The candidates are `pulldown-cmark` and `comrak`. The
   safe default is `pulldown-cmark`: it is pure Rust, fuzzed, used by rustdoc, its event stream fits the
   callbacks and the chunked output, and its broken-link callback resolves outside references. A GFM
   part the engine lacks, such as extended autolinks, is written in our layer over its events. If the
   engine needs patches, it is vendored under `vendor/` with an entry in `[patch.crates-io]`
   (`Cargo.toml:1295`), as `html5ever` is. Writing our own parser is allowed only if the catalogue shows
   that neither candidate can be patched to pass. The licence must be one `cargo deny` already allows.
5. **Bug classes are removed by construction.** Each line is written into a rule in stage 2:
   - **Raw HTML** in Markdown is never stripped and never passed through. The document's HTML goes
     through `Core\Html::sanitize`'s rebuild. A document with no raw HTML skips the rebuild, because the
     renderer writes only elements on the sanitizer's list. The property test
     `sanitizing_the_output_again_changes_nothing` proves that skip safe. `Core\Markdown` becomes a
     launderer beside `sanitize`, and `rule:core-classes/html-sanitize` is updated to say so.
   - **URLs** are decoded the way a browser decodes them (entities, percent-encoding, tab, line feed and
     carriage return removed). The result is checked against the sanitizer's scheme list and its
     scheme-relative rule. One function serves both members.
   - **Attributes** are never built by joining strings. The output is a tree, serialized by the same
     code `Core\Html` uses.
   - **Nesting** deeper than 512 flattens exactly as `rule:core-classes/html-parsing` flattens it. It is
     the same bound, not a new one: without it, sanitizing the output would change it.
   - **Added output** is output the renderer writes that does not come from input bytes one-for-one:
     a reference's URL and title written at each use, and the empty cells that pad a table row. One
     document may add at most its own input length plus 64 KiB. Past that, a reference renders as its
     source text and a row is not padded. No real document comes near this bound. Without it, one
     1 MB input can produce gigabytes of output.
   - **Work** is linear in the input. Tests count work steps over every catalogued slow input, as
     `parsed_counting_steps` does for HTML (`crates/nvs-stdlib/src/html.rs:2229`), and fail when work
     grows faster than the input. No timing.
   - **NUL** becomes U+FFFD, as the spec says. Invalid UTF-8 follows `rule:types/string-is-utf8`.
   - **Panics** are bugs. The parse never fails: Markdown has no invalid input.
   - **HTML → Markdown**: every text node is escaped, so text never becomes Markdown syntax. A
     construct Markdown cannot express is written as sanitized HTML (decision 8).
6. **Callbacks** make it flexible without making it unsafe. They are named parameters, each optional:
   - `link: fn(string $url): ?string` returns the new URL. `null` removes the link and keeps its text.
   - `image: fn(string $url): ?string` returns the new URL. `null` removes the image.
   - `heading: fn(int $level, string $text): ?string` returns the heading's `id`.
   - `code: fn(string $language, string $code): ?Core\Html\Markup` returns the block's HTML. `null`
     keeps the default.
   A URL a callback returns passes the same check as one from the document. A `Markup` is already
   trusted by its type, so the `code` result is used as it is. This is the one door for syntax
   highlighting, math and diagrams.
7. **Chunks and huge files.** `toHtmlEach` splits the document into top-level block groups and renders
   each group alone. The property `rendering_in_chunks_gives_the_same_html_as_the_whole_document` is the
   acceptance. A boundary is never placed inside an open fenced code block, an HTML block of types 1 to
   5, an unclosed raw HTML element, or a list that continues. A document that keeps one open (a single
   huge list) becomes one larger group: correctness first.
   - **References:** a reference may be defined after its first use. A file is read twice: once to
     collect the definitions, once to render, so memory is one block group plus the definitions. Every
     other source is read once and held whole under the request's memory cap, as a random read of an
     upload part already is.
   - **Footnotes** are written at the end, so their definitions are held until then.
   - **Memory:** a file costs one block group plus its reference and footnote definitions. Any other
     source costs the document. `Live` costs its open block plus the definitions. All of it is attributed
     to the request.
   - **Live** renders a block as soon as it is closed and never changes it afterwards. A reference
     defined after a finished block does not change that block. The rule says this, and the property
     test covers documents with no forward references.
8. **HTML → Markdown.**
   - Input is a string or `Markup` (parsed by `Core\Html::parse`) or a node already parsed.
   - Elements are kept or dropped by the sanitizer's list, so a `<script>`'s text never appears.
   - Whitespace collapses by the CSS rules for inline and block elements.
   - A GFM table is written when every cell has only inline content and nothing uses `colspan` or
     `rowspan`. Otherwise the table is written as sanitized HTML.
   - A code block's fence is one backtick longer than the longest run of backticks inside it.
   - Choosing the main content of a page (dropping nav, footer, ads) is not done. The program passes the
     element it wants.
   - A tainted input gives a tainted result. **Built (stage 7) as** a result that is always
     `tainted string`: a node's text is tainted and a result's mark cannot follow one union arm, so a
     plain result would launder a node's text. The xml sweep test now allows a member that takes a
     node and returns a `tainted string` alone, since text runs nowhere.
   - Stage 8 streams a large document. Its acceptance is that the streamed result equals the result
     over the whole tree. If the tree builder can still change an element after it is closed, stage 8
     records why and `fromHtml` keeps holding the tree. Its memory is then the document, as
     `Core\Html::parse`'s is. **Decided (stage 8): the tree is held.** The builder fosters text in
     front of a table and joins it to the text node there, so `a<table>b` changes the closed `a`
     into `ab`. The writer carries state across blocks (a list after a list changes its marker). And
     every input is already whole in memory and the result is one string, so streaming saves only the
     tree: memory, the last priority, bought with simplicity. The parse takes the document in pieces,
     and the test checks every cut gives the whole tree's Markdown. The rule's *Large documents*
     holds this, and its Declined list names per-subtree writing.
9. **Test data.** The CommonMark and GFM spec examples are vendored as `tests/data/markdown/spec/`
   with their licence file (CC-BY-SA 4.0). They are test data and are not compiled into the binary.
   The differential fuzz in stage 9 runs the other candidate from decision 4 as a dependency of the
   `fuzz` crate only. Each difference is triaged: our bug (fix it and add a catalogue entry) or a spec
   reading (one line in the rule).
10. **The extension guide** (`website/guides/markdown-extension/`) uses Markdown as its example of a
    Rust extension. Once `Core` has Markdown, that example teaches readers to rebuild a Core member. The
    guide becomes a syntax highlighter built on `syntect` with its `fancy-regex` feature, and its demo
    passes it to `Core\Markdown::toHtml`'s `code` callback. If `syntect` does not build for
    `wasm32-wasip2`, the session picks another pure-Rust crate that Core does not ship, and records the
    choice.
11. **The rules this goal writes** under `docs/rules/core-classes/`: `markdown-to-html` (flavour,
    members, launderer, callbacks), `markdown-cost` (linear work, nesting, added output, memory),
    `markdown-chunks` (block groups, references, Live), `markdown-from-html`. Each holds its reasons and
    its Declined list: an options bag, an "unsafe" switch, stripping raw HTML, a timeout or worker
    instead of linear work, main-content extraction, AST editing for now.
12. **Lanes.** Stages 1, 2 and 3 run in order. Stages 4, 5 and 6 share `crates/nvs-stdlib/src/markdown.rs`
    and run in order after 3. Stages 7 and 8 touch only `crates/nvs-stdlib/src/markdown/from_html.rs`
    and may run as a side goal once stage 2 is committed. Stage 11 touches only the guide and may run as
    a side goal at any point after stage 3. Stage 9 needs 3 and 7. Stage 10 needs 3 to 8.
13. **The gate with `byte-sources`:** stage 5 starts once `crates/nvs-stdlib/src/byte_source.rs` exists.
    The goal comes after `byte-sources` on the chain, so this holds when the goal goes live.
14. **The catalogue's expectations.** Decision 3's "HTML, Markdown, or a bound on work" is six keys,
    and an entry sets exactly one: `html` and `markdown` (exact), `html_lacks` and `markdown_lacks`
    (substrings that never appear), `work = "linear"` and `output = "bounded"` (bounds over `repeat`).
    The `_lacks` keys exist because a security entry's exact HTML depends on the sanitizer's rebuild,
    which stage 2 writes. A collector cannot know that output before the rule exists, and a guessed
    exact string would fail for a reason that is not a bug. A from-html `html` is what the produced
    Markdown renders back to, so a legitimate escaping style does not fail it. A grown input is
    `input`, then `repeat` n times, then `suffix`. Stage 3 adds a second growing part if an
    approximated entry needs it (*Backlog*). The test file's module doc and the catalogue's header
    hold the shape.
15. **The sanitizer's list grows by what Markdown's output needs** (stage 2, recorded in
    `rule:core-classes/markdown-to-html` and `html-sanitize`). CommonMark and GFM output carries
    `align`, a `language-` class, task-list checkboxes and anchors, which the list did not grant, and
    decision 5's fixed point needs the list to hold them. Each is granted with a closed value: `align`
    on `td` and `th` is `left`, `center` or `right`; `class` on `code` is `language-` and letters, digits
    and `+-_#.`; `input` only as `type="checkbox"` with `disabled` and `checked`; `id` on `h1` to `h6`,
    `a` and `li` starts with `md-`. The hyphen keeps an `id` from naming a script's global, so the
    DOM-clobbering reason for refusing `id` still holds. The `heading` callback's answer is written
    after `md-`, and footnotes use `md-fn-N` and `md-fnref-N`. Declined: a separate list for Markdown,
    which would make `sanitize` over Markdown's output change it.
16. **How the catalogue test reads an expectation** (stage 3, held in the test file's module doc).
    An `html_lacks` string searches only the output's tags (`nvs_stdlib::fuzz::html_tags`), because
    every such string names a tag, an attribute or a URL scheme, and the same characters in text are
    harmless. A `work` entry's bound lives in one place, the crate's
    `work_grows_linearly_over_every_catalogued_slow_input`, which can count the engine's steps; the
    integration test renders it once. `output = "bounded"` is four times plus the budget from `n` to
    `4n`, and 32 bytes per input byte plus the budget. A from-html entry is skipped until stage 7 sets
    `FROM_HTML_BUILT`. The engine counts steps only with debug assertions (`vendor/pulldown-cmark/src/steps.rs`),
    so release pays nothing; a loop a later patch adds to the engine calls `crate::steps::add` too.
17. **Where pulldown-cmark and cmark-gfm differ, cmark-gfm wins** (stage 9). Decision 2's flavour is
    GFM, and cmark-gfm is its reference and what GitHub renders with. So a table's header row
    interrupts a paragraph without a leading `|`, as cmark-gfm reads it, although upstream chose
    otherwise on purpose. comrak is a port of cmark-gfm, but where comrak departs from it (a `~`
    length mismatch stops all emphasis after it), the differential skips and cmark-gfm's reading is
    built. Each such patch is a catalogue entry, so the rule's patch list and the catalogue agree.
18. **The sanitizer parses in no-quirks mode** (stage 9, `rule:core-classes/html-sanitize`). A
    document with no doctype parsed in quirks mode, where `<table>` does not close a `<p>`, so
    `<p>a<table></p>` rebuilt as a `p` inside a `p` and was not a fixed point. The answer is
    written into a page with `<!DOCTYPE html>`, so `sanitize` and `Core\Markdown`'s rebuild parse
    as that page does (`html::parse_in_page`, html5ever's patched `never_quirks`), whatever doctype
    the input has. `Core\Html::parse` and `fromHtml` keep quirks mode for a page with no doctype,
    as a browser does. Declined: no-quirks everywhere, which would make `Core\Html::parse` build a
    different tree from a browser's for such a page.
19. **Decision 17 covers GFM's constructs; for a CommonMark one, CommonMark 0.31.2 and cmark win**
    (stage 9). cmark-gfm forked cmark before cmark stopped a lazy continuation line from starting an
    HTML block of type 7, so `>a\n<t>` ends its block quote, and comrak copies that. CommonMark's
    spec says type 7 cannot interrupt a paragraph, and decision 2's flavour is CommonMark plus GFM's
    extensions. So ours keeps `<t>` in the paragraph, and the differential skips such a document.
    A loose task item's box is GFM's, so it follows cmark-gfm: right after `<li>`, in front of the
    first paragraph, and `fromHtml` reads it back into that paragraph.
20. **The comrak differential's triage is stopped, and CI does not run it** (stage 9, the user's
    call, 2026-11-07). Its remaining finds were cmark-gfm conformance edge cases with no security
    value; its security finds, the two sanitizer fixed-point fixes, landed. `markdown_comrak` stays
    in the tree, run by hand with `bun nv fuzz markdown_comrak`, and leaves CI's `fuzz-smoke` leg 4,
    where its open differences would turn the deep run red at random. `tools/nv/test/fuzz-smoke.test.ts`
    names it in `BY_HAND`. `rule:core-classes/markdown-cost` says the output on unusual input may
    differ from cmark-gfm. The stage's 1,000,000-run item covers `markdown` and `markdown_from_html`.
21. **`markdown_from_html`'s round trip skips Markdown with raw HTML** (stage 9). Its first find
    after the break fix was the Markdown `<ul>`: sanitized `<ul></ul>`, written back as nothing.
    `fromHtml` unwraps a `div` and writes nothing for an empty element by design, so raw HTML can
    never come back tag for tag, and each such shape would be a find. Readable Markdown is the
    member's purpose, so the property narrows instead (`fuzz::markdown_has_raw_html`). Raw HTML
    stays covered by the target's other property, `fromHtml` stable over any input.
    `rule:core-classes/markdown-from-html` now states the unwrap and the narrowed promise. Declined:
    writing every `div` or empty element as HTML, which fills common pages' Markdown with tags.
22. **The guide's highlighter returns tokens, and Novis writes the HTML** (stage 11). `syntect`
    builds for `wasm32-wasip2` with `default-syntaxes` and `regex-fancy`, so decision 10's crate
    stands. The Rust returns `{text, kind}` tokens with a closed set of kinds, and
    `Example\Highlight\Code` writes `<span class="hl-KIND">` with `html` templates, the colours in a
    `<style>` it returns. A template escapes an attribute hole for HTML and not for CSS
    (`rule:core-classes/html-template`'s Declined list), so a `style` hole would teach readers the
    hazard. Declined: the Rust writing HTML, which a reader would then have to sanitize, and
    `sanitize` keeps no `class` on a `span`. `syntect`'s `parsing` feature needs `bincode` 1,
    unmaintained (RUSTSEC-2025-0141, not vulnerable), so `deny.toml` ignores it with a reason: only
    the guide reaches it, it decodes syntect's built-in grammars, and the guide is not shipped.

## Stages

### 1. the known-bug catalogue and the spec examples are in the tree

**Does:** turns "the bugs others shipped" into data that a test reads.

**Split:** four `general-purpose` subagents collect in parallel, one per group of libraries from
decision 3: (cmark, cmark-gfm, commonmark.js, md4c), (markdown-it, marked, mistune, Python-Markdown),
(pulldown-cmark, comrak, league/commonmark, Parsedown), (Turndown, html-to-markdown). Each returns
entries in the catalogue's shape, with no excerpts beyond the minimal input. The session merges them,
removes duplicates by bug class and input, and writes the file.

- [x] `tests/data/markdown/known-bugs.toml`, in the shape of decision 3. `tests/data/markdown/known-bugs.toml:1`
- [x] The spec examples and their licence file (decision 9). `tests/data/markdown/spec/LICENSE:1`
- [x] `markdown_known_bugs.rs` with the two stage-1 tests of the `catalogue` check. The third test is
  written now and marked `#[ignore]` until stage 3. `crates/nvs-stdlib/tests/markdown_known_bugs.rs:1`

### 2. the engine is chosen by the catalogue, and the rules are written

**Does:** settles the engine by measurement, then writes the rules before any member exists.

- [x] A scratch harness under `.agent-tmp/` runs the catalogue and the spec examples through both
  candidates, counts failures, work steps on the slow inputs, and time on a large document. Its result
  goes into `markdown-to-html`'s reasons. The harness is deleted afterwards. `.agent-tmp/markdown-engine/:1`
- [x] The engine in the workspace dependencies, vendored if it needs patches (decision 4). `Cargo.toml:1034`
- [x] The four rules of decision 11. `docs/rules/core-classes/markdown-to-html.md:1`
- [x] `rule:core-classes/html-sanitize` names `Core\Markdown` as a launderer that rebuilds through the
  same list. `docs/rules/core-classes/html-sanitize.md:1`
- [x] The Declined entry for rendered readmes says what is still missing now that `Core` renders
  Markdown, and a gap record names the registry goal that owes it.
  `docs/rules/packaging/search-sorts-by-the-readers-choice.md:61`
- [x] `bun nv rules --check` is green over the new fragments.

### 3. Core\Markdown::toHtml renders a whole document safely and in linear time

**Does:** the keystone. Every later member uses this renderer.

**What the engine measurement found:**
- pulldown-cmark's `push_html` writes `"` in text where the spec writes `&quot;`. Every CommonMark
  failure was that alone. Our serializer decides this, and the spec tests compare after reading
  `&quot;` as `"` in text, or the serializer writes `&quot;` there.
- Its real faults are the seven patches the rule lists, which the catalogue pins: `cmark-reg-655-definition-only-item`,
  `commonmark-js-reg-116-atx-closing-tabs`, `md4c-reg-292-closing-fence-trailing-tab`,
  `marked-cve-2026-41680-tab-vtab` (renders nothing), `md4c-pr-443-del-ends-destination`,
  `md4c-reg-201-lazy-indented-fence`, and the two `cmark-gfm-reg-footnote-strikethrough-uaf-*`.
- Ours, not the engine's: NUL to U+FFFD before parsing (six entries), a leading BOM
  (`pulldown-cmark-issue-1156`), unused footnotes not written (`cmark-gfm-reg-adjacent-unused-footnotes-uaf`),
  no doubled `language-` (`commonmark-js-reg-277-language-prefix`), an empty `tbody` not written,
  extended autolinks (11 GFM examples and `cmark-gfm-cve-2023-22485-validate-protocol-oob`), and the
  added-output budget (9 of 10 `output-blowup` entries fail in both engines).
- Nine GFM 0.29 emphasis examples disagree with CommonMark 0.31.2, and both engines follow 0.31.2.
  The rule says the GFM test takes CommonMark's output for those nine.
- Four `attribute-injection` entries (`mistune-cve-2026-44897-heading-id`,
  `league-commonmark-ghsa-f8fg-pg57-v4j8-onclick` and `-onerror`, `parsedown-issue-210`) have
  `html_lacks` strings with a `"`. Those strings may match safe text, because `Core\Html` writes `"`
  as it is in text (`crates/nvs-stdlib/src/html.rs:1819`). Stage 3 checks such an entry against the
  attributes of the parsed output, or turns it into an exact `html`. A stage 3 session must not
  delete it.

Stage 1's catalogue: `tests/data/markdown/known-bugs.toml` has 508 entries over all fourteen libraries,
and no library has fewer than 21. The classes are 190 slow, 114 wrong-output, 72 crash, 48 url-scheme,
26 deep-nesting, 19 markdown-injection, 17 attribute-injection, 12 xss and 10 output-blowup. The
merge dropped 22 duplicates by class, direction and input. The spec texts are vendored unchanged:
`commonmark-0.31.2.txt` with 652 examples and `gfm-0.29.txt` with 672, of which 2 are `disabled`.
Each example starts with a line of 32 backticks and `example`, and a GFM one may add `table`,
`strikethrough`, `autolink` or `tagfilter`. `→` in an example stands for a tab. The `catalogue`
check is green: `cargo test --test markdown_known_bugs` runs two tests and ignores
`every_known_bug_renders_as_the_catalogue_expects`, whose `outcome` stub stage 3 fills.

**The collectors' caveats**, to judge as entries fail:
- No `html` was checked against a renderer. Each came from an upstream test file pinned to a commit,
  or from the spec. md4c's outputs were rewritten into cmark's format. Two entries are the least sure:
  `pulldown-cmark-issue-398` (a blank line was added to its input) and `comrak-back-to-back-ranges`.
- Some ReDoS and advisory inputs are built from the regex or the description, because no exploit
  string was published: marked's CVE-2015-8854, GHSA-ch52, GHSA-7m7q and GHSA-wjmf, mistune's
  CVE-2022-34749, and Turndown's issues 501, 505, 429 and 58. They still hold a linear bound.
- An exploit with two parts that grow together, such as `[` n times then `]` n times, is approximated
  by one `repeat` and a fixed `suffix`.
- Four from-html `url-scheme` entries cite the converters' link code, pinned to a commit, because
  neither library has a report about it.

- [x] `crates/nvs-stdlib/src/markdown.rs` with `toHtml`, registered beside `html` and with its reference
  card. `crates/nvs-stdlib/src/lib.rs:162`
  - [x] The class row at rank 1045, its card, `address` and the `classes.rs` line.
  - [x] `Renderer` builds `Parsed` from the engine's events with cmark's newlines.
  - [x] Ours over the events: `prepared`, unused footnotes, one `language-`, no empty `tbody`.
  - [x] Extended autolinks in `markdown/autolink.rs`, linear by `MAX_DOMAIN` and one paren count.
- [x] `pulldown-cmark` vendored under `vendor/` with an entry in `[patch.crates-io]`, and the
  patches of `rule:core-classes/markdown-to-html`'s engine paragraph, each with a catalogue entry
  that pins it. `Cargo.toml:1314`
  - [x] 0.13.6's library copied to `vendor/pulldown-cmark/`, with a trimmed manifest and its licence.
  - [x] The `[patch.crates-io]` row, and `nvs-stdlib` takes the crate.
  - [x] The seven patches, each with a `// Novis:` comment that names its catalogue entry.
  - [x] Each pinning input renders as its entry says, and the spec results are unchanged:
    CommonMark 0 failures, GFM 21 (9 emphasis, 11 extended autolinks, 1 tag filter).
  - [x] `THIRD-PARTY-LICENSES.txt` regenerated.
- [x] The sanitizer's list grows by decision 15's five value-checked entries, with a test each.
  `crates/nvs-stdlib/src/html.rs:1860`
  - [x] `CHECKED` beside `ELEMENTS`: the element, the attribute and its value test.
  - [x] `input` on the list, written only as a disabled checkbox (`inert_checkbox`).
  - [x] One test per entry, each also asserting the rebuild stays a fixed point.
  - [x] The mXSS corpus test counts an attribute that passed its value test as granted.
- [x] Raw HTML through the sanitizer's rebuild, URLs through the shared check, output as a tree
  (decision 5). `crates/nvs-stdlib/src/html.rs:1671`
- [x] The nesting flatten and the added-output budget (decision 5). `crates/nvs-stdlib/src/markdown.rs:1`
- [ ] The six tests of the `to-html` check. The spec tests read the vendored files.
  `crates/nvs-stdlib/src/markdown.rs:1`
  - [x] The two spec tests, with the nine emphasis examples, the tag filter and the five
    autolinked examples as `rule:core-classes/markdown-to-html` reads them.
  - [x] `sanitizing_the_output_again_changes_nothing`, which found that `rebuilt` kept leading
    whitespace a second parse drops (`without_leading_whitespace`).
  - [x] `nesting_past_512_flattens_as_the_html_parser_does` and
    `added_output_past_its_budget_renders_as_source_text`.
  - [x] `work_grows_linearly_over_every_catalogued_slow_input`, counting the engine's steps.
    - [x] `vendor/pulldown-cmark/src/steps.rs`, and a step in each engine loop and scan primitive.
    - [x] `nvs_stdlib::fuzz::markdown_counting_steps` adds one step per byte written.
    - [x] The test over every to-html `work` entry at `n` and `4n`: all between 3.25 and 4.3.
- [x] The catalogue test is no longer ignored, and every entry passes. An entry that fails is fixed in
  the code, never removed from the catalogue. `crates/nvs-stdlib/tests/markdown_known_bugs.rs:1`
  - [x] `outcome` for `html`, `html_lacks`, `output` and `work` (decision 16).
  - [x] The `#[ignore]` removed, and the two failures judged: md4c's 271 now expects cmark's space,
    and `vbscript:` in a non-link is text.
- [x] `docs/reference/core/Markdown.md` names the class. `docs/reference/core/Markdown.md:1`

### 4. a program changes links, images, headings and code blocks through callbacks

- [x] The four callbacks of decision 6, with their results checked. `crates/nvs-stdlib/src/markdown.rs:1`
  - [x] The registry row's four optional callables, and a card line for each.
  - [x] `Callbacks` in `crates/nvs-stdlib/src/markdown/callbacks.rs`, called from the renderer.
  - [x] Returned URLs through `url_allowed`, an `id` through `markdown_id`, a `code` result spliced.
  - [x] The rule and `docs/reference/core/Markdown.md` say what each callback sees.
- [x] The four tests of the `callbacks` check. `crates/nvs-stdlib/src/markdown.rs:1`
  - [x] The four unit tests over a `Hooks` of Rust closures, each run once by name.
  - [x] A conformance case for the call path from Novis.

### 5. Core\Markdown::toHtmlEach renders a document of any size one block group at a time

**Gate:** `crates/nvs-stdlib/src/byte_source.rs` exists (decision 13).

- [x] The block-group splitter and its boundary rules (decision 7). `crates/nvs-stdlib/src/markdown.rs:1`
  - [x] `Renderer` borrows the text and the callbacks, and everything it builds is a `State`.
  - [x] `group_ends` and `take_group` cut the one walk at each top-level end where nothing is open.
  - [x] `groups::Balance` reads raw HTML and keeps a group open while an element it opened is open.
  - [x] `groups::Written` rebuilds a later group behind the first group's doctype and `<body>`.
- [x] `toHtmlEach` over a source: a file read twice, any other source held whole.
  `crates/nvs-stdlib/src/byte_source.rs:1`
  - [x] An engine patch: a `Parser` takes the definitions read outside its text, and they win.
  - [x] A file is read in segments cut before each segment's last top-level block, twice.
  - [x] The walk is filed on the request, as `decodeEach`'s is, with its class and its row.
  - [x] The conformance case and the reference page's section.
- [x] The three tests of the `chunks` check. The equality test runs over every spec example and every
  catalogue entry. `crates/nvs-stdlib/src/markdown.rs:1`
  - [x] `rendering_in_chunks_gives_the_same_html_as_the_whole_document` passes.
  - [x] `an_unclosed_raw_html_element_keeps_its_blocks_in_one_group` passes.
  - [x] `to_html_each_holds_one_block_group_of_a_file_not_the_document` passes, over bytes read as a
    file is.
  - [x] It reads a real `Core\IO\File` through `ByteSource` once the member exists.

### 6. Core\Markdown\Live renders text that arrives in pieces

- [x] `Core\Markdown\Live` with `push` and `finish` (decision 1), built on the stage-5 splitter.
  `crates/nvs-stdlib/src/markdown.rs:1`
  - [x] `Segments::live` cuts at every line end, and `Walk::live` grows its budget and fuel per segment.
  - [x] `Walk::open` renders the unfinished text with a fresh state and puts the fuel back.
  - [x] `live.rs`: `Text`, the filed state, and the five helpers, registered on the closed lists.
  - [x] A reference section and a conformance case.
- [x] The two tests of the `live` check, over every piece size from one byte up.
  `crates/nvs-stdlib/src/markdown.rs:1`
  - [x] Both tests pass, and `looper check --name live` is green.

### 7. Core\Markdown::fromHtml writes Markdown from HTML

- [x] `crates/nvs-stdlib/src/markdown/from_html.rs` over the tree `Core\Html::parse` builds, by
  decision 8. `crates/nvs-stdlib/src/xml.rs:1`
  - [x] the walk over the sanitizer's rebuilt tree, with escaping and the autolink cut
  - [x] `Core\Markdown::fromHtml` on `CLASS`, its symbol, and `xml::parsed_of` for a node
  - [x] the rule, a reference-free conformance case
- [x] The converter's catalogue entries pass. `crates/nvs-stdlib/tests/markdown_known_bugs.rs:1`
- [x] The five tests of the `from-html` check. `crates/nvs-stdlib/src/markdown/from_html.rs:1`

### 8. fromHtml reads a large document without holding the whole tree

- [x] Stream the parse and write each finished top-level subtree, or record why the tree builder
  prevents it (decision 8). `crates/nvs-stdlib/src/markdown/from_html.rs:1`
  - [x] the case: fostered text joins the closed text node before a table
  - [x] the rule's *Large documents* and Declined, the module doc, decision 8
  - [x] a reference section on `fromHtml`, run with `NVS_BIN`
- [x] The test of the `from-html-stream` check. `crates/nvs-stdlib/src/markdown/from_html.rs:1`
  - [x] `html::parsed_in_pieces`, a test-only parse fed one piece at a time
  - [x] the test over a large document at four cut sizes, run once by name

### 9. two fuzz targets, and a differential fuzz against a second engine

- [x] A `markdown` target. It asserts no panic, a sanitize fixed point, chunked output equal to the
  whole, and the added-output budget. `fuzz/Cargo.toml:124`
  - [x] `fuzz.rs` gains `markdown_groups`, `markdown_live`, `html_meaning` and
    `MARKDOWN_ADDED_OUTPUT`; the tests' `meaning` and `live_in` use them.
  - [x] `fuzz/fuzz_targets/markdown.rs`, its `[[bin]]`, the `pulldown-cmark` patch row and
    `fuzz/dicts/markdown.dict`.
  - [x] It builds in WSL and runs clean for a first short run (20,000 inputs).
- [x] A `markdown_from_html` target. It asserts no panic, an idempotent result, and that the
  Markdown → HTML → Markdown round trip keeps the meaning. `fuzz/Cargo.toml:124`
  - [x] `fuzz/fuzz_targets/markdown_from_html.rs`, its `[[bin]]` and its dictionary.
  - [x] It builds in WSL and runs clean for a first short run.
- [x] The differential target against the other candidate (decision 9). Each difference becomes a
  catalogue entry or a line in the rule. `fuzz/Cargo.toml:124`
  - [x] `fuzz/fuzz_targets/markdown_comrak.rs` and `comrak` 0.59 in `fuzz/Cargo.toml` alone.
  - [x] Triaged until the user stopped it; the remaining differences are known cmark-gfm edge
    cases, not compared in CI (decision 20).
- [x] `markdown` and `markdown_from_html` each run 1,000,000 inputs once. Every crash becomes a
  catalogue entry and a fix.
  - [x] `bun nv fuzz <target> --runs <n>` runs one target in WSL, and CI's `fuzz-smoke` has a
    fourth leg for the two targets.
  - [x] `markdown` runs 1,000,000 inputs.
  - [x] `markdown_from_html` runs 1,000,000 inputs.

### 10. the four members carry their feature proofs

**Split:** one writing subagent per member (`toHtml`, `toHtmlEach`, `Live`, `fromHtml`), side by side,
because their proof directories are disjoint. Each writes `about.md`, three examples, the bench with its
growth, and one attack, held to AGENTS.md § *Text an end user reads*. The session builds once they all
end, then runs `bun nv proofs --id` for each member.

- [x] `toHtml`'s proofs. Its attack is the catalogue's XSS and slow inputs.
  `docs/examples/core/Markdown/toHtml/about.md:1`
- [x] `toHtmlEach`'s proofs. Its bench grows the input to 1 GB and shows flat memory for a file.
  `docs/examples/core/Markdown/toHtmlEach/about.md:1`
- [x] `Live`'s proofs. One example streams model output to a page over `Core\Sse`.
  `docs/examples/core/Markdown-Live/push/about.md:1`
- [x] `fromHtml`'s proofs. One example answers `Accept: text/markdown` with a page as Markdown.
  `docs/examples/core/Markdown/fromHtml/about.md:1`
- [x] Each member's help in the binary, and `bun nv proofs --comments` over the new `.nvs` files.
- [x] Each of the eight benches has a measured record. `data/perf/members.ndjson:1`
  - [x] `cargo build --release`, then `bun nv proofs --record-perf --id '<feature>'` for each.
  - [x] `bun nv scaling --budget --write` stores each new bench's share.
  - [x] `bun nv proofs --id` for each of the eight owes nothing.
- [x] A run of unclosed raw tags costs linear time. `crates/nvs-stdlib/src/markdown/groups.rs:1`
  - [x] Find the step that rescans per tag: `groups::Balance::started` scanned the open stack for
    `svg` or `math`.
  - [x] Make it linear. The bug is ours, and the catalogue holds only other libraries' published
    bugs, so its test is `a_run_of_unclosed_raw_tags_costs_linear_work` (green by name).
  - [x] Delete the gap record and the marker in `tests/hostile/core/Markdown/toHtml/03-many-raw-html-tags.nvs`.
- [x] `toHtmlEach`'s first read stays under the memory limit. `crates/nvs-stdlib/src/markdown/segments.rs:1`
  - [x] Bound each parse of uncut text (`segments::parse_cost`, in `cut`, `definitions` and Live).
  - [x] Delete the gap record and the marker in `tests/hostile/core/Markdown/toHtmlEach/02-a-flood-of-links-defined-at-the-end.nvs`.
- [x] A callback that calls `push` or `finish` on its own `Live` throws a `RuntimeError`.
  `crates/nvs-stdlib/src/markdown/live.rs:1`
  - [x] It ended the program with `FATAL: internal error: … lost its state`.
  - [x] A step in `tests/hostile/core/Markdown-Live/push/` that catches it.
- [x] A `Live` dropped without `finish` frees its state. `crates/nvs-stdlib/src/markdown/live.rs:1`
  - [x] `ClassDesc`'s `free` hook, run by `dismantle`; `instance::FREES_STATE` sets it for `Live`.
  - [x] A step in `tests/hostile/core/Markdown/live/` that abandons 20,000 objects under 16M.

### 11. the extension guide shows a crate Core does not ship

- [x] The guide's project becomes the highlighter of decision 10. `website/guides/highlight-extension/src/lib.rs:1`
  - [x] `syntect` builds for `wasm32-wasip2` (decision 22), and `tokens` and `languages` export.
  - [x] `nvs/Code.nvs` writes the tokens with `html` templates, and `styles` gives the colours.
  - [x] `tests/HighlightTest.nvs` passes 5 of 5, one through `Core\Markdown::toHtml`'s `code`.
  - [x] `bun nv ext-templates --example` prints `built, verified, tested and called`.
- [x] The guide's page explains the new project and links `Core\Markdown`.
  `website/src/content/docs/guides/extensions/rust.mdx:1`
  - [x] Steps 1 to 9 name the new files, and Step 9's demo passes `Code::toHtml` to `code`.
  - [x] `bun nv proofs --comments` passes the three `.nvs` files.
- [x] The folder is renamed to match the new example, and every link to it is updated.
  `website/guides/highlight-extension/README.md:1`
  - [x] `website/guides/highlight-extension/`, with CI, `ci-changes.ts`, `ext-templates.ts`, the
    website's loader and README, and `rule:packaging/a-dependency-move-proves-two-things`.
  - [x] `cargo deny ... check advisories` on the guide is ok.

### 12. the goal is verified once, and every failure is fixed

- [ ] Run `looper verify` over the whole goal.
- [ ] Fix every failure it names in one pass, never deleting or loosening a test.
- [ ] Run `looper verify` once more, and it is green.

## Handoff

### State

Stages 1 to 11 are done. Stage 11 made the extension guide a syntax highlighter on `syntect`
(decision 22) in `website/guides/highlight-extension/`; the guide check
`bun nv ext-templates --example` is green on the debug binary. The folder moved file by file:
a whole-folder `git mv` was refused with "Permission denied", most likely VS Code's language
server watching it.
- The eight Markdown features owe nothing in `bun nv proofs --id` (stage 10).
- The sweep reds were stale: run 42359979484 is on `26dda36`. Its three timed-out Miri tests
  (`json::tests::a_response_document_of_any_size_peaks_at_its_size_plus_one_chunk`,
  `allocations_per_document_byte_do_not_grow_with_the_document`, `request::tests::json::
  a_decoded_body_of_any_size_is_held_once`) each have a `cfg_attr(miri, ignore)` on main. No
  `env-secret-proofs` check or log exists.
- `bun nv rules --render` also wrote fragments earlier stages never rendered into
  `docs/rules/core-classes.md`; that is its own commit.

### Next

Stage 12, the gate: `looper verify` in the background, then the goal-end gates of AGENTS.md
§ *In the loop* (`bun nv verify --doc`, `bun nv owners --closes markdown`, `bun nv ci --check`).
`website/src/data/core.json` has no `Core\Markdown` yet, so the guide's link
`/reference/core/markdown/tohtml/` is broken until `bun nv render --website` runs on a binary
built from the tree. `bun nv scaling --budget --write` exits 1 on twelve
`benches/members/novis/Spreadsheet-*` benches this goal did not touch.

Possibly red at the gate, not run this session: `tools/nv/test/ext-templates.test.ts` (it now
names `highlight.wasm` and `highlighter.nvsx`); a runtime test that counts a `ClassDesc`'s fields
or debug-prints one; a Live test that expects the old `lost its state` fatal; a `toHtmlEach` or
Live test under a low memory limit with a large single block, which the per-line cost now refuses
sooner; a conformance case or PDF test that sanitizes `<p>` around a `<table>` with no
doctype (decision 18); a case that pins a loose task list's HTML or an autolink with an entity;
a case that pins an autolink next to a backslash escape, or a sanitized table with stray content;
a card, website snippet or conformance case that pins `fromHtml`'s output for a URL with a space,
`\`, `<` or a non-ASCII character, or for nested lists on one line;
`registry.rs`'s `a_class_with_slots_has_instance_members_and_the_reverse`,
`crates/nvs-stdlib/tests/ext_world.rs`, and card or website checks that see `fromHtml`.

### Backlog

- cmark-gfm reads a `www.` or URL autolink's whole text in the source, so an escape inside its
  path stays in the link as `\` and the character. Ours ends the link before the escape
  (`crates/nvs-stdlib/src/markdown/autolink.rs`, `unescaped`). Only a scan of the source would
  match it, and the differential skips autolinks.
- Front matter (`---` YAML at the top) is common in content sites. Once `Core` reads YAML, a member
  that returns it is a candidate for a later goal.
- AST editing (decision 1) is out of scope for this goal.
- `Core\Sse::send` cannot send a `Core\Html\Markup` and throws on an empty payload, so the Sse
  example sends `Core\Html::toSource` inside JSON. A `Markup` payload is a candidate for `Core\Sse`.
- `fromHtml` writes an `<a>` nested in an `<a>` as empty links, `[](/x)`, where an empty element
  writes nothing elsewhere.
- `tests/hostile/core/Markdown/fromHtml/01-text-that-tries-to-become-markdown.nvs` takes about
  7 s on the debug binary, against the 10 s default. The proof binary runs it well under.
- A second growing part for a catalogue entry (`close`, written n times before `suffix`) would turn
  the approximated two-part exploits back into the originals: cmark's CVE-2023-24824 and
  CVE-2023-22483, md4c's issue 172, and comrak's GHSA-j7hq-qg5v-47jq, which was left out.
- The footnote patch writes an undefined label as its source text, so `[^a\_b]` keeps its
  backslash. The differential skips footnotes, so nothing has compared this with another engine.
- `groups::Balance` keeps a group open after `<p>` and other elements the parser closes on its own.
  A document with many unclosed `<p>` tags in raw HTML is then one group. That is correct, and costs
  memory only for such a document.
- Live's `open` is checked only by the conformance case and the `markdown` fuzz target.
- A `~` run beside a `~~` run is not compared with comrak, so cmark-gfm's reading there is checked
  only by the catalogue entry `pulldown-cmark-tilde-nearest-opener`. A comrak release that fixes
  its `insert_emph` lets the differential compare it again.
