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
    status: done
    context: [crates/nvs-types/src/conformance.rs]
  - id: 3
    title: "only the throwable tree is thrown, caught and asserted"
    status: done
    context: [crates/nvs-types/src/conformance.rs, crates/nvs-stdlib/src/test.rs]
  - id: 16
    title: "a redeclared property or constant never reads a value as another type"
    status: done
    context: [crates/nvs-types/src/conformance.rs, docs/rules/statements/static-is-a-member-modifier.md, docs/rules/classes/an-override-keeps-its-parents-signature.md]
  - id: 4
    title: "visibility and the property pipeline hold on every path"
    status: done
    context: [crates/nvs-runtime/src/object.rs, crates/nvs-types/src/expr/members.rs, crates/nvs-types/src/program.rs, crates/nvs-ir/src/lower/mod.rs]
  - id: 5
    title: "every expression the checker accepts lowers"
    status: done
    context: [crates/nvs-ir/src/lower/expr.rs, crates/nvs-ir/src/lower/control.rs, crates/nvs-types/src/expr/operators.rs, crates/nvs-types/src/expr/members.rs]
  - id: 6
    title: "generators and `finally` run each block once and lose no error"
    status: done
    context: [crates/nvs-ir/src/lower/generator.rs, crates/nvs-syntax/src/parser/stmt.rs]
  - id: 7
    title: "a class declaration is checked where it is written"
    status: done
    context: [crates/nvs-types/src/signatures.rs, crates/nvs-syntax/src/parser/decl.rs, crates/nvs-types/src/expr/members.rs]
  - id: 8
    title: "enums, type arguments and property hooks are checked where they are written"
    status: done
    context: [crates/nvs-types/src/enums.rs, crates/nvs-syntax/src/parser/ty.rs, crates/nvs-types/src/check.rs, crates/nvs-types/src/attributes.rs]
  - id: 9
    title: "attributes and derived JSON codecs are checked where they are written"
    status: done
    context: [crates/nvs-types/src/attributes.rs, crates/nvs-types/src/derive.rs, crates/nvs-types/src/routes.rs, crates/nvs-stdlib/src/json.rs]
  - id: 10
    title: "operators and locals give what their rules say"
    status: done
    context: [crates/nvs-types/src/expr/operators.rs, crates/nvs-types/src/locals.rs, crates/nvs-types/src/expr/mod.rs, crates/nvs-types/src/expr/assign.rs]
  - id: 11
    title: "anonymous functions, commands and tests take only what can run"
    status: done
    context: [crates/nvs-types/src/expr/calls.rs, crates/nvs-types/src/expr/assign.rs, crates/nvs-types/src/commands.rs, crates/nvs-types/src/testing.rs]
  - id: 12
    title: "a route link matches its route, and an unread body never costs the response"
    status: done
    context: [crates/nvs-types/src/routes.rs, crates/nvs-runtime/src/routes.rs, crates/nvs-stdlib/src/router.rs, crates/nvs-server/src/body.rs, crates/nvs-server/src/serve.rs]
  - id: 13
    title: "every help text names only what compiles, and each syntax is one feature"
    status: open
    context: [crates/nvs-types/src/expr/members.rs, crates/nvs-types/src/expr/assign.rs, crates/nvs-types/src/expr/isolate.rs, crates/nvs-types/src/locals.rs, tools/nv/proofs/roster.ts]
  - id: 17
    title: "a generated matrix pins every redeclaration a subclass can write"
    status: open
    context: [tools/nv/cmd/verify.ts, crates/nvs-types/src/conformance.rs, docs/rules/classes/an-override-keeps-its-parents-signature.md, docs/rules/statements/static-is-a-member-modifier.md]
  - id: 18
    title: "a checked build stops at a read whose value is not its declared type"
    status: open
    context: [crates/nvs-cli/src/main.rs, crates/nvs-codegen/src/lib.rs, crates/nvs-ir/src/lower/expr.rs, crates/nvs-ir/src/lower/mod.rs, crates/nvs-runtime/src/helpers.rs, docs/rules/testing/probes-on-is-a-tested-configuration.md]
  - id: 19
    title: "what the matrix and the checked build found is closed, and CI runs both"
    status: open
    context: [.github/workflows/ci.yml, docs/rules/testing/probes-on-is-a-tested-configuration.md]
  - id: 14
    title: "`Core\\BigInt`'s gcd and lcm are sub-quadratic"
    status: open
    context: [crates/nvs-stdlib/src/bigint.rs]
  - id: 15
    title: "`Core\\BigInt`'s modular power is sub-quadratic per step"
    status: open
    context: [crates/nvs-stdlib/src/bigint.rs]
checks:
  - id: owners-closes
    stage: 19
    name: no gap record or designed rule names this goal
    run: [bun, nv, owners, --closes, compiler-holes]
    reach: true
    floor: false
  - id: soundness-has-no-skip
    stage: 19
    name: every matrix cell is written and passes, and none is skipped
    run: [bun, nv, soundness, --check]
    reach: true
    floor: false
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
ordering, on the request path only where a program orders objects. A direct `compareTo` call that
resolves to `Comparable::compareTo` runs the same check: a static refusal would refuse every direct
call through `Comparable`, since the receiver's class is not known there.

