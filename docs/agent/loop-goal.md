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

A hole is a shape that **compiles in the front end and then refuses below it** — `mwl-ir` panics naming
itself, `mwl-codegen` refuses two representations, or the checker types an expression it has no lowering
for. Each of the crate module docs' *Known gaps* lists is an inventory of them, and this goal's item list
is that inventory ordered and grouped.

A shape is **closed** when it either

1. runs, with a fixture or a `.mwlt` case pinning what it prints, **or**
2. is refused by a **diagnostic that names the rule** — a numbered `E`-code and the ADR behind it.

The second is not a loophole. `goto` is refused by ADR 0008 § 5 and by-reference capture by ADR 0031 § 2;
a language that refuses them with a diagnostic has no hole, while one that falls through to a generic parse
error or a lowering panic does. **What may never close an item is a panic**, however well it names itself.

The mechanical end gate is Stage 8's `every_refusal_is_a_diagnostic_or_decided`: it reads `mwl-ir`'s and
`mwl-codegen`'s own sources for refusal sites and fails naming any that is not on the allowlist frozen in
the test. **That allowlist may never grow.** Every entry on it is a decision in this file's
§ *Standing decisions* with its ADR; adding an entry to make a run go green is the one move this goal
forbids outright. `python tools/holes.py` prints the same inventory as a worklist, mapped to the items
below, so that no session re-derives it.

## Stage 0 — the operator table, before anything else

**Every other stage's fixtures are written against these rules, so a case written before they land is
written around them.** That is the same compounding-cost argument the previous goal's Stage 0 made and paid
for at 48 files; here the shape is worse, because a case that spells `0.0 - 1.5` to avoid an `int`/`float`
mix reads as deliberate and nobody re-checks it. **Until this stage is empty, a session takes its group
from here.**

The items, grouped by the file set they share. `lower_binary` and `emit_binop` are the two ends of almost
all of it — one group, several sessions.

1. **ADR 0007 § 4's promotion table runs.** `1 + 1.5` does not compile today: `mwl_types` gives `$n + $f`
   a result type without converting either side, so both operands reach `mwl-codegen` in two
   representations and are refused there. The widening *is* the semantics rather than an approximation of
   it, so it belongs in `mwl-ir`'s existing conversion rows, not in the backend.
   `crates/mwl-ir/src/lower/operator.rs:470` (`lower_binary`), `crates/mwl-ir/src/lower/mod.rs:1731`
   (`coerce`), `crates/mwl-types/src/expr/operators.rs:403` (`arithmetic_result`),
   `crates/mwl-codegen/src/emit.rs:1008` (`emit_binop`). Same table decides `$n < $f`, which is the
   ordering half of the row and is refused the same way. `mwl-ir` gap 19, `mwl-codegen` gap 9.
2. **Integer `/` compiles.** ADR 0007 § 4 types `int / int` as `int|float` — PHP-exact, `6/3` an integer
   and `7/2` not. `Ty::Tagged` is the representation and `clif_ty` already gives it a machine type; what is
   missing is the operator picking at runtime, and `mwl_types` widening that union to `float` at a binding,
   which is what makes `float $avg = $sum / $n;` the ADR's own worked example.
   `crates/mwl-codegen/src/emit.rs:1008`, `crates/mwl-types/src/expr/operators.rs:96` (`binary_result`).
   `mwl-codegen` gap 5.
3. **Integer `+`/`-`/`*` throw `ArithmeticError` on overflow instead of wrapping.** ADR 0007 § 4 calls this
   the divergence from PHP it is least willing to trade, and the mechanism exists: integer `%`'s zero
   divisor already raises inline through `mwl_runtime::mwl_raise_new` and takes `mwl_ir::ir::Inst::on_error`'s
   edge, which is the shape a checked `iadd` wants. Three emit sites, one error edge each.
   `crates/mwl-codegen/src/emit.rs:1008`, `crates/mwl-ir/src/ir.rs:248` (`InstKind`). `mwl-codegen` gap 8.
4. **The bitwise operators exist.** `&`, `|`, `^`, `<<`, `>>` have no `ir::BinOp` variant at all, and unary
   `~` no `InstKind`; the grammar has had all six since M1 (`mwl_syntax::ast::BinaryOp`,
   `UnaryOp::BitNot`). Adding the variants gives `&=`, `|=`, `^=`, `<<=`, `>>=` their compound forms for
   free, because `lower_compound_assignment` rewrites `$x op= e` into the `$x = $x op e` it means.
   `crates/mwl-ir/src/ir.rs:1470` (`BinOp`), `crates/mwl-ir/src/lower/operator.rs:470`,
   `crates/mwl-ir/src/lower/stmt.rs:331`. `mwl-ir` gap 16.
