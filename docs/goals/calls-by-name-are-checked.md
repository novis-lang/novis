---
looper: 1
title: a call by name checks what a written call checks
summary: "Reached when a call through reflection, a mixed receiver or a hooked property write checks each argument against its declared type as a property write does, refuses a static method and a by-reference parameter, reaches a private member only from the class that declares it, and allocates nothing when it succeeds."
context: [crates/nvs-runtime/src/dispatch.rs, docs/rules/types/erased-member-access.md]
stages:
  - id: 1
    title: "a slot check tests what `is` tests, and the entries are listed"
    status: done
    context: [crates/nvs-runtime/src/object.rs, crates/nvs-ir/src/ir.rs, crates/nvs-ir/src/lower/mod.rs, crates/nvs-ir/src/lower/expr.rs, crates/nvs-codegen/src/lib.rs, docs/rules/enums/representation.md, tests/hostile/README.md]
  - id: 2
    title: "a method row carries each parameter's type, and a call by name checks it"
    status: open
    context: [crates/nvs-runtime/src/object.rs, crates/nvs-runtime/src/callable.rs, crates/nvs-ir/src/lower/mod.rs, crates/nvs-codegen/src/lib.rs, crates/nvs-stdlib/src/reflect.rs, tests/hostile/README.md]
  - id: 3
    title: "the hook write and a bare callable run the same check, and a test lists the entries"
    status: open
    context: [crates/nvs-runtime/src/object.rs, crates/nvs-runtime/src/callable.rs, crates/nvs-ir/src/lower/anon_fn.rs, docs/rules/types/callable-signature.md, docs/rules/classes/property-hooks.md]
  - id: 4
    title: a private member is reached by name only from the class that declares it
    status: open
    context: [crates/nvs-runtime/src/ctx/visibility.rs, crates/nvs-types/src/layout.rs, crates/nvs-stdlib/src/reflect.rs, docs/rules/security/reflection-enforces-visibility.md]
  - id: 5
    title: a successful call by name allocates nothing
    status: open
    context: [crates/nvs-runtime/src/callable.rs, crates/nvs-stdlib/src/reflect.rs, crates/nvs-stdlib/tests/allocation_policy.rs]
  - id: 6
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: slot-check-arms
    stage: 1
    name: "an enum slot admits its own cases alone, and a slot of every type kind refuses what an is test refuses"
    run: [cargo, test, --lib, --, an_enum_slot_admits_its_own_cases_alone]
    timeout: 1800
    tests:
      - an_enum_slot_admits_its_own_cases_alone
  - id: slot-check-covers-the-type-kinds
    stage: 1
    name: "a compiled class's slot check refuses each value an is test of the same type answers false for"
    run: [cargo, test, --test, objects, --, a_slot_check_refuses_what_an_is_test_refuses]
    timeout: 1800
    tests:
      - a_slot_check_refuses_what_an_is_test_refuses
  - id: method-rows-carry-types
    stage: 2
    name: "a compiled method row carries each parameter's check, the static bit and the by-reference bit"
    run: [cargo, test, --test, objects, --, a_method_row_carries_each_parameters_check_and_its_two_bits]
    timeout: 1800
    tests:
      - a_method_row_carries_each_parameters_check_and_its_two_bits
  - id: call-by-name-checks-arguments
    stage: 2
    name: "a call by name refuses an argument its parameter does not admit, a static method and a by-reference parameter, each with a throw"
    run: [cargo, test, --lib, --, a_call_by_name_refuses_an_argument_its_parameter_does_not_admit, a_static_method_is_refused_by_name, a_by_reference_parameter_is_refused_by_name, an_isolate_entry_refuses_an_argument_its_parameter_does_not_admit, construct_checks_its_arguments_as_call_does]
    timeout: 1800
    tests:
      - a_call_by_name_refuses_an_argument_its_parameter_does_not_admit
      - a_static_method_is_refused_by_name
      - a_by_reference_parameter_is_refused_by_name
      - an_isolate_entry_refuses_an_argument_its_parameter_does_not_admit
      - construct_checks_its_arguments_as_call_does
  - id: hook-and-callable-entries
    stage: 3
    name: "a hooked property write is checked before its hook runs, and a bare callable call checks what its parameter declares"
    run: [cargo, test, --lib, --, a_hooked_property_write_is_checked_before_its_hook_runs, a_bare_callable_call_checks_what_its_parameter_declares]
    timeout: 1800
    tests:
      - a_hooked_property_write_is_checked_before_its_hook_runs
      - a_bare_callable_call_checks_what_its_parameter_declares
  - id: every-entry-has-a-driver
    stage: 3
    name: "every function that passes values to compiled code is driven with wrong-typed arguments or names who typed them"
    run: [cargo, test, --test, erased_entries]
    timeout: 1800
    tests:
      - every_entry_into_compiled_code_has_a_driver
  - id: private-from-the-declaring-class
    stage: 4
    name: "a private property, method and constant is reached by name from the class that declares it and from no subclass"
    run: [cargo, test, --lib, --, a_private_member_is_refused_to_a_subclass_by_name, a_class_reaches_its_private_member_on_an_instance_of_a_subclass, a_private_method_declared_again_belongs_to_the_class_that_declared_it]
    timeout: 1800
    tests:
      - a_private_member_is_refused_to_a_subclass_by_name
      - a_class_reaches_its_private_member_on_an_instance_of_a_subclass
      - a_private_method_declared_again_belongs_to_the_class_that_declared_it
  - id: call-by-name-allocates-nothing
    stage: 5
    name: "a successful call by name counts zero allocations, for few calls and many, with one argument and with sixteen"
    run: [cargo, test, --test, allocation_policy, --, a_successful_call_by_name_allocates_nothing]
    timeout: 1800
    tests:
      - a_successful_call_by_name_allocates_nothing
