---
looper: 1
title: "secret and tainted survive every narrowing, widening and untyped call"
summary: "Reached when a type test and a conversion keep both qualifiers, mixed carries secret only where it is written, a call with no static parameter list refuses a qualified argument, every array key position checks the qualifier, and one matrix reads the checker's own list of positions that erase a type."
context: [crates/nvs-types/src/expr/quals.rs, docs/rules/security/secret-qualifier.md, docs/rules/security/taint-propagation.md]
stages:
  - id: 1
    title: a type test and a cast keep the qualifier
    status: done
    context: [crates/nvs-types/src/expr/type_test.rs, crates/nvs-types/src/locals.rs, docs/rules/security/secret-propagation.md, docs/rules/types/type-test.md]
  - id: 2
    title: "mixed keeps a secret only where secret mixed is written"
    status: done
    context: [crates/nvs-types/src/expr/assign.rs, crates/nvs-types/src/ty.rs, crates/nvs-types/src/lower.rs, docs/rules/types/unions-and-mixed.md]
  - id: 3
    title: "a call or a write with no static parameter list refuses a qualified value"
    status: done
    context: [crates/nvs-types/src/expr/calls.rs, crates/nvs-stdlib/src/reflect.rs, docs/rules/types/callable-signature.md, docs/rules/types/erased-member-access.md, docs/rules/security/sink-predicate.md]
  - id: 4
    title: "the run time keeps a secret out of an erased read and out of a failed conversion's message"
    status: done
    context: [crates/nvs-runtime/src/helpers.rs, docs/rules/security/secret-sinks-refuse.md, docs/rules/types/conversion.md]
  - id: 5
    title: "every key position checks the qualifier, and the member table says which members make keys"
    status: done
    context: [crates/nvs-types/src/expr/literals.rs, crates/nvs-types/src/expr/args.rs, crates/nvs-stdlib/src/arr.rs, crates/nvs-stdlib/tests/taint_signatures.rs]
  - id: 6
    title: "one matrix tries a secret and a tainted value at every position that erases a type"
    status: done
    context: [tools/nv/cmd/soundness.ts, docs/rules/testing/redeclaration-matrix.md]
  - id: 7
    title: "the tree's own programs launder properly, and the feature has its proofs"
    status: open
    context: [tests/hostile/README.md, docs/examples/README.md, docs/rules/testing/feature-proofs.md, docs/examples/lang/types/qualifiers-tainted-and-secret/about.md]
  - id: 8
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: narrowing-and-conversion-keep-the-qualifier
    stage: 1
    name: "a type test narrows a qualified subject to a qualified type, and a secret converts only to a type that can carry secret"
    run: [cargo, test, --test, qualifiers_hold, --, a_type_test_narrows_a_qualified_subject_to_a_type_that_keeps_the_qualifier, a_secret_value_converts_only_to_a_type_that_can_carry_secret, a_shape_conversion_over_a_class_refuses_a_secret_property]
    timeout: 1800
    tests:
      - a_type_test_narrows_a_qualified_subject_to_a_type_that_keeps_the_qualifier
      - a_secret_value_converts_only_to_a_type_that_can_carry_secret
      - a_shape_conversion_over_a_class_refuses_a_secret_property
  - id: mixed-carries-secret
    stage: 2
    name: "a secret enters mixed only where secret mixed is written, and every secret sink refuses a secret mixed"
    run: [cargo, test, --test, qualifiers_hold, --, a_secret_value_enters_mixed_only_where_secret_mixed_is_written, text_out_of_secret_mixed_is_secret_and_tainted, every_secret_sink_refuses_a_secret_mixed]
    timeout: 1800
    tests:
      - a_secret_value_enters_mixed_only_where_secret_mixed_is_written
      - text_out_of_secret_mixed_is_secret_and_tainted
      - every_secret_sink_refuses_a_secret_mixed
  - id: untyped-calls-refuse-qualified-values
    stage: 3
    name: "a bare callable, a mixed receiver, a spread, an erased property write and a reflective call refuse a qualified value, and the typed form takes it"
    run: [cargo, test, --test, qualifiers_hold, --, a_call_with_no_parameter_list_refuses_a_qualified_argument, a_typed_callable_takes_the_qualified_argument_its_signature_names, an_erased_property_write_refuses_a_qualified_value]
    timeout: 1800
    tests:
      - a_call_with_no_parameter_list_refuses_a_qualified_argument
      - a_typed_callable_takes_the_qualified_argument_its_signature_names
      - an_erased_property_write_refuses_a_qualified_value
  - id: reflection-names-are-sinks
    stage: 3
    name: "every Core\\Reflect member that picks a class, a method, a property or a constant by text refuses tainted text"
    run: [cargo, test, --test, taint_signatures, --, every_reflection_member_that_picks_by_name_refuses_tainted_text]
    timeout: 1800
    tests:
      - every_reflection_member_that_picks_by_name_refuses_tainted_text
  - id: run-time-keeps-secrets-in
    stage: 4
    name: "an erased or reflective read of a secret property throws, and a failed conversion's message never quotes a text operand"
    run: [cargo, test, --test, qualifiers_at_run_time]
    timeout: 1800
    tests:
      - an_erased_read_of_a_secret_property_throws
      - a_reflective_read_or_call_never_returns_a_secret_as_plain_text
      - a_failed_conversion_never_quotes_a_text_operand
  - id: keys-keep-the-qualifier
    stage: 5
    name: "a tainted key is checked at every key position, and a secret value is never an array key"
    run: [cargo, test, --test, qualifiers_hold, --, a_tainted_key_is_checked_at_every_key_position, a_secret_value_is_never_an_array_key, a_member_that_makes_keys_from_values_refuses_qualified_values]
    timeout: 1800
    tests:
      - a_tainted_key_is_checked_at_every_key_position
      - a_secret_value_is_never_an_array_key
      - a_member_that_makes_keys_from_values_refuses_qualified_values
  - id: key-making-members-are-marked
    stage: 5
    name: "every Core member that returns a plain-keyed array and takes outside text says where its keys come from"
    run: [cargo, test, --test, taint_signatures, --, every_member_that_returns_a_plain_keyed_array_says_where_its_keys_come_from]
    timeout: 1800
    tests:
      - every_member_that_returns_a_plain_keyed_array_says_where_its_keys_come_from
  - id: every-erasing-position-has-its-cells
    stage: 6
    name: "every position the checker lists as erasing a type has a secret cell and a tainted cell in the soundness matrix"
    run: [cargo, test, --lib, --, every_position_that_erases_a_type_has_its_soundness_cells]
    timeout: 1800
    tests:
      - every_position_that_erases_a_type_has_its_soundness_cells
  - id: every-refusal-names-a-form-that-compiles
    stage: 7
    name: "at every erasing position, the form the diagnostic's help names compiles and keeps the qualifier"
    run: [cargo, test, --test, qualifiers_hold, --, the_form_each_refusal_names_compiles_at_every_erasing_position]
    timeout: 1800
    tests:
      - the_form_each_refusal_names_compiles_at_every_erasing_position
