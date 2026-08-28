# Handoff

## State

**M4's frontier is Stage 5, and item 32 is one slice from done.** ADR 0046 § 1's attach
grammar, § 2's payload rule, § 1's shape check and ADR 0033 § 4's fifth sink have all
landed; what is left of the item is §§ 4-5's structural retrieval. The plan's *Open now*
paragraph is the one home for why each landed piece is shaped the way it is.

- **The fifth sink reads its bit off the declaration, not off a type.**
  `nvs_types::consts::ConstEntry::secret` (`crates/nvs-types/src/consts.rs:60`) is set
  syntactically from `ConstMember::ty` in the fold walk, because a class constant's
  declared type is unmodelled at every expression site;
  `reject_secret_attribute_constant` (`crates/nvs-types/src/expr/quals.rs:352`) asks
  `ConstTable::is_secret` and reports `E0727`, called from `check_value`
  (`crates/nvs-types/src/attributes.rs:206`) at every value a payload reaches.
- **ADR 0046 has no *Verification* section**, and M4's acceptance paragraph names one for
  it alongside 0014, 0023, 0028 and 0069. It is the retrieval slice's job: a section
  listing fixtures for members that do not exist yet would be written twice.
- **A class constant infers `mixed` at every expression site**, so a *named* attribute
  whose alias declares a typed field (`type Route = {path: string};`) refuses a class
  constant in it with the ordinary `E0401`. That is `signatures.rs:32`'s known gap
  surfacing through ADR 0046 § 1's check, not a rule of its own — the new case works
  round it with a `{label: mixed}` alias, and the retrieval slice should decide whether
  closing that gap belongs to it.

## Next group

**Item 32's remainder.** The file set is `crates/nvs-stdlib/src/registry.rs`,
`crates/nvs-types/src/expr/calls.rs` and `crates/nvs-ir/src/lower/expr.rs`, with
`crates/nvs-types/src/attributes.rs:88` (`check_attribute`) as the one place the payload
is already resolved against its shape.

- [ ] **ADR 0046 §§ 4-5's structural retrieval** — `Core\Attributes::get<T>` and
      `::all<T>` folded at the call site: the payload is already a compile-time literal,
      so the member answers a shape-typed value the checker builds rather than a runtime
      lookup. Anchors: `crates/nvs-types/src/attributes.rs:88`,
      `crates/nvs-stdlib/src/registry.rs` (the `Core` signature rows),
      `crates/nvs-types/src/expr/calls.rs` (`parse_call_type_args`'s consumer).
- [ ] **ADR 0046's *Verification* section**, written once the members exist — M4's
      acceptance names it alongside 0014, 0023, 0028 and 0069, and the fixtures it lists
      are the retrieval ones.

## Backlog

- A `require` whose path is not a string literal runs nothing at all, silently, in both
  forms — `nvs_hir::requires`' own known gap.
- A class constant's *declared type* is unmodelled at expression sites —
  `crates/nvs-types/src/signatures.rs:32`.
- An ADR 0036 shape literal's field carries no `secret` bit, its type being inferred —
  `nvs_stdlib::debug`'s own known gap 1.
- ADR 0092 § 6's `Throwable` record producer waits on the crate edge above
  `nvs-runtime` — stated at `nvs_stdlib::debug`.
- An attribute on an **enum case** does not parse (`E0220`), though
  `crate::attributes::check_declaration` walks the case list — `nvs-syntax`'s enum body.
