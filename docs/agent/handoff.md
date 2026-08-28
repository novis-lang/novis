# Handoff

## State

**M4's frontier is Stage 5, and item 32 is still the open one, now two slices from done.**
ADR 0046 § 1's attach grammar, § 2's payload rule and § 1's shape check have all landed;
what is left of the item is ADR 0033's fifth sink and §§ 4-5's structural retrieval. The
plan's *Open now* paragraph is the one home for why each landed piece is shaped the way it
is.

- **The named form is checked in one walk with § 2's rule.**
  `nvs_types::attributes::resolve_shape_alias` (`crates/nvs-types/src/attributes.rs:133`)
  answers three ways — `E0303` for a name nothing declares, `E0726` for one that denotes
  something that is not a shape-typed `type` alias, and the shape itself — and
  `check_attribute` (`:88`) then checks the literal against it with the ordinary
  `crate::expr::is_assignable`. A payload § 2 already refused is skipped, so one mistake
  draws one diagnostic.
- **ADR 0071's two attributes are the one exemption**, matched off `crate::derive::ATTRIBUTES`
  before the alias table is consulted. It is a closed `Core`-owned roster on purpose; anything
  userland is an alias or a mistake.
- **The fifth sink cannot be `reject_secret_debug_argument`'s shape.** That sink reads the
  argument's *inferred* type (`crates/nvs-types/src/expr/quals.rs:293`), and a class constant
  infers `mixed` at every expression site — `crates/nvs-types/src/signatures.rs:32` names that
  as a known gap and `crate::consts` holds values, not types. So the `secret` bit for a payload
  field has to be read off the constant's own annotation in `build_const_table`
  (`crates/nvs-types/src/consts.rs:113`), which is the walk that already has every
  `ConstMember` in hand.
- **ADR 0046 has no *Verification* section**, and M4's acceptance paragraph names one for it
  alongside 0014, 0023, 0028 and 0069. It is the retrieval slice's job: a section listing
  fixtures for members that do not exist yet would be written twice.

## Next group

**Item 32's remainder, in this order.** The file set is
`crates/nvs-types/src/attributes.rs`, `crates/nvs-types/src/expr/quals.rs`,
`crates/nvs-types/src/consts.rs` and `crates/nvs-stdlib/src/registry.rs`.

- [ ] **ADR 0033's fifth sink: a `secret` class constant reaching an attribute payload** — §
      2's own third bullet names it, alongside HTML output, `Core\Log`, debug dumps and
      isolate-crossing. Read the qualifier where it still exists: `ConstMember`'s annotation,
      in `crates/nvs-types/src/consts.rs:113`'s walk, recorded beside the folded value; the
      refusal itself belongs with its four siblings in
      `crates/nvs-types/src/expr/quals.rs:293`, called from the payload walk at
      `crates/nvs-types/src/attributes.rs:203`, where every field value already passes. Next
      free code is `E0727`.
- [ ] **ADR 0046 §§ 4-5's structural retrieval** — `Core\Attributes::get<T>`/`::all<T>` folded
      at the call site: no attached literal is a compiled-in `null`, exactly one is that
      constant, and more than one is a compile error (§ 5). `T` is the § 6 call-site type
      argument `Parser::parse_call_type_args`
      (`crates/nvs-syntax/src/parser/expr.rs:799`) already parses. Write ADR 0046's missing
      *Verification* section with it, not before.

## Backlog

- A class constant's declared type is unmodelled, so `Class::CONST` is `mixed` everywhere —
  `crates/nvs-types/src/signatures.rs:32` owns the gap, and closing it would also give the
  fifth sink an ordinary `is_secret` to ask.
- ADR 0046 § *Verification* does not exist; M4's acceptance paragraph names it
  (`docs/plan/m4.md`).
- A `require` whose path is not a string literal runs nothing, silently, in both forms —
  `nvs_hir::requires`' own known gap.
- `nvs_stdlib::debug` known gap 1: an ADR 0036 shape literal's field and an `array<T>` element
  carry no `secret` bit, ADR 0033's unmodelled container axis.