---

# secret and tainted survive every narrowing, widening and untyped call

## Why

Novis makes two promises in its types. A `tainted` value reaches no sink without a launderer. A
`secret` value reaches no log, dump, response or extension. The checker in `crates/nvs-types/` keeps
both where a value meets a typed parameter. It loses both wherever a type is erased or derived again.
Every case below compiles with no error today, and each was run on the release binary.

- **A type test.** `infer_against_type` adds `tainted` to the tested type only for a `mixed` subject
  (`crates/nvs-types/src/expr/type_test.rs:128`). For a `secret string` or `tainted string` subject it
  records the plain type (`type_test.rs:143`), and `type_test_residue` narrows the variable to it
  (`crates/nvs-types/src/locals.rs:701`). So `if ($key is string) { echo $key; }` prints a secret.
  The same holds in a ternary, in `match (true)`, and for `?tainted string`, `tainted bytes` and
  `array<tainted string>`.
- **A widening.** `assignable` accepts every value into `mixed` (`crates/nvs-types/src/expr/assign.rs:216`).
  Text out of `mixed` is `tainted` and never `secret`, so `mixed $m = $secret; echo $m;` prints it.
  Only an array element write is refused (`crates/nvs-types/src/expr/quals.rs:1164`).
- **An untyped call.** A bare `callable`, a method call on a `mixed` receiver and a spread into a typed
  callable check each argument against nothing (`crates/nvs-types/src/expr/calls.rs:2375`,
  `calls.rs:237`, `calls.rs:2430`). The run-time check compares representations only
  (`crates/nvs-runtime/src/callable.rs:902`). A property write through `mixed` or `object`, and
  `Core\Reflect\ClassInfo::set`, `call` and `construct`, behave the same. A `secret string`, a
  `tainted string` and text inside a `mixed` all land in a parameter declared plain `string`.
- **An erased read.** `$m->token` on a `mixed` receiver, `ClassInfo::get` and a shape conversion over
  the class return a `secret` property as plain text (`crates/nvs-runtime/src/object.rs:5244`,
  `crates/nvs-stdlib/src/reflect.rs:2903`, `type_test.rs:162`).
- **An array key.** 6776ed3d35 refuses a `tainted` key at a write and in a declared literal
  (`quals.rs:277`). It left the literal in argument position: `Core\Arr::keys([$t => 1])` returns the
  key as plain `string`. It left `Core\Arr::countBy`, whose values become keys of an `array<uint>`
  (`crates/nvs-stdlib/src/arr.rs:677`). No rule covers a `secret` key at all, and every array reads it
  back plain.
- **A conversion.** A successful `$secret as int` gives a plain `int`, which
  `rule:security/secret-propagation` records as an accepted gap. A failed one quotes the operand in its
  message (`crates/nvs-runtime/src/helpers.rs:1540`), and the message reaches the error log.
- **Reflection by name.** `Core\Reflect::forClass($name)` and five siblings take `tainted` text
  (`reflect.rs:342`). Request text then picks the class to construct and the method to call.

Who reaches it: a remote client, for every `tainted` case, since request text is the source. The
`secret` cases need no attacker. An ordinary helper that takes `callable` or `mixed` leaks a
credential into a log or a response, and no reader of the call can see it.

All of these are one cause. A qualifier is checked where a value meets a declared type, and nothing
checks it where the type goes away. The fix is at those positions, and one matrix then holds all of
them.

## Standing decisions

**A qualifier survives every change of the base type.** A type test puts the subject's qualifiers back
on the tested type before it folds or records. `secret string is string` folds to `true`.
`?tainted string is string` narrows to `tainted string`. A conversion keeps both qualifiers on every
target that can carry them, as `apply_qualifier_conversion_rule` does today (`quals.rs:832`). **What it
spends:** nothing at run time. Trade-offs: no memory, no latency, one rule fewer to remember. A
program that used `is` to drop a qualifier now writes the launderer. Declined: a warning in place of
the narrowing, because a one-keyword way around both promises is a hole.

