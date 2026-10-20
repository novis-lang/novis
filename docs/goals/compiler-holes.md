---
looper: 1
title: "Every program the checker accepts compiles and runs as its rules say"
summary: "Every gap record this goal owns is closed and deleted: no safe program reads a value as another type, every form the checker accepts lowers, every form a rule does not allow is a compile error where it is written, and the checks below pass."
context: [docs/rules/programs/memory-priority.md]
stages:
  - id: 1
    title: "a safe program never reads a value as another type"
    status: done
    context: [crates/nvs-types/src/check.rs, crates/nvs-types/src/ctor_init.rs, crates/nvs-types/src/expr/isolate.rs, crates/nvs-types/src/expr/operators.rs]
  - id: 2
    title: "an override keeps a signature its parent's callers can use"
    status: open
    context: [crates/nvs-types/src/conformance.rs]
  - id: 3
    title: "only the throwable tree is thrown, caught and asserted"
    status: open
    context: [crates/nvs-types/src/conformance.rs, crates/nvs-stdlib/src/test.rs]
  - id: 4
    title: "visibility and the property pipeline hold on every path"
    status: open
    context: [crates/nvs-runtime/src/object.rs, crates/nvs-types/src/expr/members.rs, crates/nvs-types/src/program.rs, crates/nvs-ir/src/lower/mod.rs]
  - id: 5
    title: "every expression the checker accepts lowers"
    status: open
    context: [crates/nvs-ir/src/lower/expr.rs, crates/nvs-ir/src/lower/control.rs, crates/nvs-types/src/expr/operators.rs, crates/nvs-types/src/expr/members.rs]
  - id: 6
    title: "generators and `finally` run each block once and lose no error"
    status: open
    context: [crates/nvs-ir/src/lower/generator.rs, crates/nvs-syntax/src/parser/stmt.rs]
  - id: 7
    title: "a class declaration is checked where it is written"
    status: open
    context: [crates/nvs-types/src/signatures.rs, crates/nvs-syntax/src/parser/decl.rs, crates/nvs-types/src/expr/members.rs]
  - id: 8
    title: "enums, type arguments and property hooks are checked where they are written"
    status: open
    context: [crates/nvs-types/src/enums.rs, crates/nvs-syntax/src/parser/ty.rs, crates/nvs-types/src/check.rs, crates/nvs-types/src/attributes.rs]
  - id: 9
    title: "attributes and derived JSON codecs are checked where they are written"
    status: open
    context: [crates/nvs-types/src/attributes.rs, crates/nvs-types/src/derive.rs, crates/nvs-types/src/routes.rs, crates/nvs-stdlib/src/json.rs]
  - id: 10
    title: "operators and locals give what their rules say"
    status: open
    context: [crates/nvs-types/src/expr/operators.rs, crates/nvs-types/src/locals.rs, crates/nvs-types/src/expr/mod.rs, crates/nvs-types/src/expr/assign.rs]
  - id: 11
    title: "anonymous functions, commands and tests take only what can run"
    status: open
    context: [crates/nvs-types/src/expr/calls.rs, crates/nvs-types/src/expr/assign.rs, crates/nvs-types/src/commands.rs, crates/nvs-types/src/testing.rs]
  - id: 12
    title: "a route link matches its route, and an unread body never costs the response"
    status: open
    context: [crates/nvs-types/src/routes.rs, crates/nvs-runtime/src/routes.rs, crates/nvs-stdlib/src/router.rs, crates/nvs-server/src/body.rs, crates/nvs-server/src/serve.rs]
  - id: 13
    title: "every help text names only what compiles, and each syntax is one feature"
    status: open
    context: [crates/nvs-types/src/expr/members.rs, crates/nvs-types/src/expr/assign.rs, crates/nvs-types/src/expr/isolate.rs, crates/nvs-types/src/locals.rs, tools/nv/proofs/roster.ts]
  - id: 14
    title: "`Core\\BigInt`'s gcd and lcm are sub-quadratic"
    status: open
    context: [crates/nvs-stdlib/src/bigint.rs]
  - id: 15
    title: "`Core\\BigInt`'s modular power is sub-quadratic per step"
    status: open
    context: [crates/nvs-stdlib/src/bigint.rs]
checks: []
---

# Every program the checker accepts compiles and runs as its rules say

## Why

The sessions that wrote the examples' `details.md` pages probed every feature from outside, and
recorded each place where the compiler, the runtime or the library disagrees with its rule. Those
69 gap records went to `release-hardening`, whose stages were done before they were built, and
`bun nv owners --hand-over release-hardening compiler-holes` moved them here. Most are checker or
lowering holes: a panic, an internal `E0901`, or a program that compiles and should not. Seven are
type confusions in safe code, which `## The priority ordering` puts above everything else, so they
come first.