---

# a call by name checks what a written call checks

## Why

**A call whose callee is found at run time checks each argument's representation and nothing else.**
`Core\Reflect\ClassInfo::call`, `ClassInfo::construct` and `$mixed->method(...)` all end in
`call_erased_unchecked` (`crates/nvs-runtime/src/dispatch.rs:363`). Its one judgment of the arguments
is `check_param_tags` (`crates/nvs-runtime/src/dispatch.rs:462`,
`crates/nvs-runtime/src/callable.rs:902`). That compares one nibble per parameter
(`param_tag_nibble`, `crates/nvs-ir/src/lower/mod.rs:5429`). A nibble names one of twelve
representations. It cannot name a class, an enum or what an array holds, and a `?T` parameter is
"any". The compiled method then reads its parameter at the declared type with no test. A method row
(`MethodRow`, `crates/nvs-runtime/src/object.rs:699`) holds nothing else to test against.

A property write through the same reflection has the whole check. `ClassDesc::check_slot`
(`crates/nvs-runtime/src/object.rs:1683`) tests the tag, the admitted classes and the rest of the
type. Commit `5f9c423e11` gave the decoder that check and left the call on the nibble. Four shapes
were run on a release build, and each is a wrong-type read in native code:

- An object of another class where a class is declared. A read of a `string` property ended the
  process with a segmentation fault.
- An object of another class where `?C` is declared.
- An array of the wrong element type where `array<T>` is declared. The loop read the raw payloads.
- A case of another enum where an enum is declared. The body held a value the enum has no case for.

Reflection needs no grant, so a package with an empty grant line reaches all four.

**Three more entries have the same gap.** `call_static_bound`
(`crates/nvs-runtime/src/dispatch.rs:215`) is an isolate's entry and takes its arguments from a
program's map. It runs the same nibble check (`crates/nvs-runtime/src/dispatch.rs:253`). An
anonymous function tests a class at its entry only where the parameter is a plain class name
(`callable_param_ty`, `crates/nvs-ir/src/lower/anon_fn.rs:1200`, and `checked_class`,
`crates/nvs-ir/src/lower/anon_fn.rs:1232`). A `?C`, an `array<T>` and an enum parameter get the
nibble alone, and `rule:types/callable-signature` says so. The reviewer saw a bare `callable` take a
wrong class for `?C` and a wrong element for `array<int>` once on a release build. The code confirms
the missing test. The run was not repeated for this goal.

**A reflective `set` on a property with a `set` hook goes to the hook when the slot check fails.**
`write_property` calls `check_slot` and uses its failure only for the `covariant` case
(`crates/nvs-runtime/src/object.rs:5698`). Any other refused value goes on to the hook, judged by the
hook's nibble (`crates/nvs-runtime/src/object.rs:5710`). The same write to a property with no hook is
refused (`crates/nvs-runtime/src/object.rs:5753`).

**The property check itself is weaker than an `is` test for four type kinds.** The builder
`slot_check` (`crates/nvs-ir/src/lower/mod.rs:602`) gives an enum its tag alone. It gives a
`callable`, a written callable signature and a `Core` class the object tag alone
(`crates/nvs-ir/src/lower/mod.rs:609`, `crates/nvs-ir/src/lower/mod.rs:637`). `test_shape`
(`crates/nvs-ir/src/lower/expr.rs:7881`) tests all four in full for `$x is T`. This was read in the
code and not run. A call through a written signature skips the tag check
(`crates/nvs-runtime/src/callable.rs:401`), so a callable of another signature stored by name is the
same wrong-type read.

**The entry runs a `static` method with the object in the class slot.** The method table lists
static methods beside instance ones. A static method's slot 0 is the called class
(`crates/nvs-ir/src/lower/mod.rs:1493`). The call by name puts the receiver object there
(`crates/nvs-runtime/src/dispatch.rs:463`). A run ended with a segmentation fault.

**The entry admits `null` for an `inout` parameter.** An `inout` parameter is lowered as `Ty::Ref`
(`crates/nvs-ir/src/lower/mod.rs:1537`), whose nibble is the nibble of `null`
(`crates/nvs-ir/src/lower/mod.rs:5434`). So every real value is refused and `null` passes. The
method writes through it. A run ended with a segmentation fault. `MethodRow` has no bit for either
case.

