# Handoff

## State

**Goal 11 stage 2 is whole — all four items — and stage 1's floor from goal 10 is unchanged.** The
trivia layer is on disk and `rule:ide/tokens-plus-trivia-reproduce-the-file` holds over the whole
corpus, so M4B inherits that half of `rule:ide/one-grammar-one-tree` done.

- `Trivia`/`TriviaKind` sit beside the token types (`crates/nvs-syntax/src/token.rs:31`); `Lexer` gains
  the flag (`Lexer::with_trivia`, `trivia`, `take_trivia`) and `skip_trivia` records all four kinds.
  The kind is read off the opening run of slashes, so `////` and `#` are ordinary at any length.
- `parse` returns `Parsed { stmts, trivia }`; **`parse_file` is unchanged and is the strict wrapper**,
  so none of its ~50 call sites moved. Both share `Parser::parse_all`. `SyntaxIndex` is M4B's and is
  not built.
- Nothing is blocked. `python tools/verify.py` is green.

**The fork stage 3 has to take, named here because it is cheap now and expensive to discover in
`decl.rs`:** an unattached `///` is a parser diagnostic, so the *compile* path must see doc comments —
but `parse_file` does not collect trivia, by design. The recommendation is one line in
`crates/nvs-syntax/src/lexer.rs:@push_trivia`: retain `TriviaKind::DocComment` unconditionally and let
the flag govern only the ignorable kinds, since documentation is content the language reads rather than
a byte a formatter needs. Record it in that module's doc, per the goal's standing decisions.

## Next group

**Stage 3: attachment, and the closed set** — one file set:
`crates/nvs-syntax/src/parser/decl.rs`, `crates/nvs-syntax/src/ast.rs`,
`crates/nvs-diagnostics/src/lib.rs`.

- [ ] **Attachment** — `rule:tooling/doc-comment-attaches-to-the-next-declaration`. A run of `///`
      lines separated by nothing but whitespace is one comment, attached to the declaration below it; a
      blank line breaks it; a run attached to nothing is a diagnostic. `crates/nvs-syntax/src/parser/decl.rs:639`
      is where a member and its attributes already meet, and the node grows beside
      `crates/nvs-syntax/src/ast.rs:1494`. Take the fork in `## State` first — the diagnostic has to
      fire on the compile path.
- [ ] **The two tags parse** — `rule:tooling/doc-comment-tags-are-see-and-example`. `@see <member>` and
      `@example <path>`, each on its own line in a trailing block, stored on the same node;
      `crates/nvs-syntax/src/ast.rs:481` is the neighbouring inert-metadata shape to write like.
- [ ] **Every other `@tag` at line start is a diagnostic** — the item that makes the set closed rather
      than conventional. Codes join their siblings at `crates/nvs-diagnostics/src/lib.rs:201` in the
      `E01xx` parser band (next free `E0127`), with `@param`/`@return`/`@throws` each naming what to
      write instead.

## Backlog

- Stage 4's two checks, stage 5's `nvs meta --json <entry>`, stage 6's `nvs doc` — `docs/agent/loop-goal.md`.
- Six files carry `///` today (five `.nvst` under `tests/conformance/core/`, plus
  `tests/hostile/core/Str/length/01-unbounded-and-degenerate-input.nvs`), not the seven
  `docs/agent/loop-goal.toml`'s stage 2 comment claims; they reclassify with no edit.
- `[context]` gap: the pack prints the goal's *Standing decisions* but not the stage the item came
  from, so choosing the next group cost one `sed` over `docs/agent/loop-goal.md` § *Stage 3*. A
  selector for the goal's own stage prose would close it.
- `crates/nvs-syntax/tests/lossless.rs` walks `php-src` when it is present and says so when it is not,
  as `corpus_parse.rs` does; CI therefore checks the Novis half only.
