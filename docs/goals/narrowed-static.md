---
looper: 1
title: a body returns an object it tested with is static as static
summary: "Reached when $o is static tests the class the call named at run time, a value narrowed by is static and a local declared ?static or array<static> are on E0741's list, every write to such a local is checked, and the rule, the feature proofs and the website say so."
context: [docs/rules/statements/static-is-a-member-modifier.md, crates/nvs-types/src/check.rs, crates/nvs-types/tests/returns.rs]
stages:
  - id: 1
    title: "$o is static tests the class the call named, and is refused inside an anonymous function"
    status: open
    context: [docs/rules/types/type-test.md, crates/nvs-types/src/expr/type_test.rs, crates/nvs-types/src/expr_table.rs, crates/nvs-ir/src/lower/expr.rs]
  - id: 2
    title: "a value narrowed by is static is on E0741's list"
    status: open
    context: [docs/rules/types/narrowing.md, crates/nvs-types/src/locals.rs]
  - id: 3
    title: "a local declared with a type that binds static is checked at every write, and is on E0741's list"
    status: open
    context: [crates/nvs-types/src/locals.rs, crates/nvs-types/src/expr/assign.rs, crates/nvs-diagnostics/src/code/types_e10.rs]
  - id: 4
    title: "the rule, the reference, the examples, the feature proofs and the website say so"
    status: open
    context: [docs/reference/lang/50-classes.md, docs/examples/lang/types/void-never-self-static/about.md, website/src/data/changes.json]
  - id: 5
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: called-class-test
    stage: 1
    name: "$o is static is true exactly for an instance of the called class, in a static and in an instance method, and is E0834 inside an anonymous function"
    run: [cargo, test, --test, narrowed_static]
    tests: [is_static_is_not_folded_in_a_class_that_is_not_final, is_static_inside_an_anonymous_function_does_not_compile, is_static_in_a_final_class_still_folds]
    timeout: 1800
  - id: called-class-run
    stage: 1
    name: "a run tests is static against the called class"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/lang/is-static-tests-the-class-the-call-names.nvst]
    timeout: 900
  - id: narrowed
    stage: 2
    name: "a variable narrowed by is static may be returned as static and ?static, and a write or a loop that drops the narrowing drops the proof"
    run: [cargo, test, --test, narrowed_static]
    tests: [a_value_narrowed_by_is_static_is_the_called_class, a_write_after_is_static_drops_the_proof, the_else_edge_of_is_static_proves_nothing, a_negated_is_static_guard_clause_proves_the_rest]
    timeout: 1800
  - id: static-locals
    stage: 3
    name: "a local declared ?static or array<static> takes only values on E0741's list, its return compiles, and an inout pass of it does not"
    run: [cargo, test, --test, narrowed_static]
    tests: [a_static_local_may_be_returned, a_write_to_a_static_local_must_be_the_called_class, an_element_write_to_a_static_array_must_be_the_called_class, a_static_local_cannot_be_passed_inout, a_static_local_in_a_final_class_is_any_value_of_it]
    timeout: 1800
  - id: finders-run
    stage: 3
    name: "the find-first and find-all case compiles and prints the called class's objects only"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/lang/a-finder-returns-loaded-objects-as-static.nvst]
    timeout: 900
  - id: proofs
    stage: 4
    name: "every feature the goal's work reached has its feature proofs"
    run: [bun, nv, proofs, --gate, --goal, narrowed-static]
    timeout: 1800
  - id: close
    stage: 5
    name: "the goal-end check is green: ci --check's checks of the tree's files, the owners and the goal's proofs"
    run: [bun, nv, close, --goal, narrowed-static]
    timeout: 900
---

# a body returns an object it tested with is static as static

## Why

A base class with static finders is the common shape of a model layer: `Order::find($id)`,
`Order::findAll()`. The call always names the class, so the answer is always that class, and the
signature should say so with `?static` and `array<static>`. A finder does not build its objects
with `new static`. It takes them from a request's cache of loaded objects, or builds them from rows
through the class's name, and both are typed as the base class. Today E0741 refuses every such
return, so the finder declares the base class and every caller writes `as Order`.

