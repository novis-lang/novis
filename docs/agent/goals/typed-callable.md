---
milestone: post-parity
---
# Loop goal 12 — a `callable` carries its signature

Give the type system the one thing it still cannot say about a function value: what it takes and what it
gives back. `rule:types/callable-signature` is the whole design and this
goal is its implementation — the grammar, the assignability rule, the inference that makes it free to
use, the retirement of the two bespoke binding-site variants that stood in for it, and the codegen that
finally spends the proof.

The reason this is worth a goal is in `crates/nvs-runtime/src/closure.rs`'s own module doc, not in
ergonomics. With no parameter list "**no checker can compare a call site against the body it will
reach**, and the compiled `invoke` reads argument slot *i* at its own declared representation. Hand it a
mismatch and the callee reinterprets the payload — an `int` read as an `NvsStr` pointer is an arbitrary
dereference, not a fault." Today `check_param_tags` stands in that gap with a mask-and-compare per
argument, per call, on the path `Core\Arr::map` sits on. This goal turns a **priority 1** runtime guard
into a compile-time proof and stops paying for it where it is proven.

Its floor is goal `schema`'s whole list, which is the parity program plus the temp sweep, the program id and
`Core\Db\Schema`.

## Why here

Post-parity: `rule:types/callable-signature`'s typed `callable`. Last on the chain because it is the
only entry that reaches the front end — `nvs-syntax` through `nvs-codegen` — and every goal before
it writes callbacks whose rows this one rewrites; running it earlier would mean converting the same
registry rows twice.

## The surface, in one block

```php
callable(User, string): string   $format;     // parameters, then a mandatory return type
callable(): void                 $onExit;
callable                         $anything;   // still legal: the top of the lattice

// arity is a prefix match — the runtime already trims, so the type says so
Core\Arr::map($users, fn($u) => $u->name);    // $u : User, inferred; result array<string>
```

## Stage 0 — the catch-up

Nothing. `rule:types/callable-signature` landed with this goal, and no fixture predates it.

## Stage 1 — the floor

Goal `schema`'s whole acceptance list — the parity program, the temp sweep, `Core\Program::id()` and
`Core\Db\Schema` — never traded.

## Stage 2 — the keystone: the atom, and what it compares to

One file set: `crates/nvs-syntax/src/parser/ty.rs`, `crates/nvs-syntax/src/ast.rs`,
`crates/nvs-types/src/ty.rs`, `crates/nvs-types/src/expr/assign.rs`.

1. **The grammar.** `parse_type` (`crates/nvs-syntax/src/parser/ty.rs:167`) gains `rule:types/callable-signature`'s
   production. A `(` after `callable` is only ever a parameter list — a type position has no call syntax
   — so this needs none of the checkpointed trial parse `array<T>` and `new Foo<...>` require. A missing
   `: R` and a named parameter are both diagnostics here, not parses that fail later.
2. **The representation.** A `Ty::CallableSig { params, ret }` beside `Ty::Callable`
   (`crates/nvs-types/src/ty.rs:170`), interned like every other type, and rendered by the display arm at
   `:520` as it is written.
3. **Assignability.** `is_assignable` (`crates/nvs-types/src/expr/assign.rs:56`) gains `rule:types/callable-arity` and `rule:types/callable-variance`:
   arity `n ≤ m` comparing the first *n*, parameters contravariant, return covariant, and every callable
   type assignable to bare `callable`. This is the first non-invariant relation in the checker; § 4 of
   the ADR is the one home for why `array<T>`'s invariance does not reach it.

## Stage 3 — the inference that makes it free

One file set: `crates/nvs-types/src/expr/calls.rs`, `crates/nvs-types/src/expr/args.rs`.

1. **A `fn` literal takes its parameter types from the position it is written in.** `check_fn_literal`
   (`crates/nvs-types/src/expr/calls.rs:1513`) already checks the body in a scope of its own; it gains an
   expected type, and each unannotated parameter takes the corresponding position's type from it. An
   annotated parameter is checked against it under § 4 and wins where it is wider.
2. **`E0450` is unchanged** — a block-bodied `fn` still declares its return type. `rule:types/callable-literal-inference` says so
   explicitly; do not relax it here.

## Stage 4 — the stdlib rows, and the first variant retired

One file set: `crates/nvs-stdlib/src/registry.rs`, `arr.rs`, `cli.rs`, `db/registry.rs`,
`crates/nvs-types/src/core_lib.rs`, `crates/nvs-cli/src/meta.rs`.

1. **`CoreTy::CallableTo` is deleted** (`crates/nvs-stdlib/src/registry.rs:335`), with its `Ty`
   counterpart (`crates/nvs-types/src/ty.rs:188`), its lowering (`crates/nvs-types/src/core_lib.rs:454`),
   its reader `callback_result_var` (`crates/nvs-types/src/generics.rs:137`) and its `meta` rendering
   (`crates/nvs-cli/src/meta.rs:305`).