**`secret` has three carriers: `string`, `bytes` and `mixed`.** `secret mixed` is one new type. A
`secret` value enters `mixed` only where `secret mixed` is written. `mixed $m = $secret;` does not
compile, and the help names `secret mixed`. `secret mixed $m = $secret; echo $m;` does not compile at
the `echo`. The refusal is at `assignable`, the one place every placement passes: a declaration, an
assignment, a property, a return, a parameter, a `foreach` variable and `array<secret string>` into
`array<mixed>`. Text out of `secret mixed` is `secret tainted string`. `Core`'s classified credential
positions keep taking a `secret` argument through `check_arg_admitting_quals`
(`crates/nvs-types/src/expr/args.rs:637`). The qualifier is never added behind a declaration, as
`rule:security/taint-propagation` already says for `tainted`. **What it spends:** nothing at run time,
since `secret mixed` is `mixed` below the checker. Usability: a program that keeps a credential in a
`mixed` bag writes `secret mixed`. Simplicity: one more type atom. Safe default if the new atom cannot
be carried through `nvs-ir` in one stage: refuse a `secret` value into `mixed` everywhere, with the
help naming `Core\Secret::reveal`. `mixed` already carries `tainted`, because text out of it is
`tainted`. So there is no `tainted mixed`. **Built:** every `Core` row's `mixed` parameter, not only
the classified ones, keeps taking a `secret` argument. The argument is compared against the parameter
with each `mixed` read as `secret mixed` (`mixed_admitting_secret`, `quals.rs`), so a plain `string`
position still refuses it. Reason: the `reject_secret_*_argument` family refuses at each `Core` sink
with its own code, and an `E0401` beside it would be a second report of one mistake. A parameter the
program declares `mixed` refuses.

**A `secret` value converts only to a type that can carry `secret`.** `$secret as int` does not
compile. The help says to write `Core\Secret::reveal($value, "reason") as int`. This replaces the
accepted gap in `rule:security/secret-propagation`, where a successful `as uint` gave a plain value.
Trade-offs: no run-time cost, and one conspicuous call for a numeric credential. Declined: a `secret`
form of every scalar type. It adds a qualified `int`, `uint`, `float`, `decimal` and `bool`, a rule
for every operator and a class for every numeric `Core` parameter. That is language surface nobody
has needed yet. `tainted` does not change here: a successful conversion to a type with no text stays
its launderer, since the shape is proven.

**A call with no static parameter list refuses a qualified argument at compile time.** The sites are
a bare `callable`, a method call on a `mixed` receiver, a spread into a typed callable, a property
write through `mixed` or `object`, and the value arguments of `ClassInfo::set`, `call` and
`construct`. "Qualified" means the type carries `tainted`, `secret` or unchecked text, which is
`carries_qualifier` or `carries_unchecked`. A `mixed` argument counts, because its text is `tainted`
by rule. One function decides, and `report_args_with_no_parameter_list` (`calls.rs:2023`) is where
three of the sites already meet. The diagnostic names the typed form, such as
`callable(tainted string): void`, or `$x is ClassName` for a receiver, or the launderer. A callable
type with a parameter list already works: `check_call_through_signature` (`calls.rs:2417`) checks
each argument against the written parameter. The review ran both directions, and stage 3 pins them.
**What it spends:** nothing at run time. Usability: a helper that passes decoded data through a bare
`callable` writes `callable(mixed): T`. Safe default if the `mixed` half breaks more of the tree than
stage 7 can update: keep the refusal for `tainted` and `secret`, and check `mixed` text at run time
on the dynamic call path. That costs one AND per argument there and nothing on a typed call.
**Built:** the mark on `ClassInfo::set`, `call` and `construct` is `CoreTy::Dispatched`, a
transparent wrapper read as `ParamText::Dispatched`. A new `Qual` was declined: `Qual` says what a
member does with text, and `mixed` has no text cell to carry one. `ParamText` already reaches the
checker per parameter.

**An erased read never returns a `secret` property.** A read through `mixed` or `object`,
`ClassInfo::get`, a shape test's field walk, and a reflective or erased call of a method that returns
`secret` throw a `LogicError`. The message names the class and the property and never the value.
`Core\Serialize::encode` already throws for such a property, so the bit exists at run time. A shape
conversion over a known class is refused at compile time. **What it spends:** one bit test per erased
read, on a path that already looks a name up. A typed read pays nothing. If a method's row records no
`secret` return, the compiler adds one bit per method, per program and not per request.

**A `secret` value is never an array key.** An array's type records nothing about its keys.
`Core\Arr::keys`, a `foreach`, a dump and an error message all read a key back as plain text. No
element type can change that, so the refusal holds in every array. No rule spoke of a `secret` key
before this goal, and the security priority decides it. Cost: a program keys by something that is
not secret.

**One function checks a key's qualifier, and every key position calls it.** It replaces
`reject_tainted_key_write` (`quals.rs:277`). The positions are a subscript write, every compound
write, a literal against a declared type, an inferred literal, a literal in argument position, and a
member that makes keys. A `tainted` key still enters an array whose element type carries `tainted`
or `mixed`. **What it spends:** compile time only. Declined, as the rule already declines it: a key
qualifier on the array type.

**The key mark is a transparent `CoreTy::Keys(KeyOrigin, &CoreTy)` wrapper, like
`CoreTy::Dispatched`.** A field on `CoreMethod` would change every row literal in the registry. A
wrapper sits exactly at the position whose values become keys. `KeyOrigin::Cells` covers `column`'s
`indexBy`, whose own value is a cell name. The keys come from the subject's cells only when the option
is given. So `column` without `indexBy` still takes `secret` cells. `countBy`'s element is marked
even when a `by` is given. That refuses a `secret` subject that a `by` would have keyed by something
else, which is the safe direction. **What it spends:** compile time only.

**A member that turns values into keys is marked once in the member table.** The mark sits on the
parameter or the callback result whose values become keys. The checker reads it in the generic call
path and calls the key function. The marked members are `Core\Arr::countBy`, `flip`, `fillKeys`,
`fromKeysAndValues`, `mapKeys`, `groupBy` and `column`'s `indexBy` option. Their parameter types stay
as they are. A test reads the registry and fails on any member that returns a plain-keyed array,
takes a type variable, text or a callable, and does not say where its keys come from. So a new member
cannot skip the question. Cost: one field per such row, and no run-time cost.

