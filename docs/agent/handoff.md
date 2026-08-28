# Handoff

## State

**M4's Stage 8, plus the acceptance gate's own open item.** The tree is at **867 conformance plus
189 differential**, all green. Nothing is blocked.

The guard-name debt is **5 unresolved of the 128 named test entries** `loop-goal.toml` holds, and the
`nvs-syntax (the last unparsed shapes)` block is now **0 of 4** — closed, joining both `nvs-types`
blocks. The debt file's two counts stay derived off the tree by the pass its header describes, never
carried forward.

This session closed both `nvs-syntax` names. One was a real language decision: **PHP's group-use form
`use A\{B, C};` is refused by `E0238`**, reported and skipped by `parse_use_decl`'s `recover_use_group`
so the statement still yields a `UseDecl` for the prefix. `docs/adr/README.md` § *Decisions taken at
project start* owns the rule and its priority-4 reasoning; ADR 0015 § 2's `E0212` is its sibling on the
other half of the same statement. The other was a test half only — every keyword lexes at its exact
lower-case spelling (ADR 0062 § 2), so a `PascalCase` enum case never collides with one and the parser
needed no change.

The remaining five are cause 3, and the three in the next group share one crate.

## Next group

**The three `nvs-ir` lowering names.** One file set: `crates/nvs-ir/src/lower/convert.rs`,
`crates/nvs-ir/src/lower/expr.rs`, `crates/nvs-ir/src/lower/call.rs` and
`crates/nvs-ir/src/lower/tests.rs`, plus `docs/agent/guard-name-debt.md` for the reconciliation. Start
with the first: its debt entry is **stale about the tree** and the check is one `peek.py`.

- [ ] **`an_array_conversion_walks_its_elements`** — ADR 0007 § 2's `array<T> as array<U>` row. The
      debt entry says it panics `nvs-ir` at `expr.rs:877`; that line is now a nullsafe doc comment and
      the row looks landed at `crates/nvs-ir/src/lower/convert.rs:725`, `:922` and `:969`, with the
      runtime word at `crates/nvs-ir/src/lower/mod.rs:3081`. Confirm with a scratch `.nvs`, then either
      write the guard test over the landed lowering or lower the row.
- [ ] **`an_inline_producer_releases_its_value_on_the_throw_path`** — the owned-temporaries stack
      covers a call's arguments, a receiver and `.`/interpolation/`echo` operands; a normalized
      subscript key and a `match` subject still release inline. `nvs_ir::lower::Lowering`'s
      owned-temporaries field doc names the gap.
- [ ] **`a_transferred_argument_is_released_when_a_later_one_throws`** — the same field doc's other
      named gap, in `crates/nvs-ir/src/lower/call.rs`. A new refcount edge, so it owes a `valgrind`
      run beside `verify.py` (`docs/agent/commands.md`).

## Backlog

- `a_fatal_releases_the_frames_locals` — work not done, `docs/agent/guard-name-debt.md:207`.
- `every_refusal_is_a_diagnostic_or_decided` — the gate's own name,
  `docs/agent/guard-name-debt.md:302`.
- `python tools/gaps.py` ranks the thinnest classes once Stage 8's names are closed.
- ADR 0021 § *Decision*'s import surface is now two refusals wide; if a third arrives, promote the
  README paragraph to its own ADR rather than growing that section.