**`Core\Test::assertThrows`'s second argument is a `C::class` written at the call**, and `C` is a
class of the `Throwable` tree. A string, a variable and a class outside the tree are compile errors
at the argument. The parameter is not typed `class<Throwable>`: `C::class` is a `string`
(`rule:types/class-reference`), so that type would make every call write `C::class as
class<Throwable>`.

**An override that a call through the parent could not use is a diagnostic at the override.** The
return type is the parent's or a subtype of it. Each parameter type is the parent's or a supertype
of it. The parameter count is the parent's exactly: an extra parameter with a default ran with
nothing in it through the parent, because a call fills defaults from the signature it resolved.
The visibility is not narrower, `static` is the same, and a `tainted` or `secret` qualifier on a
parameter is kept. The built-in interfaces (`Stringable`, `Comparable`, `Parses`, `Iterator<T>`) are
held to the same check.

**A redeclared instance property keeps its parent's exact type.** A property is read and written
through the parent's type, so a narrower type lets the parent's code store what the subclass cannot
read, and a wider one lets the subclass store what the parent cannot read. Any other type is a
diagnostic at the redeclaration, with its own code beside `E0833` and a help that says the property
keeps its parent's type. That holds for a plain property, a promoted constructor parameter, a
`lateinit` and a hooked property, whose hooks keep the parent's type too. A `readonly` property is
not redeclared at all, because both constructors would write it. A narrower property with a check on
each write is goal `covariant`'s, and this stage does not build it. A static property is out of
scope: each class names its own storage, and `static::$x` is already `E0499`.

**A `static::` constant read gives its value at the type the read is typed with.** A redeclared
constant still keeps any type assignable to the ancestor's (`E0833`), without the `int` to `float`
row, as an override has none. The read converts the called class's scalar to the read's type, so an
`int` read where `int|string` is declared is that union holding the `int`. Safe default if the table
does not carry each scalar's kind: store each redeclared value at the type of the topmost declaration
when the table is built, so the read stays what it is today.

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

**Stages 17 to 19 look for the type confusions nobody has reported.** Stages 1 and 16 closed the
ones a person happened to write. A rule-driven suite pins what a rule says, and no case exists where
no rule spoke, so these two tools ask the question for every form instead.

- **The matrix is generated, and its oracle is one table.** `bun nv soundness --write` writes
  `tests/conformance/soundness/<member>.nvst`, one file per member kind with many classes in it, and
  `--check` fails when a committed file differs from what it would write, or when any cell is
  skipped. `bun nv verify` runs `--check`. The generator's module doc is the home of the table, and each row cites the rule that
  gives its verdict: a compile error with its code, or the exact output. The sets are listed in the
  table, never described:
  - members: a method parameter, a method return, an interface method's parameter and return, a
    static method's parameter and return, `compareTo`'s parameter, an instance property (plain,
    promoted, `lateinit`, hooked, `array<T>`), a `readonly` property, a static property, and a class
    or interface constant read through `self::`, the class name and `static::`;
  - relations: the same type, a subclass, a superclass, an unrelated class, `int` under `int|string`,
    `int` over `float`, `float` over `int`, `T` under `?T`, `?T` over `T`, a union member under the
    union, and a wider union;
  - paths: a variable of the parent's type, of the subclass's type, of an interface's type,
    `static::`, an erased `object` or `mixed` access, a `property<T>` key, and a first-class callable.
- **Everything the matrix and the checked build find is fixed inside this goal.** No finding is
  handed to another goal, left as a gap record past the goal, or skipped when it ends. A finding too
  big for stage 19 gets a stage of its own in this goal. A cell's expectation is the rule's verdict,
  and it is never changed to match what the binary does today. Where a rule itself is wrong, the rule
  and the code change together in this goal.
- **A cell that fails is skipped only while the goal runs.** The generator keeps the cell and its
  expectation, and `tools/nv/soundness/open.json` maps each failing cell to a gap record owned by this
  goal, which tracks the work until it is fixed. A listed cell is left out of the written files.
  `--check` passes a listed cell only under `--allow-open`, which `bun nv verify` passes while the
  goal runs, so a session's verify stays green. `--allow-open` also fails when a listed cell's gap
  record is gone or a listed cell now passes, so the list cannot rot. Stage 19 deletes `open.json`,
  the `--allow-open` flag and its use in `verify.ts`. After the goal, a failing cell is a red verify,
  never a skip and never a gap.
- **The checked build is a compile switch, and production code does not change.** `NOVIS_CHECK_TYPES`
  sits beside `NOVIS_NO_OPTIMISE` (`crates/nvs-cli/src/main.rs:209`) and works like it: a
  process-wide one-way switch, with the artifact cache off, since an artifact's key does not cover
  it. With it on, the lowering adds a type test after each typed read: a property read, a parameter
  at a method's entry, a call's result, a `static::` constant read and an array element of a class
  type. The test is the one `is` lowers to. A value that fails stops the process with the source
  line, the declared type and the class it holds, and nothing can catch it, because it is a compiler
  bug and never a program's error. With it off, not one instruction is added.
- **It tests what carries its type at run time.** An object's class, a union's tag and a `null`
  where the type has none. A raw `int`, `float` or `bool` carries none, so a scalar read as another
  scalar is the matrix's to find, as the `float` row does.
- **What the two find is fixed in stage 19.** Each finding is a gap record owned by this goal, with
  the smallest program that shows it and neutral names, and an item in stage 19. The record is
  deleted in the slice that fixes it. A stage 19 that grows past one session is split into stages of
  one session each, grouped by the files they touch, all of them in this goal.
