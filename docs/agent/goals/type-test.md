---
milestone: M1
---
# Loop goal 33 — `is` tests a value against a type

Give the language the question it cannot currently ask. `as ?T` answers whether a value **can become**
a `T` — `"7" as ?int` is `7` — and nothing answered whether it **is** one, short of
`Core\Reflect::typeOf`, which does not narrow. [ADR 0150](../../decisions/0150.md) is the whole design
and this goal is its implementation: the grammar, the checker, the three refusals, the narrowing edge,
and the codegen.

`rule:types/type-test` is the operator and owns its accepted set.
`php-migration/is-takes-pattern-matchings-type-patterns` is the standing contract with PHP and is
what this goal must not quietly widen. **Neither is this goal's to re-open.**

Its floor is goal `signed-urls`'s whole list. It also depends on goal `surface`, which is where `is` becomes a reserved
word at all — this goal is the other half of that reservation, and stage 0 is what pays it off.

## Why here

Goal `surface` makes `is` a reserved word that refuses; this makes it the type test, which is what PHP
reserved the spelling for and what ADR 0150 decides the shape of. It is here rather than beside goal
15 because the design was reached after that goal was written, and because `is` narrows — so it
wants the type system settled rather than the grammar merely open.

## The surface, in one block

```php
mixed $m = Core\Request::query('id');

if ($m is int)            { … }   // narrows: $m is int inside the block
if ($m is int|uint)       { … }   // the migration spelling for PHP's is_int()
if ($m is Request)        { … }   // a class, exactly as instanceof would
if ($m is 'a'|'b')        { … }   // literal types
if ($m is Mode::Read)     { … }   // an enum case
if ($m is array<int>)     { … }   // legal, and an O(n) walk — the same one `as array<int>` does

$x instanceof $cls;               // still the only dynamic class test — `is` cannot express it
```

## Stage 0 — the catch-up

**Goal `surface` lands `is` as a refusal; this goal replaces it with a parse.** That refusal is
`report_reserved_for_future_use(Keyword::Is, …)` in
`crates/nvs-syntax/src/parser/expr.rs:@parse_instanceof`, plus whichever `tests/conformance/reject/`
case goal `surface` wrote for it. Both come out in the same slice that lands stage 2, not before — a tree
where `is` neither refuses nor parses is a tree with a hole in it.

`let`'s half of that refusal **stays**, untouched. `rule:php-migration/let-and-is-are-reserved` now
says the two are reserved for unrelated reasons, and only `let` is still the empty kind.

## Stage 1 — the floor

Goal `signed-urls`'s whole acceptance list, never traded.

## Stage 2 — the keystone: one node, and the production it already has

One file set: `crates/nvs-syntax/src/parser/expr.rs`, `crates/nvs-syntax/src/ast.rs`,
`crates/nvs-syntax/src/parser/ty.rs`.

1. **The node.** `ExprKind::TypeTest { expr: Box<Expr>, ty: Type }`, beside
   `ExprKind::Conversion { expr, ty }` — which is the shape to copy, because `as` already parses
   exactly this and the two differ only in what they do with the answer. Deliberately **not** an arm on
   `InstanceOf`: that node's right side is an `Expr` because a `class<T>` operand is a value, and the
   whole point of `rule:types/type-test`'s third refusal is that these are different kinds of thing.
2. **The parse.** `is` keeps the precedence slot the reserved-word hook already occupies in
   `parse_instanceof`, since it is a comparison like the operator beside it. Its right side is
   `parse_type`, not `parse_pipe`.
3. **`$x is $cls` is `E0812` here**, in the parser, where the `$` is in hand — not deferred to the
   checker as a type it fails to resolve. The help names `instanceof` and says the spelling is held
   because PHP's grammar binds a variable there.

## Stage 3 — the checker: total, and three refusals

One file set: `crates/nvs-types/src/expr/`, `crates/nvs-diagnostics/src/lib.rs`.

1. **The result is `bool`, always.** No subject is refused. `int $n; $n is int` checks and is `true`;
   `int $n; $n is string` checks and is `false`. This is the single most likely thing to get wrong by
   analogy — `infer_instanceof` refuses a subject that `!can_hold_an_object`, and **that reasoning does
   not transfer**: `instanceof` needs a class and a scalar has none, while every value has a
   representation. ADR 0150 § 6 is the argument; the acceptance list asserts both directions.
