# Handoff

## State

**Goal 11 stage 3 is whole — attachment and the closed tag set — so all nine names the stage's
acceptance check lists pass under `cargo test -p nvs-syntax`.** Stages 1 and 2 are unchanged.

- **The fork stage 2 named is taken.** `crates/nvs-syntax/src/lexer.rs:@push_trivia` keeps a
  `TriviaKind::DocComment` whether or not the lexer is collecting, and `collect_trivia` now governs
  only the three ignorable kinds. So `parse_file` — every compile path — sees doc comments, and both
  new diagnostics fire where a program is compiled rather than only where it is formatted.
- **Attachment is decided from source text, not from the trivia beside a run.** `doc_run_joins`
  (`crates/nvs-syntax/src/parser/mod.rs:@doc_run_joins`) asks whether the gap is whitespace crossing
  at most one line break; a compile path collects no whitespace trivia, so reading the text is the
  only way both entry points can answer identically. `Parser::take_doc_comment` walks the collected
  doc trivia backwards from the declaration's first token — before its attributes — and skips trivia
  the lookahead buffer has already run past.
- **`E0127`** is a run that documents nothing, swept at the end of `parse_all` from the runs no
  declaration took; **`E0128`** is any `@tag` at a line start that is not `@see` or `@example`, with
  its own help for `@param`, `@return`/`@returns` and `@throws`.
- `DocComment { span, lines, tags }`, `DocTag { kind, span, argument }` and `DocTagKind` are in
  `crates/nvs-syntax/src/ast.rs:1491`; `doc: Option<DocComment>` is on `ClassDecl`, `InterfaceDecl`,
  `EnumDecl`, `EnumCase`, `TypeAliasDecl` and `ClassMember`. One declaration that makes several
  members (`public int $a, $b;`) gives its run to every one of them.
- **The three items landed as one commit**, because they interleave inside the same six files and a
  commit apiece would have to stage the same file twice.
- Nothing is blocked. `python tools/verify.py` is green.

## Next group

**Stage 4: the two checks that keep a tag honest** — one file set:
`crates/nvs-hir/src/members.rs`, `crates/nvs-hir/src/requires.rs`.

- [ ] **`@see` must resolve** — `rule:tooling/doc-comment-tags-are-see-and-example`. A `@see` names a
      class, member, enum or constant, and one that names nothing is refused with a name-resolution
      code (`E03xx`; take the next free one from `python tools/brief.py`, not from this file).
      `crates/nvs-hir/src/members.rs:356` is where top-level declarations are walked and
      `crates/nvs-hir/src/members.rs:436` where a class's members are; the tests go in that file's own
      `mod tests` at `crates/nvs-hir/src/members.rs:1071`, named
      `a_see_target_that_resolves_is_accepted` and `a_see_target_that_names_no_member_is_refused`.
- [ ] **`@example` must exist, and must sit where the corpus walks it** —
      `rule:tooling/doc-comment-tags-are-see-and-example`. A path that names no file is refused, and
      so is one outside every directory the test corpus walks, which is what stops an example from
      rotting in a page. `crates/nvs-hir/src/requires.rs:323` already resolves a file and hands it to
      `check_declarations`, so it is where a path is turned into something on disk; the two tests are
      `an_example_naming_a_missing_file_is_refused` and
      `an_example_outside_every_test_directory_is_refused`.
- [ ] **Read the tags off the AST once**, rather than each check re-deriving them: a `DocTag` is
      already parsed and spanned, and both checks want the same walk over every declaration that
      carries a `doc`. Decide where that walk lives before writing the second check —
      `crates/nvs-hir/src/members.rs:436` sees members but not the file-scope declarations that
      `check_stmts` does.

## Backlog

- `examples/doc-comments.nvs` — the end-to-end fixture, stage 6's `exact` check and the file the
  driver reports missing every session. It is an open item, not a regression; it needs `nvs doc` and
  `--strict-docs` first (`docs/agent/loop-goal.toml`, stage 6).
- Stage 5: `nvs meta --json <entry>` emits a program's own declarations —
  `rule:tooling/meta-json-takes-a-program`, `crates/nvs-cli/src/meta.rs`.
- The `[context]` manifest printed nothing about `nvs-hir`; stage 4 lives there, so its `modules`
  pattern needs `nvs-hir` before the next group starts.
