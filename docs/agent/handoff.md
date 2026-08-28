# Handoff

## State

**M4's frontier is Stage 5, and item 32 is the open one.** ADR 0046's attach grammar and its
§ 2 payload rule landed this session; §§ 4-5's retrieval, § 1's named-form shape check and
ADR 0033's fifth sink are what is left of it. The plan's *Open now* paragraph is the one home
for why each piece is shaped the way it is.

- **One payload, two forms.** `nvs_syntax::ast::Attribute` is `Option<Name>` plus
  `Vec<ObjectLiteralField>` plus the payload's own span — ADR 0046 § 1's named and bare forms
  differ in whether a name was written and in nothing else. The parenthesized run goes through
  `Parser::parse_object_literal_fields` (`crates/nvs-syntax/src/parser/expr.rs:1248`), shared
  verbatim with ADR 0036 § 2's literal, so a positional value in an attribute is refused by that
  literal's own rule rather than by a second one.
- **`nvs_types::attributes` is § 2's one home** (`crates/nvs-types/src/attributes.rs:34`), and
  `is_constant` (`:129`) is a *closed* list on purpose — an `ExprKind` it does not name is
  refused, so a grammar that grows a shape does not silently gain a constant-pool entry.
- **§ 6 needed nothing.** `ExprKind::StaticCall::type_args` and `ExprKind::New::type_args` both
  read `Parser::parse_call_type_args` (`crates/nvs-syntax/src/parser/expr.rs:799`), which is the
  explicit call-site type argument that ADR names. Only the checker's use of it is open.
- **ADR 0046 has no *Verification* section**, and M4's acceptance paragraph names one for it
  alongside 0014, 0023, 0028 and 0069. Writing it is the retrieval slice's job, not a separate
  one — a section listing fixtures for a member that does not exist yet would be written twice.

## Next group

**Item 32's remainder, in this order.** The file set is
`crates/nvs-types/src/attributes.rs`, `crates/nvs-types/src/expr/quals.rs`,
`crates/nvs-types/src/check.rs` and `crates/nvs-hir`'s type-alias table.

- [ ] **ADR 0046 § 1's named form resolves to a shape-typed `type` alias, and the literal is
      checked against it** — `Name` is never a class and never a new namespace of attribute
      kinds, so an unresolvable one is the ordinary `E0303` and a resolvable one that is not a
      shape type is its own refusal. `crates/nvs-types/src/attributes.rs:34` is where the walk
      already has the attribute in hand; `nvs_hir`'s alias table is what answers.
- [ ] **ADR 0033's fifth sink: a `secret` class constant reaching an attribute payload** —
      ADR 0046's own *Amends* line adds it, and § 2 already forces every candidate value to be
      the literal/const/enum-case shape a `secret` can flow through.
      `crates/nvs-types/src/expr/quals.rs:293` is the neighbouring sink to write it beside.
- [ ] **ADR 0046 §§ 4-5's structural retrieval** — `Core\Attributes::get<T>`/`::all<T>` folded
      at compile time to the satisfying literal, `null`, or § 5's ambiguity diagnostic. This is
      the slice that writes ADR 0046's missing *Verification* section and
      `tests/conformance/lang/an-attribute-is-retrieved-by-its-own-type.nvst`, which
      `loop-goal.toml` names and nothing has written.

## Backlog

- `loop-goal.toml` names `tests/conformance/lang/a-dump-renders-one-record-and-redacts-a-secret.nvst`,
  which landed as two cases under `tests/conformance/core/` with different names — the toml is
  the stale half.
- A `require` whose path is not a string literal runs nothing at all, silently, in both forms —
  `nvs_hir::requires`' own known gap.
- `Ctx::write_diagnostic` (`crates/nvs-runtime/src/ctx.rs:960`) is a second output sink outside
  the capture stack — deliberate, and `nvs_stdlib::debug` is its home.
- An `array<T>` element and an ADR 0036 shape literal's field still carry no `secret` bit —
  ADR 0033's unmodelled container axis, `nvs_stdlib::debug` known gap 1.
- `nvs-render` depends on `nvs-syntax` and that edge inverts when `nvs-runtime` or
  `nvs-diagnostics` becomes a dependent — that crate's module doc § *Where this sits*.
