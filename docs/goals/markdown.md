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
    status: open
  - id: 9
    title: "two fuzz targets, and a differential fuzz against a second engine"
    status: open
    context: [fuzz/Cargo.toml]
  - id: 10
    title: "the four members carry their feature proofs"
    status: open
    context: [benches/members/README.md, docs/rules/testing/feature-proofs.md, docs/agent/prose.md]
  - id: 11
    title: "the extension guide shows a crate Core does not ship"
    status: open
    context: [website/guides/markdown-extension/README.md, website/src/content/docs/guides/extensions/rust.mdx]
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
     `Core\Html::parse`'s is.
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

- [ ] Stream the parse and write each finished top-level subtree, or record why the tree builder
  prevents it (decision 8). `crates/nvs-stdlib/src/markdown/from_html.rs:1`
- [ ] The test of the `from-html-stream` check. `crates/nvs-stdlib/src/markdown/from_html.rs:1`

### 9. two fuzz targets, and a differential fuzz against a second engine

- [ ] A `markdown` target. It asserts no panic, a sanitize fixed point, chunked output equal to the
  whole, and the added-output budget. `fuzz/Cargo.toml:124`
- [ ] A `markdown_from_html` target. It asserts no panic, an idempotent result, and that the
  Markdown → HTML → Markdown round trip keeps the meaning. `fuzz/Cargo.toml:124`
- [ ] The differential target against the other candidate (decision 9). Each difference becomes a
  catalogue entry or a line in the rule. `fuzz/Cargo.toml:124`
- [ ] Each target runs 1,000,000 inputs once. Every crash becomes a catalogue entry and a fix.

### 10. the four members carry their feature proofs

**Split:** one writing subagent per member (`toHtml`, `toHtmlEach`, `Live`, `fromHtml`), side by side,
because their proof directories are disjoint. Each writes `about.md`, three examples, the bench with its
growth, and one attack, held to AGENTS.md § *Text an end user reads*. The session builds once they all
end, then runs `bun nv proofs --id` for each member.

- [ ] `toHtml`'s proofs. Its attack is the catalogue's XSS and slow inputs.
  `docs/examples/core/Markdown/toHtml/about.md:1`
- [ ] `toHtmlEach`'s proofs. Its bench grows the input to 1 GB and shows flat memory for a file.
  `docs/examples/core/Markdown/toHtmlEach/about.md:1`
- [ ] `Live`'s proofs. One example streams model output to a page over `Core\Sse`.
  `docs/examples/core/Markdown-Live/push/about.md:1`
- [ ] `fromHtml`'s proofs. One example answers `Accept: text/markdown` with a page as Markdown.
  `docs/examples/core/Markdown/fromHtml/about.md:1`
- [ ] Each member's help in the binary, and `bun nv proofs --comments` over the new `.nvs` files.

### 11. the extension guide shows a crate Core does not ship

- [ ] The guide's project becomes the highlighter of decision 10. `website/guides/markdown-extension/src/lib.rs:1`
- [ ] The guide's page explains the new project and links `Core\Markdown`.
  `website/src/content/docs/guides/extensions/rust.mdx:1`
- [ ] The folder is renamed to match the new example, and every link to it is updated.
  `website/guides/markdown-extension/README.md:1`

### 12. the goal is verified once, and every failure is fixed

- [ ] Run `looper verify` over the whole goal.
- [ ] Fix every failure it names in one pass, never deleting or loosening a test.
- [ ] Run `looper verify` once more, and it is green.

## Handoff

### State

Stage 7 is done, in one commit. The `from-html` check's five tests, the catalogue (every from-html
entry) and the xml sweep test pass, each run once by name.
- **The converter** is `crates/nvs-stdlib/src/markdown/from_html.rs`. `markdown(tree)` wraps a
  non-document node, runs `html::rebuilt`, and `Writer::collect` walks the result into
  `(Part, String)` blocks that `joined` puts together. `Part` decides when a tight list item may use
  one line feed. `Inline` collapses whitespace, holds a space or break until text follows, and
  `escape_into` escapes and cuts autolinks with `<!-- -->`. Tables go through `gfm_table` or
  `html_block` (one line, `&#10;`). `DEPTH = 128`. `WORK` counts bytes for the linear-work test.