## Standing decisions

**Where a gap gives two ways to close it, the choice is made here.** The default is the compile error
where the form is written, because a smaller language surface is simpler and an error at the site is
safer than a form that does nothing. Each choice below is written into the rule the gap cites when the
stage lands, so the rule and the binary agree. The forms that are made to work instead:

- A `yield` of a literal in an `Iterator<?int>`, `Iterator<?string>` or `Iterator<int|string>` compiles and yields.
- A negative literal compared with a `uint` gives `false`, as `rule:types/ordering` already says.
- Arithmetic on an `int` narrowed by `==` keeps the type `int`.
- A `null` value in a `#[TestWith]` row fills a `?T` parameter with `null`.
- An array literal in an attribute's payload is checked against the field's declared array type.
- A positional command parameter is checked like an option, and a trailing `array<string>` collects the remaining words, as `rule:tooling/a-parameter-is-an-argument-unless-it-is-an-option` says.
- A JSON list where a derived class expects an object gives the object issue.
- `#[Core\Json\Derive()]` with an empty list is the same attribute as `#[Core\Json\Derive]`. An unknown option in it is a compile error.

**The forms that become compile errors, each at the site:** a `catch` variable that redeclares a
local (`E0406`), `echo` or `.` of a `class<T>` (the help names `as string`), `is class<T>` (the help
names `as ?class<T>`), a `void` call at `mixed` or as a `match` arm, a `yield` inside a `finally`
(its own code), `final` on a class constant (`E0106`), a `bool` route capture (`E0752`, whose help
already leaves `bool` out), a constructor reference to an enum, a user attribute on an enum, an enum
case or a class constant (no retrieval reaches it), a variadic or a default on an anonymous
function's parameter, a `Core\Task::all` field whose function takes a parameter, a `Core\Json\Derive`
class with no written constructor, and a `private` or `protected` constructor reached through
`Core\Program::implementing` or `implementingWith` at the call.

**A `throw` that leaves a `finally` while another error is in flight** is not refused. The new
error gets the error in flight as its `previous` when its own `previous` is `null`, whether a
`throw` statement or a call in the `finally` threw it. When its `previous` is already set, the new
error leaves unchanged, and `rule:statements/no-return-leaves-a-finally` states that bound.

**An ordering of two objects checks their run-time classes.** `$a < $b` on two objects throws a
`LogicError` when their run-time classes differ, before `compareTo` runs. A static refusal would
refuse every ordering of a class that is not `final`. The cost is one class comparison per object
ordering, on the request path only where a program orders objects.

**An override that a call through the parent could not use is a diagnostic at the override.** The
return type is the parent's or a subtype of it. Each parameter type is the parent's or a supertype
of it. The parameter count is the parent's exactly: an extra parameter with a default ran with
nothing in it through the parent, because a call fills defaults from the signature it resolved.
The visibility is not narrower, `static` is the same, and a `tainted` or `secret` qualifier on a
parameter is kept. The built-in interfaces (`Stringable`, `Comparable`, `Parses`, `Iterator<T>`) are
held to the same check.

**The server closes a connection only after the response is flushed.** An unread body is drained up
to the bound it has today and never further, so a client cannot make the server read more of its
bytes. The header line limit is set in `serve_connection` at `hyper`'s current default of 100, as a
named constant beside the other request-head limits, with no new setting.

**`lang:errors/try-catch-finally` and `lang:errors/throw` are the ids that stay.** The
`lang:statements/` twins are merged into them: their facts move into the kept `details.md`, their
examples move or are deleted, and every citation is rewritten.

**Each closed gap deletes its record** under `data/gaps/` in the slice that builds it, with a
conformance case in the area its rule names under `tests/conformance/`. A help text a stage rewrites
is held to AGENTS.md § *Text an end user reads*, and every expectation that pins it is re-recorded.

**Lanes.** Stages 1 to 13 share the checker's files and run in order on the chain. Stages 14 and 15
touch only `crates/nvs-stdlib/src/bigint.rs` and its benches, so they can run as a side goal beside
the chain. No stage waits on another goal.

**Trade-offs.** Every stage but 1 and 12 is compile-time work: no program that runs today runs
slower. Stage 1 adds one class comparison to an object ordering. Stage 12 holds no new memory, since
the drain bound stays. Programs that compiled and were wrong stop compiling, which is the point.
Simplicity of the language gains: twenty-odd forms that did nothing now have one meaning or none.

## Stages

### 1. a safe program never reads a value as another type

**Does:** each of the five type confusions below is a compile error or a thrown error, so no safe
program reads memory as the wrong type.

