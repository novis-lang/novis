# Handoff

## State

**Goal 45 — nvs fmt rewrites a file into its one canonical layout — has just started; nothing of it has landed yet.** Goal `markup-literal`'s whole list is this goal's Stage 1 floor.

**The style is decided and landed.** [ADR 0039](../../decisions/0039.md) and the fragments under
`docs/rules/tooling/fmt-*` are the spec, and [ADR 0173](../../decisions/0173.md) added the `?>` bullet
to `rule:tooling/fmt-novis-constructs`. No session writes a record for this goal. The one thing not to
re-decide: `nvs fmt` has no configuration and never touches a byte a program prints.

The printer reads `nvs_syntax::parse` (`crates/nvs-syntax/src/parser/mod.rs:883`), which already keeps
every comment and whitespace run as trivia, and `crates/nvs-syntax/tests/lossless.rs:140-168` already
proves the tokens and trivia reproduce every corpus file. The identity printer is that proof turned
into a program.

## Next group

**Stage 2: the identity printer** — one file set: the new `crates/nvs-fmt/` and the workspace
`Cargo.toml`.

- [ ] **The crate** — `crates/nvs-fmt/`, a library whose one entry takes a `SourceFile` and answers the
      formatted text or a refusal, added to the workspace members.
- [ ] **The identity printer** — reads `nvs_syntax::parse` and writes every token and trivia item back;
      `the_identity_printer_reproduces_every_corpus_file` walks the corpus `lossless.rs:140-168` walks.
- [ ] **The refusal** — a parse that reports an error answers a refusal and no text;
      `a_file_with_a_syntax_error_is_refused_and_left_unchanged`.

## Backlog

- Stages 3 to 5, the style — `crates/nvs-fmt/` only, with pairs under `tests/fmt/input/` and
  `tests/fmt/formatted/`. One rule family per session is the expected size.
- Stage 6, the command — `crates/nvs-cli/src/main.rs:193` and a new `crates/nvs-cli/tests/fmt.rs`. Its
  own file set.
- Stage 7, the corpus and the rulebook — `crates/nvs-fmt/tests/` and `docs/rules/tooling.json`.
- When this goal's last check goes green the driver takes goal `template-format`.