- **CI runs the checked build once the suite passes under it.** Stage 19 adds a step after
  "Conformance suite, unoptimised" (`.github/workflows/ci.yml:292`) that runs the suite with
  `NOVIS_CHECK_TYPES=1`, and `rule:testing/probes-on-is-a-tested-configuration` names it beside the
  unoptimised run. It is not added red.
- **Goal `covariant` extends the matrix.** Its rows for a `covariant` parameter and property are its
  own, and the generator takes a row as data, so they are a table edit.

**Lanes.** Stages 1 to 13 and 16 share the checker's files and run in order on the chain, 16 right
after 3 because its holes read memory as another type. Stages 17 to 19 run after 13, so the matrix
does not report again a hole a stage before it already owns. Stages 14 and 15
touch only `crates/nvs-stdlib/src/bigint.rs` and its benches, so they can run as a side goal beside
the chain. No stage waits on another goal.

**Trade-offs.** Every stage but 1, 12 and 16 is compile-time work: no program that runs today runs
slower. Stage 1 adds one class comparison to an object ordering. Stage 16 adds a scalar conversion to a
`static::` constant read only where the read's type is a union or `float`. Stage 12 holds no new memory, since
the drain bound stays. Programs that compiled and were wrong stop compiling, which is the point.
Simplicity of the language gains: twenty-odd forms that did nothing now have one meaning or none.
Stages 17 and 18 change no program: the matrix is test cases, and the checked build adds code only
under its switch. Their cost is CI time: one more run of the suite in each leg, and the matrix's
cases when a change reaches the checker, the layout or the lowering.

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
- [x] A direct `compareTo` call through `Comparable` cannot pass an object of another class to a `self` parameter. `crates/nvs-ir/src/lower/operator.rs:1`, `tests/conformance/class/a-direct-compare-to-call-through-comparable-checks-the-classes.nvst:1`

### 3. only the throwable tree is thrown, caught and asserted

**Does:** `implements`, `catch`, `throw` and `Core\Test::assertThrows` each take only what the
throwable tree can be.

- [x] `implements` with any class, built-in or not, is `E0303`. `crates/nvs-hir/src/hierarchy.rs:1`, `tests/conformance/reject/implements-with-a-class-does-not-compile.nvst:1`
- [x] A `catch` that names a class or a built-in type no `Throwable` can be is a diagnostic. An interface stays allowed. `crates/nvs-types/src/expr/members.rs:1`, `tests/conformance/reject/a-catch-names-only-what-a-throwable-can-be.nvst:1`
- [x] A `throw` of any value that is not `Throwable`, a shape included, is `E0780`. `crates/nvs-types/src/expr/members.rs:1`, `tests/conformance/reject/a-throw-of-an-anonymous-object-does-not-compile.nvst:1`
- [x] `Core\Test::assertThrows`'s second argument is a `C::class` written at the call, naming a class of the `Throwable` tree, and anything else is a compile error. `crates/nvs-types/src/testing.rs:1`, `tests/conformance/reject/assert-throws-takes-a-written-class-of-the-throwable-tree.nvst:1`

### 16. a redeclared property or constant never reads a value as another type

**Does:** a subclass can no longer change the type of a property it redeclares, and a `static::`
constant read gives the right value whichever class redeclared it. Both holes read memory as another
type today, so this stage runs right after stage 3.

- [x] A redeclared instance property at another type is a diagnostic at the redeclaration, for a plain property, a promoted constructor parameter, `lateinit`, a hooked property and `array<T>`, with a case for each. `crates/nvs-types/src/conformance.rs:1`, `tests/conformance/reject/a-redeclared-property-keeps-its-parents-type.nvst:1`
- [x] A redeclared `readonly` property is a diagnostic. `crates/nvs-types/src/conformance.rs:1`, `tests/conformance/reject/a-redeclared-property-keeps-its-parents-type.nvst:1`
- [x] A rule fragment states both decisions, with its Declined list, and its record is `shipped` with the case in `guardedBy`. `docs/rules/classes/a-redeclared-property-keeps-its-parents-type.md:1`, `data/rules/classes/a-redeclared-property-keeps-its-parents-type.json:1`
- [x] A `static::` read gives a redeclared scalar at the read's type: an `int` over `int|string`, a `bool` over `int|bool`, and through an interface. `E0833` refuses `int` over `float`. `crates/nvs-ir/src/lower/expr.rs:1`, `tests/conformance/class/a-static-constant-read-gives-the-redeclared-value-at-its-type.nvst:1`
- [x] `rule:statements/static-is-a-member-modifier` says the read converts, and drops nothing else. `docs/rules/statements/static-is-a-member-modifier.md:27`

### 4. visibility and the property pipeline hold on every path

**Does:** a `private` or `protected` member is reached from where its visibility allows and nowhere
else, and every write of a property runs the same guard and observer pipeline.

