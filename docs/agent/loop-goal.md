# Loop goal

Finish **M4 — language completeness**: close every hole in the language surface, so that nothing a CLI
program reaches for panics, silently miscompiles, or has to be spelled around. Read
[docs/plan/m4.md](../plan/m4.md) for the milestone's own scope; this file does not restate it.

**M4B — the LSP and the VS Code extension — is deferred behind this goal, by decision.**
[next-goal-m4b.md](next-goal-m4b.md) stays staged and unamended; it is the goal *after* this one. Writing
completion, hover and diagnostics against a language whose `1 + 1.5` does not compile means writing them
twice, and every `.lspt` case authored in the meantime is authored against a surface about to change.

The previous goal — M4S Part I, `Core` §§ 1–12 — is **not** discarded. It is 50 conformance cases from its
own gate, both its named guards already pass, and it is carried here whole as Stage 1's floor plus Stage 8's
suites. A session may still take a `Core` depth slice from `python tools/gaps.py`; it is simply no longer
the frontier.

## What "no holes" means here, and why it is checkable

A hole is a shape that **compiles in the front end and then refuses below it** — `nvs-ir` panics naming
itself, `nvs-codegen` refuses two representations, or the checker types an expression it has no lowering
for. Each of the crate module docs' *Known gaps* lists is an inventory of them, and this goal's item list
is that inventory ordered and grouped.

A shape is **closed** when it either

1. runs, with a fixture or a `.nvst` case pinning what it prints, **or**
2. is refused by a **diagnostic that names the rule** — a numbered `E`-code and the ADR behind it.

The second is not a loophole. `goto` is refused by ADR 0008 § 5 and by-reference capture by ADR 0031 § 2;
a language that refuses them with a diagnostic has no hole, while one that falls through to a generic parse
error or a lowering panic does. **What may never close an item is a panic**, however well it names itself.

The mechanical end gate is Stage 8's `every_refusal_is_a_diagnostic_or_decided`: it reads `nvs-ir`'s and
`nvs-codegen`'s own sources for refusal sites and fails naming any that is not on the allowlist frozen in
the test. **That allowlist may never grow.** Every entry on it is a decision in this file's
§ *Standing decisions* with its ADR; adding an entry to make a run go green is the one move this goal
forbids outright. `python tools/holes.py` prints the same inventory as a worklist, mapped to the items
below, so that no session re-derives it.

**A refusal site is not the only way a hole hides.** `holes.py` recognizes one by *phrasing* — three house
sentences — and `refusals.rs` counts *arms*. A panic worded differently is invisible to the first, and a
catch-all arm is one number to the second however many shapes fall into it. Both blind spots have now been
hit: `lower_checked_ty`'s single arm was hiding three shapes, and reading a user-declared class constant
panics in a sentence the recognizer does not match, so it is not among the four sites either tool reports.
`crates/nvs-ir/tests/type_atoms.rs` is the answer to the first — a table of *shapes*, each asserted to
reach a diagnostic or an IR — and Stage 00's item 49 widens it past declared types, which is the only
reason the count below can be trusted.

## Stage 00 — the shapes a program reaches for on its first page

**Ahead of Stage 0 and of Stage 0a, because these are not holes at the edge of the surface — they are
things a reader writes in the first twenty lines of their first program, and each one either does not
parse or panics the compiler.** A corpus written around them teaches the workaround, which is Stage 0's
own compounding argument applied to a shape rather than to an operator: the difference is that Stage 0's
items make a case *subtly* wrong, and these make it unwritable.

**Until this stage is empty, a session takes its group from here**, and item 44 is first within it because
its migration touches every other stage's cases.

44. **A `for` header declares its own counter, and the corpus moves to it.**
    [ADR 0109](../adr/0109-a-for-header-declares-its-own-counter.md).
    `for (int $i = 0; $i < 3; $i = $i + 1)` does not parse: `parse_for` reads all three
    clauses as expression lists, and a typed declaration in the first produces twelve diagnostics whose
    first is about a `;` and one of which claims the counter is `mixed`. The init clause becomes one
    typed local declaration *or* a list of expressions and never both (§ 1), with `E0124` and `E0125` for
    the two shapes that still do not parse (§ 3). The checker and `nvs-ir` are owed nothing new — § 4 —
    which is what makes this a parser slice.

    **The migration is half the item, not a follow-up.** Every `for` in `tests/conformance/`,
    `tests/differential/` and `examples/` whose counter is declared on the line above and reassigned in
    the header takes the declaration into the header. Nobody writing a counted loop declares it above by
    choice; the corpus reads that way only because there was no other option, and it is what a
    contributor greps to find the house spelling. The expected output of every one of those cases is
    unchanged — the counter is function-scoped either way (§ 2) — so the suites staying green *is* the
    review. A counter read after its loop, or shared by two, stays declared above.
    `crates/nvs-syntax/src/parser/stmt.rs:321` (`parse_for`),
    `crates/nvs-syntax/src/parser/tests/stmt.rs`, `crates/nvs-diagnostics/src/lib.rs`.