2. **`E0813`** — a `tainted` or `secret` qualifier on the right. Erased before codegen
   (`rule:security/tainted-qualifier`), so there is no bit to read. Not `E0810`, which is
   `E_DECODED_FIELD_NOT_TAINTED` and was already declared when ADR 0150 wrote its table;
   `rule:types/type-test` is the home of which code refuses what.
3. **`E0811`** — `void` or `never` on the right.
4. **A float literal** reuses `rule:types/literal-types`' existing refusal and claims no new code.
5. **Constant folding.** A result the checker settles folds to a literal `bool`. It does **not**
   warn — narrowing manufactures statically-true tests, and diagnosing them would make a flow analysis
   able to break working code.

## Stage 4 — narrowing, as the fifth spelling

One file: `crates/nvs-types/src/locals.rs`, joining the four `rule:types/narrowing` already lists.

True edge only, binding and not declared type, widened by a write inside the block — the same
contract every other spelling has. `an-instanceof-narrows-its-subject-on-the-true-edge` is the case to
mirror. **False-edge narrowing is out of scope** and is out of scope for the other four as well; ADR
0150 § 9 says why, and taking it here would leave four spellings behind.

## Stage 5 — lowering and codegen

One file set: `crates/nvs-ir/src/lower/expr.rs`, `crates/nvs-codegen/src/emit.rs`.

Three shapes behind one operator, and the cost is the reason `rule:types/type-test` states it:

- **a scalar, `object`, `null`, a literal, an enum case** — one tag comparison, and for a literal a
  payload compare after it;
- **a class or interface** — the descriptor walk `instanceof` already emits, reused and not written a
  second time;
- **`array<T>` with a named element type, and a shape** — the O(n) walk `as array<T>` already has.
  Reuse that too; a second element walk in the tree means one of them is wrong.

A test the checker folded emits no code at all.

## Stage 6 — the reference, and the cases

`is` is a new operator, so it takes its own heading in `docs/reference/lang/30-expressions.md` and a
row in the precedence table, beside the `|>` row goal `surface` added.

The conformance cases ADR 0150 § *Verification* requires, under `tests/conformance/lang/` and
`tests/conformance/reject/`. **No differential case**: PHP cannot run `is`, which is exactly why the
two divergence rows — a `uint` answering `is uint` and not `is int`, and `bytes` answering `is bytes`
— need conformance cases of their own rather than an oracle.

## Standing decisions

- **This goal opens no new ADR number.** [ADR 0150](../../decisions/0150.md) is accepted and is the
  whole design, so every call these stages reach has a section of it to read. A gap found in it is an
  edit to `rule:types/type-test`'s fragment through a record whose `changes:` block names it, never an
  overlay here. `php-migration/is-takes-pattern-matchings-type-patterns` is the contract with PHP
  and is not this goal's to widen — the lead paragraph says so and it is repeated here because a
  widening looks like a convenience at the moment a case fails.
- **The result is `bool` for every subject, and the `instanceof` analogy is the trap.** 0150 § 6 is the
  argument, stage 3 is the shape, and the acceptance list asserts both directions. A session that finds
  itself refusing a subject has reasoned from `infer_instanceof` and should stop.
- **False-edge narrowing stays out of scope**, for the four spellings already landed as much as for
  this one. 0150 § 9 is why, and taking it here is the tempting local improvement that leaves four
  spellings behind.
- **Where stage 5 cannot reuse a walk, it emits the one that exists and never a second.** The class
  path is `instanceof`'s descriptor walk and the `array<T>` path is `as array<T>`'s element walk; two
  element walks in the tree means one of them is wrong, which is a bug that reads as a performance
  choice. If reuse turns out to need a refactor to be reachable, the refactor is the slice — not a
  second emitter, and not `BLOCKED`.
- **What this spends**, per `rule:programs/memory-priority`: for a scalar, `object`, `null`, a literal
  or an enum case, one tag comparison and at most a payload compare. For a class, the walk `instanceof`
  already pays. For `array<T>` and a shape, the O(n) walk `as array<T>` already pays, and
  `rule:types/type-test` states that cost because a reader has to see it before writing the test in a
  loop. A folded test spends nothing at all.