- [x] An erased read or write through `object` or `mixed` of a property not visible at the access site throws, as a missing name does. `rule:types/erased-member-access` says so. `crates/nvs-runtime/src/object.rs:1`, `data/gaps/nvs-runtime/a-private-or-protected-property-is.json:1`
- [x] A read of a `private` class constant outside its class, or of a `protected` one outside the class and its subclasses, is a diagnostic. `crates/nvs-types/src/expr/members.rs:1`, `data/gaps/nvs-types/a-private-or-protected-class-constant.json:1`
- [x] `Core\Program::implementing` and `implementingWith` over a class with a `private` or `protected` constructor is a diagnostic at the call. `crates/nvs-types/src/program.rs:1`, `data/gaps/nvs-types/core-program-implementing-calls-a-private.json:1`
- [x] A promoted constructor parameter is stored through the same pipeline as `$this->x = $x;`, with a case for the guard and the observer. `crates/nvs-ir/src/lower/mod.rs:1`, `data/gaps/nvs-ir/a-promoted-constructor-parameter-runs-no.json:1`
- [x] A `??=` that keeps the property's value calls no guard. `crates/nvs-ir/src/lower/mod.rs:1`, `data/gaps/nvs-ir/a-coalescing-assignment-to-a-property.json:1`

### 5. every expression the checker accepts lowers

**Does:** none of the seven forms below reaches a lowering panic or `E0901`. Each is either a
diagnostic where it is written or lowered, as § *Standing decisions* says.

- [x] A `catch` variable that redeclares a local is `E0406`, with a case at the top level and in a method. `crates/nvs-ir/src/lower/expr.rs:235`, `data/gaps/nvs-ir/a-catch-variable-that-reuses-a.json:1`
- [x] `echo` and `.` of a `class<T>` are a diagnostic whose help names `as string`. `crates/nvs-ir/src/lower/expr.rs:1146`, `data/gaps/nvs-ir/a-class-reference-in-echo-or.json:1`
- [x] `is class<T>` is a diagnostic whose help names `as ?class<T>`. `crates/nvs-ir/src/lower/expr.rs:6280`, `data/gaps/nvs-ir/an-is-test-against-a-class-reference.json:1`
- [x] A `void` call at `mixed`, or as a `match` arm, is a compile error. `crates/nvs-ir/src/lower/control.rs:1`, `data/gaps/nvs-ir/a-void-call-at-a-mixed.json:1`
- [x] A static property reached through `->` or `?->` is a diagnostic naming `P::$x`. `crates/nvs-types/src/expr/members.rs:1`, `data/gaps/nvs-types/a-static-property-read-through-an.json:1`
- [x] Ordering a `bool` against a number, a chained `1 < 2 < 3` included, is `E0715`. `crates/nvs-types/src/expr/operators.rs:1`, `data/gaps/nvs-types/ordering-a-bool-against-a-number.json:1`
- [x] A `callable` after `echo` or `.` is `E0707`. `crates/nvs-types/src/expr/operators.rs:1`, `data/gaps/nvs-types/echo-of-a-callable-compiles-and.json:1`

### 6. generators and `finally` run each block once and lose no error

**Does:** a generator finishes when its body throws, a `finally` runs once, and an error in flight
is kept as § *Standing decisions* says.

- [x] A `yield` of a literal in a nullable or union element type compiles and yields, with a case for each element type. `crates/nvs-ir/src/lower/generator.rs:1`, `data/gaps/nvs-ir/a-generator-of-a-nullable-or.json:1`
- [x] A generator whose body throws is finished: a later `advance()` returns `false`, and its `finally` runs once. `crates/nvs-ir/src/lower/generator.rs:1`, `data/gaps/nvs-ir/a-generator-that-throws-is-not.json:1`
- [x] A `yield` inside a `finally` is a compile error with its own code. `crates/nvs-ir/src/lower/generator.rs:1`, `data/gaps/nvs-ir/a-yield-inside-a-finally.json:1`
- [x] An error that leaves a `finally` while another is in flight carries it as `previous`, and the rule states the bound. `crates/nvs-syntax/src/parser/stmt.rs:1`, `data/gaps/nvs-syntax/a-throw-inside-a-finally.json:1`

### 7. a class declaration is checked where it is written

**Does:** each of the six declarations below is a diagnostic at the declaration.

- [x] A second property, method or constructor of one name in one class, a promoted parameter included, is a diagnostic at the second one. `crates/nvs-types/src/signatures.rs:1`, `data/gaps/nvs-types/a-class-member-declared-twice-passes.json:1`
- [x] `static` on a `constructor` is a diagnostic. `crates/nvs-types/src/signatures.rs:1`, `data/gaps/nvs-types/a-static-constructor-compiles-and-new.json:1`
- [x] A `...` on any parameter but the last is a diagnostic at that parameter. `crates/nvs-types/src/signatures.rs:1`, `data/gaps/nvs-types/a-variadic-parameter-that-is-not.json:1`
- [x] A `void` or `never` property is `E0742`. `crates/nvs-types/src/signatures.rs:1`, `data/gaps/nvs-types/a-void-or-never-property-passes.json:1`
- [x] `final` on a class constant is `E0106`. `crates/nvs-syntax/src/parser/decl.rs:1`, `data/gaps/nvs-syntax/final-on-a-class-constant-is.json:1`
- [x] A class constant's value is checked against its declared type at the declaration. `crates/nvs-types/src/expr/members.rs:1`, `data/gaps/nvs-types/a-class-constant-whose-value-does.json:1`

### 8. enums, type arguments and property hooks are checked where they are written

**Does:** each of the six forms below is a diagnostic at the form.

