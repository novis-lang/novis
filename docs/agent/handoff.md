# Handoff

## State

**M4 — `Lowering::convert`'s catch-all is down to two rows, and the `Core` half of
`as string` is refused where it is written.** A `mixed`, a `?T` or any other union
converted to `bytes` now takes ADR 0009 § 3's row from its runtime tag; a `Core`-owned
class the spec gives no `toString` is `E0710` at the `echo` rather than a throw below it.

- **`$m as bytes` and `$m as ?bytes` are one row set.** `Helper::TaggedToBytes` and
  `Helper::ToBytesOrNull` share `mwl_runtime::to_bytes`, which hands back the same
  allocation under the other tag for a `Tag::Str` or a `Tag::Bytes` and answers nothing
  for every other tag. Neither can fault, so the `?` twin is emitted plainly — the
  `string` target is still the one `?` row carrying ADR 0002's error edge, because only
  it can run a `toString()` body. Valgrind clean over a fixture exercising both.
- **The statically typed `as bytes` reaches no helper at all** and did not change: it is
  a free `InstKind::Reinterpret`, which is why `TaggedToBytes` is the one conversion
  helper here with no static sibling. `mwl-ir`'s known gap 4 and the roster in its
  crate docs are the home for what is left.
- **`require_stringable_object` no longer exempts every `Core` class.** It asks
  `mwl_stdlib::registry::class_renders`, which joins the two rosters that answer: three
  classes have a `toString` row — `Core\Uri`, `Core\Uuid`, `Core\Time\Duration` — and the
  two sink carriers render through ADR 0088 § 5 with no member at all. That function's
  doc comment is the one home for the rule; every other `Core` class is `E0710`.
- **A `Core` class that *does* render still throws**, and that is `mwl-ir`'s known gap 12
  in full now: `resolve_method` finds the seeded signature, but the member is a native
  symbol rather than an entry in a compiled method table, so nothing a `CallVirtual`
  reaches exists.

## Next group

**The rendering half of `Core`'s `as string`, then the two rows `Lowering::convert`'s
catch-all still names.** The file set:
`crates/mwl-types/src/expr/operators.rs`, `crates/mwl-ir/src/lower/expr.rs`,
`crates/mwl-stdlib/src/registry.rs`, `crates/mwl-runtime/src/helpers.rs`,
`tests/conformance/lang/`.

- [ ] **`echo $uri` on a `Core` class that has a `toString`** — `mwl-ir`'s known gap 12,
      ADR 0028 § 1. The refusal half is done; what is missing is the *call*. A `Core`
      member is an `InstKind::CoreCall` on the registry's symbol, not a `CallVirtual`, so
      the recorded target `require_stringable_object` writes has to say which, or the
      lowering has to ask the registry. Anchors:
      `crates/mwl-types/src/expr/operators.rs:1600` (`require_stringable_object`, the
      `Core` branch is at 1603), `crates/mwl-types/src/expr/operators.rs:1652`
      (`record_to_string`), `crates/mwl-ir/src/lower/expr.rs:847`
      (`lower_to_string_call`), `crates/mwl-stdlib/src/uuid.rs:150`,
      `crates/mwl-stdlib/src/uri.rs:454`, `crates/mwl-stdlib/src/time.rs:259` (the three
      `toString` rows). The runtime half, if the answer is a helper rather than a call,
      is `crates/mwl-runtime/src/helpers.rs:985` (`stringify`).
- [ ] **`$m as Plain` — a tagged operand converted to an object** — ADR 0007 § 6's
      checked way out of `mixed`, and one of the two rows `Lowering::convert`'s
      catch-all still names. Needs a class identity `Ty::Object` deliberately does not
      carry, so the helper takes an `ir::Program::classes` label the way
      `InstKind::ClassDescConst` already hands one to `Core\Json::decodeAs`. Anchors:
      `crates/mwl-ir/src/lower/expr.rs:1288` (the catch-all),
      `crates/mwl-ir/src/lower/expr.rs:1342` (`convert_or_null`, whose `?` twin closes
      with it).
- [ ] **`$xs as array<U>`** — the last row of that catch-all and the one that is no
      single helper call: ADR 0007 § 2's O(n) element walk, "every element must satisfy
      `U`". The `?` twin is the same walk answering `null` on the first element that
      does not. Same two anchors as above; the element check is `mwl_types`' erasure
      table, so read `erase_checked_ty` before deciding where the per-element type
      comes from.

## Backlog

- `mwl-ir` gap 12's runtime half — `mwl_runtime::stringify` cannot see a native member
  (crate docs, gap 12).
- `mwl-codegen/src/ty.rs:116` and `:121` are two refusal sites `holes.py` attributes to
  no item at all.
- ADR 0010 § 5's integer *into* an enum has no lowering in either form (`mwl-ir` gap 4).
- An abandoned generator's `finally` (standing decision, `docs/agent/loop-goal.md`).
- `Core\Log` inspection and ADR 0024 § 4's sink list wait on M7/M8 `Core` classes.