2. **Five rows write an ordinary type instead** — `arr.rs:118` (`map`), `cli.rs:318` and `:327`,
   `db/registry.rs:361` and `db/transaction.rs:646`. `map` becomes
   `map(array<T> $a, callable(T, string): U $fn): array<U>`.
3. **`bind` descends into a callable type** (`crates/nvs-types/src/generics.rs`), which is `rule:types/callable-signature`'s
   first extension: one more structural position, no constraint set, no occurs check. The gap this
   closes is that module's own — a callback that is a *variable* now binds what only a written literal
   bound before.
4. **[docs/spec/01-core-library.md](../../spec/01-core-library.md) § *Arr* is edited in the same slice**
   as the registry rows it must agree with, never before them.

## Stage 5 — `Task::all`, and the second variant retired

One file set: `crates/nvs-types/src/generics.rs`, `crates/nvs-stdlib/src/task.rs`, the spec's *Task*
section.

1. **`CoreTy::CallableShapeTo` is deleted** (`crates/nvs-stdlib/src/registry.rs:361`) with
   `callable_shape_var` (`crates/nvs-types/src/generics.rs:152`), and `task.rs:137`/`:148` write
   `{name: callable(): T, …}` instead.
2. **A shape of callables rebuilds a shape** — `rule:types/callable-signature`'s second extension: walk each field, take its
   callable return type, assemble a shape with the same names. One descent, one construction, in the
   same single pass the module already makes.
3. **`Task::all`'s written-literal restriction is removed**, in the spec and in the checker: a field
   holding a `callable(): T` variable now carries what the field needs, so the compile error naming the
   field goes with it.

## Stage 6 — spending the proof

One file set: `crates/nvs-ir/src/lower/expr.rs`, `crates/nvs-ir/src/lower/closure.rs`,
`crates/nvs-runtime/src/closure.rs`, `crates/nvs-codegen`.

1. **A proven call site emits no tag check.** Where the callee's type names its parameters, the
   `check_param_tags` sequence (`crates/nvs-runtime/src/closure.rs:444`) is not emitted; where it is bare
   `callable`, it is emitted exactly as today.
2. **Both metadata slots stay on every closure object** — `CLOSURE_ARITY_SLOT` and
   `CLOSURE_PARAM_TAGS_SLOT`, written at `lower_closure_literal`
   (`crates/nvs-ir/src/lower/expr.rs:2378`). A closure does not know at its literal which kind of site
   will call it, and bare `callable` still needs both. What is removed is the work, not the metadata.
3. **The valgrind leg is not optional here.** § 7 changes what is emitted around a call whose refcount
   protocol `call_closure` owns, which is the one place `Core`'s borrow convention and a compiled
   method's ownership convention are reconciled.

## Standing decisions

- **`rule:types/callable-signature` is settled and is not re-derived.** Its five decisions — the spelling with a mandatory
  return, no parameter names, bare `callable` as the lattice top, prefix arity, contravariant
  parameters with a covariant return, and inference from the expected type — were taken with the user
  before this goal was written. A session that finds an implementation reason one of them is wrong
  records it in the ADR's *Revisiting* and implements the decision as written; it does not choose
  differently and it does not report `BLOCKED`.
- **This goal may open no new ADR number.** Every design question inside it has a home: `rule:types/callable-signature`'s body
  for the rule, the touched module's doc comment for a mechanism, the playbook for a trap.
- **The new diagnostics go in `E08xx`, opening at `E0800`** — ADR 0136 § *Diagnostics* and
  [docs/adr/README.md](../../adr/README.md) § *Decisions taken at project start* both record the
  allocation. Add the band's row to `nvs_diagnostics::code`'s legend table with the first code.
- **`E0450` is not relaxed** and whole-body return-type inference is not attempted, however tempting it
  looks once stage 3's expected type is in hand. It is ADR 0136 § *Revisiting*'s second entry and belongs
  to a later decision.
- **Optional and variadic parameters in a callable type are refused**, not deferred with a shape in
  mind — ADR 0136 § *Revisiting*'s first entry says why the prefix rule already covers the demand.
- **Ambiguity about where a rule lives resolves toward the checker, not the runtime.** The whole point
  of the goal is moving a check earlier; a case that could be answered in either place is answered in
  `nvs-types`, and the runtime keeps only what bare `callable` still needs.
- **Stages 4 and 5 edit the spec in the same slice as the registry rows**, never as a separate tidy-up:
  `rule:core-api/reference-card` makes the registry and the spec answer field-wise, and a slice that moves one without the
  other is what that rule exists to prevent.