5. **`**` and `**=` exist**, over every numeric row but the one ADR 0054 § 3 already refuses (a `decimal`
   base, which `mwl_types` reports). Same three files as item 4.
6. **`<=>` answers for a scalar.** It has no `decimal` row and no `int` one either — every scalar operand
   reaches `lower_expr`'s panic, and only ADR 0013's *object* form lowers today. ADR 0013's own
   `object_comparison_result` is the shape to match: `-1`/`0`/`1`, and the same three comparisons
   `lower_decimal_binary` already rewrites into six. `crates/mwl-ir/src/lower/operator.rs:43`, beside
   the object row at `:139`; the dispatch that panics is `crates/mwl-ir/src/lower/expr.rs:50`.
   `mwl-ir` gap 15.
7. **`$x++` and `--$x` lower**, in both positions, and with them the compound forms that inherit the same
   hole. Two things have to be split out of `lower_reassignment` first: the target's *address* computation,
   so `f()->count += 1` evaluates `f()` once where the rewrite reads it twice, and the increment's `1`,
   which has no source span to build an `ExprKind::Int` from and so cannot be desugared into an AST node
   the way every other compound form is. `crates/mwl-ir/src/lower/stmt.rs:415` (`lower_reassignment`),
   `:706` (`is_reevaluable_target`), `:331`. `mwl-ir` gap 16.
8. **`==` over two enum values compiles.** `Enum(Int)` is not on `emit_binop`'s integral list, so the
   comparison `mwl-ir` lowers is refused in the backend; the enum-to-backing reinterpretation
   `lower_literal_membership` already does for a membership chain is the same move.
   `crates/mwl-codegen/src/emit.rs:1008`. `mwl-codegen` gap 9's second shape.

## Stage 1 — the floor

The previous goal's entire acceptance list, unchanged: every Stage 1/2/3 fixture, both suites at their
previous thresholds, and both Part I guards. **Never traded for anything above it.** A session that finds
it has to change a floor fixture's expected output has found a bug in its own slice, not in the floor.

## Stage 2 — the operator fixture

`examples/operators.mwl`, which is Stage 0's item list as one program. It is listed separately so the
ledger distinguishes "the unit guards are green" from "the program runs".

## Stage 3 — control flow and calls

9. **`do`/`while` lowers.** `lower_while` with the branch moved below the body, and nothing new to build.
   `crates/mwl-ir/src/lower/control.rs:102`. `mwl-ir` gap 1.
10. **`break 2` and `continue 2` lower**, and a level past the enclosing nesting is a diagnostic rather
    than a panic — `mwl_types` does not check loop nesting at all today.
    `crates/mwl-ir/src/lower/control.rs:1365` and `:1375`, `crates/mwl-ir/src/lower/mod.rs:4212`.
11. **A ternary or a `match` whose arms lower to two representations widens to one.** Neither has a
    recorded result type to widen its arms to, which is the one thing `ExprInfo::Coalesce` supplies for
    `??`; closing it is that same recording in `mwl_types` plus `coerce` on each arm.
    `crates/mwl-ir/src/lower/expr.rs:1616` (`lower_match`), `crates/mwl-ir/src/lower/mod.rs:1552`.
    `mwl-ir` gap 5.
12. **A `finally` runs when a `catch` clause's own body throws.** Every other exit from a protected region
    already runs it, `return`/`break`/`continue` included. `crates/mwl-ir/src/lower/exception.rs:123`
    (`lower_try`), which owns the whole policy. `mwl-ir` gap 2, `mwl-codegen` gap 0.
13. **An abandoned generator runs the `finally` it is suspended inside.** Settled in
    § *Standing decisions*: a resume-to-unwind entry point, not a destructor.
    `crates/mwl-ir/src/lower/generator.rs:99`. `mwl-ir` gap 18.
14. **Every value fresh on the throw path is released.** A landing block sweeps the frame's locals and its
    owned-temporaries stack, but a producer that releases its fresh value inline — a normalized subscript
    key, a `match` subject — is not on that stack and leaks; so does an argument being *transferred* when a
    later one throws. `crates/mwl-ir/src/lower/mod.rs:1467` (`landing_block`), and `Lowering`'s own field
    doc for the second shape. `mwl-ir` gap 2's tail.
