---
looper: 1
title: callable(static) takes a function for the class the call names
summary: "Reached when a static written among a callable parameter's own parameters binds at the call to the class the call names, a call through that parameter inside the body is proven when every static argument is the called class and refused otherwise, every other use inside the body is a plain callable, the E0401 for a function that takes a subclass names callable(static) in its help, and the rules, the reference, the examples, the feature proofs and the website say so."
context: [docs/rules/statements/static-is-a-member-modifier.md, docs/rules/types/callable-variance.md, crates/nvs-types/src/signatures.rs]
stages:
  - id: 1
    title: a static among a callable parameter's parameters binds at the call, and the body proves each call through it
    status: done
    context: [crates/nvs-types/src/check.rs, crates/nvs-types/src/expr/calls.rs, crates/nvs-types/src/locals.rs, docs/rules/types/callable-values.md]
  - id: 2
    title: "the E0401 for a function that takes a subclass says what to write, and the conformance cases pin the feature"
    status: done
    context: [crates/nvs-types/src/expr/args.rs, crates/nvs-types/src/expr/assign.rs, crates/nvs-diagnostics/src/code/types.rs, tests/conformance/lang/a-static-parameter-type-is-the-class-the-call-names.nvst]
  - id: 3
    title: "the rules, the reference, the examples, the feature proofs and the website say so"
    status: done
    context: [docs/examples/lang/types/void-never-self-static/about.md, docs/examples/lang/types/void-never-self-static/details.md, docs/examples/lang/classes/methods-self-static-and-parent/about.md]
  - id: 4
    title: "the goal is verified once, and every failure is fixed"
    status: done
    gate: true
checks:
  - id: binds
    stage: 1
    name: "a static among a callable parameter's own parameters binds at a static and at an instance call, a nested one does not, and a call through the parameter is proven or refused by what each static argument is"
    run: [cargo, test, --test, static_callables]
    tests: [a_static_in_a_callable_parameter_binds_at_a_static_call, a_static_in_a_callable_parameter_binds_at_an_instance_call, a_nullable_callable_parameter_binds_too, a_static_in_a_nested_callable_or_an_array_of_callables_does_not_bind, a_static_in_a_returned_callable_does_not_bind, a_call_with_this_new_static_or_a_static_return_is_proven, a_call_with_another_value_is_refused, a_call_inside_an_anonymous_function_is_a_plain_callable_call, a_parameter_the_body_assigns_to_is_a_plain_callable, the_parameter_passed_on_is_a_plain_callable, an_override_may_redeclare_a_callable_static_parameter]
    timeout: 1800
  - id: help
    stage: 2
    name: "a function for a subclass at a callable parameter gets help naming callable(static) or the class to call the method on"
    run: [cargo, test, --test, static_callables, a_function_for_a_subclass_]
    tests: [a_function_for_a_subclass_at_the_declaring_class_names_callable_static, a_function_for_a_subclass_at_a_base_class_call_names_the_class_to_call_on, a_function_for_a_subclass_at_a_core_member_gets_no_new_help]
    timeout: 1800
  - id: cases
    stage: 2
    name: "the conformance cases for the bound parameter, its refusals and the run-time check of a passed-on function are green"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/lang/a-callable-static-parameter-takes-a-function-for-the-class-the-call-names.nvst, --case, tests/conformance/reject/a-call-through-a-callable-static-parameter-needs-the-called-class.nvst, --case, tests/conformance/callables/a-callable-static-parameter-passed-on-checks-its-argument-when-called.nvst]
    timeout: 1800
  - id: proofs
    stage: 3
    name: "every feature the goal's work reached has its feature proofs"
    run: [bun, nv, proofs, --gate, --goal, static-in-callables]
    timeout: 1800
  - id: close
    stage: 4
    name: "the goal-end check is green: ci --check's checks of the tree's files, the owners and the goal's proofs"
    run: [bun, nv, close, --goal, static-in-callables]
    timeout: 900