45. **A user-declared class constant has a type and a value.** [ADR 0011](../adr/0011-functions-and-constants-are-class-members.md)
    makes every constant a class constant, so `public const int MAX = 3;` is *the* way to write one — and
    reading it panics `nvs-ir`. Both halves are open: the checker types `Limits::MAX` and `self::MAX` as
    `mixed` (so a method declaring `int` and returning one is `E0403`), and `nvs-ir` has nothing to lower
    it to, because `nvs_types` records a value only for an enum case or a `Core` constant.
    `crates/nvs-ir/src/lower/expr.rs:262`, `crates/nvs-types/src/consts.rs`,
    `crates/nvs-types/src/expr/members.rs`. This is the half of old item 39 that was real; the other
    three clauses of that item are closed, which is why it is gone.
46. **`new static()` and a `: static` return resolve to the called class.** [docs/plan/m4.md](../plan/m4.md)'s
    acceptance names this outright — "`new static()` through two levels of inheritance returns the called
    class" — and it fails at the checker rather than at run time: with `Base::make(): static`, the call
    `Leaf::make()` types as `Base`, so `Leaf $made = Leaf::make();` is `E0401`. The resolution rule is
    [ADR 0008](../adr/0008-static-and-global.md)'s late static binding; what is missing is the *type* a
    call site gives `static`, which is the receiver's own class rather than the declaring one.
    `crates/nvs-types/src/expr/calls.rs`.
47. **`instanceof` narrows to an interface, not only to a class.** The class direction works —
    `object $a = new Square(); if ($a instanceof Square) { $a->area(); }` runs — and the interface
    direction does not: `if ($b instanceof Shape)` leaves `$b` at `object`, and the call inside the guard
    is `E0477`, whose own help tells the reader to narrow with `instanceof` first. An interface is what a
    program written against an abstraction narrows *to*, so this is the direction that matters.
    `crates/nvs-types/src/locals.rs` owns the residue and what invalidates one; item 37 in Stage 6 is the
    remaining narrowing spellings and no longer claims `instanceof` narrows nothing.
48. **`new` on a `Core` class the registry gives no constructor is a diagnostic.** `new Core\Error("x")`
    panics naming `nvs_types`' own missing zero-arity check; the same shape on a *user* class already
    answers `E0402` (`expected 0 argument(s), found 1 — `Square` declares no `constructor``). One rule,
    one diagnostic, both sides. `crates/nvs-ir/src/lower/expr.rs:2368`,
    `crates/nvs-types/src/expr/calls.rs`.
49. **The shape table covers expressions and statements, not only declared types.** This stage's own
    guard, and it is last here because it is written against what items 44-48 close.
    `crates/nvs-ir/tests/type_atoms.rs` walks every atom [ADR 0007](../adr/0007-explicit-type-system.md)
    § 3 spells through two declaration positions; item 45's panic is not a *type*, so nothing caught it —
    not that table, not `refusals.rs`, and not `holes.py`, whose recognizer is a three-phrase match the
    message does not use. Widen the table to a roster of **source shapes** — a class-constant read, a
    `new`, a call through each receiver kind, each statement form — each asserted to reach a diagnostic
    or an IR. Then widen `holes.py`'s `REFUSAL` to recognize a panic by what it *is* rather than by how it
    is worded, and **re-derive `refusals.rs`'s `CEILING` once, from the true count, saying so in the
    commit**: a ratchet set from a blind count is not a ratchet, and 4 is what the blind count said.
    `crates/nvs-ir/tests/type_atoms.rs`, `crates/nvs-ir/tests/refusals.rs`, `tools/holes.py`.