15. **`$f(...)` calls the closure the variable holds.** A closure literal lowers; the only caller today is
    native `Core` code going through `mwl_runtime::mwl_closure_call`.
    `crates/mwl-ir/src/lower/closure.rs:114`. `mwl-ir` gap 9.
16. **A named argument and a spread argument type-check and lower.** `mwl_types` does not positionally
    check either, so there is no resolved per-argument type to lower against — the checker's half lands
    first. A variadic signature is already done (`lower_variadic_tail`).
    `crates/mwl-ir/src/lower/call.rs:77`, `:219`. `mwl-ir` gap 8.
17. **A `...spread` array-literal element lowers.** `crates/mwl-ir/src/lower/expr.rs:3339`. The `&value`
    element is refused by decision instead — see § *Standing decisions*.
18. **An `inout` argument lowers in any expression position.** The copy-back is emitted at the enclosing
    statement because that is the nearest scope holding an `&mut Env`, so such a call lowers only as a bare
    expression statement or an assignment's right-hand side today. `crates/mwl-ir/src/lower/mod.rs:1090`
    (`pending_refs`). `mwl-ir` gap 10.
19. **A closure or generator may be written where an `inout` parameter is in scope.** ADR 0031 § 2 gives the
    language no by-reference *capture*, so what closes here is capturing such a parameter's **value** —
    today the whole shape panics. `crates/mwl-ir/src/lower/closure.rs:160`,
    `crates/mwl-ir/src/lower/generator.rs:155` and `:470`.
20. **`foreach (… as inout $v)` lowers.** `crates/mwl-ir/src/lower/control.rs:712`.

## Stage 4 — assignment targets, `mixed`, and the conversion table

21. **`C::$p = v` writes a static property**, and a declared default reaches one. A static property reads
    today; `Lowering`'s assignment arm has no target for one, and the per-class instance image
    `MwlObj::new` writes skips a static slot entirely. `crates/mwl-ir/src/lower/stmt.rs:415`.
    `mwl-ir` gap 6.
22. **A nested `$grid[0][1] = v` writes back.** The separated inner array has to be written into the outer
    one, and only a local or a known property is a place `mwl-ir` can write back to today; reading
    `$grid[0][1]` already works. `crates/mwl-ir/src/lower/mod.rs:1713`. `mwl-ir` gap 6.
23. **A property's declared default accepts more than a literal.** `= null`, an enum case, a `decimal`, a
    non-empty array literal and a `Class::CONST` are all `E0472` today. Widen the accepted set to every
    compile-time constant ADR 0046 § 2 already defines one as.
    `crates/mwl-diagnostics/src/lib.rs:738` (`E_PROPERTY_DEFAULT_NOT_LITERAL`).
24. **Arithmetic on a tagged operand runs.** A `Ty::Tagged` value can be built, carried and narrowed but
    not dispatched on: arithmetic on a `mixed`, ADR 0035's truthy table (which is `?bool` tested for
    truth), and an array access through a tagged base each panic naming themselves. Each closes as an
    `ir::Helper` variant dispatching on the tag — `mwl_runtime::value_truthy` is already the answer for the
    second — and never as a second representation. `crates/mwl-ir/src/ir.rs:1180` (`Helper`).
    `mwl-ir` gap 3.
25. **`object` as a declared type has a representation arm.** `erase_checked_ty` maps a *named* class to
    `Ty::Object`, and the checker's own ADR 0007 § 3 `object` top reaches no arm at all, so
    `object $o = $obj;` panics. The representation is not in question — it is the same pointer — and the
    arm is one line; what a session owes is the check that nothing below reads a class *label* off an
    operand it would now receive without one. `crates/mwl-ir/src/lower/mod.rs:2206`. `mwl-ir` gap 21.
26. **`bool as int` and `bool as string` run**, with ADR 0007 § 2's remaining non-scalar rows beside them:
    `array<T> as array<U>`'s element walk — the one row in that table that is not a single helper call —
    and a `Ty::Tagged` operand converted to `bytes`, the one target with no runtime-tag row.
    `crates/mwl-ir/src/lower/convert.rs:60` (`convert`). `mwl-ir` gap 20.