**Code in a subclass reaches its parent's `private` members by name.** `Ctx::reaches` returns `true`
when the site is the object's own class (`crates/nvs-runtime/src/ctx/visibility.rs:117`). A
descriptor is flattened, so it holds the parent's private slots and rows. Inside a subclass, a
reflective `get`, `set` and `call` of the parent's private member all ran.
`rule:security/reflection-enforces-visibility` names this case as refused. The same test refuses a
class's own body on an instance of a subclass, which ordinary code reaches.

**A successful call by name makes three heap allocations that buy nothing.** They are the vector of
entries in `ClassInfo::call` (`crates/nvs-stdlib/src/reflect.rs:3066`), the callee's name
(`crates/nvs-runtime/src/dispatch.rs:424`) and a copy of the arguments
(`crates/nvs-runtime/src/dispatch.rs:461`). The name is read only in a refusal. `$mixed->method()`
pays the last two on every call. A hooked write formats a name the same way
(`crates/nvs-runtime/src/object.rs:5705`).

## Standing decisions

**One check, at every entry where a value with no static type meets a typed parameter.** A method
row gets, per parameter, what `ClassDesc` holds per property slot: the admitted classes and the rest
of the type as a `SlotCheck`. The builder is the one that fills `field_classes` and `field_checks`
(`field_slots`, `crates/nvs-ir/src/lower/mod.rs:530`), given the declared parameter types
`lower_method` already reads (`crates/nvs-ir/src/lower/mod.rs:1503`). The lists travel on
`ir::Function` (`crates/nvs-ir/src/ir.rs:559`) to `MethodShape`
(`crates/nvs-codegen/src/lib.rs:1496`), which fills the row on both paths
(`crates/nvs-codegen/src/lib.rs:1271`, `crates/nvs-codegen/src/lib.rs:2683`). One runtime function
runs the three steps `check_slot` runs, and `check_slot` calls it too. `check_param_tags` stays as
the tag step, with its one `int` to `float` widening for a parameter declared `float`. Nothing widens
inside a union or a `?float`, as a property write does not. **What it spends:** two small vectors per
method per class, once per process. On a call by name, one class walk per class-typed argument and
one element walk per typed-array argument. A statically typed call pays nothing. Safe default: a
type the builder cannot express stays `SlotCheck::Any`, and the stage's handoff names it. Declined:
a compiled thunk per method, which `rule:types/erased-member-access` already declines.

**A variadic parameter checks its elements, and a `covariant` one is left to its entry test.** A
call by name passes one array for `...$rest`, so its row check is an array of the declared element
type. A `covariant` parameter is read at `Ty::Tagged` and tested when the method is entered, on
every call (`crates/nvs-ir/src/lower/mod.rs:1583`). Its row holds no check, so the refusal is that
test's own `LogicError`.

**The slot check tests what `$x is T` tests, for the four kinds where it tests less.** `SlotCheck`
gains one arm: a tag and a sorted list of the payload values it admits. The builder gives an enum
its case values (`test_shape` does, `crates/nvs-ir/src/lower/expr.rs:8001`, from
`sorted_enum_cases`, `crates/nvs-ir/src/lower/convert.rs:2388`). It gives an enum case and a
single-value type their one value. It gives `callable` the class arm against the callable marker and
a written signature the class arm against its marker (`crates/nvs-ir/src/lower/expr.rs:7955`,
`crates/nvs-ir/src/lower/expr.rs:7967`). It gives a `Core` class with instances the class arm
against its own name, by the roster the anonymous function's entry uses (`testable_core_class`,
`crates/nvs-types/src/expr/members.rs:465`). The builder is shared, so the property door and the
parameter door change together. **What it spends:** one sorted list of case values per enum-typed
slot or parameter, once per process, and one binary search per enum value written by name. Safe
default: an arm that would refuse a value the checker accepts falls back to today's arm, and the
handoff names the type. Declined: replacing `slot_check` with `test_shape` whole. The two differ on
a shape's hidden fields and on a class with no descriptor, and that is a goal of its own.

**An enum slot and an enum parameter compare the enum's case set, which is all the identity a value
has.** A case in a tagged value is its backing integer under a tag that names no enum
(`rule:enums/representation`). So the check is the enum tag and one of that enum's case values, on
both doors. A case of another enum is refused when this enum has no case of that value. When it has
one, the value is admitted and is this enum's case from then on, as `$any as Rank` converts an
integer. Declined: the enum's identity in the value's padding bytes. `rule:enums/representation`
declines it, and it costs a store on every materialization.

**The entries are these, and a test finds them in the code.** Checked by this goal:
`call_erased_unchecked`, which serves `$mixed->method(...)`, `ClassInfo::call` and
`ClassInfo::construct`; `call_static_bound`; the hook branch of `write_property`; and
`call_callable_with` where the call site proved nothing
(`crates/nvs-runtime/src/callable.rs:251`). The test
reads every shipped file under `crates/*/src` for a `transmute` to `NvsFn` and for a call of
`call_at`. It fails naming a function it has no driver for. A driver passes wrong-typed arguments
through that door and expects a throw, or names the checked caller or compiler pass that typed them.
Safe default: an entry the session cannot prove typed gets the check. Stage 1 read every other
entry, and each one's arguments are typed by:

- `call_method` (`crates/nvs-runtime/src/dispatch.rs:103`): the `Core` caller, against an interface
  member it names: `compareTo`, `toString`, `encode`, the router's gate and responder members, the
  test runner's no-argument tests, and a `Core\Test` partial passing its double's typed arguments on.
- `call_static` and `call_static_on` (`crates/nvs-runtime/src/dispatch.rs:128`,
  `crates/nvs-runtime/src/dispatch.rs:170`): the `Core` caller. A command handler gets values its
  parser typed, `parse` one string, and `fromRow` and a JSON `decode` one value the codec checked.
- `construct` (`crates/nvs-runtime/src/object.rs:4888`): the codec's field check, for a JSON or
  database row's constructor arguments. The test runner passes none.
- The `get` hook in `read_erased_property_hinted` (`crates/nvs-runtime/src/object.rs:5706`) and
  `call_member` (`crates/nvs-runtime/src/sequence.rs:459`): no arguments.
- `call_render` and `call_compare_to` (`crates/nvs-runtime/src/dispatch.rs:559`,
  `crates/nvs-runtime/src/dispatch.rs:605`): a `Core` member's own address, with the receiver.
- `call_unwind` (`crates/nvs-runtime/src/dispatch.rs:650`): the receiver alone.
- `raw_function` (`crates/nvs-codegen/src/lib.rs:710`) and `script`
  (`crates/nvs-cli/src/cache.rs:1133`): a script frame with no arguments. `Unit::call_static` is a
  test and embedder door whose caller typed its arguments.

**A failed check throws an error the program can catch. Never a `FATAL`.** A refused argument throws
`LogicError`, the class the tag step throws today (`crates/nvs-runtime/src/callable.rs:957`) and the
anonymous function's entry throws for a wrong class. One parameter then has one error class. A
refused property write keeps the `RuntimeError` it throws today
(`crates/nvs-runtime/src/object.rs:5757`). The argument's sentence names the callee, the position,
the declared type as the row spells it (`MethodRow::param_types`,
`crates/nvs-runtime/src/object.rs:750`) and what was given.

**A `static` method and a method with a by-reference parameter are refused through a call by name.**
Two bits on `MethodRow`. `MethodShape::of` reads both off the function it already holds: parameter 0
is `Ty::ClassDesc` for a static method, and an `inout` parameter is `Ty::Ref`
(`crates/nvs-codegen/src/lib.rs:1507`). `call_erased_unchecked` refuses either row where it refuses a
native row (`crates/nvs-runtime/src/dispatch.rs:435`), with a sentence that names the reason.
`call_static_bound` refuses a by-reference row and a row that is not static. **What it spends:** two
bytes per method per class. Declined: calling a static method by name with the right class value in
the class slot. Refusing is the reversible choice.

**The hooked-property branch calls `check_slot` first, and the hook row carries no check.** What
stays in the branch: the visibility test before it, the `covariant` sentence when `check_slot`
refuses and `ancestor_admits` holds, the guard, the hook call with the value unchanged, and the
observer. What changes: any other `check_slot` refusal throws the sentence the unhooked branch throws,
and the two branches share those lines. What goes: the hook's tag check, its formatted name and
`HookRow::param_tags` with both fills (`crates/nvs-codegen/src/lib.rs:1304`,
`crates/nvs-codegen/src/lib.rs:2715`). A `set` hook's parameter accepts the property's type or the
hook does not compile, and every hooked property has a slot (`rule:classes/property-hooks`). So a
value that passed `check_slot` is one the hook reads correctly, and a second check could only agree.
An `int` written by name to a hooked `float` property is refused, as it is for a property with no
hook. Today the hook's tag check widens it. Safe default: if a hook is reachable with no slot, the
hook row keeps a check as a method row does. Declined: per-parameter vectors on `HookRow`.

**A bare `callable` call checks from the callable class's `invoke` row.** An anonymous function's
class and a method reference's thunk class each have an `invoke` method row
(`crates/nvs-ir/src/lower/anon_fn.rs:358`). That row gets the parameter checks a method row gets, from
the literal's own parameter types (`crates/nvs-ir/src/lower/anon_fn.rs:233`,
`crates/nvs-ir/src/lower/anon_fn.rs:749`). `call_callable_with` runs the shared check where it runs
the tag check today, on a call site that proved nothing. A call through a written signature still
pays nothing. `check_param_class` stays in the body. **What it spends:** the same two vectors per
callable class, once per process, and nothing per callable value. Declined: a compiled element walk
at the body's entry, which every proven call would pay. Declined: removing `check_param_class` in
this goal. The Backlog names it.

**A `private` member is reached by name only from the class that declares it.** `reaches` keeps its
`protected` answer. For a `private` member the site must be the object's class or an ancestor of it,
the site's descriptor must have that very member, and no class the site conforms to may have it.
"That very member" is the same slot for a property, the same code address for a method, and the same
name for a constant. So a site in a subclass is refused, and a class's own body reaches its private
member on an instance of a subclass. A method a subclass declares again has its own code address, so
it belongs to the subclass. A property keeps the visibility of the topmost class that declares it,
which is the layout's rule (`crates/nvs-types/src/layout.rs:1004`). A `private` constant a subclass
declares again over its parent's is refused to the subclass by name. That is the refusing direction,
and the rule fragment says so. **What it spends:** one more descriptor lookup on a non-public access,
and nothing new on a descriptor. Safe default: a site whose descriptor the table does not hold is
refused. Declined: a declaring-class label on every row and slot, until a program needs the constant
case.