50. **PHP's `case Name = 1;` enum spelling is refused by name.** The `E02xx` band exists for a rejected
    PHP construct answered with one diagnostic naming its ADR, and it mostly delivers: `trait` is
    `E0227` quoting ADR 0043, `(int)$x` is `E0225` quoting ADR 0034. An enum body written PHP's way is
    the outlier — Novis spells a case `Hearts = 1,` in a comma list
    ([ADR 0010](../adr/0010-enums-are-a-value-type.md) § 1) and PHP's `case Hearts = 1;` produces four
    diagnostics, none of which names that. The one that fires, `E0220`, points at the `1` and advises
    "move this to a separate class", which is the answer for a *method* in an enum body and is actively
    wrong here. One `E02xx` code, raised on the `case` keyword, naming the spelling that works. Next
    free in that band is `E0239`. `crates/nvs-syntax/src/parser/decl.rs`,
    `crates/nvs-diagnostics/src/lib.rs`.

## Stage 0 — the operator table, before anything else

**Every other stage's fixtures are written against these rules, so a case written before they land is
written around them.** That is the same compounding-cost argument the previous goal's Stage 0 made and paid
for at 48 files; here the shape is worse, because a case that spells `0.0 - 1.5` to avoid an `int`/`float`
mix reads as deliberate and nobody re-checks it. **Until this stage is empty, a session takes its group
from here.**

The items, grouped by the file set they share. `lower_binary` and `emit_binop` are the two ends of almost
all of it — one group, several sessions.

1. **ADR 0007 § 4's promotion table runs.** `1 + 1.5` does not compile today: `nvs_types` gives `$n + $f`
   a result type without converting either side, so both operands reach `nvs-codegen` in two
   representations and are refused there. The widening *is* the semantics rather than an approximation of
   it, so it belongs in `nvs-ir`'s existing conversion rows, not in the backend.
   `crates/nvs-ir/src/lower/operator.rs:470` (`lower_binary`), `crates/nvs-ir/src/lower/mod.rs:1731`
   (`coerce`), `crates/nvs-types/src/expr/operators.rs:403` (`arithmetic_result`),
   `crates/nvs-codegen/src/emit.rs:1008` (`emit_binop`). Same table decides `$n < $f`, which is the
   ordering half of the row and is refused the same way. `nvs-ir` gap 19, `nvs-codegen` gap 9.
2. **Integer `/` compiles.** ADR 0007 § 4 types `int / int` as `int|float` — PHP-exact, `6/3` an integer
   and `7/2` not. `Ty::Tagged` is the representation and `clif_ty` already gives it a machine type; what is
   missing is the operator picking at runtime, and `nvs_types` widening that union to `float` at a binding,
   which is what makes `float $avg = $sum / $n;` the ADR's own worked example.
   `crates/nvs-codegen/src/emit.rs:1008`, `crates/nvs-types/src/expr/operators.rs:96` (`binary_result`).
   `nvs-codegen` gap 5.
3. **Integer `+`/`-`/`*` throw `ArithmeticError` on overflow instead of wrapping.** ADR 0007 § 4 calls this
   the divergence from PHP it is least willing to trade, and the mechanism exists: integer `%`'s zero
   divisor already raises inline through `nvs_runtime::nvs_raise_new` and takes `nvs_ir::ir::Inst::on_error`'s
   edge, which is the shape a checked `iadd` wants. Three emit sites, one error edge each.
   `crates/nvs-codegen/src/emit.rs:1008`, `crates/nvs-ir/src/ir.rs:248` (`InstKind`). `nvs-codegen` gap 8.
4. **The bitwise operators exist.** `&`, `|`, `^`, `<<`, `>>` have no `ir::BinOp` variant at all, and unary
   `~` no `InstKind`; the grammar has had all six since M1 (`nvs_syntax::ast::BinaryOp`,
   `UnaryOp::BitNot`). Adding the variants gives `&=`, `|=`, `^=`, `<<=`, `>>=` their compound forms for
   free, because `lower_compound_assignment` rewrites `$x op= e` into the `$x = $x op e` it means.
   `crates/nvs-ir/src/ir.rs:1470` (`BinOp`), `crates/nvs-ir/src/lower/operator.rs:470`,
   `crates/nvs-ir/src/lower/stmt.rs:331`. `nvs-ir` gap 16.
5. **`**` and `**=` exist**, over every numeric row but the one ADR 0054 § 3 already refuses (a `decimal`
   base, which `nvs_types` reports). Same three files as item 4.
6. **`<=>` answers for a scalar.** It has no `decimal` row and no `int` one either — every scalar operand
   reaches `lower_expr`'s panic, and only ADR 0013's *object* form lowers today. ADR 0013's own
   `object_comparison_result` is the shape to match: `-1`/`0`/`1`, and the same three comparisons
   `lower_decimal_binary` already rewrites into six. `crates/nvs-ir/src/lower/operator.rs:43`, beside
   the object row at `:139`; the dispatch that panics is `crates/nvs-ir/src/lower/expr.rs:50`.
   `nvs-ir` gap 15.