- [x] A second case of one name in one enum is a diagnostic at the second one. `crates/nvs-types/src/enums.rs:1`, `data/gaps/nvs-types/an-enum-case-declared-twice-compiles.json:1`
- [x] `new` naming an enum is one checker error that says a case is written `M::Case`. `crates/nvs-types/src/enums.rs:1`, `data/gaps/nvs-types/new-on-an-enum-passes-the.json:1`
- [x] A constructor reference to an enum, and a user attribute on an enum, an enum case or a class constant, are checker errors. `crates/nvs-types/src/attributes.rs:1`, `data/gaps/nvs-types/a-constructor-reference-to-an-enum.json:1`
- [x] A `<` after a built-in type name is an error at the `<` that names the generic types. `crates/nvs-syntax/src/parser/ty.rs:1`, `data/gaps/nvs-syntax/type-arguments-after-a-built-in-type.json:1`
- [x] A hook on a `static` property is a diagnostic, as one on a `readonly` property is `E0843`. `crates/nvs-types/src/check.rs:1`, `data/gaps/nvs-types/a-hook-on-a-static-property.json:1`
- [x] A second hook of one kind on one property is a diagnostic at the second hook. `crates/nvs-types/src/check.rs:1`, `data/gaps/nvs-types/a-property-with-two-get-hooks.json:1`

### 9. attributes and derived JSON codecs are checked where they are written

**Does:** a recognized attribute is accepted only where it builds something, and a derived codec
reads and writes each wire key once.

- [x] A recognized attribute at a site its rule does not name is a checker error, and `Derive()` follows § *Standing decisions*. `crates/nvs-types/src/attributes.rs:1`, `data/gaps/nvs-types/a-compiler-read-attribute-on-a.json:1`
- [x] An array literal in a named attribute is checked against the field's declared array type. `crates/nvs-types/src/attributes.rs:1`, `data/gaps/nvs-types/an-array-literal-in-an-attribute.json:1`
- [x] Each contradiction `rule:attributes/api-adds-and-cannot-contradict` names is `E0771`, a second `#[Core\Api]` on one method included. `crates/nvs-types/src/routes.rs:1`, `data/gaps/nvs-types/core-api-accepts-what-its-rule.json:1`
- [x] A `#[Core\Json\Derive]` class with no written constructor is a checker error. `crates/nvs-types/src/derive.rs:1`, `data/gaps/nvs-types/a-json-derive-class-with-no.json:1`
- [x] Two derived fields with equal wire keys are a checker error. `crates/nvs-types/src/derive.rs:1`, `data/gaps/nvs-types/two-json-derive-fields-with-the.json:1`
- [x] A JSON list where a derived class expects an object gives the object issue. `crates/nvs-stdlib/src/json.rs:1`, `data/gaps/nvs-stdlib/a-json-array-decodes-into-a.json:1`

### 10. operators and locals give what their rules say

**Does:** each of the six forms below gives its rule's result or is a compile error.

- [x] A negative literal compared with a `uint`, in `==`, a `switch` label and a `match` arm, gives `false`. `crates/nvs-types/src/expr/operators.rs:1`, `data/gaps/nvs-types/a-negative-literal-compared-with-a.json:1`
- [x] A static property is not in a property key's roster, for the literal and the run-time conversion. `crates/nvs-types/src/expr/operators.rs:1`, `data/gaps/nvs-types/a-property-key-names-a-static.json:1`
- [x] Arithmetic on an `int` narrowed by `==` keeps the type `int`. `crates/nvs-types/src/locals.rs:1`, `data/gaps/nvs-types/an-int-compared-with-a-literal.json:1`
- [x] A destructuring variable already declared in scope, or written twice in one pattern, is `E0406`. `crates/nvs-types/src/locals.rs:1`, `data/gaps/nvs-types/a-destructuring-variable-that-is-already.json:1`
- [x] A second `default` in a `match` or a `switch` is a compile error where it is written. `crates/nvs-types/src/expr/mod.rs:1`, `data/gaps/nvs-types/a-match-with-two-default-arms.json:1`
- [x] An assignment to `$this`, plain or compound, is a compile error. `crates/nvs-types/src/expr/assign.rs:1`, `data/gaps/nvs-types/an-assignment-to-this-compiles-and.json:1`

### 11. anonymous functions, commands and tests take only what can run

**Does:** each of the seven forms below is a compile error, or runs as § *Standing decisions* says.

- [x] A `...` on an anonymous function's parameter is a diagnostic at the parameter. `crates/nvs-types/src/expr/calls.rs:1`, `data/gaps/nvs-types/a-variadic-parameter-on-an-anonymous.json:1`
- [x] A default on an anonymous function's parameter is a diagnostic at the parameter. `crates/nvs-types/src/expr/calls.rs:1`, `data/gaps/nvs-types/an-anonymous-function-parameter-default-is.json:1`
- [x] A `Core\Task::all` field whose function takes a parameter is a diagnostic at the field. `crates/nvs-types/src/expr/assign.rs:1`, `data/gaps/nvs-types/a-task-group-function-that-takes.json:1`
- [x] A command name or `long:` that is empty or has a space, and a `short:` that is not one letter or digit, are checker errors. `crates/nvs-types/src/commands.rs:1`, `data/gaps/nvs-types/a-command-name-or-option-spelling.json:1`
- [x] A positional command parameter is checked like an option, and a trailing `array<string>` collects the remaining words. `crates/nvs-types/src/commands.rs:1`, `data/gaps/nvs-types/a-positional-command-parameter-of-a.json:1`
- [x] A `null` value in a `#[TestWith]` row fills a `?T` parameter with `null`. `crates/nvs-types/src/testing.rs:1`, `data/gaps/nvs-types/a-null-data-row-compiles-and.json:1`
- [x] An empty `skip:` or `because:`, and a `retries:` below 1, are compile errors. `crates/nvs-types/src/testing.rs:1`, `data/gaps/nvs-types/an-empty-skip-reason-and-a.json:1`