**A failed conversion never quotes a text operand.** The message names the operand's type and its
length in bytes, for every `string` and `bytes` operand. The run time has no qualifier to read, so
the rule cannot depend on one. It is fixed where the message is built (`helpers.rs:1540`,
`helpers.rs:1552`, `helpers.rs:1971`, `crates/nvs-ir/src/lower/convert.rs:1208`). A number is still
shown, since no number is `secret` or `tainted`. Usability cost: the message for a plain string no
longer shows its text. Declined: a flag from the checker and a second helper per conversion. That is
a fix per cast site, and it leaves an invariant every new conversion must remember.

**Reflection that picks code by text refuses `tainted` text.** These parameters become `Qual::Sink`:
`Core\Reflect::forClass($name)`, `ClassInfo::constant($name)`, the `$name` of `ClassInfo::get`, `set`
and `call`, and `Core\Reflect\EnumInfo::of($name)`. Two more `Text(Qual::Neutral)` rows sit at
`reflect.rs:1309` and `reflect.rs:1648`, and each becomes a sink if it picks by name.
`ClassInfo::hasProperty` and `hasMethod` stay neutral: they return a `bool` and run nothing. A
dispatcher compares the text with its own list, or writes `Core\Taint::assertTrusted` with its
reason. Cost: one call in a program that dispatches on a path segment.

**One matrix holds the whole promise, and the checker's own list feeds it.** The checker gains one
enum of the positions that erase or derive a type again: type test, conversion, widening, untyped
call, erased write, erased read, key, key-making member, reflection and generic instantiation. Every
check above is called with its position. `bun nv soundness` gains a qualifier table beside its
redeclaration table and writes the cells to `tests/conformance/soundness/`. A lib test reads the enum
and fails naming a position with no `secret` cell or no `tainted` cell. No second matrix is written.

**Where the fix breaks code in the tree, that code launders properly.** Examples, attacks, benches,
website snippets, tests and packages under the repository are updated, never exempted. No flag turns
a check off.

**The new diagnostics follow the rules for text an end user reads.** Each says what is not allowed
and names the form to write. Each new code gets its card beside `E0824`
(`crates/nvs-diagnostics/src/code/types_e08.rs:435`). A site that already reports its own code keeps
it, and the new refusal reports only where nothing else did (`rule:types/one-mistake-reports-once`).

