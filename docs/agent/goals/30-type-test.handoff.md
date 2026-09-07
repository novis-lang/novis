# Handoff

## State

**Goal 30 — `is` tests a value against a type — has just started; nothing of it has landed yet.**
Goal 29's whole list is this goal's Stage 1 floor.

The design is finished and is not this goal's to re-open. [ADR 0150](../../decisions/0150.md) holds all
of it, `rule:types/type-test` is the operator and its accepted set, and
`rule:php-migration/is-takes-pattern-matchings-type-patterns` is the standing contract with PHP.

**Two things a session must not re-decide:**

1. **`is` is total.** It never refuses because the answer is knowable. `int $n; $n is int` compiles and
   is `true`; `int $n; $n is string` compiles and is `false`. The trap is `infer_instanceof` in
   `crates/nvs-types/src/expr/members.rs`, which *does* refuse a subject that `!can_hold_an_object` —
   and that reasoning does **not** transfer, because `instanceof` needs a class to test against and a
   scalar has none, while every value has a representation. ADR 0150 § 6 is the argument. Copying the
   refusal by analogy will look right at the call site and will break narrowing, which manufactures
   statically-true tests by construction.
2. **The right-hand side is a `Type`, never an expression.** `$x is $cls` is `E0812` and stays
   refused — PHP's grammar binds a variable in that slot, so giving it our own meaning would make one
   line mean two different things in the two languages, silently, in both. `$x instanceof $cls` is the
   dynamic class test and is the one thing `is` cannot express.

## Next group

**Stage 0 + stage 2, in one slice** — the refusal comes out and the parse goes in together, because a
tree where `is` neither refuses nor parses has a hole in it. One file set:
`crates/nvs-syntax/src/parser/expr.rs`, `crates/nvs-syntax/src/ast.rs`,
`crates/nvs-syntax/src/parser/ty.rs`, `crates/nvs-syntax/src/token.rs`.

- [ ] **Retire goal 13's `is` refusal** — `report_reserved_for_future_use(Keyword::Is, …)` in
      `crates/nvs-syntax/src/parser/expr.rs:@parse_instanceof`, and whichever
      `tests/conformance/reject/` case goal 13 wrote for it. **`let`'s half stays**, untouched.
- [ ] **The node** — `ExprKind::TypeTest { expr, ty }` in `crates/nvs-syntax/src/ast.rs`, beside
      `ExprKind::Conversion { expr, ty }`, which is the shape to copy: `as` already parses
      `expr <kw> Type` and the two differ only in what they do with the answer. Not an arm on
      `InstanceOf` — that node's right side is an `Expr` because a `class<T>` operand is a value.
- [ ] **The parse** — `is` keeps the precedence slot the reserved-word hook already occupies. Its right
      side is `parse_type`, not `parse_pipe`.
- [ ] **`E0812` in the parser**, where the `$` is in hand, rather than deferred to the checker as a
      type that fails to resolve. Declare it in `crates/nvs-diagnostics/src/lib.rs` — the `E04xx` and
      `E07xx` type bands are full, `E08xx` is the live one, and the registry is the allocator.

## Backlog

- **Stage 3** — the checker: totality, `E0810` (a `tainted`/`secret` qualifier, erased before codegen
  so there is no bit to read), `E0811` (`void`/`never`), constant folding with no warning.
- **Stage 4** — narrowing as the fifth spelling, in `crates/nvs-types/src/locals.rs`. True edge only.
  False-edge narrowing is out of scope for all five spellings, not just this one.
- **Stage 5** — lowering and codegen: one tag comparison for a scalar, the descriptor walk
  `instanceof` already emits for a class, and the O(n) walk `as array<T>` already has for an element
  type. A second element walk in the tree means one of them is wrong.
- **Stage 6** — the reference heading and precedence row in `docs/reference/lang/30-expressions.md`,
  and the conformance cases ADR 0150 § *Verification* names. **No differential case** — PHP cannot run
  `is`, which is exactly why the two divergence rows need conformance cases of their own.