- [x] A `set` hook whose parameter type does not accept its property's type is a diagnostic at the hook. `crates/nvs-types/src/check.rs:1`, `tests/conformance/reject/a-set-hook-parameter-accepts-its-property-type.nvst:1`
- [x] A hooked property that a path can leave unwritten is `E0409` or `E0428`, as a plain property is. `crates/nvs-types/src/ctor_init.rs:1`, `tests/conformance/reject/a-hooked-property-is-assigned-before-it-is-read.nvst:1`
- [x] A read of a property through `$this->` in the constructor before every path assigns it is a diagnostic. `crates/nvs-types/src/ctor_init.rs:1`, `tests/conformance/reject/a-constructor-reads-a-property-only-after-assigning-it.nvst:1`
- [x] A non-static method at a `spawn script` site is a diagnostic that names the static form. `crates/nvs-types/src/expr/isolate.rs:1`, `tests/conformance/reject/a-spawn-entry-method-is-static.nvst:1`
- [x] An ordering of two objects of different run-time classes throws before `compareTo` runs, with a case for the interface-typed pair and the subclass override. `crates/nvs-ir/src/lower/operator.rs:1`, `tests/conformance/class/an-ordering-of-two-objects-of-different-classes-throws.nvst:1`

### 2. an override keeps a signature its parent's callers can use

**Does:** a subclass, an implementing class and a filled-in `abstract` method are held to the
override check in § *Standing decisions*, and so are the built-in interfaces.

**Split:** at stage start, an `Explore` subagent lists every built-in interface's methods and their
signatures, as `file.rs:NN` and one line each. That list is the last item's set.

- [x] An override with another return type, other parameter types, another parameter count, a narrower visibility or another `static` is a diagnostic at the override, with a case for each form. `crates/nvs-types/src/conformance.rs:1`, `tests/conformance/reject/a-method-override-keeps-its-parents-signature.nvst:1`
- [x] The built-in interfaces' methods are held to the same check: `toString`, `compareTo`, `parse` and `current`. `crates/nvs-types/src/conformance.rs:1`, `tests/conformance/reject/a-built-in-interface-method-keeps-its-signature.nvst:1`
- [x] An `abstract` method with a body is a diagnostic. `crates/nvs-types/src/conformance.rs:1`, `tests/conformance/reject/an-abstract-method-has-no-body.nvst:1`
- [ ] A direct `compareTo` call through `Comparable` cannot pass an object of another class to a `self` parameter. `crates/nvs-ir/src/lower/operator.rs:1`, `data/gaps/nvs-types/a-direct-compareto-call-through-comparable.json:1`

### 3. only the throwable tree is thrown, caught and asserted

**Does:** `implements`, `catch`, `throw` and `Core\Test::assertThrows` each take only what the
throwable tree can be.

- [ ] `implements` with any class, built-in or not, is `E0303`. `crates/nvs-types/src/conformance.rs:1`, `data/gaps/nvs-types/implements-with-a-class-of.json:1`
- [ ] A `catch` that names a class or a built-in type no `Throwable` can be is a diagnostic. An interface stays allowed. `crates/nvs-types/src/conformance.rs:1`, `data/gaps/nvs-types/a-catch-clause-that-names-a.json:1`
- [ ] A `throw` of any value that is not `Throwable`, a shape included, is `E0780`. `crates/nvs-types/src/conformance.rs:1`, `data/gaps/nvs-types/a-throw-of-an-anonymous-object.json:1`
- [ ] `Core\Test::assertThrows`'s second argument is a `class<Throwable>`, and anything else is a compile error. `crates/nvs-stdlib/src/test.rs:1`, `data/gaps/nvs-stdlib/assert-throws-with-a-class-that.json:1`

### 4. visibility and the property pipeline hold on every path

**Does:** a `private` or `protected` member is reached from where its visibility allows and nowhere
else, and every write of a property runs the same guard and observer pipeline.

- [ ] An erased read or write through `object` or `mixed` of a property not visible at the access site throws, as a missing name does. `rule:types/erased-member-access` says so. `crates/nvs-runtime/src/object.rs:1`, `data/gaps/nvs-runtime/a-private-or-protected-property-is.json:1`
- [ ] A read of a `private` class constant outside its class, or of a `protected` one outside the class and its subclasses, is a diagnostic. `crates/nvs-types/src/expr/members.rs:1`, `data/gaps/nvs-types/a-private-or-protected-class-constant.json:1`
- [ ] `Core\Program::implementing` and `implementingWith` over a class with a `private` or `protected` constructor is a diagnostic at the call. `crates/nvs-types/src/program.rs:1`, `data/gaps/nvs-types/core-program-implementing-calls-a-private.json:1`
- [ ] A promoted constructor parameter is stored through the same pipeline as `$this->x = $x;`, with a case for the guard and the observer. `crates/nvs-ir/src/lower/mod.rs:1`, `data/gaps/nvs-ir/a-promoted-constructor-parameter-runs-no.json:1`
- [ ] A `??=` that keeps the property's value calls no guard. `crates/nvs-ir/src/lower/mod.rs:1`, `data/gaps/nvs-ir/a-coalescing-assignment-to-a-property.json:1`