**Built: every check takes its `ErasingPosition`, and asserts it in a debug build.** A check with
several positions, `check_key_qualifier`, is told which one. `reject_qualified_untyped_value` reads it
from `UntypedTarget::position`. A check with one position asserts that it was called for it.
`GenericInstantiation` is the one variant with no check, since substitution keeps the argument's type
whole, and its matrix cells are its proof. **What it spends:** nothing in a release build.
**Built in stage 7:** `ErasingPosition`, its `ALL` and its `name` are `pub`, so the integration test
`the_form_each_refusal_names_compiles_at_every_erasing_position` matches every variant. A new
variant then fails to compile there until its typed form is written, and the lib test fails until
its matrix cells exist. Declined: keeping it `pub(crate)` so that `dead_code` names a variant no check
builds. The test was owed in `tests/qualifiers_hold.rs`, which reads only the public API, and the two
tests hold a new position more firmly than a lint about construction. The help of a bare
`callable`'s refusal names `callable(<the argument's type>): void`, so a `secret` argument is shown a
form that takes it.
`$fn->call($obj, ...)` is part of `UntypedCall`: an argument after the object that carries a
qualifier is refused, unless the callable's written parameter takes its type.

**Lanes:** one lane. Stages 1, 2, 3 and 5 all edit `crates/nvs-types/src/expr/quals.rs` and
`crates/nvs-types/tests/qualifiers_hold.rs`, so they run in order. Stage 4 edits `nvs-runtime`,
`nvs-ir` and the run-time half of `reflect.rs`. It may run beside stage 5 once stage 3 is committed.
Stage 6 needs every check in place, and stage 7 needs stage 6's enum.

## Stages

### 1. a type test and a cast keep the qualifier

**Does:** make `is` narrow a qualified subject to a qualified type, and refuse a `secret` conversion
whose target cannot carry `secret`.

- [x] `infer_against_type` puts the subject's qualifiers on `tested` before both folds:
  `tainted_result` when the subject carries `tainted` or unchecked text, `secret_result` when it
  carries `secret` (`rule:types/type-test`, `crates/nvs-types/src/expr/type_test.rs:128`).
- [x] `always_holds` folds `tainted string is string` to `true`, as its doc comment says, and the
  comment is rewritten to match (`rule:types/type-test`, `crates/nvs-types/src/expr/type_test.rs:361`).
- [x] `type_test_residue` never narrows a variable to a type with fewer qualifiers than it had
  (`rule:types/narrowing`, `crates/nvs-types/src/locals.rs:690`).
- [x] `accept_shape_conversion` refuses a shape field whose property is `secret` when the operand is a
  known class (`rule:security/secret-qualifier`, `crates/nvs-types/src/expr/type_test.rs:162`).
- [x] A conversion from a `secret` operand to a target where `secret_result` can set no bit does not
  compile, with a new code and the `Core\Secret::reveal` help (`rule:security/secret-propagation`,
  `crates/nvs-types/src/expr/quals.rs:832`, `crates/nvs-diagnostics/src/code/types_e08.rs:435`).
- [x] Test `a_type_test_narrows_a_qualified_subject_to_a_type_that_keeps_the_qualifier`: `if`, a
  ternary and `match (true)`, over `secret string`, `tainted string`, `?tainted string`,
  `string|tainted string`, `tainted bytes` and `array<tainted string>`, each followed by a sink
  (`crates/nvs-types/tests/qualifiers_hold.rs:1`).
- [x] Tests `a_secret_value_converts_only_to_a_type_that_can_carry_secret` and
  `a_shape_conversion_over_a_class_refuses_a_secret_property` (`crates/nvs-types/tests/qualifiers_hold.rs:1`).
- [x] `rule:security/secret-propagation` loses its accepted-gap paragraph and its Declined entry, and
  gains the conversion rule and the declined `secret` scalars (`docs/rules/security/secret-propagation.md:10`).
- [x] `rule:types/type-test` and `rule:types/narrowing` each gain one sentence: a test keeps the
  subject's qualifiers (`docs/rules/types/type-test.md:1`, `docs/rules/types/narrowing.md:46`).

### 2. mixed keeps a secret only where secret mixed is written

**Does:** add `secret mixed`, and refuse a `secret` value into plain `mixed` at `assignable`.

**Split:** an `Explore` subagent lists every match on `Ty::Mixed` in `nvs-ir`, `nvs-codegen`,
`nvs-lsp`, `nvs-cli` and `nvs-stdlib`, as `file.rs:NN` and one line each, before the first edit.

- [x] `Ty` gains the `secret mixed` atom, and the type grammar accepts `secret mixed`
  (`rule:security/secret-qualifier`, `rule:types/grammar`, `crates/nvs-types/src/ty.rs:130`,
  `crates/nvs-types/src/lower.rs:44`, `crates/nvs-types/src/expr/quals.rs:441`).
- [x] `is_secret`, `contains_secret`, `carries_secret`, `unsecret` and `secret_result` know the atom,
  and `secret_result(mixed)` is `secret mixed` (`crates/nvs-types/src/expr/quals.rs:429`,
  `crates/nvs-types/src/expr/quals.rs:575`, `crates/nvs-types/src/expr/quals.rs:1079`).
- [x] `assignable` refuses a value that contains `secret` into `mixed` and into a union with a `mixed`
  member, and accepts it into `secret mixed` (`rule:security/secret-qualifier`,
  `crates/nvs-types/src/expr/assign.rs:216`).
- [x] The refusal's message names `secret mixed`, and a sink that reports its own code still reports
  that one alone (`rule:types/one-mistake-reports-once`, `crates/nvs-types/src/expr/quals.rs:1164`).
- [x] Text out of `secret mixed` through `as` and `is` is `secret tainted`
  (`rule:security/secret-propagation`, `crates/nvs-types/src/expr/quals.rs:832`,
  `crates/nvs-types/src/expr/type_test.rs:128`).
- [x] Every site the subagent listed treats `secret mixed` as `mixed` below the checker
  (`crates/nvs-ir/src/lower/expr.rs:4540`).
- [x] Tests `a_secret_value_enters_mixed_only_where_secret_mixed_is_written` (every placement the
  standing decision lists), `text_out_of_secret_mixed_is_secret_and_tainted` and
  `every_secret_sink_refuses_a_secret_mixed` (`crates/nvs-types/tests/qualifiers_hold.rs:1`).
- [x] `rule:security/secret-qualifier` names the three carriers, and `rule:types/unions-and-mixed`
  names `secret mixed` (`docs/rules/security/secret-qualifier.md:1`, `docs/rules/types/unions-and-mixed.md:1`).

### 3. a call or a write with no static parameter list refuses a qualified value

**Does:** refuse a qualified value at every call and write whose target type the site cannot see,
and make reflection's name parameters sinks.

- [x] One function in `quals.rs` refuses an argument that carries `tainted`, `secret` or unchecked
  text, and `report_args_with_no_parameter_list` calls it for both callee kinds
  (`rule:types/callable-signature`, `rule:types/erased-member-access`,
  `crates/nvs-types/src/expr/calls.rs:2023`, `crates/nvs-types/src/expr/quals.rs:1106`).
- [x] The spread path of `check_call_through_signature` calls it for the spread subject's element type
  (`rule:types/callable-signature`, `crates/nvs-types/src/expr/calls.rs:2430`).
- [x] A property write through `mixed` or `object` calls it for the written value. Find the checker's
  site by Grep for `erased-member-access` in `assign.rs` (`rule:types/erased-member-access`,
  `crates/nvs-types/src/expr/assign.rs:216`, `crates/nvs-runtime/src/object.rs:5650`).
- [x] The value parameters of `ClassInfo::set`, `call` and `construct` carry one mark in the member
  table, and the checker calls the same function for a marked parameter. A literal argument is judged
  element by element (`rule:security/unclassified-parameter-refuses-tainted`,
  `crates/nvs-stdlib/src/reflect.rs:661`, `crates/nvs-stdlib/src/reflect.rs:673`,
  `crates/nvs-stdlib/src/reflect.rs:689`).
- [x] The name parameters become `Qual::Sink`: `forClass`, `constant`, `get`, `set`, `call`,
  `EnumInfo::of`, and each of the rows at `:1309` and `:1648` that picks by name
  (`rule:security/sink-predicate`, `crates/nvs-stdlib/src/reflect.rs:342`,
  `crates/nvs-stdlib/src/reflect.rs:616`, `crates/nvs-stdlib/src/reflect.rs:646`,
  `crates/nvs-stdlib/src/reflect.rs:1279`).
- [x] The reference cards of those members say that `tainted` text is not allowed
  (`rule:core-api/reference-card`, `crates/nvs-stdlib/src/reflect.rs:349`).
- [x] Tests `a_call_with_no_parameter_list_refuses_a_qualified_argument`,
  `a_typed_callable_takes_the_qualified_argument_its_signature_names` and
  `an_erased_property_write_refuses_a_qualified_value` (`crates/nvs-types/tests/qualifiers_hold.rs:1`).
- [x] Test `every_reflection_member_that_picks_by_name_refuses_tainted_text` walks the `Core\Reflect`
  rows of the registry. It fails naming a text or class-name parameter that is not a sink on a member
  that returns anything but `bool` (`crates/nvs-stdlib/tests/taint_signatures.rs:150`).
- [x] `rule:types/callable-signature`, `rule:types/erased-member-access` and
  `rule:security/sink-predicate` each state the refusal (`docs/rules/types/callable-signature.md:1`,
  `docs/rules/types/erased-member-access.md:1`, `docs/rules/security/sink-predicate.md:1`).

### 4. the run time keeps a secret out of an erased read and out of a failed conversion's message

**Does:** throw on an erased or reflective read of a `secret` property, and stop quoting text in a
failed conversion's message.

**Split:** an `Explore` subagent lists two sets before the first edit, as `file.rs:NN` and one line
each. The first is every run-time message that includes a conversion operand's text. The second is
where `Core\Serialize::encode` reads a property's `secret` bit.

- [x] `read_erased_property` throws a `LogicError` for a property whose `secret` bit is set, and the
  shape field walk and `ClassInfo::get` read through it (`rule:security/secret-sinks-refuse`,
  `crates/nvs-runtime/src/object.rs:5244`, `crates/nvs-stdlib/src/reflect.rs:2903`).
- [x] `ClassInfo::call` and an erased method call throw when the method's declared return carries
  `secret` (`rule:security/secret-sinks-refuse`, `crates/nvs-stdlib/src/reflect.rs:3044`).
- [x] `does_not_fit` and its callers name a text operand by type and length only, and every site the
  subagent listed does the same (`rule:types/conversion`, `crates/nvs-runtime/src/helpers.rs:1540`,
  `crates/nvs-runtime/src/helpers.rs:1552`, `crates/nvs-runtime/src/helpers.rs:1971`,
  `crates/nvs-ir/src/lower/convert.rs:1208`).
- [x] Every frozen output that quotes a text operand is updated, and the commit says why
  (`crates/nvs-codegen/tests/callables.rs:201`).
- [x] Tests `an_erased_read_of_a_secret_property_throws`,
  `a_reflective_read_or_call_never_returns_a_secret_as_plain_text` and
  `a_failed_conversion_never_quotes_a_text_operand`. Each runs a program and reads its output and its
  error message (`crates/nvs-codegen/tests/qualifiers_at_run_time.rs:1`).
- [x] `rule:types/conversion` and `rule:security/secret-sinks-refuse` state the message rule and the
  erased read, each with its cost line (`docs/rules/types/conversion.md:1`,
  `docs/rules/security/secret-sinks-refuse.md:1`).

### 5. every key position checks the qualifier, and the member table says which members make keys

**Does:** route every key position through one function, refuse a `secret` key everywhere, and mark
the members that make keys from values.

- [x] One function checks a key's qualifier against the array it enters, and it replaces
  `reject_tainted_key_write` at its three callers (`rule:security/taint-propagation`,
  `crates/nvs-types/src/expr/quals.rs:277`, `crates/nvs-types/src/expr/literals.rs:1202`,
  `crates/nvs-types/src/expr/literals.rs:1382`, `crates/nvs-types/src/expr/mod.rs:818`).
- [x] An array literal in argument position reaches it, whatever type the parameter expects. Today the
  check runs only when `expected` is an array (`rule:security/taint-propagation`,
  `crates/nvs-types/src/expr/literals.rs:1201`, `crates/nvs-types/src/expr/args.rs:1717`).
- [x] The function refuses a key whose type contains `secret` in every array, with a new code
  (`rule:security/secret-qualifier`, `crates/nvs-types/src/expr/quals.rs:277`,
  `crates/nvs-diagnostics/src/code/types_e08.rs:435`).
- [x] The registry gains the key mark, and the generic call path calls the key function for a marked
  parameter or callback result (`rule:security/taint-propagation`,
  `crates/nvs-stdlib/src/registry.rs:1`, `crates/nvs-types/src/expr/args.rs:1722`).
- [x] The seven members carry the mark: `countBy` and its `by` option, `flip`, `fillKeys`,
  `fromKeysAndValues`, `mapKeys`, `groupBy` and `column`'s `indexBy`
  (`crates/nvs-stdlib/src/arr.rs:677`, `crates/nvs-stdlib/src/arr.rs:2127`,
  `crates/nvs-stdlib/src/arr.rs:488`, `crates/nvs-stdlib/src/arr.rs:561`,
  `crates/nvs-stdlib/src/arr.rs:579`, `crates/nvs-stdlib/src/arr.rs:156`,
  `crates/nvs-stdlib/src/arr.rs:175`, `crates/nvs-stdlib/src/arr.rs:2169`).
- [x] Test `every_member_that_returns_a_plain_keyed_array_says_where_its_keys_come_from` walks every
  class of the registry and fails naming a member without the answer
  (`crates/nvs-stdlib/tests/taint_signatures.rs:150`).
- [x] Tests `a_tainted_key_is_checked_at_every_key_position`, `a_secret_value_is_never_an_array_key`
  and `a_member_that_makes_keys_from_values_refuses_qualified_values`
  (`crates/nvs-types/tests/qualifiers_hold.rs:1`).
  - [x] `a_tainted_key_is_checked_at_every_key_position` passes
  - [x] `a_secret_value_is_never_an_array_key` passes
  - [x] `a_member_that_makes_keys_from_values_refuses_qualified_values` passes
- [x] `rule:security/taint-propagation` names the argument literal and the member mark, and
  `rule:security/secret-qualifier` states the key rule (`docs/rules/security/taint-propagation.md:65`,
  `docs/rules/security/secret-qualifier.md:11`).
  - [x] the argument literal in `rule:security/taint-propagation`
  - [x] the key rule in `rule:security/secret-qualifier`
  - [x] the member mark in `rule:security/taint-propagation`
- [x] A marked member refuses a `tainted` value whose key would land in an answer whose keys read
  back plain, such as `Core\Arr::countBy` over `array<tainted string>`, which answers `array<uint>`
  (`rule:security/taint-propagation`, `crates/nvs-types/src/expr/args.rs:check_key_marks`).
  - [x] `check_key_marks` and the option check in `check_options_arg` pass the call's answer type
    as `base` to `check_key_qualifier`
  - [x] `a_member_that_makes_keys_from_values_refuses_qualified_values` gains the `tainted` rows of
    `countBy`, its `by`, `mapKeys` over plain elements, and `column`'s `indexBy`, and passes
  - [x] `rule:security/taint-propagation`'s member-mark paragraph states the `tainted` half

### 6. one matrix tries a secret and a tainted value at every position that erases a type

**Does:** give the checker one list of erasing positions, and write the qualifier cells of
`bun nv soundness` from it.

- [x] The checker gains one enum of erasing positions, and every check of stages 1 to 5 is called
  with its position (`rule:testing/redeclaration-matrix`, `crates/nvs-types/src/expr/quals.rs:1`).
- [x] `bun nv soundness` gains a qualifier table: one row per position, a `secret` cell and a
  `tainted` cell each, and a sink after the position. Each cell's verdict is the rule's error code
  (`rule:testing/redeclaration-matrix`, `tools/nv/cmd/soundness.ts:106`).
- [x] `bun nv soundness --write` writes `tests/conformance/soundness/qualifier-<position>.errors.nvst`,
  and each cell names its position in a comment the lib test reads
  (`tests/conformance/soundness/qualifier-type-test.errors.nvst:1`).
- [x] Lib test `every_position_that_erases_a_type_has_its_soundness_cells` reads the enum and the
  written files, and fails naming a position with a missing cell
  (`crates/nvs-types/src/expr/quals.rs:1`).
- [x] Run `bun nv soundness --probe` once, and fix every cell it names. A cell's verdict is never
  changed to match the binary (`tools/nv/cmd/soundness.ts:8`).
- [x] The rule fragment for the matrix describes the qualifier table and its positions
  (`docs/rules/testing/redeclaration-matrix.md:1`).

### 7. the tree's own programs launder properly, and the feature has its proofs

**Does:** update every program in the repository that the new refusals break, and write the attack,
the example and the help text the feature owes.

**Split:** an `Explore` subagent runs the built `nvs check` over every `.nvs` file under
`docs/examples/`, `tests/hostile/`, `benches/members/`, `website/snippets/` and the packages in the
repository. It reports each new error as `path:line` and its code. Writing subagents then update
disjoint folders side by side, and none of them builds.

- [x] Every program the subagent listed compiles again with the typed form or the launderer, and its
  `.out` is unchanged (`rule:security/taint-propagation`, `docs/examples/README.md:1`).
  - [x] The sweep ran `nvs check` over 8270 files: 12 new errors, in `core/Reflect-ClassInfo`
    examples and attacks, three attacks on `mixed` and bare `callable` calls, and one bench.
  - [x] The three `Reflect-ClassInfo` examples compile and their `.out` is unchanged.
  - [x] The eight attacks compile and still reach the run-time error they were written for.
  - [x] `benches/members/novis/Pdf-Reader/invoiceXml.nvs` compiles.
- [ ] Every Rust test and `.nvst` case that expected the old acceptance expects the refusal
  (`rule:security/secret-qualifier`, `crates/nvs-types/tests/secret.rs:1`,
  `crates/nvs-types/tests/tainted.rs:1`, `crates/nvs-types/tests/type_test.rs:1`).
- [x] Test `the_form_each_refusal_names_compiles_at_every_erasing_position`: for each position of the
  enum, through an exhaustive `match`, the form the help names compiles and the sink still refuses
  (`crates/nvs-types/tests/qualifiers_hold.rs:1`).
- [ ] One attack reads a `secret` property through every untyped read and through a failed
  conversion, and each step should throw without the value in its message
  (`rule:testing/feature-proofs`,
  `tests/hostile/lang/types/qualifiers-tainted-and-secret/02-read-a-secret-through-every-untyped-door.nvs:1`).
- [x] One example shows `secret mixed`, with its `.out`
  (`rule:testing/feature-proofs`,
  `docs/examples/lang/types/qualifiers-tainted-and-secret/08-a-secret-in-a-mixed-value.nvs:1`).
- [x] The feature's `about.md` and `details.md` state the three carriers, the conversion rule and the
  key rule (`docs/examples/lang/types/qualifiers-tainted-and-secret/about.md:1`,
  `docs/examples/lang/types/qualifiers-tainted-and-secret/details.md:1`).
- [ ] A `secret` value written in an array literal or an anonymous object that is passed to a `Core`
  `mixed` parameter keeps its qualifier, so `Core\Json::encode([$token])` and
  `Core\Json::encode(['k' => $token])` are `E0791`. Today both compile and print the secret: a
  literal checked against a `mixed` expectation returns the expectation, and a written argument is
  not asked the container question (`reject_secret_encoded_argument`,
  `crates/nvs-types/src/expr/quals.rs:1612`, and `check_array_literal`). `Core\Log::write` and
  `Core\Queue::push` walk the literal for a bare variable only. Fix it once, where the literal's
  type is made, so every `Core` sink reads `contains_secret` off the argument
  (`rule:security/secret-qualifier`, `crates/nvs-types/tests/qualifiers_hold.rs:1`).
- [ ] `bun nv proofs --comments` passes over every `.nvs` and `about.md` this stage wrote
  (`rule:testing/feature-proofs`, `docs/examples/lang/types/qualifiers-tainted-and-secret/about.md:1`).

### 8. the goal is verified once, and every failure is fixed

- [ ] Run `looper verify` over the whole goal.
- [ ] Fix every failure it names in one pass, never deleting or loosening a test.
- [ ] Run `looper verify` once more, and it is green.

## Handoff

### State

Stages 1 to 6 are done, and stage 6's acceptance check passes. `ErasingPosition` (`quals.rs`) lists
the ten positions, and each check takes its position. `bun nv soundness` writes ten
`qualifier-<position>.errors.nvst` files with 22 cells, and `--probe` over the debug build passes all
487 cells. `$fn->call($obj, ...)` now refuses a qualified argument after the object, unless the
callable's written parameter takes it (`infer_callable_rebind`, `calls.rs`). The sweep's reds are
stale: `env-secret-proofs` is defined nowhere, and `305c04f660` fixed nightly `43001191130`'s test.

Expected red at the gate. Stage 6: a program that passes `tainted`, `secret` or `mixed` text to
`$fn->call(...)` through a bare `callable`. Stage 1: converting a `secret` to a number, bool, enum or class; narrowing
`tainted` with `is` into a plain sink. Stage 2: a `secret` into a declared `mixed`; tests expecting
`$secret as mixed` to be `E0894`. Stage 3: qualified or `mixed` values through a bare `callable`, a
`mixed` receiver, an erased write, `ClassInfo::set`/`call`/`construct`, or a `Core\Reflect` name.
Stage 4: any output the grep missed that quotes a conversion's string operand; any read of a
`secret` property through `mixed`, `object`, a shape or `ClassInfo::get`; a dump or encoder test that
showed a `?secret string`. `website/public/llms-full.txt` and `docs/reference/` were edited by hand, and
`bun nv proofs --record-perf` was not run. Stage 7 updates the programs. Stage 5: any program that
keys an array by a `secret` value, or puts a `tainted` key in a literal passed to a plain-keyed
`array<…>` parameter. The member mark: any program or test that calls `countBy`, `flip`, `fillKeys`,
`fromKeysAndValues`, `mapKeys`, `groupBy` or `column` with `indexBy` over `secret` values, and
`countBy` or its `by` over `tainted` values. Registry lib tests whose walkers match `CoreTy` with `_`
and do not know `Keys` (the ones with a `Nullable` or-pattern got the arm).

### Next

Stage 7, the tree's own programs and the proofs. Follow its `**Split:**` line: first one `Explore`
subagent runs `target/debug/nvs.exe check` over every `.nvs` under `docs/examples/`, `tests/hostile/`,
`benches/members/`, `website/snippets/` and `packages/`, and lists each new error as `path:line` and
its code. Write the item's checkpoints first.

- [ ] `docs/examples/README.md:1`: each listed program compiles again with the typed form or
  `Core\Secret::reveal` / `Core\Taint::assertTrusted`, and its `.out` is unchanged. Writing subagents
  take disjoint folders.
- [ ] `crates/nvs-types/tests/qualifiers_hold.rs:1`: test
  `the_form_each_refusal_names_compiles_at_every_erasing_position`, with an exhaustive `match` over
  the positions. The cells of `QUALIFIER_ROWS` (`tools/nv/cmd/soundness.ts`) are the refused forms
  to start from.
- [ ] `tests/hostile/README.md:1`: the attack the stage names, then the
  `secret mixed` example, then `about.md` and `details.md`.

### Backlog

- A property-path or string-with-holes failure still names the bad segment's text, such as
  ``has no public property `$nmae` `` (`nvs_property_path_mismatch`, `nvs_string_holes_mismatch`,
  `crates/nvs-runtime/src/helpers.rs`). The whole operand is no longer shown, but a segment is part
  of it. Name the segment by position instead.
- `$fn->call($obj, $a)` on a typed callable does not check `$a` against the written parameter, only
  its qualifier (`infer_callable_rebind`, `crates/nvs-types/src/expr/calls.rs`). A plain type
  mismatch there is found at run time.
- A `.=` compound write through an erased property is not asked the stage 3 question, only the plain
  `=` (`crates/nvs-types/src/expr/assign.rs:1083`).
- A third-party extension method that takes a resource may return text built from an earlier
  `tainted` argument as plain text (`crates/nvs-types/src/ext_lib.rs:290`). Not traced to a run.
- `slot_check` and `test_shape` are two maps from a type to a run-time test, and an intersection
  type and `iterable` stay `SlotCheck::Any` in the first, and a single-value `string` keeps its tag
  alone (`crates/nvs-ir/src/lower/mod.rs:602`, `crates/nvs-ir/src/lower/expr.rs:7881`).
- A callable's `invoke` row carries no parameter type spellings, so a bare `callable`'s refusal
  names no type (`crates/nvs-ir/src/lower/anon_fn.rs:374`, `crates/nvs-runtime/src/dispatch.rs:286`).
- `check_param_class` runs on every call of an anonymous function, a proven one too, and the row
  check may make it removable (`crates/nvs-ir/src/lower/anon_fn.rs:454`).
- `Core\Str::format('%1$s', $o)` over a `Stringable` object throws "it does not implement
  `Stringable`": `stringify` finds no compiled `toString` (`crates/nvs-runtime/src/helpers.rs:2049`).
  Once it works, `crates/nvs-codegen/tests/borrowed_reads.rs` gains a row for it.