**A successful call by name allocates nothing.** The refusal text is formatted on the refusing path
only. The checked arguments sit in a stack array of sixteen values, the bound the tag step already
refuses past (`crates/nvs-runtime/src/callable.rs:146`). `call_callable_with` holds its slots the
same way (`crates/nvs-runtime/src/callable.rs:201`). The reflective members hand the `array<mixed>`
to the runtime, which fills that array from its first live entries. **What it spends:** 256 bytes of
stack per call by name, and no heap. A count guard proves it with the allocation counter
`crates/nvs-stdlib/tests/allocation_policy.rs:142` uses. It asserts no wall-clock time.

**Which classes `construct` may build is not this goal's.** `crossings-carry-data-only` owns that
predicate. This goal only makes `construct`'s arguments pass the argument check, which they do
through `call_erased_unchecked` (`crates/nvs-runtime/src/dispatch.rs:518`).

**Lanes:** one lane. Stages 1, 2 and 3 change `object.rs`, `lower/mod.rs` and `codegen/src/lib.rs`
in turn. Stage 5 rewrites the entry stage 2 changed. Stage 4 touches `visibility.rs`, its own tests
and one rule fragment, so it can run as a side goal beside stage 2 or 3.

## Stages

### 1. a slot check tests what `is` tests, and the entries are listed

**Does:** give the slot check the arms an `is` test has and it lacks, on the builder both doors
share, and list every entry into compiled code before one is changed.

**Split:** the entry list goes to an `Explore` subagent. The `.nvst` case and the attack go to one
writing subagent once the runtime arm is on disk. It does not build.

- [x] An `Explore` subagent lists every function under `crates/*/src` that holds a `transmute` to
  `NvsFn` or calls `call_at`, as `file.rs:NN` and the function's name. The session reads each caller
  it does not know and writes one line per entry into the handoff: who typed its arguments
  (`rule:types/erased-member-access`, `crates/nvs-runtime/src/dispatch.rs:1430`,
  `crates/nvs-runtime/src/callable.rs:228`).
- [x] `SlotCheck` gains the arm for a tag and a sorted list of admitted payload values, in both
  crates, with its `admits` arm and its conversion (`rule:enums/representation`,
  `crates/nvs-ir/src/ir.rs:542`, `crates/nvs-runtime/src/object.rs:5932`,
  `crates/nvs-runtime/src/object.rs:5969`, `crates/nvs-codegen/src/lib.rs:1955`).
- [x] `slot_check` gives an enum its case values, an enum case and a single-value type their value,
  `callable` and a written signature their marker class, and a `Core` class with instances its own
  name. `field_slots` takes the enum table. `deep_check` keeps every arm the tag and the admitted
  classes do not already cover (`rule:types/erased-member-access`,
  `crates/nvs-ir/src/lower/mod.rs:602`, `crates/nvs-ir/src/lower/mod.rs:530`,
  `crates/nvs-ir/src/lower/mod.rs:581`, `crates/nvs-ir/src/lower/expr.rs:8001`).
- [x] Test `an_enum_slot_admits_its_own_cases_alone` in `object.rs` tests: a case value passes, a
  value no case has is refused, and an `int` of a case's value is refused by the tag
  (`crates/nvs-runtime/src/object.rs:6266`).
- [x] Test `a_slot_check_refuses_what_an_is_test_refuses` in the codegen tests. One compiled class
  has a property of each type kind `test_shape` has an arm for. For each, `check_slot` refuses a
  value `is` answers `false` for and admits one it answers `true` for
  (`crates/nvs-codegen/tests/objects.rs:427`, `crates/nvs-ir/src/lower/expr.rs:7881`).
- [x] `.nvst` case: a reflective `set` of an enum property, a callable-signature property and a
  `Core`-class property. A case value the enum lacks, a callable of another signature and an object
  of another class each throw. A value that fits is stored
  (`tests/conformance/core/reflect-set-checks-an-enum-a-signature-and-a-core-class.nvst:1`).
- [x] Attack `02-values-of-another-enum-signature-and-class.nvs`. Each step writes a value of the
  wrong kind by name and then uses the property. Every write should throw
  (`rule:testing/feature-proofs`,
  `tests/hostile/core/Reflect-ClassInfo/set/02-values-of-another-enum-signature-and-class.nvs:1`).
- [x] `rule:enums/representation` says a declared slot's write check and a parameter's check by name
  test the enum's case set, and what a case of another enum with a shared value becomes.
  `rule:types/erased-member-access` says the write check tests what `is` tests for an enum, a
  callable and a `Core` class. The module doc of `object.rs` follows
  (`docs/rules/enums/representation.md:6`, `docs/rules/types/erased-member-access.md:13`,
  `crates/nvs-runtime/src/object.rs:156`). The parameter half waits for stage 2, which lands it.

### 2. a method row carries each parameter's type, and a call by name checks it

