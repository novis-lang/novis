---
looper: 1
title: "a written ::class is a class reference wherever one is expected"
summary: "Reached when Foo::class, self::class, parent::class, static::class and $obj::class compile without as at every place whose type is class<T>, each checked against T while compiling, a string literal and a string variable still need as, and the rule, the tests, the examples and the change entry say so."
context: [docs/rules/types/class-reference.md, docs/rules/types/class-constant.md, crates/nvs-types/src/expr/members.rs, crates/nvs-types/src/expr/operators.rs]
stages:
  - id: 1
    title: "the checker types a written ::class against the class<T> it is given"
    status: done
    context: [crates/nvs-types/src/expr/mod.rs, crates/nvs-types/src/expr/assign.rs, crates/nvs-types/src/expr/args.rs, crates/nvs-types/src/defaults.rs, crates/nvs-types/src/expr_table.rs, crates/nvs-types/tests/classes.rs]
  - id: 2
    title: "a written ::class at a class<T> lowers to the descriptor, with no name string"
    status: done
    context: [crates/nvs-ir/src/lower/expr.rs, crates/nvs-ir/src/lower/convert.rs, crates/nvs-ir/tests/class_reference.rs, tests/conformance/class/a-class-reference-instantiates-the-subclass-it-names.nvst]
  - id: 3
    title: "the rules, the examples, the proofs and the website"
    status: done
    context: [docs/agent/prose.md, docs/examples/lang/types/callable-classes-object-shapes/about.md, docs/examples/lang/types/the-conversion-operator-as/about.md, website/src/data/changes.json]
  - id: 4
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: checker
    stage: 1
    name: "a written ::class is a class<T> where one is expected, is checked against T while compiling, and every string still needs as"
    run: [cargo, test, --test, classes]
    timeout: 1800
    tests:
      - a_written_class_name_is_a_class_reference_where_one_is_expected
      - a_written_class_name_outside_the_bound_is_e0708_without_as
      - static_class_is_a_class_reference_when_the_enclosing_class_is_in_the_bound
      - an_objects_class_is_a_class_reference_of_its_declared_type
      - a_string_literal_and_a_string_variable_still_need_as
      - a_written_class_name_stays_a_string_where_a_string_is_expected
      - a_parameter_default_may_be_a_written_class_name
  - id: lowering
    stage: 2
    name: "a written ::class at a class<T> lowers to a descriptor constant or the frame's descriptor, never a name lookup"
    run: [cargo, test, --test, class_reference]
    timeout: 1800
    tests:
      - a_written_class_name_at_a_class_reference_lowers_to_a_descriptor_constant
      - static_class_at_a_class_reference_reads_the_frame_descriptor_without_a_name
  - id: conformance
    stage: 2
    name: "a program passes Foo::class, static::class and $obj::class to a class<T> parameter without as, and each call runs on the class it names"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/class/a-written-class-name-is-a-class-reference-where-one-is-expected.nvst]
    timeout: 1800
  - id: proofs
    stage: 3
    name: "every feature this goal's work reached has its feature proofs"
    run: [bun, nv, proofs, --gate, --goal, class-name-as-reference]
    timeout: 1800
  - id: close
    stage: 4
    name: "the goal-end check is green: ci --check's checks of the tree's files, the owners and the goal's proofs"
    run: [bun, nv, close, --goal, class-name-as-reference]
    timeout: 900
---

# a written ::class is a class reference wherever one is expected

## Why

The user asked on 2026-11-23 whether a program can declare that a string parameter is a class name. The
answer is `class<T>`, but every caller has to write `Foo::class as class<Storable>`, even though the
parameter already says what it needs. The user's words: nobody would use it, because it is cumbersome to
write when the parameter already defines the type.

`rule:types/class-reference` says the `as` is "the only source" of a class reference, and that the
safety rests on `class<T>` and `Foo::class` being visibly different. The danger that sentence guards
is a **string that arrives while the program runs** and names a class. A written `Foo::class` is not
that: the compiler resolves it, and `reject_impossible_class_reference_conversion`
(`crates/nvs-types/src/expr/operators.rs:174`) already decides it while compiling. `static::class` is
always the enclosing class or a subclass, and `$obj::class` is always the receiver's declared class
or a subclass. None of the five forms can name a class the source did not write, so none needs the
`as`. The rule's Declined list turns down "a bare `string` accepted at the three sites", which this
goal keeps. It has never considered the written forms. The user approved the design on 2026-11-23.

What the user gets: `register(Invoice::class)` compiles where `register` takes `class<Storable>`,
`register(Rock::class)` is `E0708` as it is under `as` today, and `$name` still needs `as
class<Storable>`.

## Standing decisions

