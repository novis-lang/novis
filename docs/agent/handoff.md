# Handoff

## State

**Goal 12 — the resilient tree — has stages 2, 3, 4, 5 and 6 landed and green.** Recovery is explicit
(`rule:ide/recovery-is-explicit`), the `SyntaxIndex` is the walk flattened, `crates/nvs-syntax/tests/prefixes.rs`
cuts every `examples/*.nvs` at every token boundary, `fuzz/fuzz_targets/prefix.rs` is the third leg of the
nightly `fuzz-smoke` matrix, and every offset conversion lives in `crates/nvs-diagnostics/src/source.rs`
(`rule:ide/positions-have-one-home`).

**Stage 6's section lexer is `crates/nvs-test/src/section.rs`**, declared at `crates/nvs-test/src/lib.rs:164`.
It holds the *shape* — `Section`, `StrayText`, `lex` and `header` — and knows no section name at all, which
is what lets `.lspt` reuse it and add only a roster (`docs/decisions/0099.md` § 5). `case.rs` keeps the
`.nvst` roster and now judges the lexed sections in written order, so the first thing wrong with a file is
still what it is refused for, message and line unchanged. The lexer's one refusal, text before the first
header, carries a line and no wording: the name of the section a case must start with is a format's.

**Two residues, both unchanged.** `ExprKind::ClassConstAccess` carries a bare `Span` for its name, so
`Foo::` at the caret still carries a name nobody wrote; the `::` path routes `Missing` into the `Ident` arm
rather than inventing a second diagnostic, and the comment at that site says so. And `aux_path` stayed in
`case.rs`: its `RESERVED_NAMES` are the `.nvst` runner's own output files, so it is roster, not shape.

**Stage 0 is empty and stays empty**, unchanged: the four M4 language holes are closed and `bool as int` is
`E0708` under `rule:types/conversion`, not a hole.

## Next group

**Stage 7's `nvs ast --json`**, in one crate: `crates/nvs-cli/src/main.rs`, a new `crates/nvs-cli/tests/`
file for the four named tests, and `docs/reference/tools/10-cli.md`'s existing `# nvs ast` heading — two
flags on a subcommand that already ships, so it adds no row to `rule:testing/four-proofs`'s roster. The
whole specification is `docs/agent/goals/12-resilient-tree.md` § *Stage 7*.

- [ ] **`--json` and `--resilient` on the subcommand** — the `Ast { file }` variant at
      `crates/nvs-cli/src/main.rs:170` and its arm at `crates/nvs-cli/src/main.rs:707` take both flags, with
      `--resilient` the default and strict opt-in. A node is `kind`, `span` as `[start, end]`, its own
      scalar fields and `children`. `rule:ide/ast-json-schema-is-frozen` — `ast_resilient_is_the_default_and_strict_is_opt_in`.
- [ ] **The schema is frozen, and includes what the panel needs** — a snapshot over `examples/` in a new
      test file beside `crates/nvs-cli/tests/meta.rs:1`, pinning `ast_json_matches_its_frozen_schema`,
      `ast_json_renders_a_file_that_does_not_compile` and `ast_json_includes_trivia_and_recovery_nodes`.
      Trivia and recovery nodes are in the JSON on purpose: the panel is least useful on a file that
      compiles. `rule:ide/the-ast-panel-shells-out-to-the-cli`.
- [ ] **The reference chapter regenerates** — the two flags under the existing `# nvs ast` heading at
      `docs/reference/tools/10-cli.md:337`, whose synopsis is at `docs/reference/tools/10-cli.md:339` and
      whose summary row is `docs/reference/tools/10-cli.md:30`; then `python tools/reference.py --check`,
      an acceptance check of its own because a chapter edit that stops regenerating is how `docs/novis.md`
      goes quietly stale.

## Backlog

- `ExprKind::ClassConstAccess` should carry a `MemberName` — `crates/nvs-syntax/src/ast.rs`, owed to `rule:ide/recovery-is-explicit`.
- The `.lspt` runner is goals 14/15's, not this one — the goal's § *Standing decisions*.
- A BOM is left in the text and counted as one code unit; stripping it is the document store's — `rule:ide/positions-have-one-home`.
- `aux_path`'s containment argument is `.nvst`'s reserved names plus a shape check; `.lspt` will want the shape half — `crates/nvs-test/src/case.rs:@aux_path`.