- **The member** `fromHtml` on `CLASS` returns `tainted string` always (decision 8 records why);
  `nvs_core_markdown_from_html` reads a string, a `Markup`, or a node through the new
  `crate::xml::parsed_of`. `a_parsed_tree_has_no_path_back_into_execution` now skips a member
  returning `TaintedStr`.
- **Tests** beside the live tests (`markdown/tests.rs:1045` on): the five, plus
  `from_html_work_grows_linearly_over_every_catalogued_slow_input` and
  `from_html_reads_a_node_a_program_already_holds`. `slow_inputs` takes a direction. The catalogue
  test's `from_html_outcome` replaces `FROM_HTML_BUILT`. `nvs_stdlib::fuzz::from_html` is new.
- **Rule and case**: `rule:core-classes/markdown-from-html` states the built shape, and
  `tests/conformance/core/markdown-from-html-writes-markdown-from-a-string-markup-and-a-node.nvst`
  passes on the debug binary (`bun nv try` needs `NVS_BIN=target/debug/nvs.exe`).
- **Stale sweep reds**: no `env-secret-proofs` check exists; nightly `42201104003` predates `f13f4064`.

### Next

Stage 8: stream `fromHtml` over a large document, or record why the tree builder prevents it.
Write its checkpoints under stage 8's items first.

- [ ] **Decide streaming** at `crates/nvs-stdlib/src/markdown/from_html.rs:78`: can html5ever's
  tree builder still change a closed top-level element (adoption agency, foster parenting, `</p>`)?
  The sink is `crate::html::Sink`, and `from_html::markdown` takes a whole `Parsed`. If streaming is
  unsafe, record why in the rule and decision 8 and keep the tree; the test then checks that the
  per-body-child conversion equals the whole.
- [ ] **The test** at `crates/nvs-stdlib/src/markdown/tests.rs:1045`:
  `from_html_streamed_gives_the_same_markdown_as_the_whole_tree`, beside the from-html tests.
- [ ] **A reference section** at `docs/reference/core/Markdown.md:89`, after the Live section, with
  the conformance case's first example; run it with `NVS_BIN`.

Expected red: the floor check `proofs-gate`, since the four members owe their feature proofs until
stage 10. Possibly red at the gate, not run this session: `registry.rs`'s
`a_class_with_slots_has_instance_members_and_the_reverse`, `crates/nvs-stdlib/tests/ext_world.rs`,
and card or website checks that see the new `fromHtml` member. Stage 11 may run as a side goal.

### Backlog

- Front matter (`---` YAML at the top) is common in content sites. Once `Core` reads YAML, a member
  that returns it is a candidate for a later goal.
- AST editing (decision 1) is out of scope for this goal.
- A second growing part for a catalogue entry (`close`, written n times before `suffix`) would turn
  the approximated two-part exploits back into the originals: cmark's CVE-2023-24824 and
  CVE-2023-22483, md4c's issue 172, and comrak's GHSA-j7hq-qg5v-47jq, which was left out.
- The footnote patch writes an undefined label as its source text, so `[^a\_b]` keeps its
  backslash. Stage 9's differential fuzz triages what cmark-gfm writes there.
- `groups::Balance` keeps a group open after `<p>` and other elements the parser closes on its own.
  A document with many unclosed `<p>` tags in raw HTML is then one group. That is correct, and costs
  memory only for such a document.
- Live's `open` is not checked by a test beyond the conformance case. A property that `open` is a
  sanitize fixed point after every push would cover it, and stage 9's fuzz target can assert it.
