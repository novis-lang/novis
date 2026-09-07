# Handoff

## State

**Goal 11 stage 5 is whole — one JSON, one more input.** The three names that stage's acceptance
check lists pass under `cargo test -p nvs-cli`, and `python tools/reference.py --check` still
regenerates `docs/novis.md` from the binary. Stages 1-4 are unchanged.

- **`nvs meta --json [entry]` adds exactly one key.** With no argument the document is the registry
  alone, byte for byte what it was before the argument existed; with an entry point the program's own
  declarations sit under `program`, and nothing else moves. `crates/nvs-cli/src/meta.rs`'s module doc
  § *The program half* is that seam's home, and
  `meta_json_with_no_argument_is_byte_identical_to_the_registry_dump` is it asserted by removing the
  key and comparing what is left.
- **The program half is the program, not the file**: `crate::front_end` parses, resolves and
  type-checks the whole `require`/`autoload` graph, and every file it reached contributes its
  declarations in load order. A program that does not check prints nothing and exits non-zero.
- **Its rosters are `classes`, `interfaces`, `enums` and `types`**, in the registry's own shape —
  `members` plus `constants` on a class or interface, `cases` on an enum, the aliased type on a
  `type`. A method's `kind` is the registry's vocabulary including `constructor`, decided by the
  name. A signature is read off the source spans rather than re-printed from the tree.
- **A declaration's card is the `///` run above it**: prose under `short` (the key the registry
  already spells prose with), `@see` and `@example` as their own lists. The omission rule is the
  registry's — no `doc` key without a run, no array emitted empty.
- **Stage 5's three items landed as one commit**, because the test slice 1 names asserts the shape
  slice 2 defines and slice 3 fills: they are one change to one file, and git stages files.
- The driver's failing check, `examples/doc-comments.nvs`, is stage 6's `exact` fixture with four
  `want` lines — an item still open, not a regression. It is item 3 below.
- The goal's `[context] modules` names `nvs-cli/src/meta.rs` already; it does **not** name
  `crates/nvs-hir/src/members.rs`, which item 1 below edits, so that pattern is still missing.
- Nothing is blocked. `python tools/verify.py` is green.

## Next group

**Stage 6: the renderer and the lint** — one file set: `crates/nvs-cli/src/main.rs`, a new
`crates/nvs-cli/src/doc.rs`, `crates/nvs-hir/src/members.rs`, `examples/doc-comments.nvs`.

- [ ] **`nvs check --strict-docs` reports a public member with no `///`** —
      `rule:tooling/strict-docs`. Silent by default and never satisfiable by an autofix. The walk is
      already there: `crates/nvs-hir/src/members.rs:501` is `check_doc`, reached from `check_stmts`
      (`crates/nvs-hir/src/members.rs:369`) and `check_members`
      (`crates/nvs-hir/src/members.rs:461`), so the flag rides in beside it rather than growing a
      second walk. The code is a fresh `E03xx` — take the next free one from `python
      tools/brief.py`, not from this file. `crates/nvs-cli/src/main.rs:171` is the `Check` variant
      that gains the flag and `crates/nvs-cli/src/main.rs:687` the dispatch into `run_check`
      (`crates/nvs-cli/src/main.rs:1004`). The tests are
      `strict_docs_reports_an_undocumented_public_member`,
      `strict_docs_is_silent_about_a_private_member` and
      `nvs_check_is_silent_about_documentation_without_the_flag`.
- [ ] **`nvs doc <entry>` writes one Markdown page per class from the JSON** —
      `rule:tooling/nvs-doc-renders-and-decides-nothing`. It has no source of truth of its own: it
      reads the document `crates/nvs-cli/src/meta.rs:103`'s `run` builds, so the renderer takes the
      `Value` rather than re-deriving anything, and a `@see` target renders as a link.
      `crates/nvs-cli/src/main.rs:398` is the `Meta` variant to add the new subcommand beside and
      `crates/nvs-cli/src/main.rs:795` its dispatch arm. The tests are
      `nvs_doc_writes_one_page_per_class` and `nvs_doc_renders_a_see_target_as_a_link`.
- [ ] **`examples/doc-comments.nvs` is the end-to-end fixture** — the driver's currently failing
      `exact` check, whose four `want` lines are `docs/agent/loop-goal.toml:4864`. One runnable file
      showing the surface, both tags and the default silence.
      `crates/nvs-cli/tests/fixtures/meta/program.nvs` is the shape to write it from, and its
      `@example` target has to name a path under an `examples/` or `tests/` component relative to
      the file that wrote it.

## Backlog

- `[context] modules` gains no `crates/nvs-hir/src/members.rs` pattern — `docs/agent/loop-goal.toml`.
- Stage 7's website `sync:core` consumer over the program half — `website/README.md`.
- `@param`-shaped per-parameter structure stays parked — `rule:tooling/doc-comment-is-three-slashes` § *Revisiting*.