### 5. every expression the checker accepts lowers

**Does:** none of the seven forms below reaches a lowering panic or `E0901`. Each is either a
diagnostic where it is written or lowered, as § *Standing decisions* says.

- [ ] A `catch` variable that redeclares a local is `E0406`, with a case at the top level and in a method. `crates/nvs-ir/src/lower/expr.rs:235`, `data/gaps/nvs-ir/a-catch-variable-that-reuses-a.json:1`
- [ ] `echo` and `.` of a `class<T>` are a diagnostic whose help names `as string`. `crates/nvs-ir/src/lower/expr.rs:1146`, `data/gaps/nvs-ir/a-class-reference-in-echo-or.json:1`
- [ ] `is class<T>` is a diagnostic whose help names `as ?class<T>`. `crates/nvs-ir/src/lower/expr.rs:6280`, `data/gaps/nvs-ir/an-is-test-against-a-class-reference.json:1`
- [ ] A `void` call at `mixed`, or as a `match` arm, is a compile error. `crates/nvs-ir/src/lower/control.rs:1`, `data/gaps/nvs-ir/a-void-call-at-a-mixed.json:1`
- [ ] A static property reached through `->` or `?->` is a diagnostic naming `P::$x`. `crates/nvs-types/src/expr/members.rs:1`, `data/gaps/nvs-types/a-static-property-read-through-an.json:1`
- [ ] Ordering a `bool` against a number, a chained `1 < 2 < 3` included, is `E0715`. `crates/nvs-types/src/expr/operators.rs:1`, `data/gaps/nvs-types/ordering-a-bool-against-a-number.json:1`
- [ ] A `callable` after `echo` or `.` is `E0707`. `crates/nvs-types/src/expr/operators.rs:1`, `data/gaps/nvs-types/echo-of-a-callable-compiles-and.json:1`

### 6. generators and `finally` run each block once and lose no error

**Does:** a generator finishes when its body throws, a `finally` runs once, and an error in flight
is kept as § *Standing decisions* says.

- [ ] A `yield` of a literal in a nullable or union element type compiles and yields, with a case for each element type. `crates/nvs-ir/src/lower/generator.rs:1`, `data/gaps/nvs-ir/a-generator-of-a-nullable-or.json:1`
- [ ] A generator whose body throws is finished: a later `advance()` returns `false`, and its `finally` runs once. `crates/nvs-ir/src/lower/generator.rs:1`, `data/gaps/nvs-ir/a-generator-that-throws-is-not.json:1`
- [ ] A `yield` inside a `finally` is a compile error with its own code. `crates/nvs-ir/src/lower/generator.rs:1`, `data/gaps/nvs-ir/a-yield-inside-a-finally.json:1`
- [ ] An error that leaves a `finally` while another is in flight carries it as `previous`, and the rule states the bound. `crates/nvs-syntax/src/parser/stmt.rs:1`, `data/gaps/nvs-syntax/a-throw-inside-a-finally.json:1`

### 7. a class declaration is checked where it is written

**Does:** each of the six declarations below is a diagnostic at the declaration.

- [ ] A second property, method or constructor of one name in one class, a promoted parameter included, is a diagnostic at the second one. `crates/nvs-types/src/signatures.rs:1`, `data/gaps/nvs-types/a-class-member-declared-twice-passes.json:1`
- [ ] `static` on a `constructor` is a diagnostic. `crates/nvs-types/src/signatures.rs:1`, `data/gaps/nvs-types/a-static-constructor-compiles-and-new.json:1`
- [ ] A `...` on any parameter but the last is a diagnostic at that parameter. `crates/nvs-types/src/signatures.rs:1`, `data/gaps/nvs-types/a-variadic-parameter-that-is-not.json:1`
- [ ] A `void` or `never` property is `E0742`. `crates/nvs-types/src/signatures.rs:1`, `data/gaps/nvs-types/a-void-or-never-property-passes.json:1`
- [ ] `final` on a class constant is `E0106`. `crates/nvs-syntax/src/parser/decl.rs:1`, `data/gaps/nvs-syntax/final-on-a-class-constant-is.json:1`
- [ ] A class constant's value is checked against its declared type at the declaration. `crates/nvs-types/src/expr/members.rs:1`, `data/gaps/nvs-types/a-class-constant-whose-value-does.json:1`