7. **`$x++` and `--$x` lower**, in both positions, and with them the compound forms that inherit the same
   hole. Two things have to be split out of `lower_reassignment` first: the target's *address* computation,
   so `f()->count += 1` evaluates `f()` once where the rewrite reads it twice, and the increment's `1`,
   which has no source span to build an `ExprKind::Int` from and so cannot be desugared into an AST node
   the way every other compound form is. `crates/nvs-ir/src/lower/stmt.rs:415` (`lower_reassignment`),
   `:706` (`is_reevaluable_target`), `:331`. `nvs-ir` gap 16.
8. **`==` over two enum values compiles.** `Enum(Int)` is not on `emit_binop`'s integral list, so the
   comparison `nvs-ir` lowers is refused in the backend; the enum-to-backing reinterpretation
   `lower_literal_membership` already does for a membership chain is the same move.
   `crates/nvs-codegen/src/emit.rs:1008`. `nvs-codegen` gap 9's second shape.

## Stage 1 — the floor

The previous goal's entire acceptance list, unchanged: every Stage 1/2/3 fixture, both suites at their
previous thresholds, and both Part I guards. **Never traded for anything above it.** A session that finds
it has to change a floor fixture's expected output has found a bug in its own slice, not in the floor.

## Stage 2 — the operator fixture

`examples/operators.nvs`, which is Stage 0's item list as one program. It is listed separately so the
ledger distinguishes "the unit guards are green" from "the program runs".

## Stage 3 — control flow and calls

9. **`do`/`while` lowers.** `lower_while` with the branch moved below the body, and nothing new to build.
   `crates/nvs-ir/src/lower/control.rs:102`. `nvs-ir` gap 1.
10. **`break 2` and `continue 2` lower**, and a level past the enclosing nesting is a diagnostic rather
    than a panic — `nvs_types` does not check loop nesting at all today.
    `crates/nvs-ir/src/lower/control.rs:1365` and `:1375`, `crates/nvs-ir/src/lower/mod.rs:4212`.
11. **A ternary or a `match` whose arms lower to two representations widens to one.** Neither has a
    recorded result type to widen its arms to, which is the one thing `ExprInfo::Coalesce` supplies for
    `??`; closing it is that same recording in `nvs_types` plus `coerce` on each arm.
    `crates/nvs-ir/src/lower/expr.rs:1616` (`lower_match`), `crates/nvs-ir/src/lower/mod.rs:1552`.
    `nvs-ir` gap 5.
12. **A `finally` runs when a `catch` clause's own body throws.** Every other exit from a protected region
    already runs it, `return`/`break`/`continue` included. `crates/nvs-ir/src/lower/exception.rs:123`
    (`lower_try`), which owns the whole policy. `nvs-ir` gap 2, `nvs-codegen` gap 0.
13. **An abandoned generator runs the `finally` it is suspended inside.** Settled in
    § *Standing decisions*: a resume-to-unwind entry point, not a destructor.
    `crates/nvs-ir/src/lower/generator.rs:99`. `nvs-ir` gap 18.
14. **Every value fresh on the throw path is released.** A landing block sweeps the frame's locals and its
    owned-temporaries stack, but a producer that releases its fresh value inline — a normalized subscript
    key, a `match` subject — is not on that stack and leaks; so does an argument being *transferred* when a
    later one throws. `crates/nvs-ir/src/lower/mod.rs:1467` (`landing_block`), and `Lowering`'s own field
    doc for the second shape. `nvs-ir` gap 2's tail.
15. **`$f(...)` calls the closure the variable holds.** A closure literal lowers; the only caller today is
    native `Core` code going through `nvs_runtime::nvs_closure_call`.
    `crates/nvs-ir/src/lower/closure.rs:114`. `nvs-ir` gap 9.
16. **ADR 0027's first-class callable syntax lowers.** The named/spread half this item opened with is
    landed — `ResolvedCall::arg_slots` carries the mapping, `lower_variadic_tail` the tail — and what the
    file's refusal sites turned out to hold instead is `(...)`. Two of its four shapes are now diagnostics
    (`E0740` for `new C(...)`, `E0732` for a `mixed` receiver) and the two ADR 0027 § 1 keeps record
    `ExprInfo::CallableRef`, so what is left is lowering that record to a closure, plus `$g(...)` on a
    value already typed `callable`, which is PHP's identity and the one standing site.
    `crates/nvs-ir/src/lower/call.rs:766`, `crates/nvs-ir/src/lower/expr.rs:2433` and `:2622`.
    `nvs-ir` gap 8.