### 12. a route link matches its route, and an unread body never costs the response

**Does:** a link a program builds is a path its route matches, and the server answers a request
whose body the program did not read.

- [x] A literal `Core\Router::url` value that the capture's conversion would refuse is a compile error. `crates/nvs-types/src/links.rs:converts`
  - [x] `CaptureConv::convert` is public and the matcher calls it
  - [x] `nvs_stdlib::router::native_conv` is the one type-to-conversion map, and `segment_converts` asks it
  - [x] `links::converts` reports `E0772` for a literal the conversion refuses, and a `float` literal for `int`, `uint` and `Core\Uuid`
  - [x] the rule says it and `reject/a-route-link-refuses-a-value-its-capture-cannot-convert.nvst` pins it
- [x] A `bool` route capture is `E0752`, with a case for the match, the link and the dispatch. `crates/nvs-types/src/routes.rs:converts_from_segment`
  - [x] a capture and a `#[Query]` declared `bool`, `true` or `false` is `E0752`
  - [x] the rule says it and `reject/a-bool-route-capture-does-not-compile.nvst` pins it
- [x] The connection closes only after the response is flushed, with a case that pipelines a request behind a large unread POST. `crates/nvs-server/src/serve.rs:close_after_the_response`
  - [x] a raw-socket probe shows the reset from 64 KB and the lost response at 2 MB
  - [x] `NvsConnection::shutdown_write`, and `close_after_the_response` sends the FIN and reads until the peer closes or `LINGER` passes
  - [x] `a_post_whose_body_is_never_read_is_still_answered`, which fails with the close removed
  - [x] `rule:http-server/a-response-is-not-lost-to-an-unread-body` says it, and the gap record is deleted
- [x] The header line limit is a named constant in `serve_connection`, stated by a rule, with a case at the bound and one past it. `crates/nvs-server/src/serve.rs:MAX_HEADER_LINES`
  - [x] `MAX_HEADER_LINES` is 100 and set on the `http1::Builder`
  - [x] `a_request_head_is_answered_up_to_its_header_line_limit` pins 100 and 101
  - [x] `rule:http-server/a-request-head-has-at-most-100-header-lines` says it, and the gap record is deleted

### 13. every help text names only what compiles, and each syntax is one feature

**Does:** no diagnostic prints a rule id, a form of "refuse" or syntax that does not compile, and
`try`/`catch`/`finally` and `throw` are one feature each.

**Split:** at stage start, two `Explore` subagents go out together. One lists every diagnostic
message and help text under `crates/` that prints `rule:` or a form of "refuse", as `file.rs:NN` and
one line each. The other lists every citation of `lang:statements/try-catch-finally` and
`lang:statements/throw` in the tree. The two lists are the sets of the fifth and sixth items.

- [x] `E0479`'s help suggests `!=`, and `lang:expressions/and-the-ternary`'s example 05 is blessed again. `crates/nvs-types/src/expr/assign.rs:check_assign`
  - [x] the help and the `E0479` and `E0482` reference cards write `!=`
  - [x] example 05's `.out`, `docs/reference/lang/30-expressions.md` and `llms-full.txt` are re-recorded, and the gap record is deleted
- [x] `E0802`'s help names only forms the parser accepts. `crates/nvs-types/src/expr/isolate.rs:refuse_entry`
  - [x] both helps name only a path and `Class::method(...)`, and neither names `spawn worker`
  - [x] `an_anon_fn_is_refused_as_a_spawn_target_naming_the_method_form` pins the whole help, and the gap record is deleted
- [x] `E0233`'s help says a nested `use` covers the code below it. `crates/nvs-types/src/locals.rs:nested_declaration`
  - [x] the help says to move it above the code that needs it, and that it applies to the code below it
  - [x] `lang/a-namespace-use-or-autoload-inside-a-body-is-a-compile-error.nvst` is re-recorded, and the gap record is deleted
- [x] `E0303` lists the `autoload` roots it searched. `crates/nvs-hir/src/autoload.rs:note_searched_roots`
  - [x] `AutoloadMap::note_searched_roots` adds a note with the files probed, or the prefixes when none covers the name, and `front_end_in` runs it on a failed compile
  - [x] `hierarchy::undeclared_subject` reads the name back off the headline it writes
  - [x] `lang/an-undeclared-class-names-the-autoload-roots-it-searched.nvst` pins both notes, `rule:programs/autoload` says it, and the gap record is deleted
- [ ] No diagnostic's message or help prints a `rule:` id or a form of "refuse", and a test scans the help texts. `crates/nvs-types/src/expr/members.rs:1`, `data/gaps/nvs-types/diagnostic-help-prints-rule-ids.json:1`
  - [x] `crates/nvs-syntax/src/` and `crates/nvs-hir/src/`: every message, label, help and note is rewritten, and each expectation that pins one is re-recorded
  - [x] `crates/nvs-types/src/expr/`
  - [x] the rest of `crates/nvs-types/src/`
  - [ ] `crates/nvs-config/src/`, `crates/nvs-diagnostics/src/code/config.rs` and the `eprintln!` lines in `crates/nvs-cli/src/`
  - [ ] a test scans every help text and card, and the gap record is deleted