- **A written `::class` takes the type its target expects.** Where the expected type is `class<T>` or
  `?class<T>`, the expression's type is `class<C>`, where `C` is the class it names, and covariance
  (`rule:types/class-reference-variance`, `assign.rs:433`) carries it to `class<T>`. Everywhere else it
  stays a `string`, exactly as today. The precedent is `rule:types/numeric-literal-placement`: the
  position decides what a literal is. *Why:* no new conversion row and no second check; the existing
  hierarchy test and covariance do the work.
- **The five forms, and only these:**

  | Written | Type at a `class<T>` place | Checked |
  |---|---|---|
  | `Foo::class`, `self::class`, `parent::class` | `class<Foo>` / the class it folds to | while compiling, `E0303` and `E0708` as under `as` |
  | `static::class` in a method of class `C` | `class<C>` | while compiling, against `C`. The run-time class is `C` or a subclass, so no run-time check |
  | `$obj::class` where `$obj` is a `U` | `class<U>` | while compiling, against `U`. A `?U` or `mixed` receiver stays `E0702` |

  A string literal `'Shop\Dog'`, a class constant holding a name, a concatenation and a `string`
  variable keep needing `as`. *Why:* those are data. The literal looks like text, and a `string`
  variable may hold request input. Tracking where a variable's string came from would make the rule hard
  to predict.
- **An enum's `::class` is not a class reference.** `T` names a class or interface
  (`E0795`), so `Status::class` at a `class<T>` place fails the hierarchy test like any other class
  outside `T`.
- **A union keeps the string when it can.** Where the expected type has a `string` member
  (`string|class<T>`), the expression stays a `string`. Where it has one or more `class<…>` members and
  no `string`, the expression is `class<C>`, and assignability picks the member. The `E0708` message
  with its help is given where exactly one `class<T>` member is expected; a union with several gives the
  ordinary mismatch.
- **The `E0708` message is the one `as` gives today.** `reject_impossible_class_reference_conversion`'s
  report becomes one function both paths call, so the two never word it differently. Its help names the
  class the argument expects.
- **A `string` at a `class<T>` place gets help naming `as class<T>`.** The ordinary mismatch stays the
  error, and its help says `write $name as class<T>`. This is the one place a user who writes a variable
  learns the conversion exists.
- **A parameter default and a property default may be a written `Foo::class`** at a `class<T>`
  declaration. It folds to `ConstArg::ClassRef` (`defaults::class_reference_default`) and lowers to
  the same descriptor constant. `static::class` and `$obj::class` are not constants and stay refused
  there (`E0451` at a parameter, `E_PROPERTY_DEFAULT_NOT_LITERAL` at a property). An instance or
  `static` property default lowers to `nvs_ir::ir::PropertyDefault::Class(label)`. `nvs-codegen`'s
  `Classes::link_default` resolves it to `nvs_runtime::FieldDefault::Class(ClassAddr)` once every
  descriptor exists, the way `link_codecs` resolves a codec's nested class. *Why a label in the IR and
  an address at run time:* no descriptor exists while lowering, and a run-time lookup by name on every
  `new` would cost a hash per instance. *What it spends:* one word per class-defaulted property per
  class, once per process. An instance pays one slot write.
- **`public class<T> $p` parses as a property.** In a class body, `class` followed by `<` is a type
  (`at_class_reference`), not a nested declaration. Before this goal no class body could declare a
  `class<T>` property after a modifier.
- **An enum is refused by the shared test on both paths.** `admit_class_reference`
  (`operators.rs`) refuses an enum even where it implements `T`, so `Status::class as class<HasLabel>`
  is `E0708` too. *Why:* an enum has no class descriptor (`rule:enums/no-class-machinery`), so the
  `as` form would have lowered to a descriptor that does not exist.
- **An existing `Foo::class as class<T>` keeps compiling, with no warning.** The `as` is harmless, and a
  warning would make every existing program noisy for no gain. The examples drop it where it is no longer
  needed (stage 3).
- **The lowering reads the descriptor directly.** A folded name lowers to `InstKind::ClassDescConst`,
  `static::class` to `Self::lsb()`, and `$obj::class` to `InstKind::ClassDescOf`. None of them builds the
  name string, and none runs `ClassDescIn`'s walk. *What it spends:* nothing; it removes one string
  allocation and one hierarchy walk per run-time form against today's `static::class as class<T>`.
- **`Core` parameters marked `CoreTy::ClassName` stay `string`.** Changing a `Core` signature is a
  separate API change, and this goal does not take it.
- **Lanes:** the stages run in order. Stage 2 reads the `ExprInfo` stage 1 records, and stage 3 states
  what both built.

## Stages

### 1. the checker types a written ::class against the class<T> it is given