### 8. enums, type arguments and property hooks are checked where they are written

**Does:** each of the six forms below is a diagnostic at the form.

- [ ] A second case of one name in one enum is a diagnostic at the second one. `crates/nvs-types/src/enums.rs:1`, `data/gaps/nvs-types/an-enum-case-declared-twice-compiles.json:1`
- [ ] `new` naming an enum is one checker error that says a case is written `M::Case`. `crates/nvs-types/src/enums.rs:1`, `data/gaps/nvs-types/new-on-an-enum-passes-the.json:1`
- [ ] A constructor reference to an enum, and a user attribute on an enum, an enum case or a class constant, are checker errors. `crates/nvs-types/src/attributes.rs:1`, `data/gaps/nvs-types/a-constructor-reference-to-an-enum.json:1`
- [ ] A `<` after a built-in type name is an error at the `<` that names the generic types. `crates/nvs-syntax/src/parser/ty.rs:1`, `data/gaps/nvs-syntax/type-arguments-after-a-built-in-type.json:1`
- [ ] A hook on a `static` property is a diagnostic, as one on a `readonly` property is `E0843`. `crates/nvs-types/src/check.rs:1`, `data/gaps/nvs-types/a-hook-on-a-static-property.json:1`
- [ ] A second hook of one kind on one property is a diagnostic at the second hook. `crates/nvs-types/src/check.rs:1`, `data/gaps/nvs-types/a-property-with-two-get-hooks.json:1`

### 9. attributes and derived JSON codecs are checked where they are written

**Does:** a recognized attribute is accepted only where it builds something, and a derived codec
reads and writes each wire key once.

- [ ] A recognized attribute at a site its rule does not name is a checker error, and `Derive()` follows § *Standing decisions*. `crates/nvs-types/src/attributes.rs:1`, `data/gaps/nvs-types/a-compiler-read-attribute-on-a.json:1`
- [ ] An array literal in a named attribute is checked against the field's declared array type. `crates/nvs-types/src/attributes.rs:1`, `data/gaps/nvs-types/an-array-literal-in-an-attribute.json:1`
- [ ] Each contradiction `rule:attributes/api-adds-and-cannot-contradict` names is `E0771`, a second `#[Core\Api]` on one method included. `crates/nvs-types/src/routes.rs:1`, `data/gaps/nvs-types/core-api-accepts-what-its-rule.json:1`
- [ ] A `#[Core\Json\Derive]` class with no written constructor is a checker error. `crates/nvs-types/src/derive.rs:1`, `data/gaps/nvs-types/a-json-derive-class-with-no.json:1`
- [ ] Two derived fields with equal wire keys are a checker error. `crates/nvs-types/src/derive.rs:1`, `data/gaps/nvs-types/two-json-derive-fields-with-the.json:1`
- [ ] A JSON list where a derived class expects an object gives the object issue. `crates/nvs-stdlib/src/json.rs:1`, `data/gaps/nvs-stdlib/a-json-array-decodes-into-a.json:1`

### 10. operators and locals give what their rules say

**Does:** each of the six forms below gives its rule's result or is a compile error.

- [ ] A negative literal compared with a `uint`, in `==`, a `switch` label and a `match` arm, gives `false`. `crates/nvs-types/src/expr/operators.rs:1`, `data/gaps/nvs-types/a-negative-literal-compared-with-a.json:1`
- [ ] A static property is not in a property key's roster, for the literal and the run-time conversion. `crates/nvs-types/src/expr/operators.rs:1`, `data/gaps/nvs-types/a-property-key-names-a-static.json:1`
- [ ] Arithmetic on an `int` narrowed by `==` keeps the type `int`. `crates/nvs-types/src/locals.rs:1`, `data/gaps/nvs-types/an-int-compared-with-a-literal.json:1`
- [ ] A destructuring variable already declared in scope, or written twice in one pattern, is `E0406`. `crates/nvs-types/src/locals.rs:1`, `data/gaps/nvs-types/a-destructuring-variable-that-is-already.json:1`
- [ ] A second `default` in a `match` or a `switch` is a compile error where it is written. `crates/nvs-types/src/expr/mod.rs:1`, `data/gaps/nvs-types/a-match-with-two-default-arms.json:1`
- [ ] An assignment to `$this`, plain or compound, is a compile error. `crates/nvs-types/src/expr/assign.rs:1`, `data/gaps/nvs-types/an-assignment-to-this-compiles-and.json:1`

### 11. anonymous functions, commands and tests take only what can run

**Does:** each of the seven forms below is a compile error, or runs as § *Standing decisions* says.

