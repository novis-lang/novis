# Handoff

## State

**Stage 0c — the reference findings — is the open stage and runs ahead of everything else in this
goal, stage 9 included.** `docs/agent/loop-goal.md` § *Stage 0c* is items 31–35;
[docs/reference/findings.md](../reference/findings.md) § *Triage* is each item's verdict and owner.
Items 31 and 32 are closed; **item 33 has eight of its nine findings landed** — U1, U2, U3, U4, U6,
U12, U14 and now P5. **U5 is the only one left**, and the item-33 `[[check]]` names no case for it.

**An `array<T>` class constant folds.** `crates/nvs-types/src/defaults.rs`'s `eval_const_value` is
the class-constant entry point `signatures.rs` now collects through: `literal_default`'s
type-directed grid first, then an array literal whose elements are each placed in the declared
element type, recursing for a nested `array<array<T>>`. `nvs-ir` needed no change — the
`ConstArg::Array` ADR 0046 § 5's folded retrieval already produced lowers to one `ArrayNew`. The
untyped decoder that fold shares with the retrieval moved from `retrieval.rs` to `defaults.rs` as
`fold_constant_value`, so there is one constant evaluator rather than two.

**A read of a constant that still folds to nothing is `E0792`, not a panic.** Reported at the read
(`crates/nvs-types/src/expr/members.rs`'s `report_unfoldable_const`) so that declaring one and never
naming it stays legal. Two declared types return early because the *declaration* is the mistake:
`mixed` is already `E0246`, and `bytes` has no literal to write (ADR 0009 § 1) — see the playbook
bullet, which is how those two arrived. `nvs-ir`'s `ClassConstAccess` panic is kept and reworded as
the unreachable it now is.

**What still has no constant form** is ADR 0046 § 2's named set — another class's `const`, an enum
case, `Foo::class` — written as the value or nested in an array. `crates/nvs-types/src/defaults.rs`'s
`const_reference_default` already resolves exactly those at a *property* default and needs the `Ctx`
the constant-collection pass holds; that is the whole of what closes it, and it is in `## Backlog`.

**`E0126` is spoken for and must not be handed out.** ADR 0119 § 3 names it in prose for the
expression-`catch` arm that refuses `return`, which stage 9 has not written yet. The next free
`E02xx` is `E0247`, and `E07xx` is now `E0793`.

**Stage 9 stands where it stood** — ADR 0119 accepted, items 21–23 written with their anchors,
nothing implemented — and resumes when stage 0c is green. **Stage 8**: conformance 1052,
differential 206.

**`orient.py`'s `[context]` gaps, still costing time.** `[context] adrs` names none of stage 0c's own
ADRs: this session's item was ADR 0047 § 2 and the pack printed 0103, 0078, 0042 and 0119 instead —
add **0047 § 2** and **0011**, as well as **0033 § 4**, **0086 § 6** and **0096 §§ 1-1a** from the two
sessions before. `[context] modules` still misses `crates/nvs-types/src/attributes.rs`, `routes.rs`,
`commands.rs`, `derive.rs`, `testing.rs` and — new this session — `crates/nvs-types/src/defaults.rs`,
`consts.rs`, `signatures.rs` and `retrieval.rs`, which is the file set a constant-folding item lives
in. `orient.py` itself still warns that `crates/nvs-host/src/budget.rs` matches nothing.

## Next group

**Item 33's last finding, then item 34's cheapest. They do not share a file set — take U5 first: it
closes item 33, and it is `crates/nvs-stdlib`'s five-edit shape for a `Core` member
(conventions.md § *A `Core` member*), with `crates/nvs-stdlib/src/json.rs` the small module to read
whole as the worked example.**

- [ ] **U5 `Core\Secret::reveal` exists** — findings.md § *Unexpected* U5, ADR 0033 § 3's named
      escape hatch, which every `secret` refusal's help already tells the author to call. A new
      `crates/nvs-stdlib/src/secret.rs` holding the class, its row and its card; registered at
      `crates/nvs-stdlib/src/lib.rs:229` (the `pub mod` list) and `crates/nvs-stdlib/src/lib.rs:324`
      (the `address` dispatch chain); the class added to `crates/nvs-stdlib/src/registry.rs:984`'s
      `CLASSES`. `Qual::Reveal` is declared at `crates/nvs-stdlib/src/registry.rs:141` and no row
      writes it — this is the row that does. The second argument is § 3's written reason, and the
      return is the operand's own type with the qualifier dropped. Case:
      `tests/conformance/core/a-reveal-drops-the-secret-qualifier.nvst`.
- [ ] **P1 a first-class callable lowers** — findings.md § *Panics* P1, item 34.
      `crates/nvs-ir/src/lower/expr.rs:2870` panics for the `ExprInfo::CallableRef` the checker
      records; ADR 0027 keeps the spelling, and the only place it works today is as the argument of
      `Core\Attributes::get/all`, where `crates/nvs-types/src/retrieval.rs` folds it at check time.
      Case: `tests/conformance/core/a-first-class-callable-lowers.nvst`, which the item-34
      `[[check]]` already names.

## Backlog

- Add the `[context]` selectors this session paid for — `docs/agent/loop-goal.toml`'s `modules`
  (line 61) and `adrs` (line 123) lists, per `## State`; then copy the file over
  `docs/agent/goals/<goal>.toml`, which the playbook bullet owns.
- A named constant inside a class constant's value — `crates/nvs-types/src/defaults.rs`'s
  `const_reference_default`, one position along; it closes the rest of `E0792`.
- A **shape**-typed class constant folds: each field placed against the declared shape's own type,
  into the `ConstArg::Shape` `emit_const_shape` already lowers. `E0792` today.
- A written array literal erases the `secret` qualifier before any sink sees it — the container axis,
  `crates/nvs-types/src/expr/literals.rs:718`. ADR 0033 § 4.
- `const bytes B = "…";` folds to nothing and a read of it still panics in `nvs-ir` — ADR 0009 § 1
  says there is no `bytes` literal, so the *declaration* is what wants refusing.
- Stage 9's items 21–23 (ADR 0119's expression `catch`), unstarted, anchors already written.
- `docs/reference/findings.md` items 34 and 35 — the lowering/library gaps and the doc-only fixes.