**Does:** passes the expected type into `check_class_name_const` and gives the five forms their
`class<C>` type there, records a new `ExprInfo` entry for the lowering, and adds the help for a string.

**Split:** none, one session.

- [x] `crates/nvs-types/src/expr/mod.rs:784`: the `ClassNameConst` arm passes `expected` to
  `members::check_class_name_const` (`crates/nvs-types/src/expr/members.rs:1855`).
- [x] `members.rs:1855`: unwrap `?class<T>` the way `nullable_inner_target` does
  (`operators.rs:195`), then apply *Standing decisions*' table and union bullet. Record
  `ExprInfo::ClassRefConst { class }` for a folded side and `ExprInfo::ClassRefOf` for `static::class`
  and `$obj::class` (`crates/nvs-types/src/expr_table.rs`), and return `class<C>`.
- [x] `operators.rs:174`: the hierarchy test and its `E0708` report move into one function that both the
  `as` path and `check_class_name_const` call.
- [x] `crates/nvs-types/src/expr/assign.rs:747` `report_mismatch`: a `string` at a `class<T>` place
  gets the help of *Standing decisions*, and no other mismatch changes.
- [x] `crates/nvs-types/src/defaults.rs:552`: a default `= Foo::class` at a `class<T>` parameter is
  accepted and recorded the same way. The property default moved to stage 2 (*Standing decisions*).
- [x] `crates/nvs-types/tests/classes.rs:1`: the seven tests the check `checker` names, each with the
  rule id in its doc comment. Run `cargo test --test classes` once.

### 2. a written ::class at a class<T> lowers to the descriptor, with no name string

**Does:** lowers the two new `ExprInfo` entries to a descriptor, and pins the behaviour with a
conformance case.

**Split:** none, one session.

- [x] `crates/nvs-ir/src/lower/expr.rs:584`: the `ClassNameConst` arm lowers `ClassRefConst` to
  `InstKind::ClassDescConst`, and `ClassRefOf` to `self.lsb()` or `InstKind::ClassDescOf`, with no
  `Helper::ClassDescName` call. Landed with stage 1, so the tree never compiled a program it could
  not lower; no test pins it yet.
- [x] The parameter default lowers `ConstArg::ClassRef` to `InstKind::ClassDescConst`
  (`crates/nvs-ir/src/lower/call.rs` `emit_const_arg`, landed). The property default:
  - [x] `crates/nvs-types/src/defaults.rs` `eval_property_default`: call `class_reference_default`
    for an `ExprKind::ClassNameConst` the way `eval_param_default` does.
  - [x] `crates/nvs-runtime/src/object.rs:1043` `FieldDefault`: a variant carrying the class, resolved
    to its descriptor where the class image is built. Say what it spends in its doc comment.
  - [x] `crates/nvs-ir/src/lower/mod.rs:375` `field_default`: map `ConstArg::ClassRef` to it.
  - [x] A test in `crates/nvs-types/tests/classes.rs` beside `a_parameter_default_may_be_a_written_class_name`
    for the property default, and a line in the conformance case below that reads one.
- [x] `crates/nvs-ir/tests/class_reference.rs:70`: the two tests the check `lowering` names, next to
  `a_folded_class_constant_lowers_to_a_descriptor_constant`.
- [x] `tests/conformance/class/a-written-class-name-is-a-class-reference-where-one-is-expected.nvst:1`:
  `Foo::class`, `static::class` from an inherited method, and `$obj::class` each passed to a
  `class<Base>` parameter; the function calls `new $cls()` and prints the class each one built. Run the
  check `conformance` once.

### 3. the rules, the examples, the proofs and the website

**Does:** states the new behaviour in the rules, updates the examples and pages that show the `as`, and
pays every proof the goal owes.

**Split:** the examples batch (the list below) goes to one writing subagent once the rule fragments are
written, with [prose.md](../agent/prose.md) in its brief. The session writes the rules and the change
entry.

- [x] `docs/rules/types/class-reference.md:1`: "`as` is the only source" becomes "`as` and a written
  `::class` at a `class<T>` place are the two sources", with the table of *Standing decisions*, the
  default bullet and *What it spends*. Add to Declined: "A string literal or a `string` variable accepted
  at a `class<T>` place without `as`" (the reason from *Standing decisions*) and "A warning on an `as`
  that is no longer needed".
- [x] `docs/rules/types/class-constant.md:1`: one sentence saying that `::class` is a `class<C>` where
  a `class<T>` is expected, pointing at `types/class-reference`. Then `bun nv rules --render`.