- [ ] A `...` on an anonymous function's parameter is a diagnostic at the parameter. `crates/nvs-types/src/expr/calls.rs:1`, `data/gaps/nvs-types/a-variadic-parameter-on-an-anonymous.json:1`
- [ ] A default on an anonymous function's parameter is a diagnostic at the parameter. `crates/nvs-types/src/expr/calls.rs:1`, `data/gaps/nvs-types/an-anonymous-function-parameter-default-is.json:1`
- [ ] A `Core\Task::all` field whose function takes a parameter is a diagnostic at the field. `crates/nvs-types/src/expr/assign.rs:1`, `data/gaps/nvs-types/a-task-group-function-that-takes.json:1`
- [ ] A command name or `long:` that is empty or has a space, and a `short:` that is not one letter or digit, are checker errors. `crates/nvs-types/src/commands.rs:1`, `data/gaps/nvs-types/a-command-name-or-option-spelling.json:1`
- [ ] A positional command parameter is checked like an option, and a trailing `array<string>` collects the remaining words. `crates/nvs-types/src/commands.rs:1`, `data/gaps/nvs-types/a-positional-command-parameter-of-a.json:1`
- [ ] A `null` value in a `#[TestWith]` row fills a `?T` parameter with `null`. `crates/nvs-types/src/testing.rs:1`, `data/gaps/nvs-types/a-null-data-row-compiles-and.json:1`
- [ ] An empty `skip:` or `because:`, and a `retries:` below 1, are compile errors. `crates/nvs-types/src/testing.rs:1`, `data/gaps/nvs-types/an-empty-skip-reason-and-a.json:1`

### 12. a route link matches its route, and an unread body never costs the response

**Does:** a link a program builds is a path its route matches, and the server answers a request
whose body the program did not read.

- [ ] A literal `Core\Router::url` value that the capture's conversion would refuse is a compile error. `crates/nvs-types/src/routes.rs:1`, `data/gaps/nvs-types/a-link-value-of-the-wrong.json:1`
- [ ] A `bool` route capture is `E0752`, with a case for the match, the link and the dispatch. `crates/nvs-runtime/src/routes.rs:1`, `crates/nvs-stdlib/src/router.rs:1`, `data/gaps/nvs-runtime/a-bool-route-capture-is-never.json:1`
- [ ] The connection closes only after the response is flushed, with a case that pipelines a request behind a large unread POST. `crates/nvs-server/src/body.rs:1`, `data/gaps/nvs-server/an-unread-post-body-can-lose-the-response.json:1`
- [ ] The header line limit is a named constant in `serve_connection`, stated by a rule, with a case at the bound and one past it. `crates/nvs-server/src/serve.rs:1`, `data/gaps/nvs-server/header-line-limit-is-hyper-default.json:1`

### 13. every help text names only what compiles, and each syntax is one feature

**Does:** no diagnostic prints a rule id, a form of "refuse" or syntax that does not compile, and
`try`/`catch`/`finally` and `throw` are one feature each.

**Split:** at stage start, two `Explore` subagents go out together. One lists every diagnostic
message and help text under `crates/` that prints `rule:` or a form of "refuse", as `file.rs:NN` and
one line each. The other lists every citation of `lang:statements/try-catch-finally` and
`lang:statements/throw` in the tree. The two lists are the sets of the fifth and sixth items.

- [ ] `E0479`'s help suggests `!=`, and `lang:expressions/and-the-ternary`'s example 05 is blessed again. `crates/nvs-types/src/expr/assign.rs:1`, `data/gaps/nvs-types/a-nullsafe-write-help-suggests-not-identical.json:1`
- [ ] `E0802`'s help names only forms the parser accepts. `crates/nvs-types/src/expr/isolate.rs:1`, `data/gaps/nvs-types/a-spawn-entry-help-names-spawn-worker.json:1`
- [ ] `E0233`'s help says a nested `use` covers the code below it. `crates/nvs-types/src/locals.rs:1`, `data/gaps/nvs-types/a-nested-use-help-says-it-covers.json:1`
- [ ] `E0303` lists the `autoload` roots it searched. `crates/nvs-types/src/expr/mod.rs:1`, `data/gaps/nvs-types/an-undefined-class-names-no-autoload-root.json:1`
- [ ] No diagnostic's message or help prints a `rule:` id or a form of "refuse", and a test scans the help texts. `crates/nvs-types/src/expr/members.rs:1`, `data/gaps/nvs-types/diagnostic-help-prints-rule-ids.json:1`
- [ ] The two `lang:statements/` features are merged into their `lang:errors/` twins. `tools/nv/proofs/roster.ts:1`, `data/gaps/nv/two-lang-features-are-one-syntax.json:1`

### 14. `Core\BigInt`'s gcd and lcm are sub-quadratic