- [ ] The two `lang:statements/` features are merged into their `lang:errors/` twins. `tools/nv/proofs/roster.ts:1`, `data/gaps/nv/two-lang-features-are-one-syntax.json:1`

### 17. a generated matrix pins every redeclaration a subclass can write

**Does:** every member kind, relation and access path in § *Standing decisions* is a cell with the
verdict its rule gives, and each cell either passes or is skipped by an open gap record.

**Split:** a writing subagent writes the generator and its table from § *Standing decisions*; the
session checks the table row by row against the rules it cites, then runs it and files the gaps.

- [ ] `bun nv soundness --write` and `--check`, with the oracle table in its module doc. `tools/nv/cmd/soundness.ts:1`
- [ ] The generated files, one per member kind. `tests/conformance/soundness/properties.nvst:1`
- [ ] `open.json`, and `--check` fails on a listed cell that passes or whose gap is gone. `tools/nv/soundness/open.json:1`
- [ ] `bun nv verify` runs `bun nv soundness --check --allow-open` while the goal runs. `tools/nv/cmd/verify.ts:167`
- [ ] A gap record for each failing cell, listed in `open.json`, and an item for it in stage 19. `tools/nv/soundness/open.json:1`

### 18. a checked build stops at a read whose value is not its declared type

**Does:** with `NOVIS_CHECK_TYPES=1`, a compiled program tests every typed read of a value that
carries its type, and a failure stops the process naming the line and both types.

- [ ] `NOVIS_CHECK_TYPES` sets a process-wide switch beside `compile_unoptimised`, with the artifact cache off. `crates/nvs-cli/src/main.rs:209`, `crates/nvs-codegen/src/lib.rs:905`
- [ ] The lowering adds the type test after a property read, a parameter at entry, a call's result, a `static::` constant read and an array element of a class type, only under the switch. `crates/nvs-ir/src/lower/expr.rs:5361`, `crates/nvs-ir/src/lower/mod.rs:1518`, `crates/nvs-ir/src/lower/mod.rs:1297`, `crates/nvs-ir/src/lower/expr.rs:6569`
- [ ] The runtime helper that stops the process with the line, the declared type and the class. `crates/nvs-runtime/src/helpers.rs:141`
- [ ] A Rust test that compiles one confused read with the switch on and sees it stop, and with it off sees no added instruction. `crates/nvs-codegen/tests/checked_types.rs:1`
- [ ] One local run of the suite under the switch, and a gap record and a stage 19 item for each case it stops. `crates/nvs-cli/src/main.rs:209`

### 19. what the matrix and the checked build found is closed, and CI runs both

**Does:** every finding of stages 17 and 18 is fixed and its gap record deleted, the skip list is
gone, and CI runs the suite under the checked build on every leg. Nothing they found is left for
another goal.

- [ ] Each finding of stages 17 and 18 gets one item here, added by the session that files it, and is fixed in this goal. `tools/nv/soundness/open.json:1`
- [ ] `open.json`, the `--allow-open` flag and its use in `verify.ts` are deleted, and every matrix cell is in the written files and passes. `tools/nv/cmd/soundness.ts:1`, `tools/nv/cmd/verify.ts:167`
- [ ] The CI step after "Conformance suite, unoptimised", with `NOVIS_CHECK_TYPES=1`. `.github/workflows/ci.yml:292`
- [ ] `rule:testing/probes-on-is-a-tested-configuration` names the checked run, and a rule fragment states what the matrix guarantees, `shipped` with the matrix files in `guardedBy`. `docs/rules/testing/probes-on-is-a-tested-configuration.md:14`, `docs/rules/testing/redeclaration-matrix.md:1`, `data/rules/testing/redeclaration-matrix.json:1`

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

Stages 1 to 12 and 16 are done, and every gap record they owned is deleted. Stage 13's four
single-help items landed, and their four gap records are deleted:

- `E0479`'s help writes `if ($x != null) { $x->p = …; }`, and so do the `E0479` and `E0482`
  reference cards.
- `E0802`'s two helps name only a path and `Class::method(...)`.
- `E0233`'s help for a nested `use`, `namespace` or `autoload` says to move it above the code that
  needs it, and that it applies to the code below it.
- `E0303` carries a note naming where `autoload` looked: the files probed for a name under a
  prefix, or the prefixes when none covers it. `nvs_hir::AutoloadMap::note_searched_roots` adds it
  and `front_end_in` (`crates/nvs-cli/src/main.rs`) runs it on a failed compile. It finds its
  diagnostics through `nvs_hir::hierarchy::undeclared_subject`, which reads the name back off the
  headline `undeclared_name_at` writes. The editor (`crates/nvs-lsp/src/document.rs`) does not run
  it yet.