- [x] The examples that write `Foo::class as class<…>` drop the `as` where a written `::class` now
  needs none, and keep it where a string arrives at run time. Each is reread and its `.out` checked:
  - `docs/examples/lang/classes/inheritance/03-a-class-the-program-picks-while-it-runs.nvs`
  - `docs/examples/lang/classes/inheritance/10-a-static-call-through-a-class-value.nvs`
  - `docs/examples/lang/classes/inheritance/11-new-through-a-class-value-checks-every-subclass.nvs`
  - `docs/examples/lang/expressions/is-new-clone-throw-print-exit-isset-empty/01-checking-what-kind-of-value-you-have.nvs`
  - `docs/examples/lang/expressions/is-new-clone-throw-print-exit-isset-empty/05-a-class-name-in-a-string-after-is.nvs`
  - `docs/examples/lang/types/callable-classes-object-shapes/03-pick-the-report-format-from-a-setting.nvs`
  - `docs/examples/lang/types/the-conversion-operator-as/05-a-class-name-read-while-the-program-runs.nvs`
  - `docs/examples/lang/types/the-conversion-operator-as/06-a-configured-class-with-a-default.nvs`
  - `docs/examples/lang/types/the-conversion-operator-as/07-a-class-name-outside-the-hierarchy.nvs`
- [x] A new example in `docs/examples/lang/types/callable-classes-object-shapes/` at the next free
  number: a function takes `class<Storable>`, and the caller passes `Invoice::class`.
- [x] `docs/examples/lang/types/callable-classes-object-shapes/about.md`, its `details.md`,
  `docs/examples/lang/types/the-conversion-operator-as/about.md`, its `details.md` and
  `docs/examples/lang/classes/inheritance/details.md` say when the `as` is needed and when it is not.
- [x] `website/src/data/changes.json`: one `changed` entry for `lang:types/callable-classes-object-shapes`
  and `lang:types/the-conversion-operator-as`: "You can pass `Foo::class` where a `class<T>` is
  expected. You do not need to write `as class<T>`. A class name in a string variable still needs `as`."
  Every page `bun nv site --check stale` names is reread and stamped (`bun nv site --stamp`).
- [x] `bun nv proofs --owed --goal class-name-as-reference` names nothing.

### 4. the goal is verified once, and every failure is fixed

- [x] The newcomer review of the change entry and the new example ([prose.md](../agent/prose.md) § *A newcomer reads it*).
- [ ] Run the check the pack names for this stage: `looper verify` at a full gate, or a build and the tests this goal wrote, by name, at a close.
- [ ] Only if it is red: fix every failure it names in one pass, never deleting or loosening a test, then run it once more, and it is green.

## Handoff

### State

Stages 1 to 3 are done. Stage 3 wrote `rule:types/class-reference` (the two sources, the table of the
five forms, the defaults, *What it spends*, two Declined bullets) and the pointer sentence in
`rule:types/class-constant`. Five examples drop the `as` on a written `::class` or a quoted default.
`07-a-class-name-outside-the-hierarchy.nvs` keeps `'Rock' as class<Animal>`, because its folder is
about `as` and a literal still needs it. The new example is
`callable-classes-object-shapes/06-pass-a-class-to-a-function.nvs`. The pages say when the `as` is
needed, the `changed` entry is in `website/src/data/changes.json`, and the conformance case now
`covers:` both features, which is what moved their behaviour hash. The old `unseen` record of
`lang:types/the-conversion-operator-as` is deleted, since a real entry covers it now.

The newcomer review of stage 4 is done: 11 texts reviewed, their findings fixed and stamped. The
re-render made 54 `lang:` perf figures stale, and they are recorded with the release binary. `site
--check stale`, `review`, `questions` and `changes` are green.

No test is expected to go red.

### Next

Stage 4 closes the goal.

- [ ] `crates/nvs-types/tests/classes.rs:1`: the build, clippy, and the `checker`, `lowering` and
  `conformance` acceptance runs by name, then `bun nv close --goal class-name-as-reference`, each red
  fixed. At a full gate, `looper verify` instead, as the pack says.
- [ ] `docs/goals/class-name-as-reference.md:1`: before `DONE`, move any decision only this file holds
  to the rule (the *Standing decisions* are already in `rule:types/class-reference`), then delete the
  file and `bun nv relink --build`.

### Backlog

- Hover on a `Foo::class` at a `class<T>` place shows `class<Foo>` (`docs/rules/ide.md`).
- `Core` parameters marked `CoreTy::ClassName` could take `class<T>` (`docs/rules/core-api/`).
- The help of `E_PARAM_DEFAULT_NOT_LITERAL` and `E_PROPERTY_DEFAULT_NOT_LITERAL`
  (`crates/nvs-types/src/defaults.rs`) lists the constants a default may be, and does not yet name a
  written `Foo::class`.