**Does:** `gcd` and `lcm` run a half-gcd, so a request's text no longer buys quadratic time.

- [ ] `gcd` and `lcm` use a half-gcd above a measured crossover, and Stein's algorithm below it. `crates/nvs-stdlib/src/bigint.rs:1`
- [ ] Their benches declare a sub-quadratic complexity, lose the gap's marker, and gain a `.scale.nvs` sibling. `crates/nvs-stdlib/src/bigint.rs:1`

### 15. `Core\BigInt`'s modular power is sub-quadratic per step

**Does:** `powMod` over an odd modulus runs its Montgomery step over Karatsuba multiplication.

- [ ] `powMod`'s Montgomery step multiplies with Karatsuba above a measured crossover. `crates/nvs-stdlib/src/bigint.rs:1`
- [ ] Its bench declares the new complexity, loses the gap's marker, gains a `.scale.nvs` sibling, and the gap record is deleted. `crates/nvs-stdlib/src/bigint.rs:1`, `data/gaps/nvs-stdlib/bigint-gcd-and-modular-power-grow-quadratically.json:1`

## Handoff

### State

Stage 1 is done and committed. Stage 2's three items are built, verified and committed, and its two
gap records are deleted. Stage 2 stays open for one item it found:

- `conformance.rs`'s `check_method_overrides` compares every method a class or interface declares
  again with each direct ancestor's resolved declaration (`E0404`), through `compare_override` and
  `parent_params_problem`. The relation is `field_fits` (no `int` → `float` row). A generic
  interface's `T` is bound from the class's `implements` clause (`bind_interface_args`). A method a
  class inherits from its superclass is compared with each interface the class names, at the class
  name. `rule:classes/an-override-keeps-its-parents-signature` is the new rule.
- **The parameter count is exact**, which narrows the standing decision (now rewritten): an extra
  parameter with a default crashed when called through the parent, because a call fills defaults
  from the signature it resolved.
- **Only `Comparable::compareTo` itself takes `self`.** A subclass that narrows `compareTo(self)`
  again is refused: a direct `$price->compareTo(new Price())` through the parent ran
  `SalePrice::compareTo` with a `Price` and printed a garbage number. The ordering case's `SalePrice`
  now takes `Price`.
- An `abstract` method with a body is `E0106` (`reject_abstract_bodies`).
- Two older tests relied on a refused override and now keep the parent's signature:
  `interface_defaults.rs`'s `a_class_can_override_a_default_interface_method` and the
  `core/reflect-an-inherited-methods-parameter-names-…` case.

### Next

**Close stage 2's last item, then stage 3.**

- [ ] A direct `compareTo` call through `Comparable` passes another class to a `self` parameter. The safe options are a run-time class check on a direct call, as an ordering has (`lower_object_comparison`), or a compile error unless the argument's static type is the receiver's class. `crates/nvs-ir/src/lower/operator.rs:1`, `data/gaps/nvs-types/a-direct-compareto-call-through-comparable.json:1`
- [ ] Stage 3: `implements` with any class is `E0303`. `crates/nvs-types/src/conformance.rs:1`, `data/gaps/nvs-types/implements-with-a-class-of.json:1`
- [ ] Stage 3: a `catch` naming what no `Throwable` can be is a diagnostic. `data/gaps/nvs-types/a-catch-clause-that-names-a.json:1`
- [ ] Stage 3: a `throw` of anything not `Throwable` is `E0780`. `data/gaps/nvs-types/a-throw-of-an-anonymous-object.json:1`
- [ ] Stage 3: `Core\Test::assertThrows` takes a `class<Throwable>`. `crates/nvs-stdlib/src/test.rs:1`, `data/gaps/nvs-stdlib/assert-throws-with-a-class-that.json:1`

### Backlog

The bullets below were carried from `release-hardening`'s backlog when it was reached.

