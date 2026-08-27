# Handoff

## State

**Item 24 is closed at both ends: an operand's runtime tag now answers ADR
0007 § 4's arithmetic and bitwise rows as well as its ordering ones.** Eleven
`Helper::ValueAdd`-family members, one per operator, share one table in
`nvs_runtime::helpers::value_arith`, whose doc comment is that table's home;
`Helper::ValueAdd`'s own is the home of the three *lowering* decisions (the
`Ty::Tagged` result, the error edge every one of them carries, and why a
`decimal` operand is a row of this table rather than of `DecimalAdd`'s).
`emit_binop`'s representation catch-all has no `Ty::Tagged` target of any kind
left and its comment says so.

- **`float` `%` is refused at both ends and is the one row this slice did not
  settle.** `nvs-codegen` lowers no static `float` `%` either ("does not lower
  the binary operator Mod yet"), and PHP's `%` converts to an integer where
  § 4's "either operand a `float`" row would not, so the tagged table refuses
  it rather than inventing the answer. It is in the Backlog, not a hole this
  slice left.
- **`tools/leak-check.sh` still `cd`-ed to `/mnt/d/nvs`** after the rename, so
  every WSL valgrind run died on the `cd`. Fixed in place; the checkout
  directory itself is still `<repo>` on purpose.
- **`orient.py`'s pack was complete for this item.** The two standing gaps are
  unchanged: `[context] modules` has no `nvs-runtime` entry — this session read
  `crates/nvs-runtime/src/helpers.rs` for the row table — and none for
  `nvs-diagnostics`.

## Next group

**`nvs-codegen`'s remaining internal-consistency sites, over the file this
session already opened.** The files: `crates/nvs-codegen/src/emit.rs`, and
`crates/nvs-ir/src/lower/convert.rs` for the third. `python tools/holes.py
--item N` prints any of these in full.

- [ ] **`emit.rs`'s six remaining internal panics** — `reinterpret`, the tagged
      widen/narrow pair, the unary catch-all, the refcount one, the terminator
      one and the runtime-helper one. Each is an internal-consistency check
      rather than a hole; what a session owes is the roster comment, in the
      shape `emit_binop`'s now has. `crates/nvs-codegen/src/emit.rs:660`,
      `:690`, `:721`, `:1799`, `:2807`, `:2958`, `:3329`.
- [ ] **Item 26, `bool as int` and `bool as string`** —
      `crates/nvs-ir/src/lower/convert.rs:60`. The playbook records that
      `bool as string` renders `false` as the empty string and that
      `bool as int` does not lower at all; ADR 0007 § 2's grid is what decides
      whether either is a row.
- [ ] **`float` `%`, at both ends at once** — `crates/nvs-codegen/src/emit.rs`'s
      operator-table catch-all (`:1329` region, the `Unsupported` naming the
      operator) and `value_arith`'s guard in
      `crates/nvs-runtime/src/helpers.rs`. Either ADR 0007 § 4 gains the row
      PHP's integer-only `%` implies, or `nvs_types` refuses a `float` operand
      of `%` where it is written; the two ends must land together.

## Backlog

- The refusal messages say "a `int`" and "a `array<T>`" — `no_ordering` and
  `no_arithmetic` share `tag_name`, so one article fix touches both and one
  pinned case (`an-ordering-over-a-mixed-operand-is-decided-by-its-tag.nvst`).
- A helper `Fault` still promotes to `RuntimeError` rather than to the
  `ArithmeticError` ADR 0007 § 4 names — `nvs_runtime::helpers::overflowed`'s
  doc comment records the gap, shared with `does_not_fit` and
  `arithmetic_error`.
- `bool` arithmetic (`true + true`) reaches `nvs-codegen` as `Ty::Bool` and is
  answered by `iadd`, where § 4 gives it no row at all
  (`nvs_types::expr::operators::arithmetic_result`'s `_ => mixed`).
- Unary `-`/`~` over a `Ty::Tagged` operand is the same deferral one operator
  down and has no arm yet (`nvs_ir::lower::expr`'s `UnaryOp` dispatch).
- ADR 0024 § 5's `string as Core\Html\Markup` is `nvs-ir`'s last conversion
  catch-all target and waits on `Core\Html` (M7).
- `[context] modules` in `docs/agent/loop-goal.toml` has no `nvs-runtime` or
  `nvs-diagnostics` entry.