---

# callable(static) takes a function for the class the call names

## Why

A user reported this shape. A base class `Model` has a static method that takes a function, and a
subclass `Order` calls it with a function written for `Order`:

```nvs
abstract class Model
{
    public static function check(callable(static): ?string $rule): void { … }
}

class Order extends Model { … }

Order::check(fn(Order $order): ?string => …);
```

Every typed form of the parameter is refused, and the user ends up writing a plain `callable`:

| The parameter is typed | `fn(Order $order)` passed in |
|---|---|
| `callable(Model): ?string` | `E0401`. This is correct: the body may call it with any `Model` (`rule:types/callable-variance`). |
| `callable(static): ?string` | `E0401` too, because a `static` among a callable's parameters keeps the declaring class |
| plain `callable` | accepted. Each call checks the argument's class at run time, and the result is `mixed` |

The second row is a choice of ours, not a soundness limit. `rule:statements/static-is-a-member-modifier`
binds `static` at the call in every position of a signature except a written callable's parameters
(`crates/nvs-types/src/signatures.rs:2585`). The reason is the body: it types `static` as the declaring
class, so it could call the function with a `Model` that is not an `Order`. The way out the user takes
is worse on every count. Plain `callable` moves the class check from compile time to every call
(`nvs_runtime::callable::check_param_tags`), and its result is `mixed`, which the caller must convert
(`crates/nvs-types/src/expr/calls.rs:2706`).

A sound answer exists, and it follows the one `property<static>` already takes. The call binds the
parameter. The body sees the parameter as a plain `callable`, as it sees a `property<static>` as a
`string` (`crates/nvs-types/src/check.rs:1451`). A direct call through the parameter in the method's
own body is proven when every argument at a `static` position is one E0741's list already accepts as
the called class (`yields_the_called_class`, `crates/nvs-types/src/check.rs:1627`), and refused
otherwise. The user asked for this goal on 2026-11-24.

## Standing decisions

**Where `static` binds.** `static` binds at the call when it is written directly among the parameters
of the callable that is the method parameter's own type: `callable(static)`, `callable(static, int)`,
`?callable(static)` and `callable(static)|null`. It binds to the class the call names, at
`Order::check(...)` and at `$order->check(...)` alike, as every other position does. `static_use`
(`signatures.rs:2588`) gains a `in_callable` flag for that case. Every deeper position keeps today's
meaning and keeps the declaring class: an `array<callable(static)>`, a shape field, a callable among a
callable's parameters, a `property<static>` inside a callable, and a written callable in a return type.
`bind_static`'s `CallableSig` arm (`signatures.rs:2729`) rewrites the parameters too, which it does not
today. Default if `bind_static` cannot tell a bound parameter from a written class name: the
`blocked` rule of `static_use` already refuses a type that also writes the declaring class's name, so
the rewrite only ever sees `static`.

**What the body sees.** Inside the body the parameter is a plain `callable`. Every use is then sound
with no new rule: passing it on, storing it, capturing it and assigning it all keep the run-time check
of a plain `callable`. A `callable(Model)` slot does not take it without `as`, which is checked at run
time. A new helper beside `static_key_as_text` (`signatures.rs:2737`) gives the body type, and
`check.rs:1455` calls it. The session checks that `nvs_ir`'s own lowering of the parameter slot
(`nvs_ir::lower::lower_method`) agrees with the checker, as the comment at `check.rs:1466` requires.

**The one proven use: a direct call in the method's own body.** The scope keeps, for each bound
parameter, the declaring class's signature and which of its positions are `static`. A call
`$rule(...)` written in the method's own body checks each argument against that signature. Each
argument at a `static` position must pass `yields_the_called_class`: `$this`, `new static(...)`, `null`
where the position is nullable, or a call on `static`, `self`, `parent` or `$this` whose return binds
`static`. A call that passes is recorded as `ExprInfo::CallThroughSignature` (`calls.rs:2764`), so it
returns the signature's type and pays no run-time check. A call that fails is a new error, below.

