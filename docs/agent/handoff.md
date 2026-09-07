# Handoff

## State

**Goal 11 stage 4 is whole — both tags now buy a check**, so the four names that stage's acceptance
check lists pass under `cargo test -p nvs-hir`. Stages 1-3 are unchanged.

- **The walk over a `DocTag` has one home**, `crates/nvs-hir/src/members.rs:@check_doc`, reached from
  `check_stmts` (class, interface, enum and its cases, `type` alias) and from `check_members` (every
  member inside a body). That is every declaration that carries a `doc`, and it closes the group's
  third item: nothing re-derives where one hangs.
- **`@see` is `E0323`** and resolves against the very `MemberTable` a `Class::member` reference does —
  `self`, `static` and `parent` name a class here exactly as in code, a `Core` target is trusted the
  same way, and a member is looked up against all four `MemberKind`s because the tag says *what* is
  named and never which kind. A `$` sigil and a trailing `()` are allowed on the member half.
- **`@example` is two codes**: `E0325` for a path in no directory the corpus walks, asked first and on
  the written path alone, and `E0324` for one that is walked but holds no file. **The path is relative
  to the file that wrote it, exactly as a `require` path is** — decided here rather than escalated,
  because the alternative was project-root discovery and `nvs-hir` depends on `nvs-diagnostics`,
  `nvs-syntax` and `rustc-hash` alone. A file with no path of its own (a fixture, an unsaved buffer)
  resolves against the process's directory, which is its only base. `WALKED_DIRECTORIES` is
  `examples`, `tests`; ADR 0137 § 2 names the first.
- This is the only question `members.rs` asks of the filesystem, and its module doc is where that is
  recorded.
- **The driver's failing check, `examples/doc-comments.nvs`, is stage 6's** `exact` fixture with four
  `want` lines — an item still open, not a regression.
- The goal's `[context] modules` names no `nvs-hir` file, so the pack printed these anchors only
  because the handoff item carried them; `crates/nvs-hir/src/members.rs` belongs in that manifest.
- Nothing is blocked. `python tools/verify.py` is green.

## Next group

**Stage 5: one JSON, one more input** — one file set: `crates/nvs-cli/src/meta.rs`,
`crates/nvs-cli/src/main.rs`.

- [ ] **`nvs meta --json` takes an optional entry** — `rule:tooling/meta-json-takes-a-program`. The
      no-argument form stays byte-identical, which is the whole point of
      `rule:tooling/one-json-several-renderers`: `tools/reference.py` and the website's `sync:core`
      are renderers, not sources. `crates/nvs-cli/src/main.rs:394` is the `Meta` variant that gains
      the argument and `crates/nvs-cli/src/main.rs:786` the dispatch that still calls `meta::run()`.
      The tests are `meta_json_with_no_argument_is_byte_identical_to_the_registry_dump` and
      `meta_json_with_an_entry_emits_the_programs_declarations`.
- [ ] **The program's declarations join the registry's, in the registry's own shape** —
      `rule:tooling/meta-json`, whose omission rule holds at every level: nothing written means no
      `doc` key, never an empty one. `crates/nvs-cli/src/meta.rs:75` is `document()`, where the two
      halves meet, and `crates/nvs-cli/src/meta.rs:396` is `doc_json`, the shape a card is already
      emitted in.
- [ ] **A user declaration's card carries prose, `@see` and `@example`** —
      `rule:tooling/doc-comment-tags-are-see-and-example`, read off the `DocComment` the parser
      already spanned rather than re-split from text. `crates/nvs-cli/src/meta.rs:167` is
      `member_json`, the row a user member's card must line up with; the test is
      `a_user_declarations_shape_carries_prose_see_and_example`.

## Backlog

- `examples/doc-comments.nvs`, stage 6's `exact` fixture — docs/agent/loop-goal.toml.
- Stage 6's renderer and lint — `rule:tooling/nvs-doc-renders-and-decides-nothing`,
  `rule:tooling/strict-docs`; `--strict-docs` needs the next free `E03xx`.
- `[context] modules` gains `crates/nvs-hir/src/members.rs` — docs/agent/loop-goal.toml.
