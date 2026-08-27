# Handoff

## State

**The goal changed.** M4S Part I reached 552 of 600 conformance cases with both its named guards
green, and the loop is now aimed at **M4 — language completeness**: every shape that compiles in the
front end and then refuses below it, closed, before M4B's LSP is written against the surface.
[loop-goal.md](loop-goal.md) is the target, the 43 items and the standing decisions;
[loop-goal.toml](loop-goal.toml) is every check. M4S Part I is not discarded — it is Stage 1's floor
and Stage 8's suites, and a `Core` depth slice from `python tools/gaps.py` is still a legitimate
slice when a group is blocked.

**Nothing is implemented yet: this session authored the goal, not the code.** What landed is the
goal's two files, four new acceptance fixtures with frozen output (`examples/operators.mwl`,
`control.mwl`, `callable.mwl`, `targets.mwl`), `cases` support in `tools/loop.py`, and
`tools/holes.py`. Every check above Stage 1 is red by construction, which is the point: the ledger
now names which hole the loop is standing on.

**`python tools/holes.py` is the worklist and no session re-derives it.** It reads the refusal
sites out of `mwl-ir` and `mwl-codegen` live, attributes each to the goal item that names its
function, and prints what is unattributed — 45 sites, 13 items, 2 unattributed today.
`--item N` is one item in full: its anchors, its sites, its cases. `python tools/loop.py --list`
prints the 32 named `.mwlt` cases and which are not written yet.

**The group cap is 3 slices, raised from 1 this session on a fresh `python tools/loop-stats.py`**:
27 sessions now mean 115k of a 200k ceiling for one slice, the largest ever reached is 168k, and the
projection puts 3 slices at 170k with 30k of headroom. The measurement says 4 at 197,908, which has
none — so the knee, not the fastest. AGENTS.md § *Session workflow* step 2 carries it.

## Next group

Three slices, **all three in `emit_binop` and `lower_binary`** — the tightest group in the goal, and
Stage 0, which `tools/loop.py` runs before the program legs because every fixture and case in every
stage below is written against these rules. `docs/adr/0007-explicit-type-system.md` § 4 is the
table; the pack prints it.

- [ ] **[1] ADR 0007 § 4's promotion table runs** — `1 + 1.5` does not compile today, and neither
      does `$n < $f`: `mwl_types` gives the pair a result type without converting either side, so
      both operands reach the backend in two representations and are refused there. The widening is
      the semantics, so it belongs in the existing conversion rows and not in a backend repair.
      `crates/mwl-ir/src/lower/expr.rs:2270` (`lower_binary`), `crates/mwl-ir/src/lower/mod.rs:1552`
      (`coerce`), `crates/mwl-types/src/expr/operators.rs:403` (`arithmetic_result`),
      `crates/mwl-codegen/src/emit.rs:1008` (`emit_binop`). Sites: `emit.rs:1069`, `:1138`.
- [ ] **[2] Integer `/` compiles** — ADR 0007 § 4 types `int / int` as `int|float`, so `6/3` is an
      integer and `7/2` is not. `Ty::Tagged` is the representation and `clif_ty` already gives it a
      machine type; what is left is the operator picking at runtime, plus `mwl_types` widening that
      union to `float` at a binding, which is what makes `float $avg = $sum / $n;` the ADR's own
      worked example. `crates/mwl-codegen/src/emit.rs:1104` is the refusal, verbatim;
      `crates/mwl-types/src/expr/operators.rs:96` (`binary_result`) is the widening.
- [ ] **[3] Integer `+`/`-`/`*` throw `ArithmeticError` on overflow** — the mechanism exists: `%`'s
      zero divisor already raises inline through `mwl_runtime::mwl_raise_new` and takes
      `mwl_ir::ir::Inst::on_error`'s edge, which is the shape a checked `iadd` wants. Three emit
      sites, one error edge each. `crates/mwl-codegen/src/emit.rs:1008`,
      `crates/mwl-ir/src/ir.rs:248` (`InstKind`).

`examples/operators.mwl` is the fixture all three feed (Stage 2), and it also holds items 4–7, so
it stays red until the whole of Stage 0 lands — **that is expected, and not a reason to touch its
expected output.** Each slice's own proof is its `cargo-named` test in Stage 0, plus its `.mwlt`
case: `every-arithmetic-row-promotes-the-narrower-operand`,
`an-integer-division-is-exact-only-where-it-divides`,
`an-integer-overflow-throws-rather-than-wrapping`, all under `tests/conformance/lang/`.

## Backlog

- **The rest of Stage 0** — items 4–8 (the bitwise operators, `**`, `<=>` over a scalar, `++`/`--`
  with the target's address split out of `lower_reassignment`, and `Enum(Int)` on `emit_binop`'s
  integral rows). Items 4–6 share `ir.rs`'s `BinOp` with each other and are the natural next group;
  item 7 is its own, because the split it needs is in `stmt.rs` and touches no operator table.
- **Stages 3–7 in order.** Each stage's items are grouped by file set in `loop-goal.md` already;
  take the grouping from there rather than inventing one.
- `Core\Uri::resolve` and `compareTo` — RFC 3986 § 5 reference resolution and the normalized order;
  `crates/mwl-stdlib/src/uri.rs:499`, `:506`. `docs/spec/01-core-library.md` § 7. A Stage 8 corpus
  slice, not a language one.
- `Core\Time\Instant`'s and `DateTime`'s depth rows read low only because their values are never
  spelled out; the playbook's false-alarm bullet owns it.
- **Two unattributed refusal sites**, `crates/mwl-codegen/src/ty.rs:116` and `:121` — the static tag
  of a tagged value and a tagged value crossing a call boundary. Both look like item 24's, and
  neither is anchored by any item. The session that opens item 24 either claims them or writes the
  decision that says they stay.