**Does:** carry each parameter's admitted classes and check to the method row, with the static bit
and the by-reference bit, and run the one check in `call_erased_unchecked` and `call_static_bound`.

**Split:** an `Explore` subagent lists every `MethodRow {` literal in the tree, about fifty in test
modules. One writing subagent adds the new fields to them after the struct change. The `.nvst` cases
and the attacks go to a second writing subagent once the check is on disk. Neither builds.

- [ ] `ir::Function` carries, per declared parameter, the admitted classes and the check.
  `lower_method` fills them with stage 1's builder: a variadic as an array of its element type, and
  none for a `covariant` or an `inout` parameter (`rule:types/erased-member-access`,
  `crates/nvs-ir/src/ir.rs:559`, `crates/nvs-ir/src/lower/mod.rs:1503`).
- [ ] `MethodShape` carries the two lists, a static bit read off parameter 0 and a by-reference bit
  read off the parameter list. `MethodRow` gains the four fields, filled on both paths
  (`crates/nvs-codegen/src/lib.rs:1496`, `crates/nvs-codegen/src/lib.rs:1271`,
  `crates/nvs-codegen/src/lib.rs:2683`, `crates/nvs-runtime/src/object.rs:699`).
- [ ] Every other `MethodRow {` literal gains the fields, from the subagent's list.
- [ ] One runtime function runs the tag step, the class step and the rest-of-type step for a slot or
  a parameter. `check_slot` calls it, and its sentences stay as they are
  (`crates/nvs-runtime/src/object.rs:1683`, `crates/nvs-runtime/src/object.rs:5893`,
  `crates/nvs-runtime/src/callable.rs:902`).
- [ ] `call_erased_unchecked` runs it over the trimmed arguments, and refuses a static row and a
  by-reference row beside the native row. Its doc comment's paragraph on variadic and `inout` is
  rewritten (`rule:types/erased-member-access`, `crates/nvs-runtime/src/dispatch.rs:435`,
  `crates/nvs-runtime/src/dispatch.rs:461`, `crates/nvs-runtime/src/dispatch.rs:299`).
- [ ] `call_static_bound` runs it, and refuses a by-reference row and a row that is not static
  (`rule:security/isolate-shares-nothing`, `crates/nvs-runtime/src/dispatch.rs:236`).
- [ ] Test `a_method_row_carries_each_parameters_check_and_its_two_bits` in the codegen tests
  (`crates/nvs-codegen/tests/objects.rs:427`).
- [ ] Tests in `object.rs` tests, beside the method-row tests, since `dispatch.rs` has no test
  module. `a_call_by_name_refuses_an_argument_its_parameter_does_not_admit` drives another class,
  another class for `?C`, a wrong element, a wrong variadic element and a value no case has, each
  as a `LogicError`. `a_static_method_is_refused_by_name`,
  `a_by_reference_parameter_is_refused_by_name`,
  `an_isolate_entry_refuses_an_argument_its_parameter_does_not_admit` and
  `construct_checks_its_arguments_as_call_does` follow (`crates/nvs-runtime/src/object.rs:6536`).
- [ ] `.nvst` cases, one per door: `ClassInfo::call`, `ClassInfo::construct`, a `mixed` receiver and
  a `spawn script` entry. Each passes the five wrong values above and prints the error's class. The
  `call` case also names a static method and a method with an `inout` parameter.
- [ ] Attacks: `call/02`, `construct/02` under `tests/hostile/core/Reflect-ClassInfo/`, and
  `tests/hostile/lang/types/mixed/03`. Each passes a wrong value and then makes the method use it.
  Every step should throw (`rule:testing/feature-proofs`).
- [ ] `rule:types/erased-member-access` says a call through `mixed` is checked against each
  parameter's declared type, lists the two new refusals and states the cost.
  `rule:enums/representation` says a parameter's check by name tests the enum's case values, as
  the slot's write check already does (`docs/rules/enums/representation.md:8`). The comment on
  `param_tag_nibble` says the row check covers what a nibble cannot
  (`docs/rules/types/erased-member-access.md:56`, `crates/nvs-ir/src/lower/mod.rs:5406`).
- [ ] The `call` and `construct` cards and their `about.md` say, in one plain sentence each, that an
  argument of the wrong type throws a `LogicError`, and that `call` does not run a `static` method
  (`crates/nvs-stdlib/src/reflect.rs:1003`, `docs/examples/core/Reflect-ClassInfo/call/about.md:1`,
  `docs/examples/core/Reflect-ClassInfo/construct/about.md:1`).

### 3. the hook write and a bare callable run the same check, and a test lists the entries

**Does:** put `check_slot` first in the hooked branch, check a bare `callable` call from its `invoke`
row, and pin the whole entry list with a test that reads the code.

**Split:** the `.nvst` cases and the two attacks go to one writing subagent once both checks are on
disk. It does not build.

- [ ] The hooked branch of `write_property` calls `check_slot` first and shares the unhooked branch's
  refusal lines. The hook's tag check and its formatted name go
  (`rule:classes/property-hooks`, `crates/nvs-runtime/src/object.rs:5695`,
  `crates/nvs-runtime/src/object.rs:5753`).