17. **A `...spread` array-literal element lowers.** `crates/nvs-ir/src/lower/expr.rs:3339`. The `&value`
    element is refused by decision instead — see § *Standing decisions*.
18. **An `inout` argument lowers in any expression position.** The copy-back is emitted at the enclosing
    statement because that is the nearest scope holding an `&mut Env`, so such a call lowers only as a bare
    expression statement or an assignment's right-hand side today. `crates/nvs-ir/src/lower/mod.rs:1090`
    (`pending_refs`). `nvs-ir` gap 10.
19. **A closure or generator may be written where an `inout` parameter is in scope.** ADR 0031 § 2 gives the
    language no by-reference *capture*, so what closes here is capturing such a parameter's **value** —
    today the whole shape panics. `crates/nvs-ir/src/lower/closure.rs:160`,
    `crates/nvs-ir/src/lower/generator.rs:155` and `:470`.
20. **`foreach (… as inout $v)` lowers.** `crates/nvs-ir/src/lower/control.rs:712`.

## Stage 4 — assignment targets, `mixed`, and the conversion table

21. **`C::$p = v` writes a static property**, and a declared default reaches one. A static property reads
    today; `Lowering`'s assignment arm has no target for one, and the per-class instance image
    `NvsObj::new` writes skips a static slot entirely. `crates/nvs-ir/src/lower/stmt.rs:415`.
    `nvs-ir` gap 6.
22. **A nested `$grid[0][1] = v` writes back.** The separated inner array has to be written into the outer
    one, and only a local or a known property is a place `nvs-ir` can write back to today; reading
    `$grid[0][1]` already works. `crates/nvs-ir/src/lower/mod.rs:1713`. `nvs-ir` gap 6.
23. **A property's declared default accepts more than a literal.** `= null`, an enum case, a `decimal`, a
    non-empty array literal and a `Class::CONST` are all `E0472` today. Widen the accepted set to every
    compile-time constant ADR 0046 § 2 already defines one as.
    `crates/nvs-diagnostics/src/lib.rs:738` (`E_PROPERTY_DEFAULT_NOT_LITERAL`).
24. **Arithmetic on a tagged operand runs.** A `Ty::Tagged` value can be built, carried and narrowed but
    not dispatched on: arithmetic on a `mixed`, ADR 0035's truthy table (which is `?bool` tested for
    truth), and an array access through a tagged base each panic naming themselves. Each closes as an
    `ir::Helper` variant dispatching on the tag — `nvs_runtime::value_truthy` is already the answer for the
    second — and never as a second representation. `crates/nvs-ir/src/ir.rs:1180` (`Helper`).
    `nvs-ir` gap 3.
25. **`never`, `iterable` and an intersection reach a representation or a diagnostic.** `object` — what
    this item used to be about — landed: `erase_checked_ty` maps `CheckedTy::Object` to `Ty::Object`
    beside `Class`, `Callable` and `Shape`, and a fixture declaring one, passing one and returning one
    runs. What still falls into the same catch-all is three shapes ADR 0007 § 3's grammar spells and
    nothing below the checker has an arm for, each of which type-checks and then panics
    `lower_checked_ty`:

    * **`never`** in a return *and* in a parameter. The parameter half is the checker's, not the IR's:
      § 3 says "`void` and `never` are return-only", `void` in a parameter is diagnosed and `never` is
      not, so one of the two rows closes with a diagnostic rather than a representation.
    * **`iterable`**, which additionally has **no arm in `nvs_types::expr::assign`** — neither an
      `array<int>` nor a `Core\Generator` is assignable to one, so it is a type no value can inhabit
      today and the IR arm alone would not make it usable.
    * **an intersection** (`A&B`), same two halves: no representation, and nothing is assignable to one
      even where every member is implemented, so `E0477` reports "names no class" for a member call
      through it.

    `crates/nvs-ir/src/lower/mod.rs:2206` (`erase_checked_ty`), `crates/nvs-types/src/expr/assign.rs`.
    `nvs-ir` gap 21. The six rows are named in `crates/nvs-ir/tests/type_atoms.rs`'s `KNOWN_ICE`, which
    is the ratchet that fails when one of them stops panicking as loudly as when a new one starts.
