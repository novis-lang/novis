# Handoff

## State

Goal `m8-stdlib-depth`. **Stages 0, 2, 3, 4, 5, 6 and 7 are done; stage 8 (`Core\Cli\Text`) is next.**
All three of stage 7's acceptance checks are green: the two `.nvst` cases, the two named `nvs-stdlib`
tests, and `cargo metadata --manifest-path fuzz/Cargo.toml` naming `"name":"ast"`.

The `ast` fuzz target is `fuzz/fuzz_targets/ast.rs`: it calls `nvs_syntax::walk::of_source`, which is the
whole of `Core\Ast::parse`'s body past the tag check, and asserts every kind the walk answers with is one
`walk::KINDS` names — so the member is fuzzed without `nvs-stdlib` linked. Its seeds are
`fuzz/seeds/parse/` (six sources that parse, two the compiler refuses), shared with the `parse` target
because both read one source text; `.github/workflows/ci.yml`'s seed step maps `ast` to that directory
and the matrix now carries the target.

`crates/nvs-stdlib/src/ast.rs`'s `core_ast_parse_gives_the_compilers_verdict_on_every_parse_seed` replays
that corpus on stable — libFuzzer needs nightly and does not build on Windows — taking the verdict from
`nvs_syntax::parse_file`'s own diagnostics rather than from the walk, and requiring both verdicts of the
corpus by counting. `nvs-diagnostics` is a new **dev**-dependency of `nvs-stdlib` for exactly that.
Nothing is blocked.

## Next group

**Stage 8: a `Core\Cli\Text` of runs** — one file set: `crates/nvs-stdlib/src/cli.rs` and
`crates/nvs-stdlib/src/out.rs`. `rule:tooling/styling-is-a-value-not-a-grammar` and
`rule:tooling/the-terminal-profile-resolves-once` are the rules; the goal's § *Standing decisions* fixes
that colour depth stays a process answer and a non-terminal stream gets no styling, and that the
per-stream proof is a Rust test with a fake terminal answer because a `.nvst` case pipes both streams.

- [ ] **A `Text` holds runs, not rendered bytes** — gap 2 at `crates/nvs-stdlib/src/cli.rs:135`, and
      `crates/nvs-stdlib/src/cli.rs:1941` is the class. The sink renders the runs for the stream in
      force, so one `Text` is styled on a terminal standard output and plain on a redirected standard
      error in the same run. The two named tests the acceptance asks for are
      `a_text_keeps_its_runs_and_is_rendered_at_the_sink` and
      `one_text_is_styled_on_a_terminal_stream_and_plain_on_a_redirected_one`.
- [ ] **A `Text` can be read, so `{through:}` can rewrite what it captured** — gap 1 at
      `crates/nvs-stdlib/src/out.rs:36`, whose instance surface is what
      `tests/conformance/core/out-capture-through-rewrites-the-captured-text.nvst` needs to exist before
      it can be written. The spelling comes from `rule:core-api/verb-lexicon`.

## Backlog

- `uri` is a fuzz target with no row in `fuzz-smoke`'s matrix (`.github/workflows/ci.yml:616`), so it
  is built and never run; `ast`, `lex`, `parse` and `prefix` are the four that are.
- A parameter's declared type and its default have no road to the descriptor; `ParameterInfo` would
  read them where it reads the name (ADR 0019 § 1, `crates/nvs-types/src/layout.rs:101`).
- `Core\Reflect`'s remaining roster classes — `ConstantInfo`, `AttributeInfo`, `EnumInfo` — are
  listed in `NOT_YET_BUILT` and owned by ADR 0019 § 1 and § 4.
- `walk::KINDS` is held to the match arms by a scan of the file's own text
  (`crates/nvs-syntax/src/walk.rs:@SPANS`); a `const {}` assertion at each arm would be the stronger
  gate if the arms are ever touched wholesale.
- Stage 9 and later of this goal are untouched; `docs/agent/loop-goal.toml` is the list.
