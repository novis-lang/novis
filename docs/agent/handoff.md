# Handoff

## State

**Spec § 7's `Core\Bytes` is eleven members of twelve** — only `pack`/`unpack` are left, and they are
unwritten for the ordinary reason rather than blocked. `join` landed with `registry::Const::Bytes`,
the `bytes` parameter default that was its one blocker; `crates/mwl-stdlib/src/bytes.rs`'s own module
doc owns why that variant exists and the four signature calls the spec's prose does not state.

**ADR 0009 § 3's conversion pair lowers**, by two different mechanisms on purpose: `string as bytes`
is an `InstKind::Reinterpret` over the same allocation and emits no call, `bytes as string` is
`Helper::BytesToString` and throws naming the offset. `mwl-ir`'s crate doc § *conversions* owns that
split. Both are valgrind-clean, including the throwing path. What still panics in
`Lowering::convert` is `array<T> as array<U>` and a `Ty::Tagged` operand converted to `bytes`.

Conformance is **384** of 600 (two new cases); differential is 86 of 150 and has not moved.
`examples/collect.mwl`'s frontier is unchanged — `Core\Uri::parseQuery` at `collect.mwl:36`, then
`Core\Csv::parse`, `Core\Validate::isEmail`, `Core\Out::capture`.

## Next group — § 7's last two members, and the test that says when § 7 is done

**Shared file set:** `crates/mwl-stdlib/src/bytes.rs` (`:100` `CLASS`'s rows, `:205` `address()`,
`:587` the last helper — append after it), `crates/mwl-stdlib/src/registry.rs` (`:113` `CoreTy::Mixed`,
`:285` `CoreTy::Variadic`, `:459` `CoreMethod::variadic`), `crates/mwl-stdlib/tests/`, and
`tests/conformance/core/`. The spec rows are `docs/spec/01-core-library.md:610-614`.

- [ ] **`Core\Bytes::pack(string $format, mixed ...$values): bytes`** (`01-core-library.md` § 7,
      `:611`). The variadic tail is **one** argument whatever the call writes, so the row is
      `params: &[CoreTy::Str, CoreTy::Variadic(&CoreTy::Mixed)]` and the helper is `args: [2]`.
      § 7's prose at `:612` also makes the format string an ADR 0057 intrinsic and a **sink** — the
      sink half is ADR 0088's registry-wide item, which does not exist yet, so land the member and
      say in its doc comment that the classification is owed rather than inventing half of it.
- [ ] **`Core\Bytes::unpack(bytes $b, string $format): array<mixed>`** (`01-core-library.md` § 7,
      `:611`). `pack`'s inverse over the same format grammar, so write the grammar once as a private
      parser both members read; PHP's own `pack` letters are the compatibility target (§ 7 replaces
      `pack`/`unpack`, R6 keeps the names).
- [ ] **`every_part_one_spec_member_is_registered`** (`docs/implementation-plan.md`, `Open now`, and
      `docs/agent/loop-goal.md`'s acceptance list). The loop's own definition of done and it does not
      exist: read the spec's §§ 1-12 member rows and check each against `registry::CLASSES`, failing
      with the members that are missing. `crates/mwl-stdlib/tests/conformance_coverage.rs` is the
      shape to copy — it already walks the registry the other way.

## Backlog

- `Core\Uri::parseQuery` — PHP's bracket convention in full (`loop-goal.md` § *Standing decisions*).
- `array<T> as array<U>` — the last `Lowering::convert` panic (`mwl-ir`'s crate doc, known gaps).
- A `Ty::Tagged` operand `as bytes` — no runtime-tag row (same known-gaps list).
- `Core\Random::bytes` and `Core\Hash::stream` — § 11's streaming half (`docs/implementation-plan.md`).
- `do`/`while` — the one M4 control-flow statement that does not lower (`mwl-ir` gap 1's neighbour).
- ADR 0088's registry-wide qualifier classification — blocks `Core\Str::format` being a sink.