26. **`bool as int` and `bool as string` run**, with ADR 0007 § 2's remaining non-scalar rows beside them:
    `array<T> as array<U>`'s element walk — the one row in that table that is not a single helper call —
    and a `Ty::Tagged` operand converted to `bytes`, the one target with no runtime-tag row.
    `crates/nvs-ir/src/lower/convert.rs:60` (`convert`). `nvs-ir` gap 20.
27. **ADR 0066 § 3's two refusals are diagnostics.** `as ?T` where the conversion cannot fail
    (`$i as ?string`) or does not exist at all (`$arr as ?int`) is a **compile error** by that ADR;
    `nvs_types` refuses neither, so both reach lowering and panic naming the ADR.
    `crates/nvs-ir/src/lower/convert.rs:402` (`convert_or_null`) is where they arrive; the fix is in
    `nvs_types`. `nvs-ir` gap 4.
28. ~~**`echo $someCoreObject` answers or is diagnosed.**~~ **Done**, both halves and behind an erased
    operand too. `nvs_stdlib::registry::class_renders` is the one answer to "which `Core` classes does
    ADR 0028 § 1 make stringifiable", and it has two rows: a class the spec gives a `toString`, and a
    sink carrier that renders through ADR 0088 § 5 with no member at all. `require_stringable` reads it
    and refuses the rest where they are written (`E0710`); `nvs-ir` emits the native call the member
    written out takes. Where the operand's static type names *no* class there is nothing to resolve, so
    the runtime decides: `nvs_stdlib::instance` puts that same registered `toString` on the class's
    descriptor as `ClassDesc::renderer` and `nvs_runtime::stringify` asks for it before the compiled
    method table a `Core` class has no entry in. One implementation, reached two ways — the two
    agreement tests are `a_core_class_stringifies_exactly_where_the_registry_says_so` and
    `every_rendering_class_carries_a_renderer_or_is_a_carrier`.
29. **A nullsafe assignment target is a diagnostic.** `$a?->b = v` panics; PHP refuses it outright and
    `nvs_types` does not diagnose it. Same for an array-element write through a hooked property — see
    § *Standing decisions* for both. `crates/nvs-ir/src/lower/mod.rs:1702`.

## Stage 5 — the declared M4 features with no slice at all

30. **`PropertyObserver` runs** — ADR 0014 §§ 2–3, the half M4 names beside the hooks. The interface name
    is known to `nvs-syntax` and `nvs-hir` and appears in **no** other crate: no checker rule, no lowering,
    no runtime. Hooks themselves are done and are not this item. The `abi-probe` guard
    `a_class_without_a_property_observer_costs_nothing_extra` already exists and must stay green.
31. **ADR 0043's `by`-delegation runs.** Its syntax and its default/private-method slice landed; the
    delegation itself did not.
32. **ADR 0046 §§ 4–6 run** — `Core\Attributes::get<T>`/`::all<T>`, the structural compile-time retrieval,
    § 5's ambiguity error, and § 6's explicit call-site type argument. The attach grammar has parsed since
    M1 (`crates/nvs-syntax/src/ast.rs:419`); M4 names the call-site `<T>` here even though the retrieval
    body waits for M8.
33. **ADR 0092 and `Core\Debug::dump` land**, including ADR 0033's redaction of a `secret`-qualified
    property, which that ADR's own *Verification* defers to M4 by name.
34. **Inline HTML at file scope lowers.** The lowering is the `Helper::EchoStr` call `echo` already emits
    over the raw span; it is out only because `nvs_types` treats `InlineHtml` as a no-op too, so landing it
    widens two crates at once. `nvs-ir` gap 13.
35. **A `require`d file's own top-level statements run**, and ADR 0021 § 3's value form
    (`$c = require './config.nvs';`) has an arm. `lower_program` gives a script frame to `files[0]` and
    takes only the *declarations* of every other file. The shape that closes both is one frame per file,
    called from the site. `crates/nvs-ir/src/lower/mod.rs:387`. `nvs-ir` gap 22.
36. **A `FATAL` releases the frame's locals.** A `THROWN` does; an outcome no cleanup path and no `catch`
    can act on gets no landing block at all. `nvs_ir::ir::Inst::on_error` owns the asymmetry.
    `nvs-codegen` gap 3.

## Stage 6 — the checker and the front end

37. **ADR 0007 § 6's other narrowing spellings.** `== null`/`!= null` over a plain local narrows, and so
    does `instanceof` against a *class* — Stage 00's item 47 is its interface half, and this item is what
    is left once that lands: a comparison against a literal-typed value, and `match (true)`.
    `crates/nvs-types/src/locals.rs` owns the rule and what invalidates one.