- [ ] `HookRow::param_tags` goes with both fills and the test that reads it
  (`crates/nvs-runtime/src/object.rs:799`, `crates/nvs-codegen/src/lib.rs:1300`,
  `crates/nvs-codegen/src/lib.rs:2711`, `crates/nvs-codegen/tests/property_hooks.rs:107`).
- [ ] The `invoke` row of an anonymous function's class and of a method reference's thunk class
  carries the parameter checks (`rule:types/callable-signature`,
  `crates/nvs-ir/src/lower/anon_fn.rs:233`, `crates/nvs-ir/src/lower/anon_fn.rs:749`).
- [ ] `call_callable_with` runs the shared check from that row where it runs the tag check
  (`crates/nvs-runtime/src/callable.rs:251`).
- [ ] Tests `a_hooked_property_write_is_checked_before_its_hook_runs` and
  `a_bare_callable_call_checks_what_its_parameter_declares`.
- [ ] Test `every_entry_into_compiled_code_has_a_driver` in a new integration test. It reads the
  shipped files under `crates/*/src`, and its drivers come from stage 1's list
  (`crates/nvs-stdlib/tests/erased_entries.rs:1`; the scan's shape is
  `crates/nvs-stdlib/src/cache.rs:3119`).
- [ ] `.nvst` cases: a hooked property written by name with each wrong value, and a bare `callable`
  called with each. Two existing cases pin today's answer and are rewritten:
  `tests/conformance/core/arr-a-mixed-or-nullable-callback-parameter-is-unchecked.nvst` and
  `tests/conformance/core/arr-a-callback-enum-parameter-checks-the-enum-tag.nvst`.
- [ ] Attacks: `set/03` under `tests/hostile/core/Reflect-ClassInfo/` for a hooked property, and
  `tests/hostile/lang/types/callable-classes-object-shapes/02` for a bare `callable`.
- [ ] `rule:types/callable-signature` says a bare `callable` call checks each parameter's whole
  declared type, and its paragraph on what the entry check does not do is rewritten.
  `rule:classes/property-hooks` says a write by name is checked before the hook runs
  (`docs/rules/types/callable-signature.md:28`, `docs/rules/classes/property-hooks.md:13`).

### 4. a private member is reached by name only from the class that declares it

**Does:** make `Ctx::reaches` answer a `private` member to its declaring class alone, for a
property, a method and a constant.

- [ ] `reaches` takes "has that very member" for each kind and applies the private rule the standing
  decision states (`rule:security/reflection-enforces-visibility`,
  `crates/nvs-runtime/src/ctx/visibility.rs:107`, `crates/nvs-runtime/src/ctx/visibility.rs:39`,
  `crates/nvs-runtime/src/ctx/visibility.rs:58`, `crates/nvs-runtime/src/ctx/visibility.rs:79`).
- [ ] Tests `a_private_member_is_refused_to_a_subclass_by_name`,
  `a_class_reaches_its_private_member_on_an_instance_of_a_subclass` and
  `a_private_method_declared_again_belongs_to_the_class_that_declared_it`, each over a property, a
  method and a constant where the kind has the case. They sit in `reflect.rs` tests, beside the
  private-method fixture (`crates/nvs-stdlib/src/reflect.rs:3163`).
- [ ] `.nvst` case: inside a subclass, `get`, `set`, `call` and `constant` of the parent's private
  member each throw. Inside the parent, the same four on an instance of the subclass succeed. A
  private constructor inherited from the parent is refused to the subclass through `construct`.
- [ ] Attacks: `get/02`, `set/04`, `call/03` and `constant/03` under
  `tests/hostile/core/Reflect-ClassInfo/`. A subclass tries each private member of its parent.
- [ ] `rule:security/reflection-enforces-visibility` says which class declares a member for this
  test, and names the redeclared constant (`docs/rules/security/reflection-enforces-visibility.md:5`).

### 5. a successful call by name allocates nothing

**Does:** remove the three allocations of a successful call by name, and guard the count.

- [ ] The refusals of `call_erased_unchecked` and of the shared check format the callee's name where
  they are raised. The tag step takes the class and the name, not a formatted string
  (`crates/nvs-runtime/src/dispatch.rs:424`, `crates/nvs-runtime/src/callable.rs:902`).
- [ ] The checked arguments sit in a stack array of sixteen values
  (`crates/nvs-runtime/src/dispatch.rs:461`, `crates/nvs-runtime/src/callable.rs:233`).
- [ ] One runtime door takes the `array<mixed>` and fills that array from its first live entries.
  `ClassInfo::call` and `ClassInfo::construct` call it
  (`crates/nvs-stdlib/src/reflect.rs:3066`, `crates/nvs-stdlib/src/reflect.rs:3137`).
- [ ] Test `a_successful_call_by_name_allocates_nothing`: the allocation counter reads zero over 8
  calls and over 64, with one argument and with sixteen, through `ClassInfo::call` and through a
  `mixed` receiver (`crates/nvs-stdlib/tests/allocation_policy.rs:142`).
- [ ] `rule:types/erased-member-access` states the cost line: no allocation on a call that succeeds,
  and 256 bytes of stack (`docs/rules/types/erased-member-access.md:69`).

### 6. the goal is verified once, and every failure is fixed

