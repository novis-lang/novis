---
looper: 1
title: "@see names a file in the project, and a heading inside a Markdown file"
summary: "Reached when @see accepts a path to a file inside the project with an optional #heading anchor checked against the Markdown file's headings, nvs pkg pack refuses a package whose @see or @example names a file it does not carry, the language server links, follows, completes and renames those targets, nvs meta --json and nvs doc write them as links, and the rule, the reference, the example and the feature proofs say so."
context: [docs/rules/tooling/doc-comment-tags-are-closed-and-checked.md, crates/nvs-syntax/src/ast.rs]
stages:
  - id: 1
    title: the checker resolves a file target and its heading
    status: open
    context: [crates/nvs-hir/src/members.rs, crates/nvs-hir/src/lenders.rs, crates/nvs-syntax/src/lib.rs, crates/nvs-diagnostics/src/code/names.rs, docs/rules/statements/require-is-the-only-inclusion-construct.md]
  - id: 2
    title: nvs pkg pack and the registry build refuse a doc tag naming a file the package does not carry
    status: open
    context: [crates/nvs-cli/src/package/pack.rs, crates/nvs-cli/src/package/build.rs, docs/rules/packaging/a-package-carries-only-its-source-and-what-it-lists.md]
  - id: 3
    title: "the language server links, follows, completes and renames file targets"
    status: open
    context: [crates/nvs-lsp/src/hover.rs, crates/nvs-lsp/src/definition.rs, crates/nvs-lsp/src/completion.rs, crates/nvs-lsp/src/server.rs, crates/nvs-lsp/src/render.rs]
  - id: 4
    title: nvs meta --json and nvs doc write a file target as a link
    status: open
    context: [crates/nvs-cli/src/meta.rs, crates/nvs-cli/src/doc.rs, docs/rules/tooling/meta-json-takes-a-program.md, docs/rules/tooling/nvs-doc-renders-and-decides-nothing.md]
  - id: 5
    title: "the rule, the reference, the example, the feature proofs and the website say so"
    status: open
    context: [docs/reference/lang/10-programs.md, docs/examples/lang/programs/doc-comments/about.md, docs/examples/lang/programs/doc-comments/details.md]
  - id: 6
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: checker
    stage: 1
    name: "the checker accepts a file target in the project, and refuses a missing file, a missing heading, an anchor on a non-Markdown file and a path that leaves the project"
    run: [cargo, test, --lib, a_see_]
    tests: [a_see_file_target_that_exists_is_accepted, a_see_file_target_that_names_no_file_is_refused, a_see_anchor_that_names_no_heading_is_refused, a_see_anchor_on_a_file_that_is_not_markdown_is_refused, a_see_path_that_leaves_the_project_is_refused, a_see_anchor_matches_a_github_heading_slug]
    timeout: 1800
  - id: pack
    stage: 2
    name: "a package whose @see or @example names a file it does not carry does not pack, and one that lists the file packs"
    run: [cargo, test, --lib, a_package_whose_doc_tag_names]
    tests: [a_package_whose_doc_tag_names_a_file_it_does_not_carry_is_refused, a_package_whose_doc_tag_names_a_listed_file_packs]
    timeout: 1800
  - id: editor
    stage: 3
    name: "hover links, go-to-definition follows, completion offers and a file rename rewrites a @see file target"
    run: [cargo, test, --test, see_files]
    tests: [hover_links_a_see_file_target_to_its_heading, definition_follows_a_see_file_target_to_its_heading, completion_offers_files_and_headings_after_see, renaming_a_file_rewrites_the_see_and_example_tags_that_name_it]
    timeout: 1800
  - id: meta
    stage: 4
    name: "nvs meta --json writes a @see file target relative to the program root"
    run: [cargo, test, --test, meta, a_see_file_target]
    tests: [a_see_file_target_is_written_from_the_program_root]
    timeout: 1800
  - id: doc
    stage: 4
    name: "nvs doc links a @see file target relative to the page it writes"
    run: [cargo, test, --test, doc, a_see_file_target]
    tests: [a_see_file_target_links_relative_to_the_page]
    timeout: 1800
  - id: proofs
    stage: 5
    name: "every feature the goal's work reached has its feature proofs"
    run: [bun, nv, proofs, --gate, --goal, see-files]
    timeout: 1800
  - id: close
    stage: 6
    name: "the goal-end check is green: ci --check's checks of the tree's files, the owners and the goal's proofs"
    run: [bun, nv, close, --goal, see-files]
    timeout: 900
---

# @see names a file in the project, and a heading inside a Markdown file

