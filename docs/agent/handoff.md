# Handoff

## State

**Goal `markup-literal`, stage 4 is two thirds landed: `` html`…` `` lowers.** In value position it is one
`Core\Html\Markup` holding the joined bytes; at a sink it is a run of writes and no carrier at all. Stages
1–3 are the floor beneath it (the lexer, the AST node, and the checker answering the carrier class).

`Lowering::lower_markup_literal` (`crates/nvs-ir/src/lower/expr.rs:2117`) cooks each segment, sends each
hole through `Lowering::lower_markup_hole`, folds the pieces with the same n-ary `InstKind::Concat` an
interpolated string uses, and lifts the join once. `Lowering::echo_markup_parts`
(`crates/nvs-ir/src/lower/expr.rs:600`) is the sink half: one `Helper::EchoStr` per piece.

A hole asks one question the `Ty`s cannot answer — carrier or not — so `infer_markup_literal` records the
hole's checked type at the hole's own span and `Lowering::hole_is_carrier` reads it back. The two answers
are new row-less symbols, `Core\Html`'s `ESCAPE_TEXT_SYMBOL` and `MARKUP_TEXT_SYMBOL`, which answer a
hole's **bytes** where `escape` answers a carrier.

**A hole-free literal still builds a carrier per execution**, which `rule:core-classes/html-literal`
§ *What it costs to run* says it must not. That is the one stage-4 check still red and it is the next
group; nothing else in the goal is blocked on it.

## Next group

**Stage 4: the hole-free fold** — one file set: `crates/nvs-runtime/src/string.rs`,
`crates/nvs-ir/src/ir.rs`, `crates/nvs-codegen/src/emit.rs` and the `Markup` arm already in
`crates/nvs-ir/src/lower/expr.rs`.

- [ ] **An immortal instance in the unit's data section** — `crates/nvs-runtime/src/string.rs:94`,
      § *An immortal string, and why the `Cell` survives it*, which is the precedent: a literal's string
      header is written into the compiled unit by `immortal_header_bytes` and its refcount is a sentinel
      the primitives compare against. Decide whether an *object* header can carry the same sentinel
      without putting a branch on every release, and what a shared, request-crossing instance owes
      `rule:programs/memory-priority`'s isolation bound. ADR 0169 § 4 authorises the fold itself.
- [ ] **The instruction that names one** — beside `crates/nvs-ir/src/ir.rs:604`, whose `SourceConst` is
      the shape to copy (an engine-owned address in a slot spelling, not refcounted, zero for "none"),
      and its emission beside `crates/nvs-codegen/src/emit.rs:634`.
- [ ] **The fold at the arm that already exists** — `crates/nvs-ir/src/lower/expr.rs:131`: a `parts` with
      no `StringPart::Expr` in it takes that constant instead of `ConstStr` plus a lift, in both the value
      and the sink path. `a_hole_free_markup_literal_folds_to_one_constant` and
      `a_hole_free_markup_literal_in_a_loop_allocates_once_for_the_whole_loop` go in
      `crates/nvs-codegen/tests/markup.rs:14`, whose `count_insts` already counts what they assert on.

## Backlog

- `[context]` gap: nothing in the goal's `modules` or `playbook` named `crates/nvs-stdlib/src/html.rs`, so
  the pack did not print *Writing Novis itself > adding a row-less* — the trap that a row-less `Core`
  symbol is registered twice. Add the path.
- `\{` may not mean what `rule:core-classes/html-literal` § *A hole is `{$`* reads as: the lexer makes
  ``html`\{$name}` `` a literal `{`, an ordinary simple interpolation and a `}`, not the literal text
  `{$name}`. `crates/nvs-syntax/src/lexer.rs:2178` is the test that fixes it.
- `Core\Html::escape`'s own member row is unchanged and still answers a carrier; only the literal's
  lowering reaches the bytes-answering pair.