There is also a bug under it. `$o is static` is folded with `static` read as the declaring class
(`crates/nvs-types/src/expr/type_test.rs:130`, `always_holds`). In `Model`, `Model $o; $o is static`
is the constant `true`, so `Other::isCalledClass(new Child())` prints `true`, in a static method and
in an instance method alike. `rule:statements/static-is-a-member-modifier` says `static` is the class
the call names. And `is static` is accepted inside an anonymous function, where every `static::` is
E0834. A body therefore cannot test whether an object is the called class at all, except by
comparing `$o::class` with `static::class` by hand.

The order follows from the first priority, security and request isolation. Item 2 without item 1
would type an `Other` as a `Child`, so stage 1 lands first and stages 2 and 3 build on it.

## Standing decisions

**The user's calls (2026-11-26):** the `is static` bug is stage 1 of this goal, not a fix of its own;
option A below; the goal sits on the chain after `test-only`, because both edit
`crates/nvs-types/src/check.rs`. Every other call here is the session's that wrote the goal, open
until the user changes one.

**Stage 1: `is static` is a run-time test against the called class.** In a class that is not
`final`, `$o is static` and `$o is ?static` are never folded by `always_holds` or `never_holds`. The
checker records a test against the called class: a new `ExprInfo` variant beside `ClassRefTest`
(`crates/nvs-types/src/expr_table.rs`), or `ClassRefTest` with a flag if that is smaller. The
session picks one and says why in its doc. `nvs-ir` lowers it as the value arm does
(`crates/nvs-ir/src/lower/expr.rs:7838`), with `self.lsb()` (`crates/nvs-ir/src/lower/mod.rs:3461`)
as the descriptor: `InstKind::ClassTest { value, class: TestedClass::Descriptor(desc) }`. The true
edge still narrows the subject to the declaring class's type. In an instance method the called class
is `$this`'s class, which `lsb()` already loads. In a `final` class `static` is exactly the class, so
the fold stays and nothing changes there. Inside an anonymous function, `is static` is E0834, at the
check `crates/nvs-types/src/expr/mod.rs:1488` already makes for `static::`. That check's message names
`static::`, so it gets a second wording for the type position. `static` written deeper in a tested
type (`$xs is array<static>`) reads the same frame class; if the tested type's lowering cannot carry
it, that spelling is E0834's sibling refusal "`static` inside a tested type", and the session writes
it to the rule. Cost: one descriptor walk per test, the walk `$x is $cls` already pays, instead of a
constant. No memory.

**Stage 2: a narrowed value is the called class.** On the true edge of `$o is static`, the subject
carries a "called class" fact beside its narrowing. `yields_the_called_class`
(`crates/nvs-types/src/check.rs:1763`) accepts a variable, or a `readonly` path, that carries it. The
fact lives and dies with the narrowing it came with: every write, `drop_narrowing`, loop suspension
and `Narrowing::restore` that removes the narrowing removes the fact
(`crates/nvs-types/src/locals.rs:437`, `narrow` at `:535`). The simplest shape is a set of names kept
next to `LocalScope::narrowed`, checked together with the entry that `is static` installed. The type
in `narrowed` stays the declaring class. So the Declined item "a `static` type kept apart from the
declaring class inside the body" stays declined: no type gains a second meaning, only E0741's list
gains an entry. `null` already passes for `?static`. The guard clause `if (!($o is static)) {
continue; }` proves the rest of the block, by the same rule every other narrowing follows.

**Stage 3: a local declared with a type that binds `static`.** A local whose written type binds
`static` (`crate::signatures::static_use(...).binds`, in a class that is not `final`) is recorded in
the body's `LocalScope`, as `static_callbacks` records a `callable(static)` parameter
(`crates/nvs-types/src/locals.rs:222`). Its type stays the declaring class's. Every write to it is held
to E0741's list: its initializer, an assignment, an append `$xs[] = $v`, an element write
`$xs[$k] = $v`, a nested element write and a compound write. `check_write_target`
(`crates/nvs-types/src/expr/assign.rs:1434`) and `check_assign` (`:1133`) are where the writes meet.
A value off the list is the new error `E1020` (the next free code after `test-only`'s `E1019`; the
session takes the next free one if that has moved): "a variable declared `static` gets a value that is
not the called class", primary label on the value, secondary on the declaration. An `inout` pass of
such a local is `E1020` too, because the callee's parameter type is the declaring class and may write
any value of it. A `foreach` over it with a by-value element is a read and is allowed. Removing an
element is allowed. `yields_the_called_class` then accepts a read of such a local. This replaces the
rule's sentence "a local variable is none of these, so an `array<static>` built up in a loop is
refused, and declaring `array<self>` is the way to write it". Cost: check time only, one small record
per such local for one body. Nothing at run time.