- **A call inside an anonymous function is a plain `callable` call.** `self::m()` there names the
  written class, not the called one (`rule:statements/static-is-a-member-modifier`), so
  `yields_the_called_class` would accept a value that is not the called class. The record is not
  visible inside a nested anonymous function. The call is checked at run time and returns `mixed`.
- **A parameter the body assigns to is a plain `callable` everywhere.** After a write the value is
  no longer the function the caller passed. The checker finds a write before it checks the body, with
  the walk the body already uses to find written variables, or a walk over assignments and `inout`
  arguments to the parameter's name if there is none. No record is kept for that parameter.

**The new error.** `E1015` in `crates/nvs-diagnostics/src/code/types_e10.rs`, the next free number
the code table allows (the `E07xx` range beside E0741 is full),
`E_STATIC_CALLBACK_ARGUMENT_NOT_CALLED_CLASS`. Draft text, held
to AGENTS.md § *Text an end user reads*:
- message: "this argument may not be the class the call named"
- primary: "the method was called on a class that may be a subclass of this value's class"
- secondary at the parameter: "`$rule` takes the class the call named"
- help: "Pass `$this`, `new static(...)` or the result of a call that returns `static`. To pass any
  other value, declare the parameter as `callable`."

**Help on E0401 (stage 2).** The E0401 for a function argument at a callable parameter gets help when
the function's parameter is a class strictly below the class the slot takes, and the method is a
user declaration. `Core` members keep today's message.
- Where the parameter writes the declaring class's own name or `self`: "This function takes only
  `Order`. Declare the parameter as `callable(static)`. Then a call on `Order` takes a function for
  `Order`."
- Where the parameter writes `static` and the call named a class above the function's: "This function
  takes only `Order`. Call the method on `Order`, for example `Order::check(...)`." At an instance
  call the example is left out.

**Overrides.** An override may redeclare a `callable(static)` parameter, or declare the base's class
there. Both stay sound, because the body calls the function only with the called class, which is below
every class in the chain. The test `an_override_may_redeclare_a_callable_static_parameter` pins it; the
session reads how `compare_override` (`crates/nvs-types/src/conformance.rs:879`) compares the two and
changes nothing unless the test is red.

**Declined, and written into the rule's Declined list.**
- **Keeping the declaring class.** It is today's answer. It sends the user to a plain `callable`,
  which adds a run-time check to every call and returns `mixed`.
- **Taking `callable(Order)` where `callable(Model)` is declared, with a run-time check.** A typed
  callable would then no longer promise that every `Model` works. Plain `callable` already says that.
- **Binding `static` in an array of callables, a shape field or a nested callable.** An element read
  out of the array is no longer the parameter the scope keeps a record for, so no call through it can
  be proven. Default until a program needs it.
- **Binding `static` in a callable return type.** E0741 checks a bound return for the called class,
  and a function is not an instance. The return keeps the declaring class.
- **Proving a call inside an anonymous function.** `self::` there names the written class, so the
  proof would need a second list. The call is checked at run time.

**Cost.** Nothing at run time and nothing per request. A proven call does less work than the plain
`callable` the user writes today: no `check_param_tags` per call, which the `CallThroughSignature`
record counts (`rule:programs/less-work-is-taken`). At check time, one small record per bound
parameter for the length of one method body. Simplicity: the rule loses an exception, since `static`
then binds in every first-level position of a parameter's type. It gains one proven call shape, which
reuses E0741's list.

**Feature proofs.** The feature is `static` in a callable parameter. It owes what `bun nv proofs
--owed --goal static-in-callables` names. At least: a sixth example in
`docs/examples/lang/types/void-never-self-static/`, an attack that tries to call the bound function
with a base-class object and to call it after storing it, and a bench of a proven call against the
same call through a plain `callable`.

