---
looper: 1
title: "@see names a file in the project, and a heading inside a Markdown file"
summary: "Reached when @see accepts a path to a file inside the project with an optional #heading anchor checked against the Markdown file's headings, nvs pkg pack refuses a package whose @see or @example names a file it does not carry, the language server links, follows, hovers, completes, colours, finds, renames and quick-fixes every @see target and moves them with a renamed file, nvs meta --json and nvs doc write file targets as links, and the rules, the reference, the editor page, the example and the feature proofs say so."
context: [docs/rules/tooling/doc-comment-tags-are-closed-and-checked.md, crates/nvs-syntax/src/ast.rs]
stages:
  - id: 1
    title: the checker resolves every @see target, records it and suggests the closest name
    status: done
    context: [crates/nvs-hir/src/members.rs, crates/nvs-hir/src/lenders.rs, crates/nvs-syntax/src/lib.rs, crates/nvs-diagnostics/src/code/names.rs, docs/rules/statements/require-is-the-only-inclusion-construct.md]
  - id: 2
    title: nvs pkg pack and the registry build refuse a doc tag naming a file the package does not carry
    status: done
    context: [crates/nvs-cli/src/package/pack.rs, crates/nvs-cli/src/package/build.rs, docs/rules/packaging/a-package-carries-only-its-source-and-what-it-lists.md]
  - id: 3
    title: "the editor links, follows, hovers, completes and colours every @see target"
    status: open
    context: [crates/nvs-lsp/src/links.rs, crates/nvs-lsp/src/definition.rs, crates/nvs-lsp/src/hover.rs, crates/nvs-lsp/src/completion.rs, crates/nvs-lsp/src/semantic.rs, editors/vscode/syntaxes/nvs.tmLanguage.json, docs/rules/ide/the-request-set-is-closed.md]
  - id: 4
    title: "the editor finds, renames and quick-fixes @see targets, and moves them with a renamed file"
    status: open
    context: [crates/nvs-lsp/src/index.rs, crates/nvs-lsp/src/rename.rs, crates/nvs-lsp/src/server.rs, crates/nvs-lsp/src/render.rs, crates/nvs-lsp/src/capabilities.rs, docs/rules/ide/five-features-are-one-reference-index.md]
  - id: 5
    title: nvs meta --json and nvs doc write a file target as a link
    status: open
    context: [crates/nvs-cli/src/meta.rs, crates/nvs-cli/src/doc.rs, docs/rules/tooling/meta-json-takes-a-program.md, docs/rules/tooling/nvs-doc-renders-and-decides-nothing.md]
  - id: 6
    title: "the rules, the reference, the editor page, the example, the feature proofs and the website say so"
    status: open
    context: [docs/reference/lang/10-programs.md, docs/examples/lang/programs/doc-comments/about.md, docs/examples/lang/programs/doc-comments/details.md, website/src/content/docs/tools/editor/index.mdx]
  - id: 7
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: checker
    stage: 1
    name: "the checker accepts a file target in the project, refuses a missing file, a missing heading, an anchor on a non-Markdown file and a path that leaves the project, and suggests the closest name for a miss"
    run: [cargo, test, --lib, a_see_]
    tests: [a_see_file_target_that_exists_is_accepted, a_see_file_target_that_names_no_file_is_refused, a_see_anchor_that_names_no_heading_is_refused, a_see_anchor_on_a_file_that_is_not_markdown_is_refused, a_see_path_that_leaves_the_project_is_refused, a_see_anchor_matches_a_github_heading_slug, a_see_miss_suggests_the_closest_name, a_see_target_is_recorded_with_what_it_resolved_to]
    timeout: 1800
  - id: pack
    stage: 2
    name: "a package whose @see or @example names a file it does not carry does not pack, and one that lists the file packs"
    run: [cargo, test, --bin, nvs, a_package_whose_doc_tag_names]
    tests: [a_package_whose_doc_tag_names_a_file_it_does_not_carry_is_refused, a_package_whose_doc_tag_names_a_listed_file_packs]
    timeout: 1800
  - id: editor-read
    stage: 3
    name: "document links, go-to-definition, hover, completion and semantic tokens cover every @see target, @example path and @throws name"
    run: [cargo, test, --test, see_targets]
    tests: [a_document_link_opens_a_see_file_at_its_heading, a_document_link_opens_an_example_file, definition_follows_a_see_member_target, definition_follows_a_see_file_target_to_its_heading, hover_on_a_see_member_target_shows_the_member, hover_on_a_see_file_target_shows_the_heading_section, a_declarations_hover_links_its_see_targets, completion_offers_files_and_headings_after_see, see_and_throws_targets_get_semantic_tokens]
    timeout: 1800
  - id: editor-edit
    stage: 4
    name: "references list, rename rewrites, a quick fix repairs and a file or folder move rewrites every @see target"
    run: [cargo, test, --test, see_edits]
    tests: [references_list_a_see_target_without_counting_it, renaming_a_member_rewrites_its_see_targets, renaming_a_class_rewrites_its_see_and_throws_targets, prepare_rename_on_a_see_target_renames_the_member, renaming_a_file_rewrites_the_see_and_example_tags_that_name_it, renaming_a_folder_rewrites_the_see_targets_inside_it, a_file_rename_no_tag_names_answers_nothing, a_missing_heading_offers_the_closest_one_as_a_quick_fix]
    timeout: 1800
  - id: handshake
    stage: 4
    name: "the declared capabilities are exactly the closed set, with willRenameFiles over every file"
    run: [cargo, test, --test, handshake]
    timeout: 1800
  - id: meta
    stage: 5
    name: "nvs meta --json writes a @see file target relative to the program root"
    run: [cargo, test, --test, meta, a_see_file_target]
    tests: [a_see_file_target_is_written_from_the_program_root]
    timeout: 1800
  - id: doc
    stage: 5
    name: "nvs doc links a @see file target relative to the page it writes"
    run: [cargo, test, --test, doc, a_see_file_target]
    tests: [a_see_file_target_links_relative_to_the_page]
    timeout: 1800
  - id: proofs
    stage: 6
    name: "every feature the goal's work reached has its feature proofs"
    run: [bun, nv, proofs, --gate, --goal, see-files]
    timeout: 1800
  - id: close
    stage: 7
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
is renamed. The user asked for it on 2026-11-24 and agreed to the design below, then asked that the
goal cover editor support for `@see` as fully as the editor allows.