**What stays refused.** `$o as static` and `$o as ?static` stay E0473 (`rule:types/conversion`): `is`
is how the language narrows a class, and the reporter of the case did not ask for it. A call through
a class value, `$class::find($id)` with `class<Model> $class`, keeps returning the declaring
class. Binding `static` in a property type stays as it is.

**Rules.** `docs/rules/statements/static-is-a-member-modifier.md` gets: `is static` tests the called
class and is refused inside an anonymous function; E0741's list adds the narrowed value and the
`static` local; the `array<self>` sentence is replaced; the Memory paragraph names the descriptor walk
and the per-local record; the Declined list gains "a run-time checked `as static`" with its reason.
`docs/rules/types/type-test.md` names the one written type whose answer depends on the frame.
`docs/ground-rules.md` gets its line from `bun nv rules` if a title changes.

**Feature proofs.** The feature is `lang:types/void-never-self-static`; `bun nv proofs --owed --goal
narrowed-static` names every feature the work reached. The new example
`07-a-finder-that-returns-the-class-the-call-names.nvs` under
`docs/examples/lang/types/void-never-self-static/`, with neutral names (`Model`, `Order`,
`Customer`): one base class keeps loaded objects, and `Order::findAll()` returns only orders. The
attack: a call on one subclass tries to get an object of a sibling subclass typed as itself, through a
narrowed return, a `static` local, an `inout` pass and an anonymous function, and every attempt fails
to compile or returns `null`. The bench measures a finder's `is static` filter over many loaded
objects and states its growth, linear in them. about.md and details.md say what changed, in AGENTS.md
§ *Text an end user reads*.

**Website.** One `website/src/data/changes.json` entry of kind `fixed` for `is static`, and one of
kind `changed` for the new returns, both on `lang:types/void-never-self-static`. The reference
`docs/reference/lang/50-classes.md:647` and the E0741 listing at `:846` say the same as the rule.

**What stops it.** One attempt per item and two sessions per stage. An item that cannot land in one
attempt is written to `### Backlog` with what blocked it.

**Lanes.** It runs on the chain after `test-only`. No other goal edits `type_test.rs` or `locals.rs`.

## Stages

### 1. $o is static tests the class the call named, and is refused inside an anonymous function

- [ ] `crates/nvs-types/src/expr/type_test.rs:113`: in a class that is not `final`, a tested type that
  is `static` or `?static` skips both folds and records the called-class test (§ *Stage 1*). The
  module doc (`:1`) is rewritten whole to name it.
- [ ] `crates/nvs-ir/src/lower/expr.rs:7804`: `lower_type_test` lowers that record as a `ClassTest`
  against `self.lsb()`, and the doc above it names the fourth shape.
- [ ] `crates/nvs-types/src/expr/mod.rs:1488`: `is static` inside an anonymous function is E0834 with
  its own wording, and `crates/nvs-diagnostics/src/code/types_e08.rs:600`'s card names the type position.
- [ ] `crates/nvs-types/tests/narrowed_static.rs` (new): the three tests of the check
  `called-class-test`, then `cargo test --test narrowed_static` once.
- [ ] `tests/conformance/lang/is-static-tests-the-class-the-call-names.nvst` (new): a static and an
  instance method, called on the class itself and on a sibling, printing `false` for the sibling; then
  the check `called-class-run` once.

### 2. a value narrowed by is static is on E0741's list

- [ ] `crates/nvs-types/src/locals.rs:535`: `narrow` installs the called-class fact on the true edge of
  a recorded called-class test, and every path that drops or restores a narrowing drops or restores
  it. The module doc's narrowing section (`:37`) says so.