## Why

`@see` names a class or a member and nothing else (`check_see`, `crates/nvs-hir/src/members.rs:1254`).
A doc comment that wants to point at a design note, a README section or a SQL schema has only a
Markdown link in its prose, and `rule:tooling/doc-comment-tags-are-closed-and-checked`'s Declined list
already says why that is not enough: a link in prose buys no check. A file target checked like
`@example`'s path gives the one thing prose cannot: the build fails when the file moves or the heading
is renamed. The user asked for it on 2026-11-24 and agreed to the design below.

Most of the machinery exists. The parser keeps the whole argument as one span, `/`, `.` and `#`
included (`crates/nvs-syntax/src/parser/mod.rs:454`). `check_example` (`members.rs:1359`) already
resolves a path relative to the writing file, as `require` does, and records the lookup with
`nvs_footprint::exists`. `project_root` (`crates/nvs-hir/src/lenders.rs:460`) already finds a file's
nearest `nvs.toml` or `package.toml`. `workspace/willRenameFiles` already rewrites `require` literals
(`crates/nvs-lsp/src/server.rs:1815`, `render.rs:283`).

Found while writing it: nothing in `nvs pkg pack` or in a consumer's build handles `@example` targets
of a published package. `pack` carries only `src/**/*.nvs`, `package.toml`, the readme, the license
and `[package] files` (`crates/nvs-cli/src/package/pack.rs:5`), so a package whose doc comment says
`@example examples/charge.nvs` very likely fails the consumer's check with E0324. Stage 2 proves it
with a test first and fixes it for both tags.

## Standing decisions

**The shape.** `@see <path>[#<anchor>]`. A target containing `/` or `.` is a file target, and every
other target is a member target exactly as today. A class or member name never contains either, so the
two forms never overlap. One classifier decides it, `nvs_syntax::doc_target`, a new module in
`crates/nvs-syntax/src/doc_target.rs`, because the checker, the language server, `pack`, `meta` and
`doc` all depend on `nvs-syntax`. No new tag: the set stays five.

**Where a path is relative to.** The writing file's folder, as `@example` and `require` are
(`rule:statements/require-is-the-only-inclusion-construct`). One answer in the language.

**The path stays inside the project.** The project is the nearest folder at or above the writing file
that holds `nvs.toml` or `package.toml`, found by `project_root`'s walk without its top bound; a file
with neither has its own folder as the project. The written path is judged before it is joined: an
absolute path, a drive or UNC prefix, and a `..` that climbs past the project are refused on the
written text alone. After the join, the canonical path must still start with the canonical project
folder, so a symlink cannot leave it. The target must be a regular file. Fallback, if `project_root`
cannot be reused without its bound: copy its walk into `doc_target` with a comment naming the twin.

**Any file may be a target, and only Markdown takes an anchor.** `.md` and `.markdown` take
`#<anchor>`; an anchor on any other file is an error. The anchor must equal the slug of one heading in
the file. Slugs follow GitHub's rules, which VS Code's Markdown preview shares: lowercase, every
character that is not a letter, a digit, a space, `-` or `_` removed, each space turned into `-`, and a
repeated slug suffixed `-1`, `-2` in order. Headings are ATX (`#` to `######`) and setext (`===`,
`---` underlines), and nothing inside a fenced code block counts. `doc_target` scans lines itself and
depends on no Markdown crate: the stdlib's parser is a runtime crate, and a heading scan is twenty lines.

**Declined, and written into the rule's Declined list.**
- **A URL.** The checker never touches the network, so a URL cannot be checked, and every tag is
  checked. A Markdown link in the prose already carries an unchecked URL.
- **A line anchor, `#L42`.** It points at the wrong line after the next edit above it.
- **An HTML anchor, `<a id="x">`, as a target.** Only headings. One kind of anchor, one scan.
- **A sixth tag such as `@link`.** `@see` already means "related", and PHPDoc and Javadoc readers
  expect it to take a document.

**One diagnostic code.** Every file-target failure is E0323, `E_DOC_SEE_UNRESOLVED`
(`crates/nvs-diagnostics/src/code/names.rs:194`), with a message and help per case: no such file, no
such heading (the help lists up to five slugs the file has), an anchor on a file that is not Markdown,
a path that leaves the project. Its card (`names.rs:195`) is rewritten to cover both shapes.