**What stops it.** One attempt per item and two sessions per stage. An item that cannot land in one
attempt is recorded in the handoff with what failed, and the stage goes on.

**Lanes.** The stages run in order: stage 2 reads the binding stage 1 adds, and stage 3 describes both.
Nothing here touches the files of the goals ahead of it on the chain, except
`crates/nvs-types/src/check.rs`, which `extend-blocks` may also edit; this goal runs after it.

## Stages

### 1. a static among a callable parameter's parameters binds at the call, and the body proves each call through it

- [x] `crates/nvs-types/src/signatures.rs:2680`: `static_use` takes `param` and reports
  `in_callable` for a `static` directly among the parameters of the top-level callable of a parameter
  type, through `Nullable`, `Paren` and a union with `null` only (`top_callable`). Every deeper
  `static` among a callable's parameters stays `blocked`. The doc comment says both.
- [x] `crates/nvs-types/src/signatures.rs:1851`: `params_bind_static` is true for an `in_callable`
  parameter, and `bind_static`'s `CallableSig` arm rewrites the parameters as well as the return.
- [x] `crates/nvs-types/src/check.rs:1523`: the body types an `in_callable` parameter as plain
  `callable` (`static_callback_as_callable`), and `LocalScope::static_callbacks`
  (`crates/nvs-types/src/locals.rs`, `StaticCallback`) keeps its declaring-class signature and
  `static` positions, unless `locals::body_writes` finds a write. `nvs_ir::lower::lower_method_of`
  lowers both `callable` and a written signature to `Ty::Object`, so it agrees.
- [x] `crates/nvs-types/src/expr/calls.rs`: `check_call_through_static_callback`, called from
  `expr/mod.rs`'s `ExprKind::Call` arm, checks the arguments, requires `yields_the_called_class` at
  each `static` position, and records `CallThroughSignature`. An anonymous function has a fresh
  scope, so it never sees the record.
- [x] `crates/nvs-diagnostics/src/code/types_e10.rs`: `E1015`
  `E_STATIC_CALLBACK_ARGUMENT_NOT_CALLED_CLASS`, with its card.
- [x] `crates/nvs-types/tests/static_callables.rs:1` (new): the 11 tests the check `binds` names,
  green.

### 2. the E0401 for a function that takes a subclass says what to write, and the conformance cases pin the feature

**Split:** the three conformance cases go to one writing subagent once the help lands. Its brief names
the three paths, the neutral names `Model` and `Order`, and AGENTS.md § *Text an end user reads* for
their comments.

- [x] The argument check in `crates/nvs-types/src/expr/args.rs` that reports E0401 for a call's
  argument (found with `bun nv peek --locate`): the two help texts of the standing decision, only for a
  user declaration. `crates/nvs-types/src/expr/assign.rs:779`'s `report_mismatch` stays as it is.
- [x] `crates/nvs-types/tests/static_callables.rs`: the tests the check `help` names, then `cargo test
  --test static_callables a_function_for_a_subclass_` once.
- [x] `tests/conformance/lang/a-callable-static-parameter-takes-a-function-for-the-class-the-call-names.nvst:1`
  (new): a static call, an instance call, a nullable parameter, and calls with `$this`, `new static()`
  and a `static`-returning call, each printing what the function returned.
- [x] `tests/conformance/reject/a-call-through-a-callable-static-parameter-needs-the-called-class.nvst:1`
  (new): a call with a base-class object and with a local variable gets the new code, and
  `Model::check(fn(Order $o) => …)` gets E0401 with the help naming `Order`.
- [x] `tests/conformance/callables/a-callable-static-parameter-passed-on-checks-its-argument-when-called.nvst:1`
  (new): the body stores the function in an `array<callable>` and later calls it with another
  subclass; the call throws `LogicError` naming the class.
- [x] The check `cases` run once.

### 3. the rules, the reference, the examples, the feature proofs and the website say so