- A warm baseline run is 30.6 M instructions, and 61% is the front end. The largest parts are `build_signatures` (10 M), `seed_natives` (6.5 M), `builtin_manifests` (4.5 M, which parses every built-in extension's manifest on every start) and the TOML parse of `default.toml` (4.4 M). Precomputing the manifests at build time is the plainest cut (`crates/nvs-ext/src/load.rs:704`).
- `nvs-runtime`'s pool hands a block back through a pointer narrowed to its last owner's size. Exposing each block's provenance and rebuilding the pointer in `push` and the same-class `realloc` removed Miri's report, but no unit test reproduced it, and the user chose the `sanitizer` feature instead (`crates/nvs-runtime/src/alloc.rs:176`, the `miri-units` comment in `.github/workflows/ci.yml`).
- The suite's probes-on run is still owed: the coverage flags are refused over a `.nvst` tree (`crates/nvs-cli/src/main.rs:2394`, `rule:testing/probes-on-is-a-tested-configuration`).
- `NvsStr::new` takes any `&[u8]`, so the UTF-8 invariant rests on every caller, and the HTTP Garden bug was one that forgot. A checked constructor for bytes from outside, with `new` kept for payloads already text, closes the class (`crates/nvs-runtime/src/string.rs:600`).
- Two more unchecked `string`s: a MySQL column name (`crates/nvs-stdlib/src/db/column.rs:67`, also `db/execute.rs` and `db/stream.rs`), and a signed cookie's plaintext, opened with empty AAD (`crates/nvs-stdlib/src/signed_cookie.rs`).
- `Core\IO::within` does not refuse what `rule:errors/path-component-refusals` lists, beyond device names (`crates/nvs-stdlib/src/io.rs`).
- `test / macos-aarch64` on `be3e8a605`: the `nvs-cli` `nvs` test binary died of SIGSEGV with no stack-overflow line, after 116 of 247 tests. The test is not named, and it has not repeated since. A repeat needs a run with `--test-threads=1` on macOS (`.github/workflows/ci.yml:131`).
- A forged payload can put an enum case with a backing value the enum does not declare into an enum-typed property (`rule:enums/representation`). Whether a typed read of it is safe is not checked (`crates/nvs-runtime/src/object.rs`).
- `bun nv verify` fails on load only in `nvs-ext` `spreadsheet_read`, `nvs-cli` `live_edit` (its 30 s swap), `init` and `examples` (`cache-shared-tls.nvs` missed its connect deadline), and `nvs-stdlib` `ldap` (its socket deadline). Each passes alone (`tools/nv/cmd/verify.ts`'s module doc).
- The nightly tests GitHub's `main`, so its `supply chain` failure says nothing about the current lock until the user pushes (`.github/workflows/ci.yml:838`).
- The cost guard missed `serve_throughput_scales_from_one_core_to_four_by_the_margin_this_test_names` at 1.38× of 1.5× on Windows and 1.33× on Linux, and each passed on the next run. A timing guard that fails with no change behind it is run again. It is a single-bound timing, which `rule:testing/perf-two-mechanisms` declines. Ask the user before giving it a soft and a hard bound (`crates/nvs-cli/src/serve.rs:4120`).
- `bun nv verify` runs neither `nv render --website --check` nor `bun nv site --check structure stale questions prose`, which needs no binary. So the website data and 788 pages' questions went stale with only CI's legs failing (`tools/nv/cmd/verify.ts:153`).
- `test_request_dispatches_in_process_through_the_compiled_route_table` was red once alone, because `Core\Env::context()` was not `Request`, and red again on CI's Windows leg on `be3e8a605` with 200 for 201 (`crates/nvs-cli/src/runner.rs:3410`).
- `Core\Ldap::connect` dials each address of a host in turn, so a `localhost` block pays 2 s on Windows (`crates/nvs-stdlib/src/ldap/mod.rs:489`, `crates/nvs-host/src/net.rs:438`).
- The Cranelift panic on the shared worker (`cranelift-codegen` 0.136.3 `machinst/vcode.rs:1901`) has not repeated. If it does, rerun `RUST_BACKTRACE=1 target/debug/nvs test --shared-worker tests/conformance` and read the backtrace: it happened on one thread compiling many units, after other cases ran on it (`crates/nvs-cli/src/shared.rs:171`).
- `fullRun` and `runBenches` still take a `deadline` that no caller passes, and `runBenches` an `order` and `onStart` that only the debt passed (`tools/nv/select/full.ts:63`, `tools/nv/proofs/select.ts:206`).
- Three Miri tests ran 203 to 218 s on run 39258294723, against a 300 s limit: `regex`'s flag test, `every_registered_member_has_an_implementation_address` and the transport's compressed replies. A slower runner day could reach the limit (`crates/nvs-stdlib/src/regex.rs`, `crates/nvs-stdlib/src/lib.rs`).
- The shared-worker run added about 6 minutes to each `test` leg's conformance step (11 to 17 minutes on Windows). It runs every eligible case a second time (`crates/nvs-cli/src/shared.rs`).
- `Core\BigInt::parse`'s `ParseError` message repeats the whole text, so a long bad text a request sent gives a message as long. `nvs_runtime::helpers` bounds a failed conversion's operand to a head, and this message could do the same. It is a message change, so it is held to `AGENTS.md` § *Text an end user reads* (`crates/nvs-stdlib/src/bigint.rs:438`).
- The `proofs` legs reach the examples on all three runners. Windows' Docker runs Windows containers, so every `requires:` program is skipped there with that reason. Stamp an example with an `nvs` built from the tree being stamped.