**What the check reads, and what it records.** A file target costs one `stat`, as `@example` does. An
anchored target also reads the Markdown file once and scans its headings, and records
`nvs_footprint::file` for the read so test selection reruns a case when the Markdown changes. The
language server rechecks an open `.nvs` file when it is edited, not when the Markdown file changes;
`nvs check` is the authority. Memory: nothing at run time; at check time, one Markdown file's text per
anchored tag, dropped when the tag is checked.

**Packages carry their targets (stage 2).** `nvs pkg pack` and the registry build
(`crates/nvs-cli/src/package/build.rs:177`) refuse a package in which a `@see` file target or an
`@example` target names a file outside the set `file_set` (`pack.rs:204`) returns. The refusal names
the tag, the file and the line, and says to add the file to `[package] files`. This keeps one rule
everywhere, "the target exists", and the consumer's check finds the file. The cost is a slightly larger
package for a publisher who documents with examples. Declined: skipping the tag checks for a fetched
package's files, because the consumer's hover and `nvs doc` would then link to a file that is absent.

**The language server (stage 3).** Hover renders a file target as a Markdown link to its `file:` URI,
with `#L<line>` of the heading when anchored, so VS Code opens it at the heading. Go-to-definition on
the target returns that location. Completion after `@see ` offers paths once the typed text contains
`/` or `.`, relative to the writing file and limited to the project, and offers the file's heading
slugs after `#`. `workspace/willRenameFiles` rewrites `@see` file targets and `@example` paths beside
`require` literals. A heading rename does not rewrite tags: the check names the broken tag. Member
targets stay prose in hover (`hover.rs:87`), out of scope; a member link is a resolution of its own.

**The JSON shape does not change (stage 4).** `nvs meta --json`'s `see` stays a list of strings
(`crates/nvs-cli/src/meta.rs:1503`). A file target is written relative to the program root `nvs meta`
was given, with `/` separators and its anchor, so a consumer tells the two shapes apart by the same
`/`-or-`.` rule and finds the file without knowing which source file wrote it. `nvs doc`'s `see`
(`crates/nvs-cli/src/doc.rs:373`) writes a file target as a Markdown link relative to the output page.
`rule:tooling/meta-json-takes-a-program` says which root.

**Feature proofs.** The feature is the `@see` tag's file shape. It owes what
`bun nv proofs --owed --goal see-files` names; at least a sixth example
`docs/examples/lang/programs/doc-comments/06-point-at-a-document-with-see.nvs`, an attack that tries a
`../` path and a symlink out of the project, and a bench of the check over many anchored tags. The
example points at a heading of a Markdown file in its own folder; the session checks first that the
examples tooling tolerates an extra `.md` there, and otherwise points at a heading of `about.md`.

**What stops it.** One attempt per item and two sessions per stage. An item that cannot land in one
attempt is recorded in the handoff with what failed, and the stage goes on.

**Lanes.** Stage 1 is the keystone: every later stage calls `doc_target`. After it, stages 2
(`crates/nvs-cli/src/package/`), 3 (`crates/nvs-lsp/`) and 4 (`crates/nvs-cli/src/meta.rs`,
`doc.rs`) touch disjoint files and may run side by side. Stage 5 runs after all three.

## Stages

### 1. the checker resolves a file target and its heading

**Split:** none, one session.

- [ ] `crates/nvs-syntax/src/doc_target.rs:1` (new, `pub mod` in `crates/nvs-syntax/src/lib.rs:1`):
  `DocTarget::{Member, File { path, anchor }}` from an argument's text, and
  `heading_slugs(text) -> Vec<(String, u32)>` (slug and 0-based line), with unit tests including
  `a_see_anchor_matches_a_github_heading_slug` (ATX, setext, a fenced block, a repeated heading,
  punctuation, a non-ASCII letter).
- [ ] `crates/nvs-hir/src/members.rs:1254`: `check_see` sends a file target to a new `check_see_file`,
  which judges the written path, finds the project with `crates/nvs-hir/src/lenders.rs:460`'s walk,
  joins, canonicalizes, tests a regular file, then checks the anchor. `rule:tooling/doc-comment-tags-are-closed-and-checked`.
- [ ] `crates/nvs-diagnostics/src/code/names.rs:194`: E0323's card covers both shapes.
- [ ] `crates/nvs-hir/src/members.rs:2456`'s tests: the five `a_see_*` tests the check `checker`
  names, each writing its files under a temporary folder with an `nvs.toml`, then
  `cargo test --lib a_see_` once.

### 2. nvs pkg pack and the registry build refuse a doc tag naming a file the package does not carry

**Split:** none, one session.

