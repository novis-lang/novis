# Handoff

## State

Goal `m8-stdlib-depth`. **Stages 0, 2, 3, 4, 5 and 6 are done; stage 7 (`Core\Ast`) has its roster and
its two cases, and only the fuzz target is left in it.**

`nvs_syntax::walk::KINDS` is the production table — every kind the walk answers with, sorted — and
`crates/nvs-stdlib/src/ast.rs`'s `PRODUCTIONS` is one `CoreClass` per entry at the same index, so
`class_of` is a binary search over the grammar's own table. A parsed node is an instance of *its
production's* class; `Core\Ast\Node` stays the written type of `parse`, `children` and `nodes`, and no
instance carries it. The production classes hold **no `registry::CLASSES` row** on purpose — that
decision and its reasoning are `crates/nvs-stdlib/src/ast.rs`'s second module-doc decision and the
playbook bullet beside it.

`parseFile` is **struck**, not deferred: `rule:core-classes/ast-is-inert` now says parsing a file is
`Core\IO::read` composed with `parse`, the module doc's first decision carries the trade, and the
carried-gaps entry is deleted. `crates/nvs-stdlib/src/ast.rs`'s known gaps are down to one — positions
and text, owner `unowned-closures` — so the goal's § *Not this goal* line naming "`ast.rs:52` gap 2
(positions)" now means gap **1**. Nothing is blocked.

## Next group

**Stage 7: the `ast` fuzz target and its seed replay** — one file set: `fuzz/`,
`crates/nvs-stdlib/src/ast.rs` and `crates/nvs-syntax/src/walk.rs`.
`rule:core-classes/ast-is-inert` is the rule; the goal's § *Standing decisions* fixes that the
acceptance never runs nightly.

- [ ] **An `ast` fuzz target beside the parser's** — `fuzz/Cargo.toml:45` is the last `[[bin]]` block
      and `fuzz/fuzz_targets/parse.rs` the model. The body calls `nvs_syntax::walk::of_source`, which
      is what `crates/nvs-stdlib/src/ast.rs:@nvs_core_ast_parse` reaches, so the target fuzzes the
      member without linking `nvs-stdlib`. The acceptance is `cargo metadata --manifest-path
      fuzz/Cargo.toml` naming `"name":"ast"`, never `cargo +nightly fuzz`.
- [ ] **`core_ast_parse_gives_the_compilers_verdict_on_every_parse_seed`** (new, `cargo test -p
      nvs-stdlib`) — a test beside `crates/nvs-stdlib/src/ast.rs:449`, which is
      `nvs_core_ast_parse`'s body. It replays the parse target's seed corpus on stable and asserts
      the member's verdict is the compiler's: a seed `nvs_syntax::parse_file` reports an error for is
      a `ParseError` here, and one it accepts answers a tree.

## Backlog

- A parameter's declared type and its default have no road to the descriptor; `ParameterInfo` would
  read them where it reads the name (ADR 0019 § 1, `crates/nvs-types/src/layout.rs:101`).
- `Core\Reflect`'s remaining roster classes — `ConstantInfo`, `AttributeInfo`, `EnumInfo` — are
  listed in `NOT_YET_BUILT` and owned by ADR 0019 § 1 and § 4.
- `walk::KINDS` is held to the match arms by a scan of the file's own text
  (`crates/nvs-syntax/src/walk.rs:@SPANS`); a `const {}` assertion at each arm would be the stronger
  gate if the arms are ever touched wholesale.
- Stage 8 and later of this goal are untouched; `docs/agent/loop-goal.toml` is the list.