27. **ADR 0066 § 3's two refusals are diagnostics.** `as ?T` where the conversion cannot fail
    (`$i as ?string`) or does not exist at all (`$arr as ?int`) is a **compile error** by that ADR;
    `mwl_types` refuses neither, so both reach lowering and panic naming the ADR.
    `crates/mwl-ir/src/lower/convert.rs:402` (`convert_or_null`) is where they arrive; the fix is in
    `mwl_types`. `mwl-ir` gap 4.
28. ~~**`echo $someCoreObject` answers or is diagnosed.**~~ **Done**, both halves and behind an erased
    operand too. `mwl_stdlib::registry::class_renders` is the one answer to "which `Core` classes does
    ADR 0028 § 1 make stringifiable", and it has two rows: a class the spec gives a `toString`, and a
    sink carrier that renders through ADR 0088 § 5 with no member at all. `require_stringable` reads it
    and refuses the rest where they are written (`E0710`); `mwl-ir` emits the native call the member
    written out takes. Where the operand's static type names *no* class there is nothing to resolve, so
    the runtime decides: `mwl_stdlib::instance` puts that same registered `toString` on the class's
    descriptor as `ClassDesc::renderer` and `mwl_runtime::stringify` asks for it before the compiled
    method table a `Core` class has no entry in. One implementation, reached two ways — the two
    agreement tests are `a_core_class_stringifies_exactly_where_the_registry_says_so` and
    `every_rendering_class_carries_a_renderer_or_is_a_carrier`.
29. **A nullsafe assignment target is a diagnostic.** `$a?->b = v` panics; PHP refuses it outright and
    `mwl_types` does not diagnose it. Same for an array-element write through a hooked property — see
    § *Standing decisions* for both. `crates/mwl-ir/src/lower/mod.rs:1702`.

## Stage 5 — the declared M4 features with no slice at all

30. **`PropertyObserver` runs** — ADR 0014 §§ 2–3, the half M4 names beside the hooks. The interface name
    is known to `mwl-syntax` and `mwl-hir` and appears in **no** other crate: no checker rule, no lowering,
    no runtime. Hooks themselves are done and are not this item. The `abi-probe` guard
    `a_class_without_a_property_observer_costs_nothing_extra` already exists and must stay green.
31. **ADR 0043's `by`-delegation runs.** Its syntax and its default/private-method slice landed; the
    delegation itself did not.
32. **ADR 0046 §§ 4–6 run** — `Core\Attributes::get<T>`/`::all<T>`, the structural compile-time retrieval,
    § 5's ambiguity error, and § 6's explicit call-site type argument. The attach grammar has parsed since
    M1 (`crates/mwl-syntax/src/ast.rs:419`); M4 names the call-site `<T>` here even though the retrieval
    body waits for M8.
33. **ADR 0092 and `Core\Debug::dump` land**, including ADR 0033's redaction of a `secret`-qualified
    property, which that ADR's own *Verification* defers to M4 by name.
34. **Inline HTML at file scope lowers.** The lowering is the `Helper::EchoStr` call `echo` already emits
    over the raw span; it is out only because `mwl_types` treats `InlineHtml` as a no-op too, so landing it
    widens two crates at once. `mwl-ir` gap 13.
35. **A `require`d file's own top-level statements run**, and ADR 0021 § 3's value form
    (`$c = require './config.mwl';`) has an arm. `lower_program` gives a script frame to `files[0]` and
    takes only the *declarations* of every other file. The shape that closes both is one frame per file,
    called from the site. `crates/mwl-ir/src/lower/mod.rs:387`. `mwl-ir` gap 22.
36. **A `FATAL` releases the frame's locals.** A `THROWN` does; an outcome no cleanup path and no `catch`
    can act on gets no landing block at all. `mwl_ir::ir::Inst::on_error` owns the asymmetry.
    `mwl-codegen` gap 3.

## Stage 6 — the checker and the front end

37. **ADR 0007 § 6's other three narrowing spellings.** `== null`/`!= null` over a plain local narrows;
    `instanceof`, a comparison against a literal-typed value and `match (true)` do not, and the residue is
    restricted to a class. `crates/mwl-types/src/locals.rs` owns the rule and what invalidates one.
38. **Exhaustive control-flow reachability** — "every path through this non-`void` function returns", and
    `switch`/`try` bodies contributing to definite assignment after them rather than conservatively
    nothing.
