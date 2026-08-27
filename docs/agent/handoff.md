# Handoff

## State

**M4 — language completeness.** A `mixed` in a condition and `$m as bool` are ADR 0035
§ 2's last table row now: one `Helper::ValueTruthy` reading the operand's tag, no untag
(an unchecked one over an `int` payload is a pointer the next instruction dereferences).
The runtime row already existed for native `Core` code, so the slice was reaching it from
compiled code. `bytes` in a condition lowers too — the same panic one arm away, and a live
abort — falsy iff **empty**, dropping the `"0"` case that is PHP's numeric-string rule.
`truthy_convert` panics for `Ty::Void` alone now. `python tools/holes.py` reads **24
sites, 6 items**, unchanged: it read 24 before this session as well, so both panics closed
here were ones no item claimed, exactly as the previous handoff predicted for the first.

`verify.py` 6 of 6 green — conformance **602**, differential **167**, 1632 unit tests.
`tools/leak-check.sh` green over two fixtures covering the new refcount edges: a fresh
`Ty::Tagged`/`Ty::Bytes` operand released after its one truthy read, a bare variable read
not released, elvis reusing its operand as its value, a short-circuiting pair, and a throw
with a refcounted operand live.

Facts recorded where they belong rather than here: ADR 0035 § 2's table owns the `bytes`
row and the paragraph under it owns why the `"0"` case is dropped;
`mwl_runtime::value_truthy` owns the tag dispatch and its two callers;
`mwl_ir::Helper::ValueTruthy` owns why it is the truthiness twin of `Helper::Identical`.

## Next group

**The remaining aborts in `crates/mwl-ir/src/lower/expr.rs`**, still the file this session
had open, with `crates/mwl-ir/src/lower/mod.rs` (`emit_fallible`, `landing_block`) beside
it. Item [1] below is fully designed in this handoff on purpose — two viable shapes were
weighed with the whole file in context and the cheaper one is named, so the next session
implements rather than re-derives.

- [ ] **`as ?T` over a literal or enum target** — ADR 0066 § 3 row 2 makes it *available*
      ("the non-throwing twin" of the checked conversion) and
      `crates/mwl-ir/src/lower/expr.rs:983` panics instead. `lower_conversion`'s
      `Some(target)` arm (`expr.rs:4030`) never consults `closed_literal_set`
      (`expr.rs:4196`) at all, so a set target erases to its base and reaches
      `convert_or_null`, whose match is `Int|Uint|Float|Decimal`.
      **Two designs, and take the second.** (a) Redirect the checked lowering's error
      edge to a null-producing block — elegant, generalizes to `array<T> as ?array<U>`,
      but `landing_block` (`crates/mwl-ir/src/lower/mod.rs:1547`) ends in
      `Terminator::Catch`/`Propagate` and a redirect must discard the pending `Throwable`,
      which is `lower/exception.rs` plumbing and its refcount. (b) Mirror
      `lower_conversion`'s non-nullable arm with two substitutions: give
      `lower_literal_membership` (`expr.rs:4347`) a `miss: Option<BlockId>` parameter
      (`None` = today's `Helper::LiteralMismatch` throw, `Some` = jump), and take the base
      conversion through `convert_or_null` where it can fail. The `from == Ty::Tagged` and
      non-enum case needs **no conversion at all**: membership runs first on the tag
      (`expr.rs:4098` says why), and on a hit the operand *is* the `?T`, since both are
      `Ty::Tagged`. Result joins with an `InstKind::Phi`; `lower_ternary` is the shape.
- [ ] **The `lower_expr` dispatch catch-all** — `crates/mwl-ir/src/lower/expr.rs:258`.
      Start by listing which `ExprKind` variants no arm above it names; each is either a
      lowering or a diagnostic naming the rule, never a panic.
- [ ] **A `Class::CONST` on a user-declared class** — `crates/mwl-ir/src/lower/expr.rs:247`.
      **Not a lowering slice**: the panic says the value is unmodeled in `mwl_types`, so
      the work starts in `crates/mwl-types` recording the constant's value and only then
      adds the `ExprInfo` arm. Different file set — do it last, or in its own session.

## Backlog

- **An enum case in a `mixed` reads as its backing integer**, so a case backed by `0` is
  falsy where ADR 0035 § 4 says truthy, and `Core\Reflect::typeOf` reports `int`. Fix is
  ADR 0010 § 6's reserved enum tag, written by `mwl_codegen::ty::tag_of:99`, whose comment
  deferred it only until the `mixed` representation was settled — which it now is.
- `Core\Json::decodeAs<T>`'s wider codec-reachable set and its two default-bearing rows —
  `mwl_stdlib::json`'s own module doc.
- ADR 0088's qualifier classification — `mwl_stdlib::hash`'s module doc.
- `docs/spec/02-php-migration.md` is 31% classified (`python tools/check-migration.py`).
- `python tools/gaps.py` still ranks the thin `Core` classes; a depth slice is legitimate
  when a language group is blocked, never a reason to leave one unfinished.
- The `[context]` manifest wanted nothing this session did not have, except ADR 0066 § 3,
  which cost one `peek.py`. Add `'0066'` to `[context] adrs` if the next group is taken.