The fifth item's first three crate groups landed. No message, label, help or note under
`crates/nvs-syntax/src/`, `crates/nvs-hir/src/` or `crates/nvs-types/src/` prints a `rule:` id or
a form of "refuse": 54 texts in the first group, 79 in `expr/`, and 94 in the rest of
`nvs-types`. The third group re-recorded 4 conformance cases, 11 example `.out` files, three
`docs/reference/lang/` chapters and `llms-full.txt`. `lower.rs`'s `E_TYPE_ARGS_NOT_GENERIC` help
now ends "Remove the type arguments", as `expr/args.rs`'s does. What is left of the grep in
`nvs-types` is test text: `core_lib.rs`'s registry asserts, `program.rs`'s and `conformance.rs`'s
test messages, and `#[expect(reason)]` strings in `defaults.rs`. The scan test (item 4 below) has
to skip those.

**`nvs test` cannot re-record a `.nvst` case**: `--update` is for inline snapshots only. Copy each
changed `= help:` line from the failing run's `actual:` block into the case. A scratch script did it
for each group: run the whole suite into a log, then for each `FAIL` block pair the `expected:` and
`actual:` help lines in order, and replace each changed one in the file. An example's `.out` is `bun nv proofs --bless <file.nvs>`, then
`bun nv render` writes the reference chapters and `bun nv render --website` writes `llms-full.txt`.
The fragment search must split each old source line at its `\` escapes: a fragment with `\\` or
`\"` in it matches no expectation, and eight `.out` files were found only that way. Then run the
whole `nvs test tests/conformance` once, which takes about a minute, to catch what a fragment missed.

The tree holds uncommitted work this goal does not own: `docs/rules/ide.md`,
`website/src/lib/core.ts`, `website/src/data/numbers.json` and
`tools/nv/test/core-type-links.test.ts`. Leave them out of this goal's commits. `docs/rules/ide.md`
differs from what `bun nv rules --render` writes, so `bun nv verify` stops at `rules-render` until
that work lands. This group's gate was therefore `bun nv verify --fast -p` over `nvs-types`,
`nvs-cli` and `nvs-lsp`, `cargo test --test examples`, and the whole conformance suite, 3025
cases. All were green. `bun nv verify --fast` takes one `-p`: given several, it runs only the last, so run it
once per crate.

### Next

**Stage 13's fifth item is split by crate**, one session each, in this order. The list is a grep
of the crate's non-comment lines for `rule:` and `[Rr]efus`, with test files, `assert!`,
`#[expect(reason)]` and identifiers filtered out. A multi-line string puts the id on a line of its
own, which the grep still finds. Leave out doc comments, tests, `panic!` text and run-time error
messages. Each rewritten text is held to AGENTS.md § *Text an end user reads*. Then find each
expectation that pins an old text: put a distinctive fragment of each into a file and
`grep -rlF -f` it over `tests/`, `docs/examples/` and `crates/*/tests/`.

1. Done: `crates/nvs-syntax/src/` and `crates/nvs-hir/src/`.
2. Done: `crates/nvs-types/src/expr/`.
3. Done: the rest of `crates/nvs-types/src/`.
4. Next: `crates/nvs-config/src/` (about 50), `crates/nvs-diagnostics/src/code/config.rs:298`, and six
   `eprintln!` lines in `crates/nvs-cli/src/{bundle,queue,schema}.rs`. Then the test that scans
   every help text and card, and delete `data/gaps/nvs-types/diagnostic-help-prints-rule-ids.json`.

**The sixth item** (`data/gaps/nv/two-lang-features-are-one-syntax.json`) has its list:
`docs/examples/lang/statements/chapter.md:16` and `:18` are the two headings; the examples are
`docs/examples/lang/statements/{throw,try-catch-finally}/` (4 and 5 pairs, `about.md`, one
`details.md`); the attacks `tests/hostile/lang/statements/{throw,try-catch-finally}/`; the benches
`benches/members/lang/statements/{throw,try-catch-finally}.nvs`; `// covers:` lines in
`tests/conformance/error/{two-catch-clauses-pick-by-class,a-nested-finally-runs-inside-out,throwing-a-nullable-throwable-throws-the-object-it-holds,a-throw-crosses-two-frames}.nvst`;
`website/src/content/docs/syntax/errors/throw-and-catch.mdx:3,12,18`; `website/site.lock.json:178`;
and the perf rows in `data/perf/members.md`, `bench-budget.json` and `members.ndjson`.

The installed `looper` binary has no `verify` subcommand, so these sessions run
`bun nv affected --run` as the gate. The next free diagnostic code is `E0881`
(`crates/nvs-diagnostics/src/code/types_e08.rs`). A new checker error can break an attack under
`tests/hostile/` that the gate does not run: grep them for the refused form before the gate.

### Backlog

The bullets below were carried from `release-hardening`'s backlog when it was reached.

- With `use Core\Json;`, `#[Json\Derive]` is `E0303` ("`Json\Derive` is not declared"), though `rule:core-classes/derive-attribute`'s own example writes `#[Json\Derive]`. Whether a `use` of a class reaches a qualified name below it is not settled (`crates/nvs-types/src/derive.rs`'s `attribute_is`).

- A class's constant table stores a `uint` constant as `ConstantValue::Int`, so a `static::` read typed with a union that holds `uint` (`int|uint`, `uint|string`) gets an `int` tag for it. The bits are right and the read is sound, but `is uint` on it gives `false`. A `Uint` row in `ConstantValue` and `nvs_types::ConstValue` closes it (`crates/nvs-runtime/src/helpers.rs`'s `nvs_class_constant`).

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