- [ ] First, a test that shows today's failure: a fixture package (`crates/nvs-cli/src/package/fixture.rs:167`)
  whose class has `@example examples/charge.nvs` not in `[package] files`. Record in the handoff what a
  consumer's check printed before the fix.
- [ ] `crates/nvs-cli/src/package/pack.rs:125`: after `file_set`, parse each packed `.nvs` file's doc
  comments and refuse a `@see` file target or `@example` target outside the set, as a `Refusal` naming
  file, line and tag. `rule:packaging/a-package-carries-only-its-source-and-what-it-lists`.
- [ ] `crates/nvs-cli/src/package/build.rs:177`: the registry build runs the same check.
- [ ] `crates/nvs-cli/src/package/pack.rs:652`'s tests: the two tests the check `pack` names, then
  `cargo test --lib a_package_whose_doc_tag_names` once.
- [ ] `docs/rules/packaging/a-package-carries-only-its-source-and-what-it-lists.md:1`: one paragraph
  saying a doc tag's target must be in the set.

### 3. the language server links, follows, completes and renames file targets

**Split:** none, one session. The `.lspt` cases, if any, go to one writing subagent after the code lands.

- [ ] `crates/nvs-lsp/src/hover.rs:87`: a file target renders as a link with `#L<line>`; the module doc
  says member targets stay prose.
- [ ] `crates/nvs-lsp/src/definition.rs:1`: a cursor on a `@see` file target returns the file and the
  heading's line.
- [ ] `crates/nvs-lsp/src/completion.rs:2709`: `see_targets` offers paths once the text has `/` or `.`,
  and heading slugs after `#`.
- [ ] `crates/nvs-lsp/src/server.rs:1815` and `crates/nvs-lsp/src/render.rs:283`: `willRenameFiles`
  rewrites `@see` file targets and `@example` paths.
- [ ] `crates/nvs-lsp/tests/see_files.rs:1` (new): the four tests the check `editor` names, then
  `cargo test --test see_files` once.

### 4. nvs meta --json and nvs doc write a file target as a link

**Split:** none, one session.

- [ ] `crates/nvs-cli/src/meta.rs:1503`: a file target is written relative to the program root, `/`
  separators, anchor kept; a member target is unchanged.
- [ ] `crates/nvs-cli/src/doc.rs:373`: a file target becomes a link relative to the page.
- [ ] `crates/nvs-cli/tests/meta.rs:756` and `crates/nvs-cli/tests/doc.rs:15`: the two tests the
  checks `meta` and `doc` name, each run once by name.
- [ ] `docs/rules/tooling/meta-json-takes-a-program.md:15` and
  `docs/rules/tooling/nvs-doc-renders-and-decides-nothing.md:11`: one sentence each on file targets.

### 5. the rule, the reference, the example, the feature proofs and the website say so

**Split:** the example, the attack and the bench go to one writing subagent once the rule is rewritten;
the session writes the rule, the reference and the website entries.

- [ ] `docs/rules/tooling/doc-comment-tags-are-closed-and-checked.md:25`: the `@see` shape gains the
  file form, the project bound and the anchor rule; the Declined list gains URL, line anchor, HTML
  anchor and a sixth tag; § *Memory* says what the check reads.
- [ ] `crates/nvs-hir/src/members.rs:1250`: `check_see`'s doc says both shapes.
- [ ] `docs/reference/lang/10-programs.md:411` and `:424`, `docs/examples/lang/programs/doc-comments/about.md:7`
  and `details.md:4`: the file form, written to AGENTS.md § *Text an end user reads*.
- [ ] `docs/examples/lang/programs/doc-comments/06-point-at-a-document-with-see.nvs:1` and its `.out`.
- [ ] The attack and the bench `bun nv proofs --owed --goal see-files` names, at the paths it prints.
- [ ] `website/src/data/changes.json:1`: one `changed` entry for the `@see` tag. Each page above
  reread and stamped with `bun nv site --stamp`.
- [ ] `bun nv proofs --owed --goal see-files` names nothing.

### 6. the goal is verified once, and every failure is fixed

- [ ] The newcomer review of docs/agent/prose.md § *A newcomer reads it* over the pages stage 5 touched.
- [ ] Run the check the pack names for this stage: `looper verify` at a full gate, or a build and the tests this goal wrote, by name, at a close.
- [ ] Only if it is red: fix every failure it names in one pass, never deleting or loosening a test, then run it once more, and it is green.

## Handoff

### State

Written by the interactive session that settled the design with the user on 2026-11-24. Nothing is
built. Every anchor above was read on that day.

### Next

Stage 1 from its first item. Every check is red until its stage's tests exist.

### Backlog