- [ ] Run `looper verify` over the whole goal.
- [ ] Fix every failure it names in one pass, never deleting or loosening a test.
- [ ] Run `looper verify` once more, and it is green.

## Handoff

### State

Stage 1 is done. A write by name to a declared slot tests what `is` tests: `SlotCheck::Values`
(a tag and sorted payload values) covers an enum, an enum case, a single-value `int` and `true` or
`false`, and the class arm covers `callable`, a written signature, a `Core` class with instances
and a `Core` interface's implementors (`slot_check`, `crates/nvs-ir/src/lower/mod.rs:640`). A
single-value `string` keeps its tag alone. The checker now makes a marker class for every signature
a declared property type names, not only for a tested one (`resolve`,
`crates/nvs-types/src/callables.rs:65`), and the tested set is sorted by label. Both stage-1 tests
pass by name, and the new `.nvst` case and attack pass on a fresh debug binary. Every entry into
compiled code is listed, with who typed its arguments, under the standing decision *The entries
are these*.

The sweep's `env-secret-proofs` red is stale: `looper check --name env-secret-proofs` answers "no
check matches", and no file in the tree names it.

Stage 2 onward is open: a call by name checks representations only, a hooked write ignores a failed
slot check, a static or `inout` method is reached by name, and a subclass reaches its parent's
`private` members by name.

### Next

Stage 2, a method row carries each parameter's type, and a call by name checks it. Files:
`crates/nvs-ir/src/ir.rs`, `crates/nvs-ir/src/lower/mod.rs`, `crates/nvs-codegen/src/lib.rs`,
`crates/nvs-runtime/src/object.rs`, `crates/nvs-runtime/src/dispatch.rs`,
`crates/nvs-runtime/src/callable.rs`, `crates/nvs-stdlib/src/reflect.rs`. Follow the stage's
`**Split:**` line.

- [ ] `crates/nvs-ir/src/lower/mod.rs:1527`: `lower_method` fills, per declared parameter, the
  admitted classes and the check, through `SlotScope` and `deep_check` (`crates/nvs-ir/src/lower/mod.rs:583`).
  A variadic is an array of its element type; a `covariant` or `inout` parameter has none. The lists
  ride on `ir::Function` (`crates/nvs-ir/src/ir.rs:563`).
- [ ] `crates/nvs-codegen/src/lib.rs:1496`: `MethodShape` carries the lists and the two bits, and
  `MethodRow` (`crates/nvs-runtime/src/object.rs:704`) gains four fields, filled on both paths. An
  `Explore` subagent lists every other `MethodRow {` literal, and a writing subagent fills them.
- [ ] `crates/nvs-runtime/src/object.rs:1699`: one runtime function runs the tag, class and
  rest-of-type steps for a slot or a parameter. `check_slot` calls it with its sentences unchanged,
  and `check_param_tags` (`crates/nvs-runtime/src/callable.rs:918`) stays the tag step.
- [ ] `crates/nvs-runtime/src/dispatch.rs:363`: `call_erased_unchecked` runs it, and so does
  `call_static_bound` (`crates/nvs-runtime/src/dispatch.rs:215`). Each refuses a static row and a
  by-reference row as the stage says.
- [ ] `docs/rules/enums/representation.md:8`: the stage's tests, `.nvst` cases, attacks, rule text
  and cards. The rule item now also owes this rule's parameter sentence.

Tests that may go red: an IR or `--dump-ir` snapshot that lists marker descriptors, since a
signature a property declares now has one and the markers are sorted by label; and
`tests/conformance/lang/a-property-write-through-a-mixed-receiver-checks-the-whole-declared-type.nvst`
or `tests/conformance/core/reflect-set-round-trips-through-get-for-every-visible-property.nvst` if
either writes an enum, a callable or a `Core` object of another kind.

### Backlog

- `check_param_class` runs on every call of an anonymous function, a proven one too. Once stage 3's
  test holds, the row check may make it removable (`crates/nvs-ir/src/lower/anon_fn.rs:454`).
- A `private` constant a subclass declares again is refused to that subclass by name. A
  declaring-class label on the row would make it exact (`crates/nvs-runtime/src/object.rs:1123`).
- `slot_check` and `test_shape` are two maps from a type to a run-time test
  (`crates/nvs-ir/src/lower/mod.rs:602`, `crates/nvs-ir/src/lower/expr.rs:7881`).
- An intersection type and `iterable` stay `SlotCheck::Any`, and a single-value `string` keeps its
  tag alone (`crates/nvs-ir/src/lower/mod.rs:640`).
- `Core\Str::format('%1$s', $o)` over an object whose class implements `Stringable` throws "it does
  not implement `Stringable`" at run time. The rendering in `nvs_stdlib::format` reaches
  `stringify`, which finds no `toString` for a compiled class, and throws at
  `crates/nvs-runtime/src/helpers.rs:2049`. It is a call by name to a compiled method, and the
  lowering already holds every format argument against it (`rule:expressions/an-operand-keeps-its-value-while-later-operands-run`).
  Once it works, `crates/nvs-codegen/tests/borrowed_reads.rs` gains a row that drives a
  `toString` from a rendered format.