38. **Exhaustive control-flow reachability** — "every path through this non-`void` function returns", and
    `switch`/`try` bodies contributing to definite assignment after them rather than conservatively
    nothing.
39. ~~**The four signature-level gaps.**~~ **Gone**, and the audit that emptied it is worth one line
    because three of the four were closed without anyone noticing they had been. A promoted
    constructor-parameter property works; a `new` with arguments on a user class with no `constructor` is
    `E0402`; a `foreach` key binding declared at anything but `string` is `E0723`, a diagnostic quoting
    ADR 0007 § 5, not the `nvs-ir` assertion this item described. The fourth — a class constant's type at
    an expression site — was real, and is Stage 00's item 45 together with the lowering half this item
    never mentioned.
40. **One equality-operand compatibility pass.** No general check exists for any type pair today — not
    `int` against `uint`, not two different enums — which is why singling enums out was declined. ADR 0090
    § 2's `reject_disjoint_equality` is the one-sided half that exists; this is the rest of it.
41. **References declare the same type on both sides.** `nvs_types` does not require it.
42. **Each unparsed front-end construct parses or is refused by name**: grouped `use` and
    `use function`/`use const` (settle against ADR 0015 and do whichever it says), a `goto` target label
    (ADR 0008 § 5 refuses `goto` — the label must be refused by the same diagnostic rather than falling
    through), a bare inline shape type on a local declaration, and an enum case whose name is a reserved
    keyword spelling. `crates/nvs-syntax/src/lib.rs` § *Known gaps* is the list.

## Stage 7 — `#[Test]`, the testing capability Novis programs use

43. **ADR 0079 lands** — `#[Test]` and the table the compiler builds from it, `#[Fixture]`, `#[TestWith]`,
    the generic `Core\Test` assertion roster, the ledger behind a catchable failure, and the human/JUnit/JSON
    reporters. That ADR's § 24 milestone table says which pieces land later; everything it puts at M4 is in
    scope here and nothing else is. **`.nvst` and `#[Test]` answer different questions and are never
    unified** (§ 23) — `nvs-test` is built and is not this item.

This stage is last because it is the only one that adds a *surface* rather than closing a hole, and because
every assertion in that roster is written against the operator table Stage 0 lands.

## Stage 8 — the corpus and the guards

The two suites, the Part I guards, the existing named guards, and the mechanical end gate. The conformance
floor rises to **750**. It is not M4's own 1000: that number is the milestone's acceptance and stays in
[docs/plan/m4.md](../plan/m4.md) unchanged, to be reached as the corpus keeps growing through M4S depth and
M8. **The stop condition for this loop is the feature gate, not the count** — 750 is the floor that says
the corpus grew with the features rather than around them, and the previous goal's 600 is inside it.

## Acceptance

**The checks live in [`loop-goal.toml`](loop-goal.toml), and only there.** Every fixture, its exact expected
output, every suite, every named guard test and the `.nvst` cases each item owes are in that file as data;
the driver reads it directly. Read it, or `python tools/loop.py --list`.

Every check must pass. Nothing else counts as done — not a passing unit test, not a session claiming
`DONE`. What the check kinds mean, and how the native/WSL legs and the valgrind sweep are ordered:
[coordinator.md](coordinator.md) § *The acceptance test*.

**The expected output in that file is frozen; a fixture's *source* is not.** The five new fixtures were
written by someone who could not compile them, so every signature, every `Core` member name and every
options bag in them is a reading that may be wrong. Correcting one is a bug fix, not a decision; re-freeze,
say why in the commit message, move on. What may never change is the expected output, or a fixture's reason
for existing. That is not licence to weaken a check to make it pass.

## Standing decisions — pre-authorized, do not stop the loop for these

Every one of these is settled. Implement it; do not re-open it.

- **Decide and record; never `BLOCKED` for a design call.** Settle it under AGENTS.md's priority ordering
  and record it in the home AGENTS.md already names — a paragraph in `docs/adr/README.md`
  § *Decisions taken at project start*, or the crate's own module doc. **Do not open a numbered ADR.**
  Reserve `BLOCKED` for a decision that is expensive to reverse *and* has no safe default.
- **An abandoned generator's `finally` runs, through a resume-to-unwind entry point on the state machine
  plus a release-path call to it.** PHP resumes a destroyed generator in a return-like mode and prints its
  `finally`; priority 2 (PHP-compatible observable behaviour) outranks priority 4 (simplicity), so the
  divergence is not kept. **This is not a destructor and does not re-open ADR 0028 § 2**: no user code
  runs that the program did not already suspend inside, no `__destruct` is recognized, and no class gains
  a lifecycle hook. The session that lands it folds one sentence into ADR 0028 § 2 saying so, and records
  the mechanism in `nvs_ir::lower::generator`'s module doc. Fallback if the state machine cannot express
  it: keep the divergence, pin it in `tests/differential/` as a named, deliberate difference, and say so
  in ADR 0028 § 2 — never leave it undocumented.