Most of the checker's machinery exists. The parser keeps the whole argument as one span, `/`, `.` and
`#` included (`crates/nvs-syntax/src/parser/mod.rs:454`). `check_example` (`members.rs:1359`) already
resolves a path relative to the writing file, as `require` does, and records the lookup with
`nvs_footprint::exists`. `project_root` (`crates/nvs-hir/src/lenders.rs:460`) already finds a file's
nearest `nvs.toml` or `package.toml`.

The editor treats `@see` as prose today. Hover says so (`crates/nvs-lsp/src/hover.rs:87`). Completion
offers member targets (`crates/nvs-lsp/src/completion.rs:2709`), and nothing else reads them: no link,
no definition, no reference, no rename, no semantic token (`crates/nvs-lsp/src/semantic.rs:809` colours
only `@param`'s name). `workspace/willRenameFiles` rewrites `require` literals only, and is registered
for `**/*.nvs` files (`crates/nvs-lsp/src/capabilities.rs:240`).

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

**One diagnostic code, with a suggestion.** Every `@see` failure is E0323, `E_DOC_SEE_UNRESOLVED`
(`crates/nvs-diagnostics/src/code/names.rs:194`), with a message and help per case: no such class or
member, no such file, no such heading, an anchor on a file that is not Markdown, a path that leaves the
project. A miss carries a `Suggestion` that replaces the written name with the closest one, measured
with `edit_distance` (`members.rs:2439`), when one is within a third of the written length: the closest
member of the class, or the closest heading slug of the file. No suggestion for a missing file. The
editor's quick fix is that suggestion translated
(`rule:ide/a-code-action-ships-only-a-fix-a-diagnostic-already-knows`). Its card (`names.rs:195`) is
rewritten to cover both shapes.

**One resolution, many readers.** `check_see` records what each target resolved to, keyed by the tag's
argument span: a member target as its class's qualified name and member name and kind, a file target as
the canonical path and the heading's line. `check_example` records its resolved path the same way, and
`@throws` its resolved class. Every editor feature in stages 3 and 4 reads that record and none parses
the tag again, as `crates/nvs-lsp/src/links.rs:1`'s module doc requires of a link. The session
picks where the record lives by how `nvs_types::ExprTypeTable::written_path` reaches `links.rs`, and
follows the same road.

**What the check reads, and what it records.** A file target costs one `stat`, as `@example` does. An
anchored target also reads the Markdown file once and scans its headings, and records
`nvs_footprint::file` for the read so test selection reruns a case when the Markdown changes. The
language server rechecks an open `.nvs` file when it is edited, not when the Markdown file changes;
`nvs check` is the authority. Memory: nothing at run time; at check time, one Markdown file's text per
anchored tag, dropped when the tag is checked, and the record of one resolved target per tag for as long
as the file's analysis is kept.

**Packages carry their targets (stage 2).** `nvs pkg pack` and the registry build
(`crates/nvs-cli/src/package/build.rs:177`) refuse a package in which a `@see` file target or an
`@example` target names a file outside the set `file_set` (`pack.rs:204`) returns. The refusal names
the tag, the file and the line, and says to add the file to `[package] files`. This keeps one rule
everywhere, "the target exists", and the consumer's check finds the file. The cost is a slightly larger
package for a publisher who documents with examples. Declined: skipping the tag checks for a fetched
package's files, because the consumer's hover and `nvs doc` would then link to a file that is absent.

**The editor covers every request that can read a target (stages 3 and 4).** No new request: each is
one the server already answers, so `rule:ide/the-request-set-is-closed` grows in its lists, not its
count.
- **`textDocument/documentLink`**: every `@see` file target and every `@example` path is a link. A file
  target with an anchor opens at the heading's line (`file:` URI with `#L<line>`).
- **`textDocument/definition`**: a cursor on a member target goes to the declaration; on the class
  part of `Class::member`, to the class; on a file target, to the file at the heading's line; on an
  `@example` path, to the file. `@throws Name` goes to the class.
- **`textDocument/hover`**: on a member target, the same hover the member shows at a use site. On a file
  target, the path and, when anchored, the heading and its section's text up to the next heading,
  at most 20 lines, as Markdown. The hover of a documented declaration lists its `@see` targets as
  links, where `hover.rs:1260` lists them as text today.
- **`textDocument/completion`**: member targets as today; paths once the typed text contains `/` or `.`,
  relative to the writing file and limited to the project, with the same lister `@example` completion
  uses (`completion.rs:478`); heading slugs of the file after `#`.
- **`textDocument/semanticTokens`**: a member target is coloured by what it resolved to (class, method,
  property, constant, enum case), and `@throws Name` as a class. A file target gets no token: the
  TextMate grammar (`editors/vscode/syntaxes/nvs.tmLanguage.json:121`) scopes `@see` and `@example`
  paths as `markup.underline.link`, so a theme shows them before the server answers.
- **`textDocument/references` and `documentHighlight`**: each `@see` member target and each `@throws`
  name is an occurrence in the index (`crates/nvs-lsp/src/index.rs:579`), listed by references and
  highlight. It is not counted by the CodeLens count and does not keep a private member from dimming as
  unused: a doc comment that points at a member uses nothing, as a `use` line imports and uses nothing
  (`rule:ide/five-features-are-one-reference-index`). The disk index's `FORMAT`
  (`crates/nvs-lsp/src/disk_index.rs:81`) is raised once for the new occurrences.
- **`textDocument/rename` and `prepareRename`**: renaming a class or member rewrites every `@see`
  target and `@throws` name that resolved to it, because they are index occurrences. A cursor on a
  `@see` target renames the member it names. Only the renamed segment changes.
- **`workspace/willRenameFiles`**: the file filter becomes every file (`**/*`), because any file may be
  a target. A moved file or folder rewrites each `@see` file target and `@example` path that named it,
  beside `require` literals, keeping the anchor. A rename that no recorded target names answers `null`
  from the record without reading a file, so the wider filter costs one lookup per rename.
- **Code actions**: the E0323 suggestion above is the quick fix. Nothing new is built for it.

`@throws` is included because it resolves like a member target's class and the same index pass and
rename reach it; leaving it out would make renaming an error class rewrite `@see` and miss `@throws`.

**Declined, for the editor.**
- **Serving Markdown files.** The Novis server registers `.nvs` documents only. Finding references
  from a Markdown heading, or renaming a heading and its tags together, would mean answering requests
  on `.md` documents beside VS Code's own Markdown support. A renamed heading is caught by the check,
  and the quick fix offers the closest heading.
- **A preview of a non-Markdown file in hover.** The link opens it; a hover of a SQL or TOML file's
  first lines tells the reader little the file name does not.

**The JSON shape does not change (stage 5).** `nvs meta --json`'s `see` stays a list of strings
(`crates/nvs-cli/src/meta.rs:1503`). A file target is written relative to the program root `nvs meta`
was given, with `/` separators and its anchor, so a consumer tells the two shapes apart by the same
`/`-or-`.` rule and finds the file without knowing which source file wrote it. `nvs doc`'s `see`
(`crates/nvs-cli/src/doc.rs:373`) writes a file target as a Markdown link relative to the output page.
`rule:tooling/meta-json-takes-a-program` says which root.

**Feature proofs.** The feature is the `@see` tag's file shape and its editor support. It owes what
`bun nv proofs --owed --goal see-files` names; at least a sixth example
`docs/examples/lang/programs/doc-comments/06-point-at-a-document-with-see.nvs`, an attack that tries a
`../` path and a symlink out of the project, and a bench of the check over many anchored tags. The
example points at a heading of a Markdown file in its own folder; the session checks first that the
examples tooling tolerates an extra `.md` there, and otherwise points at a heading of `about.md`.

**What stops it.** One attempt per item and two sessions per stage. An item that cannot land in one
attempt is recorded in the handoff with what failed, and the stage goes on.

**Lanes.** Stage 1 is the keystone: every later stage reads `doc_target` or the record. After it,
stage 2 (`crates/nvs-cli/src/package/`), stage 5 (`crates/nvs-cli/src/meta.rs`, `doc.rs`) and the
editor stages touch disjoint files and may run side by side. Stages 3 and 4 both build `nvs-lsp` but
share no source file (3: `links.rs`, `definition.rs`, `hover.rs`, `completion.rs`, `semantic.rs`, the
grammar; 4: `index.rs`, `disk_index.rs`, `rename.rs`, `server.rs`, `render.rs`, `capabilities.rs`) and
each has its own test file, so they may run side by side too. Stage 6 runs after all of them.

## Stages

### 1. the checker resolves every @see target, records it and suggests the closest name

**Split:** none, one session.

- [x] `crates/nvs-syntax/src/doc_target.rs:1` (new, `pub mod` in `crates/nvs-syntax/src/lib.rs:1`):
  `DocTarget::{Member, File { path, anchor }}` from an argument's text, and
  `heading_slugs(text) -> Vec<(String, u32)>` (slug and 0-based line), with unit tests including
  `a_see_anchor_matches_a_github_heading_slug` (ATX, setext, a fenced block, a repeated heading,
  punctuation, a non-ASCII letter).
- [x] `crates/nvs-hir/src/members.rs:1254`: `check_see` sends a file target to a new `check_see_file`,
  which judges the written path, finds the project with `crates/nvs-hir/src/lenders.rs:460`'s walk,
  joins, canonicalizes, tests a regular file, then checks the anchor.
  `rule:tooling/doc-comment-tags-are-closed-and-checked`.
- [x] `crates/nvs-hir/src/members.rs:1324` and the new file miss: the closest-name `Suggestion`, with
  `edit_distance` (`members.rs:2439`).
- [x] `crates/nvs-hir/src/members.rs:412`: the record of resolved targets for `@see`, `@example` and
  `@throws`, carried out of `check` by the road `written_path` takes to `crates/nvs-lsp/src/links.rs:81`.
- [x] `crates/nvs-diagnostics/src/code/names.rs:194`: E0323's card covers both shapes.
- [x] `crates/nvs-hir/src/members.rs:2456`'s tests: the `a_see_*` tests the check `checker` names, each
  writing its files under a temporary folder with an `nvs.toml`, then `cargo test --lib a_see_` once.

### 2. nvs pkg pack and the registry build refuse a doc tag naming a file the package does not carry

**Split:** none, one session.

- [x] First, a test that shows today's failure: a fixture package (`crates/nvs-cli/src/package/fixture.rs:167`)
  whose class has `@example examples/charge.nvs` not in `[package] files`. Record in the handoff what a
  consumer's check printed before the fix.
- [x] `crates/nvs-cli/src/package/pack.rs:125`: after `file_set`, parse each packed `.nvs` file's doc
  comments and refuse a `@see` file target or `@example` target outside the set, as a `Refusal` naming
  file, line and tag. `rule:packaging/a-package-carries-only-its-source-and-what-it-lists`.
- [x] `crates/nvs-cli/src/package/build.rs:177`: the registry build runs the same check.
- [x] `crates/nvs-cli/src/package/pack.rs:652`'s tests: the two tests the check `pack` names, then
  `cargo test --bin nvs a_package_whose_doc_tag_names` once.
- [x] `docs/rules/packaging/a-package-carries-only-its-source-and-what-it-lists.md:1`: one paragraph
  saying a doc tag's target must be in the set.

### 3. the editor links, follows, hovers, completes and colours every @see target

**Split:** none, one session. `.lspt` cases, where the runner can hold a Markdown fixture beside the
`.nvs` one, go to one writing subagent after the code lands.

- [ ] `crates/nvs-lsp/src/links.rs:81`: `@see` file targets and `@example` paths from the record, an
  anchored one with `#L<line>`; the module doc's list gains both.
- [ ] `crates/nvs-lsp/src/definition.rs:1`: a cursor on a `@see` target (member, its class part, file),
  an `@example` path or a `@throws` name goes to what the record says.
- [ ] `crates/nvs-lsp/src/hover.rs:87` and `:1260`: hover on a member target shows the member's hover;
  on a file target, the path and the heading's section; a declaration's hover lists its `@see` targets
  as links. The module doc's prose paragraph at `:87` is rewritten to say so.
- [ ] `crates/nvs-lsp/src/completion.rs:2709`: `see_targets` offers paths once the text has `/` or `.`,
  with `completion.rs:478`'s lister, and heading slugs after `#`.
- [ ] `crates/nvs-lsp/src/semantic.rs:809`: tokens for `@see` member targets by resolved kind and for
  `@throws` names, on methods and on every other documented declaration.
- [ ] `editors/vscode/syntaxes/nvs.tmLanguage.json:121`: `@see` and `@example` paths scoped
  `markup.underline.link`; a case in `editors/vscode/test/grammar/fixtures/doc-tags.nvs:1`.
- [ ] `crates/nvs-lsp/tests/see_targets.rs:1` (new): the tests the check `editor-read` names, then
  `cargo test --test see_targets` once.

### 4. the editor finds, renames and quick-fixes @see targets, and moves them with a renamed file

**Split:** none, one session.

- [ ] `crates/nvs-lsp/src/index.rs:579`: `@see` member targets and `@throws` names become occurrences,
  marked so the CodeLens count and unused-member dimming leave them out; `crates/nvs-lsp/src/disk_index.rs:81`
  `FORMAT` raised.
- [ ] `crates/nvs-lsp/src/rename.rs:157`: rename and prepareRename cover a `@see` target and a
  `@throws` name; the module doc's list (`rename.rs:10`) gains them.
- [ ] `crates/nvs-lsp/src/capabilities.rs:240`: the file filter becomes `**/*`, and its comment at
  `:229` says why.
- [ ] `crates/nvs-lsp/src/server.rs:1815` and `crates/nvs-lsp/src/render.rs:283`: `willRenameFiles`
  rewrites `@see` file targets and `@example` paths from the record, keeping the anchor, and answers
  `null` without a file read when no target names the moved path.
- [ ] `crates/nvs-lsp/tests/handshake.rs:170`: the declared filter updated.
- [ ] `crates/nvs-lsp/tests/see_edits.rs:1` (new): the tests the check `editor-edit` names, then
  `cargo test --test see_edits` and `cargo test --test handshake` once.

### 5. nvs meta --json and nvs doc write a file target as a link

**Split:** none, one session.

- [ ] `crates/nvs-cli/src/meta.rs:1503`: a file target is written relative to the program root, `/`
  separators, anchor kept; a member target is unchanged.
- [ ] `crates/nvs-cli/src/doc.rs:373`: a file target becomes a link relative to the page.
- [ ] `crates/nvs-cli/tests/meta.rs:756` and `crates/nvs-cli/tests/doc.rs:15`: the two tests the
  checks `meta` and `doc` name, each run once by name.
- [ ] `docs/rules/tooling/meta-json-takes-a-program.md:15` and
  `docs/rules/tooling/nvs-doc-renders-and-decides-nothing.md:11`: one sentence each on file targets.

### 6. the rules, the reference, the editor page, the example, the feature proofs and the website say so

**Split:** the example, the attack and the bench go to one writing subagent once the rule is rewritten;
the session writes the rules, the reference, the editor page and the website entries.

- [ ] `docs/rules/tooling/doc-comment-tags-are-closed-and-checked.md:25`: the `@see` shape gains the
  file form, the project bound, the anchor rule and the suggestion; the Declined list gains URL, line
  anchor, HTML anchor and a sixth tag; § *Memory* says what the check reads.
- [ ] `docs/rules/ide/the-request-set-is-closed.md:9`: `hover`, `definition`, `completion` and
  `documentLink`'s lists gain doc-tag targets. `docs/rules/ide/five-features-are-one-reference-index.md:1`:
  doc-tag occurrences are listed and renamed, not counted, with their memory. The editor's Declined
  items above go into one of the two.
- [ ] `crates/nvs-hir/src/members.rs:1250`: `check_see`'s doc says both shapes.
- [ ] `docs/reference/lang/10-programs.md:411` and `:424`, `docs/examples/lang/programs/doc-comments/about.md:7`
  and `details.md:4`: the file form, written to AGENTS.md § *Text an end user reads*.
- [ ] `website/src/content/docs/tools/editor/index.mdx:1`: what the editor does with a `@see` target,
  in the page's own list.
- [ ] `docs/examples/lang/programs/doc-comments/06-point-at-a-document-with-see.nvs:1` and its `.out`.
- [ ] The attack and the bench `bun nv proofs --owed --goal see-files` names, at the paths it prints.
- [ ] `website/src/data/changes.json:1`: one `changed` entry for the `@see` tag and one for the editor.
  Each page above reread and stamped with `bun nv site --stamp`.
- [ ] `bun nv proofs --owed --goal see-files` names nothing.

### 7. the goal is verified once, and every failure is fixed

- [ ] The newcomer review of docs/agent/prose.md § *A newcomer reads it* over the pages stage 6 touched.
- [ ] Run the check the pack names for this stage: `looper verify` at a full gate, or a build and the tests this goal wrote, by name, at a close.
- [ ] Only if it is red: fix every failure it names in one pass, never deleting or loosening a test, then run it once more, and it is green.

## Handoff

### State

Stages 1 and 2 are done; `checker` and `pack` are green. `pack`'s `run` names `--bin nvs`, where
`nvs-cli`'s unit tests live.

- `nvs_syntax::doc_target` has `DocTarget::of`, the one classifier (a `/` or `.` makes a file
  target, split at the first `#`), `is_markdown`, `heading_slugs` and `slug`.
- `crates/nvs-hir/src/members.rs` `check_see_file` resolves a file target inside the project found
  by `nvs_hir::lenders::project_of`. A member or heading miss carries a fix from `closest_name`.
- The record is `nvs_hir::doc_targets::DocTargets` on `Module::doc_targets`, filled only when
  `resolve_program` gets a parse cache (the language server's path). Each `DocTarget` has `span`,
  `class_span`, and `Resolved::{Class, Member { class, owner, name, kind }, File { path, heading }}`.
  `@throws` records `Class`, `@example` a `File` with no heading. `DocTargets::at(file, offset)`
  finds the target under a cursor.
- `crates/nvs-cli/src/package/pack.rs` `check_doc_targets` lexes each packed `.nvs` file, resolves
  each `@see` file and `@example` path from its folder (`in_package`), and refuses one not in the
  archive, naming file, line and tag. `pack` and the registry build (`build.rs`) call it. Before it,
  such a package packed and the consumer's check failed with `E_DOC_EXAMPLE_NOT_FOUND`.

### Next

Stages 3, 4 and 5 may run side by side (Lanes). The next session takes stage 3 from its first item.
The editor stages read `Module::doc_targets`: find how `Analysed` keeps the `Module`
(`crates/nvs-lsp/src/document.rs`) and read `module.doc_targets.in_file(analysed.entry)`. Stage 5
starts at `crates/nvs-cli/src/meta.rs:1515`, where `see` and `example` are written as text today.

No test is expected to go red from stages 1 and 2: no existing `@see` in the corpus has a `/` or a
`.`, and no package source in the tree writes an `@example`.

### Backlog
