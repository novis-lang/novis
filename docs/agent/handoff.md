# Handoff

## State

**M4's Stage 8, plus the acceptance gate's own open item.** The tree is at **867 conformance plus
189 differential**, all green. Nothing is blocked.

The guard-name debt is **7 unresolved of the 128 named test entries** `loop-goal.toml` holds, and
the `nvs-types (narrowing and reachability)` block is now **0 of 8** — closed, as is
`nvs-types (targets and refusals)`. Both of the debt file's counts stay derived off the tree by the
pass its header describes, never carried forward.

This session closed both of Stage 6's `nvs-types` names. The second was a real language rule, not a
test: **a non-`void` method whose body can reach its closing brace is now `E0739`**, because
`nvs_ir::lower::lower_method` seals that path with `Terminator::Return(None)` and the caller read a
slot the callee never wrote. `nvs_types::returns` owns the analysis and its asymmetry — every shape
it cannot prove reaches the end is treated as an exit — and ADR 0007 § 7 row 16 owns the divergence
from PHP's implicit `null`. It cost the corpus nothing: 867 + 189 cases were already green under it.

The remaining seven are all cause 3, and no two of them share a crate except the pair below.

## Next group

**The two `nvs-syntax` parser names, both cause 3.** One file set:
`crates/nvs-syntax/src/parser/decl.rs` and `crates/nvs-syntax/src/parser/tests/decl.rs`, plus
`docs/agent/guard-name-debt.md` and `docs/agent/loop-goal.toml` for the reconciliation. Read each
item's own `loop-goal.toml` comment, but check the crate before believing it — the playbook bullet
about a stale comment is there for a reason.

- [ ] **`a_grouped_use_parses_or_names_the_rule_that_refuses_it`** — `use A\{B, C};`.
      `parse_use_decl` (`crates/nvs-syntax/src/parser/decl.rs:240`) handles one path per statement
      and nothing writes the grouped form; `use_import_plain_and_rejected_alias`
      (`crates/nvs-syntax/src/parser/tests/decl.rs:311`) is the neighbour to extend. Decide which
      it is — ADR 0021 is single-file inclusion, not imports, so if the form is simply unsupported
      the answer is a diagnostic naming that, and the next free parser code is `E0124`.
- [ ] **`an_enum_case_named_with_a_keyword_parses`** — `enum E { Match }`. `parse_enum_decl`
      (`crates/nvs-syntax/src/parser/decl.rs:976`) is the arm; `enum_cases_and_explicit_backing_type`
      (`crates/nvs-syntax/src/parser/tests/decl.rs:184`) names only ordinary identifiers. A case name
      is a member name, so the keyword should parse as one — find out whether it does before writing
      the test that says so.
- [ ] **`an_array_conversion_walks_its_elements`** if those two land cheaply — a different file set
      (`crates/nvs-ir/src/lower/expr.rs:877`, the `array<T> as array<U>` panic), so take it only as a
      third and only with the context to spare.

## Backlog

- `every_refusal_is_a_diagnostic_or_decided` — Stage 8's own guard; `python tools/holes.py` still
  reads standing refusal sites, so it is red on its merits (`docs/agent/guard-name-debt.md`).
- Two `nvs-ir` release-path names, both genuinely open and named in `Lowering`'s owned-temporaries
  field doc (`docs/agent/guard-name-debt.md`, Stage 3).
- `a_fatal_releases_the_frames_locals` — nothing asserts what a `FATAL` does to a frame's locals, and
  the valgrind sweep skips `examples/fatal.nvs` (`docs/agent/guard-name-debt.md`, Stage 5).
- A `get` property hook with a block body is **not** covered by `E0739`; only methods are
  (`crates/nvs-types/src/returns.rs`).
- The parameter half of ADR 0046 § 2's constant set, at `nvs_types::defaults`.
- ADR 0028 § 2's abandoned-generator `finally` (`docs/agent/loop-goal.md` § *Standing decisions*).