39. **The four signature-level gaps**: a user-declared class constant's type at an expression site, a
    promoted constructor-parameter property, a class with no explicit `constructor` held to a
    zero-argument arity check on `new`, and a `foreach` **key** binding declared at anything but `string`
    (ADR 0007 § 5 gives an array one stored key type, so it is always wrong, and today it type-checks and
    then trips an assertion in `mwl-ir`). `crates/mwl-types/src/expr/mod.rs`, `signatures`.
40. **One equality-operand compatibility pass.** No general check exists for any type pair today — not
    `int` against `uint`, not two different enums — which is why singling enums out was declined. ADR 0090
    § 2's `reject_disjoint_equality` is the one-sided half that exists; this is the rest of it.
41. **References declare the same type on both sides.** `mwl_types` does not require it.
42. **Each unparsed front-end construct parses or is refused by name**: grouped `use` and
    `use function`/`use const` (settle against ADR 0015 and do whichever it says), a `goto` target label
    (ADR 0008 § 5 refuses `goto` — the label must be refused by the same diagnostic rather than falling
    through), a bare inline shape type on a local declaration, and an enum case whose name is a reserved
    keyword spelling. `crates/mwl-syntax/src/lib.rs` § *Known gaps* is the list.

## Stage 7 — `#[Test]`, the testing capability MWL programs use

43. **ADR 0079 lands** — `#[Test]` and the table the compiler builds from it, `#[Fixture]`, `#[TestWith]`,
    the generic `Core\Test` assertion roster, the ledger behind a catchable failure, and the human/JUnit/JSON
    reporters. That ADR's § 24 milestone table says which pieces land later; everything it puts at M4 is in
    scope here and nothing else is. **`.mwlt` and `#[Test]` answer different questions and are never
    unified** (§ 23) — `mwl-test` is built and is not this item.

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
output, every suite, every named guard test and the `.mwlt` cases each item owes are in that file as data;
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
  the mechanism in `mwl_ir::lower::generator`'s module doc. Fallback if the state machine cannot express
  it: keep the divergence, pin it in `tests/differential/` as a named, deliberate difference, and say so
  in ADR 0028 § 2 — never leave it undocumented.
- **`int / int` widens at the binding, not at the operator.** `mwl_types` widens `int|float` to `float`
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
  `mwl_stdlib::registry` states it, `require_stringable` reads it, and a `Core` class with none is an
  ordinary `E`-code at the `echo` rather than a panic below it.
- **The named/spread argument checker half lands before the lowering half**, in that order, in
  `mwl_types` — a lowering with no resolved per-argument type to lower against is how gap 8 got here.
- **`object` erases to the same pointer a named class does.** The representation is settled; only the
  "does anything below read a class label" check is work.
- **Not in scope, and not holes**: virtual dispatch by slot rather than by name (a lookup cost, M12), a
  `br_table` for a dense `switch` (M12), string-literal deduplication in a unit's data section
  (`mwl-codegen` gap 4), freeing executable memory (ADR 0017, M6), ADR 0018's `BRANCH` probe and the
  `COLLECT`/`DEBUG_BREAK` safepoint flags (no collector and no debugger exist to hand a frame to), a cycle
  collector (decided against, ADR 0004), ADR 0024 § 4's sink list and ADR 0033's `Core\Log` inspection
  (both need `Core` classes that arrive at M7/M8), and ADRs 0091, 0093, 0097 and 0100 § 3 (M6, M7, M8,
  M10). A session that finds one of these on its path puts it in the handoff's `## Backlog` and moves on.
- **The allowlist in `every_refusal_is_a_diagnostic_or_decided` may never grow.** Every entry is a bullet
  in this section. A session that believes it needs a new one has found a decision, and takes it here —
  in this file, in the same session, with the reason — or it has found a hole it is trying to skip.
- **Picking every dependency but the two the user named** stays pre-authorized, unchanged from the
  previous goal.

## What this goal does not touch

`docs/` trimming (the user fires [doc-cleanup.md](doc-cleanup.md), never a session), dependency sweeps
([dependency-update.md](dependency-update.md), same rule), and M4S `Core` **breadth** beyond what Stage 1's
floor and Stage 8's suites already hold. A `Core` depth case from `python tools/gaps.py` is a legitimate
slice when a session's group is blocked or when the conformance floor is what is left; it is never the
reason to leave an item above unfinished.