- [ ] `crates/nvs-types/src/check.rs:1763`: `yields_the_called_class` accepts a variable or `readonly`
  path carrying the fact; the list in `reject_static_return_of_another_class`'s doc (`:1676`) and
  E0741's help text name it.
- [ ] The four tests of the check `narrowed`, then `cargo test --test narrowed_static` once.

### 3. a local declared with a type that binds static is checked at every write, and is on E0741's list

- [ ] `crates/nvs-types/src/locals.rs:222`: a record of every local whose declared type binds `static`,
  beside `static_callbacks`, made where a typed declaration is checked.
- [ ] `crates/nvs-diagnostics/src/code/types_e10.rs`: `E1020` with its card.
- [ ] `crates/nvs-types/src/expr/assign.rs:1133` and `:1434`: every write of § *Stage 3* to such a
  local is held to `yields_the_called_class`, and an `inout` pass of it is `E1020`
  (`crates/nvs-types/src/expr/calls.rs`; the session finds the `inout` argument check by name).
- [ ] `crates/nvs-types/src/check.rs:1763`: a read of such a local is on the list.
- [ ] The five tests of the check `static-locals`, then `cargo test --test narrowed_static` once.
- [ ] `tests/conformance/lang/a-finder-returns-loaded-objects-as-static.nvst` (new): the case under
  § *The finder case*, then the check `finders-run` once.

### 4. the rule, the reference, the examples, the feature proofs and the website say so

**Split:** the example, the attack and the bench go to one writing subagent once the rule fragment is
written. Its brief names AGENTS.md § *Text an end user reads* and § *Feature proofs* above.

- [ ] `docs/rules/statements/static-is-a-member-modifier.md` and `docs/rules/types/type-test.md`: per
  § *Rules*, then `bun nv rules`.
- [ ] `docs/reference/lang/50-classes.md:647` and `:846`.
- [ ] The feature proofs `bun nv proofs --owed --goal narrowed-static` names: about.md, details.md,
  the example, the bench, the attack, the Novis and Rust tests and the help text.
- [ ] `website/src/data/changes.json:1`: the two entries, then `bun nv site --check stale`, `changes`
  and `review` green.
- [ ] `bun nv proofs --owed --goal narrowed-static` names nothing.

### 5. the goal is verified once, and every failure is fixed

- [ ] Run the check the pack names for this stage: `looper verify` at a full gate, or a build and the tests this goal wrote, by name, at a close.
- [ ] Only if it is red: fix every failure it names in one pass, never deleting or loosening a test, then run it once more, and it is green.

## The finder case

The conformance case of stage 3, which today fails `nvs check` with E0741 at both returns:

```nvs
<?nvs
class Model
{
    private static array<Model> $loaded = [];

    public static function remember(Model $object): void
    {
        self::$loaded[] = $object;
    }

    public static function findFirst(): ?static
    {
        foreach (self::$loaded as Model $object) {
            if ($object is static) {
                return $object;
            }
        }
        return null;
    }

    public static function findAll(): array<static>
    {
        array<static> $found = [];
        foreach (self::$loaded as Model $object) {
            if ($object is static) {
                $found[] = $object;
            }
        }
        return $found;
    }
}

final class Order extends Model
{
}

final class Customer extends Model
{
}

Model::remember(new Customer());
Model::remember(new Order());
?Order $order = Order::findFirst();
array<Order> $orders = Order::findAll();
echo $order == null ? 'none' : 'order', ' ', Core\Arr::count($orders), "\n";
echo Core\Arr::count(Customer::findAll()), "\n";
```

```output
order 1
1
```

## Handoff

### State

Written by an interactive session on 2026-11-26. The user made the three calls under § *Standing
decisions*; every other call there is the session's. Nothing is built. Every anchor was read on that
day, while `mail-complete` ran. The bug of stage 1 was reproduced on that day's release binary: a
`Model` method returning `$o is static` printed `true` for a sibling class, in a static and in an
instance method, and `fn(): bool => $o is static` compiled.

### Next

Stage 1 from its first item. The check `called-class-test` is red until
`crates/nvs-types/tests/narrowed_static.rs` exists with its three tests.

### Backlog