**Split:** the example, the attack and the bench go to one writing subagent once the rule is
rewritten. The session writes the rules, the docs and the website entry.

- [x] `docs/rules/statements/static-is-a-member-modifier.md:15`: the paragraph says `static` binds
  among a callable parameter's own parameters, the body sees a plain `callable`, a direct call is
  proven by E0741's list, and the new code. The Declined list gains the five items above, and
  § *Memory* says a bound callable parameter costs nothing at run time.
- [x] `docs/rules/types/callable-variance.md:13`: one sentence after the two refusals pointing at
  `callable(static)` for a function that takes the class the call names.
- [x] `crates/nvs-types/src/check.rs`: `yields_the_called_class`'s doc names its second user.
- [x] `docs/examples/lang/types/void-never-self-static/about.md:9` and `details.md:1`: one short
  paragraph on `callable(static)`, written to AGENTS.md § *Text an end user reads*.
  `docs/examples/lang/classes/methods-self-static-and-parent/about.md:1` gets one sentence if it lists
  where `static` may be written.
- [x] `docs/examples/lang/types/void-never-self-static/06-a-check-for-the-class-the-call-names.nvs:1`
  and its `.out`.
- [x] The attack and the bench `bun nv proofs --owed --goal static-in-callables` names, at the paths
  it prints.
- [x] The generated reference pages (`docs/reference/lang/20-types.md`, `50-classes.md`) rebuilt by
  the command that writes them, never by hand.
- [x] `website/src/data/changes.json:1`: one `changed` entry for `static` in a callable parameter.
  Each page above reread and stamped with `bun nv site --stamp`.
- [x] `bun nv proofs --owed --goal static-in-callables` names nothing.

### 4. the goal is verified once, and every failure is fixed

- [x] The newcomer review of docs/agent/prose.md § *A newcomer reads it* over the pages stage 3 touched.
- [x] Run the check the pack names for this stage: `looper verify` at a full gate, or a build and the tests this goal wrote, by name, at a close.
- [x] Only if it is red: fix every failure it names in one pass, never deleting or loosening a test, then run it once more, and it is green.

## Handoff

### State

Stages 1 to 3 are done and committed. The rule paragraph is at
`docs/rules/statements/static-is-a-member-modifier.md:24`, with the five Declined items and the
§ *Memory* line; `callable-variance.md:13` points at it. Example 06 is in
`docs/examples/lang/types/void-never-self-static/`, and two attacks are under
`tests/hostile/lang/types/void-never-self-static/`: `02-…` throws at run time on a stored function,
`03-…` is `expect-refusal` (two E1015 and the E0401 with help). A hostile case cannot assert both a
refusal and a run-time throw, so they are two files. No second bench was written: the feature
`lang:types/void-never-self-static` already has its one bench, and `--owed` named nothing.
`docs/reference/lang/20-types.md` was re-rendered, `changes.json` has its entry, the pages are
stamped, and `--record-perf --goal` re-measured the 20-types benches the re-render made stale.

Stage 4 is done. The newcomer review read example 06, `about.md`, `details.md`, both attacks and
the `changes.json` entry. Its findings were fixed: the entry and example 06 now say what "the class
the call names" is before they use it, and `details.md` says a stored or passed-on `$rule` is a
`callable` with no parameter types. The texts and `syntax/functions/declaring.mdx` are restamped,
`site --check stale` is green, the build is green and `static_callables` passes 14 of 14.

The nightly run 45609291264 ran on `53e94a9181`, before `1c3196b0e8` and `8ca2f09e1b`. Each of
its ten failed jobs is one of the failures those two commits fixed: the ten Miri `Core\Time` tests
now carry the Miri ignore, the quotient attack compiles, and macOS has its json-api count.

### Next

The goal's DONE runs `binds`, `help`, `cases`, `proofs` and `close`. No test is expected to go red.

### Backlog
