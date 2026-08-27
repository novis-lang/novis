# Handoff

## State

**M4 — ADR 0007 § 2's `array<T> as array<U>` row runs, in both its checked and
its `null`-answering spelling, and an element type no runtime tag decides is
refused where it is written.** `$m as array<string>`, `$m as ?array<string>`
and `$grid as array<array<string>>` all walk; `array<Cell>`, `array<Rank>`,
`array<?string>` and `array<array<Cell>>` are `E0711`. `Lowering::convert`'s
catch-all has one target left and it is M7's.

- **It is a walk, not a row, and that is why it is not in `convert`.** Both
  sides of `array<int> as array<string>` erase to one `Ty::Array`, so `convert`
  took its free `from == to` row and handed the `int`s through — `$foo as Bar`'s
  confusion one container in. `Lowering::lower_array_restamp`
  (`crates/mwl-ir/src/lower/expr.rs:5046`) reads the element type off the
  annotation instead and passes `lower::array_element_tags`' word — one
  `param_tag_nibble` per level of `U`, shared with the closure-entry check
  rather than a second encoding — to `Helper::ToArrayOf`.
- **Nothing is copied.** An MWL array is copy-on-write, so the helper hands back
  the operand's own allocation under one more reference and whichever view
  writes first separates itself. ADR 0007 § 5's "a real copy" sentence said
  otherwise and is corrected there, which is that fact's one home.
  Valgrind-clean over a fixture that converts and fails to convert 200 times,
  borrowed and fresh operands both.
- **The refusal's roster is `mwl_types::expr::operators`'
  `reject_uncheckable_element_type`.** A tag is four bits: a class label does
  not fit and a named-class binding then reads at a fixed offset; an enum would
  admit any integer as a case; a literal type or a union admits some values of
  its representation and not others. `array<mixed>` is the target every tag
  satisfies and the way round all four — and the one that runs nothing when the
  operand is already an array.

## Next group

**`mwl-ir` gap 12: a `Core` object behind an erased operand renders.** The file
set: `crates/mwl-runtime/src/helpers.rs`, `crates/mwl-runtime/src/dispatch.rs`,
`crates/mwl-stdlib/src/registry.rs`, `tests/conformance/core/`.

- [ ] **A `Core` object behind a `mixed` renders** — `echo $m` over a `mixed`
      holding a `Core\Uri` throws where `echo $uri` renders, because
      `mwl_runtime::stringify` (`crates/mwl-runtime/src/helpers.rs:985`)
      dispatches through `crate::dispatch::call_method`
      (`crates/mwl-runtime/src/dispatch.rs:77`), a compiled method table a
      `Core` class has no entry in. `mwl-runtime` sits below `mwl-stdlib` and
      cannot read `mwl_stdlib::registry::class_renders`
      (`crates/mwl-stdlib/src/registry.rs:876`), so the slice is what the two
      share — `mwl_runtime::ctx::is_carrier`
      (`crates/mwl-runtime/src/ctx.rs:179`) is the list already read rather
      than copied, and is the shape to follow.
- [ ] **The two sink carriers behind a `mixed`** — ADR 0088 § 5 renders those
      with no member at all, so they are the half of the row above that needs
      no method table and can land first as its own answer.

## Backlog

- `array<U>` where `U` is a union, an enum or a literal type has no spelling —
  `E0711` names it, and giving it one needs more than a tag per element
  (ADR 0007 § 2, `reject_uncheckable_element_type`'s doc comment).
- `string as Core\Html\Markup` is the last target `Lowering::convert`'s
  catch-all names, and waits on `Core\Html` existing (M7, ADR 0024 § 5).
- `array<T> as array<U>` between two *identical* types still pays the walk: the
  representation does not carry the element type, so nothing tells it apart
  from a real restamp (`docs/adr/0007-explicit-type-system.md` § 5).
- An element of a `?T` shape (`array<?string>`) is the most likely of the four
  refusals to be missed by real code; a nibble pair would carry it.
- ADR 0007 § 2's implicit `int → float` widening does not apply per element —
  it would have to rewrite the element, which is the copy this row runs
  without (`mwl_runtime`'s `element_has_tag`).
