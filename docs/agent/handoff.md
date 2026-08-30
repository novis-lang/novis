# Handoff

## State

**Stage 10 item 36 is on disk — ADR 0125 and the front end.** `class<T>` is `TypeAtom::ClassRef(Box<Type>)`
(`crates/nvs-syntax/src/ast.rs:153`), parsed in `parse_type_atom` the way the `array<` arm is and closed
through the same `expect_type_close_angle`, so `array<class<Animal>>` splits its `>>`. ADR 0125 is the
decision in full — the type, `as` as its only source, covariance, the three sites, the constructor rule at
a dynamic `new` (`E0794`, which item 37 declares) and what it costs. ADR 0007 gained the `atom` production
line in § 3 and the `string`/`class<U>` → `class<T>` row in § 2's grid, with `0125` in its `Amended by:`.

**`class` is recognised by two tokens, in two places.** `Parser::at_class_reference`
(`crates/nvs-syntax/src/parser/ty.rs:78`) asks for a `<` immediately after the keyword; it is called from
`can_start_type` *and* as a guard on `parse_statement`'s class-declaration arm, because that arm sits above
the `can_start_type` fallthrough. The playbook bullet under *Writing Novis itself* owns the trap.

**Below the parser, nothing knows the atom yet, and that is item 37/38's whole content.**
`nvs_types::lower::lower_atom` (`crates/nvs-types/src/lower.rs:102`) has a `_ => mixed()` arm, so
`class<Animal> $x` type-checks as `mixed` today; `nvs_ir`'s `lower_decl_type` fallback panics on it exactly
as it already does for a shape or a literal atom used as a declared annotation. Only `nvs-hir`'s two type
walks were taught the arm (`aliases.rs`'s `record_names`/`substitute`, `requires.rs`'s `walk_type`), so the
argument's class name is harvested like `array<T>`'s.

**`orient.py`'s `[context]` gaps.** Standing, proven again: the pack prints the goal item but not the
`[[check]]` grading it, so this session re-read `loop-goal.toml` for the stage 10 check block. No field
selects `docs/reference/lang/*.md` (item 39 needs it) or `docs/reference/core/*.md`; `docs/adr/README.md`
and `docs/adr/ground-rules.md` are not in `modules` either, and every ADR-writing item edits both. In
`adrs`: **0007 §§ 2-3** — an item that amends 0007 pays for the slice every time. `orient.py` still warns
that `crates/nvs-host/src/budget.rs` matches nothing.

## Next group

**Item 37, the checker — one file set, `crates/nvs-types/src/`, four slices in this order.** Each rests on
the one before it, and the acceptance check
(`nvs-types (the class reference)`) grades the last three.

- [ ] **`Ty::ClassRef(TypeId)` and its widening.** The variant beside `Ty::Array`
      (`crates/nvs-types/src/ty.rs:64`), lowered from the atom in `lower_atom`
      (`crates/nvs-types/src/lower.rs:102`, replacing the `_ => mixed()` it falls into now) — the argument
      names a class or an interface and anything else is refused where it is written. `class<Dog>` widens
      to `class<Animal>` wherever `Dog` widens to `Animal`, never back (ADR 0125 § 3). Test:
      `a_class_reference_widens_to_its_supertype_and_not_back`.
- [ ] **The two conversion rows and the compile-time fold.** `string → class<T>` and the `class<U> →
      class<T>` narrowing in `conversion_row_exists` (`crates/nvs-types/src/expr/operators.rs:1747`),
      checked in `infer_conversion` (`crates/nvs-types/src/expr/operators.rs:78`); a `Foo::class` operand
      (`ExprKind::ClassNameConst`, `crates/nvs-types/src/expr/mod.rs:479`) is decided at compile time and
      is a refusal when `Foo` is not a `T`; the qualifier strips through
      `apply_qualifier_conversion_rule` (`crates/nvs-types/src/expr/quals.rs:220`), unchanged. ADR 0125
      § 2. Test: `a_string_becomes_a_class_reference_only_through_as`.
- [ ] **The three sites, and `E0496`'s help.** `NewTarget::Expr` (`crates/nvs-types/src/expr/calls.rs:1088`)
      types its arguments against `T`'s constructor as `NewTarget::StaticTy` does
      (`crates/nvs-types/src/expr/calls.rs:1076`); the `::` class side
      (`crates/nvs-types/src/expr/calls.rs:231`) resolves the member on `T`; `instanceof`
      (`crates/nvs-types/src/expr/members.rs:250`) takes the operand.
      `reject_dynamic_class_name` (`crates/nvs-types/src/expr/members.rs:532`) keeps `E0496` for every
      other operand, its help now naming `as class<T>`. ADR 0125 § 4.
- [ ] **The constructor rule, `E0794`.** At a `new` over `class<T>`, every implementor of `T`
      (`implementors`, `crates/nvs-hir/src/hierarchy.rs:530`) whose constructor fails
      `check_class_conformance` (`crates/nvs-types/src/conformance.rs:57`) is a refusal at the `new` site
      naming that subclass; the code is declared beside `E0784`
      (`crates/nvs-diagnostics/src/lib.rs:2460`). ADR 0125 § 5 has the wording. Test:
      `a_dynamic_new_is_refused_naming_the_subclass_whose_constructor_differs`. Fixtures in
      `crates/nvs-types/tests/classes.rs`.

## Backlog

- Stage 8's differential floor: four oracle cases under `tests/differential/`, `min_passing = 210` against
  206 on disk; `python tools/gaps.py --differential` ranks them. `Core\Math::fdiv` against `fdiv`
  (`crates/nvs-stdlib/src/math.rs:1821`) is the ranked first.
- Item 38, the lowering and the runtime lookup; item 39, the corpus and the reference tables. Both named
  in `docs/agent/loop-goal.md` § *Stage 10*.
- `lower_ternary` does not seal a `throw` arm's block the way `lower_catch` does, so
  `$c ? "a" : throw …` joins at `Ty::Tagged` — a latent inefficiency, `crates/nvs-ir/src/lower/expr.rs`.
- `docs/reference/lang/20-types.md:185` needs the `class<T>` atom beside `callable`, and `:511` the `as`
  row — item 39 owns both.