- **`int / int` widens at the binding, not at the operator.** `nvs_types` widens `int|float` to `float`
  where a binding, parameter or return declares one, exactly as ADR 0007 § 4's worked example spells it.
  The operator itself keeps the union.
- **Overflow throws `ArithmeticError`.** ADR 0007 § 4 already decided it; the cost is three checked emit
  sites and is not re-litigated against the wrapping we have.
- **A nullsafe assignment target (`$a?->b = v`) is a compile error**, matching PHP, and so is an
  array-element write through an ADR 0014 § 1 hooked property — PHP raises "indirect modification of
  overloaded property" there, so a diagnostic *is* the PHP-compatible answer and no write-back rule needs
  inventing. Both take a new `E`-code; the pack's *next free number* section names it.
- **`&value` as an array-literal element does not exist.** ADR 0031 § 2 removed by-reference capture and
  ADR 0023 fixes what a copy means; an aliasing array element has no owner in either. It is refused by a
  diagnostic naming this decision, and that is what closes item 17's other half.
- **A `Core` class is stringifiable exactly where the spec gives it a `toString`.**
  `nvs_stdlib::registry` states it, `require_stringable` reads it, and a `Core` class with none is an
  ordinary `E`-code at the `echo` rather than a panic below it.
- **The named/spread argument checker half lands before the lowering half**, in that order, in
  `nvs_types` — a lowering with no resolved per-argument type to lower against is how gap 8 got here.
- **`object` erases to the same pointer a named class does.** The representation is settled; only the
  "does anything below read a class label" check is work.
- **Not in scope, and not holes**: virtual dispatch by slot rather than by name (a lookup cost, M12), a
  `br_table` for a dense `switch` (M12), string-literal deduplication in a unit's data section
  (`nvs-codegen` gap 4), freeing executable memory (ADR 0017, M6), ADR 0018's `BRANCH` probe and the
  `COLLECT`/`DEBUG_BREAK` safepoint flags (no collector and no debugger exist to hand a frame to), a cycle
  collector (decided against, ADR 0004), ADR 0024 § 4's sink list and ADR 0033's `Core\Log` inspection
  (both need `Core` classes that arrive at M7/M8), and ADRs 0091, 0093, 0097 and 0100 § 3 (M6, M7, M8,
  M10). A session that finds one of these on its path puts it in the handoff's `## Backlog` and moves on.
- **The allowlist in `every_refusal_is_a_diagnostic_or_decided` may never grow.** Every entry is a bullet
  in this section. A session that believes it needs a new one has found a decision, and takes it here —
  in this file, in the same session, with the reason — or it has found a hole it is trying to skip.
- **A refusal site an open numbered item claims is scheduled, not undecided**, so it passes
  `every_refusal_is_a_diagnostic_or_decided` without being on that allowlist. `tools/holes.py`'s own
  attribution decides which item claims a site, and the last item closing is what leaves a site with
  nowhere to belong — which is when the gate starts refusing everything the allowlist does not hold,
  the end state its check's name describes. Attribution is by file, so the same test carries a
  ceiling on the *total* count that ratchets down and never up; a new refusal beside an old one in a
  claimed file would otherwise be invisible.
- **A `CodegenError` no program can reach is `Internal`, not `Unsupported`.** `tools/holes.py` reads
  the `Unsupported` constructor as the inventory of shapes the language still refuses, so an engine
  invariant wearing that type is an item no session could ever close. `nvs_codegen::ty::tag_of`'s
  two arms — a tagged value's static tag, and a representation that is not a value — are the first
  two, and their doc comment already said no path asks.
- **Picking every dependency but the two the user named** stays pre-authorized, unchanged from the
  previous goal.

## What this goal does not touch

`docs/` trimming (the user fires [doc-cleanup.md](doc-cleanup.md), never a session), dependency sweeps
([dependency-update.md](dependency-update.md), same rule), and M4S `Core` **breadth** beyond what Stage 1's
floor and Stage 8's suites already hold. A `Core` depth case from `python tools/gaps.py` is a legitimate
slice when a session's group is blocked or when the conformance floor is what is left; it is never the
reason to leave an item above unfinished.
