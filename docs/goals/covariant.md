---
looper: 1
title: "A covariant parameter or property takes a narrower type, and a check guards each value that enters"
summary: "A method parameter and an instance property may be redeclared at a narrower type with covariant, each value that enters is checked where it enters, every other narrowing stays a compile error, the feature lang:classes/covariant has its proofs and its website page, every designed rule this goal owns is shipped, and the checks below pass."
context: [docs/rules/classes/an-override-keeps-its-parents-signature.md, docs/rules/classes/a-covariant-type-is-checked-where-a-value-enters.md]
stages:
  - id: 1
    title: "the rule, and `covariant` parses, formats and is highlighted"
    status: done
    context: [crates/nvs-syntax/src/token.rs, crates/nvs-syntax/src/ast.rs, crates/nvs-syntax/src/parser/expr.rs, crates/nvs-syntax/src/parser/decl.rs, crates/nvs-fmt/src/modifiers.rs]
  - id: 2
    title: "the checker takes a covariant parameter, and refuses every other narrowing"
    status: done
    context: [crates/nvs-types/src/conformance.rs, crates/nvs-diagnostics/src/code/types_e08.rs]
  - id: 3
    title: "a covariant parameter is checked when the method is entered"
    status: done
    context: [crates/nvs-ir/src/lower/mod.rs, crates/nvs-ir/src/lower/expr.rs, crates/nvs-ir/src/lower/operator.rs]
  - id: 4
    title: "a covariant property is checked by every typed write"
    status: done
    context: [crates/nvs-types/src/conformance.rs, crates/nvs-types/src/layout.rs, crates/nvs-ir/src/lower/stmt.rs, crates/nvs-ir/src/lower/mod.rs, docs/rules/classes/a-redeclared-property-keeps-its-parents-type.md]
  - id: 5
    title: "an erased, dynamic or hooked write of a covariant property is checked"
    status: done
    context: [crates/nvs-runtime/src/object.rs, crates/nvs-ir/src/lower/stmt.rs, docs/rules/types/erased-member-access.md, docs/rules/types/property-key-access.md]
  - id: 6
    title: "`lang:classes/covariant` has its feature proofs, and the rule is shipped"
    status: done
    context: [docs/rules/testing/feature-proofs.md, docs/examples/lang/classes/inheritance/about.md, benches/members/lang/classes/inheritance.nvs]
  - id: 7
    title: "the website explains `covariant` on a page of its own"
    status: done
    context: [website/src/content/docs/syntax/classes/methods-and-inheritance.mdx, website/src/content/docs/syntax/classes/properties.mdx, website/astro.config.ts]
checks:
  - id: owners-closes
    stage: 7
    name: no gap record or designed rule names this goal
    run: [bun, nv, owners, --closes, covariant]
    reach: true
    floor: false
---

# A covariant parameter or property takes a narrower type, and a check guards each value that enters

## Why

A subclass often wants a narrower type than its parent declares: `Feeder::feed(Animal $a)` and a
`DogFeeder` that only feeds a `Dog`. `rule:classes/an-override-keeps-its-parents-signature` refuses
that override (`E0404`), and it is right to: a call through `Feeder` may pass any `Animal`, and Novis
has no check below the type system, so the override would read a `Cat` as a `Dog`. What a program
writes instead is `if (!($a is Dog)) { throw ... }` or `$a as Dog` in every such method, and the
signature still says `Animal`. PHP programs hit the same wall and have no answer.

Generic classes would be the usual answer, and `rule:types/no-user-generics` declines them. Dart's
`covariant` is the answer without them: the override says it is narrower, and the compiler adds the
check where a value enters. The same holds for an instance property, which compiler-holes stage 16
makes keep its parent's exact type.

## Standing decisions

**The syntax is a contextual `covariant` before the type.** `public function feed(covariant Dog $a)`,
`inout covariant Dog $a`, `covariant Dog ...$all`, `public covariant Dog $pet;` and a promoted
`constructor(public covariant Dog $pet)`. It is written after the modifiers and before the type.
It lexes as an identifier and is recognised only there, like `spawn` and `type`
(`crates/nvs-syntax/src/token.rs:84`), so no program that names a method or a variable `covariant`
breaks. The formatter writes it after every other modifier.

**Where it is allowed.** On a parameter of a method that overrides a class's method or implements an
interface's, static methods and `Comparable::compareTo` included, and on an instance property that
redeclares an ancestor's. Anywhere else it is a compile error with one new code: a method that
replaces nothing, a constructor parameter that is not promoted, an anonymous function, a static
property, a constant and a `readonly` property. A static property needs no keyword, because each
class names its own storage. A constant needs none, because it is only read. A `readonly` property is
not redeclared at all (stage 16 of compiler-holes): a subclass passes the narrower value to the
parent's constructor and adds a method whose return type is narrower.

**Which types it takes.** Every type `is` tests in constant time: a class, an interface, an enum, a
scalar, and a nullable or a union of those. An `array<T>`, a shape, a callable, a `class<T>` and a
`property<T>` are refused with their own code, because their test walks the value and a call would
cost its size. The help says to take the wider type and test inside.

**Narrower is measured against the topmost declaration, and the keyword is written at every level.**
A parameter or property whose type is narrower than the first ancestor that declares it needs
`covariant`, also in a grandchild that only repeats its parent's narrowed type. Without it the
override is `E0404` and the property is stage 16's code, and both helps name `covariant`. A
`covariant` whose type is not narrower than the topmost declaration's is the misplaced-keyword code,
because it would cost a check that tests nothing. A `tainted` or `secret` qualifier is kept: the
keyword narrows the type, never a qualifier. A default the topmost declaration gives a parameter must
pass the narrower type, or the override is a compile error with its own code, since a call through the
parent fills that default.

**A parameter is checked when the method is entered, on every call.** Each covariant parameter gets
one type test before the body runs, an `inout` one on entry and each element of a variadic one. The
test is the one `is` lowers to (`crates/nvs-ir/src/lower/expr.rs:6569`). A failure throws a
`LogicError`, as an ordering of two different classes does (`crates/nvs-ir/src/lower/operator.rs:256`),
and its message names the method, the parameter, the type it takes and the class it got. The check is
not skipped for a call through the subclass's own type: that saves one compare, and a second calling
convention is not worth it until a bench shows the compare.

**A property is checked by each write that can reach it with a wider value.** The compiler sees every
class of a program (`rule:programs/no-runtime-autoload`), so it knows which properties some class
narrows. A write to any other property is unchanged. A write to a narrowed one is checked unless the
receiver's static class is the narrowing class or below and the value's static type is the narrowed
one. The class descriptor carries, for each slot some class narrows, the type that class's instances
take, and the checked write loads it from the receiver's descriptor and tests the value. The slot stays
the one the topmost declaration has (`crates/nvs-types/src/layout.rs:1004`). The erased writes through
`object` or `mixed`, a `property<T>` key write and a reflected write run the same test in the runtime.
A hooked property tests before its `set` hook runs, and the guard and observer pipeline runs after the
test. A failure throws the same `LogicError`, naming the class, the property, the type and the class it
got. A read is never checked.

**The feature id is `lang:classes/covariant`**, its examples under
`docs/examples/lang/classes/covariant/`, and its website page
`website/src/content/docs/syntax/classes/covariant.mdx`, listed in the sidebar after "Methods and
inheritance". Its help in the binary is its reference section, which `bun nv render` writes into
`docs/reference/lang/50-classes.md` from the examples and `nvs agent show` embeds
(`crates/nvs-cli/src/agent.rs:355`). Every page, example, comment, `about.md` and diagnostic this goal
writes follows AGENTS.md § *Text an end user reads*.

**The rule is `classes/a-covariant-type-is-checked-where-a-value-enters`.** Its Declined list carries
Eiffel's narrowing with no keyword (the risk is invisible at the override), generic classes
(`rule:types/no-user-generics`), Scala's abstract type members (generics under another name), a
covariant array (a check that costs the array's length) and skipping the entry check for a direct
call. `rule:classes/an-override-keeps-its-parents-signature` and compiler-holes stage 16's
`classes/a-redeclared-property-keeps-its-parents-type` each gain one sentence that names `covariant`
as the way to narrow.

**Gate.** Stage 4 starts only when compiler-holes stage 16 has landed:
`docs/rules/classes/a-redeclared-property-keeps-its-parents-type.md` exists. Stage 5's matrix rows
need compiler-holes stage 17: `tools/nv/cmd/soundness.ts` exists. The goal sits behind
compiler-holes on the chain, so both gates hold when it starts.

**Lanes.** One lane. Stages 1 to 5 share the parser, the checker and the lowering and run in order.
Stage 6's examples print what stages 3 to 5 return, and stage 7's page embeds stage 6's examples.

**Trade-offs.** Security holds: no body and no read ever sees a value of another type, because the
test runs before either. Latency: one class or scalar test per call of a method with a covariant
parameter, and one descriptor load and test per write that can reach a narrowed property from a wider
type. No other call or write changes. Memory: one type per narrowed slot in the class descriptor,
process-wide and never per request. Simplicity: the language gains one contextual keyword and one rule,
and it replaces the hand-written `is` test or `as` conversion in every override that wants a narrower
type. Usability: the risk is visible in the signature, and a wrong value throws an error that names
both types.

## Stages

### 1. the rule, and `covariant` parses, formats and is highlighted

**Does:** the rule exists as `designed`, and `covariant` is part of a parameter and a property in the
syntax tree. Nothing checks it yet.

- [x] The rule fragment with its reasons and Declined list, and its record `designed` with `goal: covariant`. `docs/rules/classes/a-covariant-type-is-checked-where-a-value-enters.md:1`, `data/rules/classes/a-covariant-type-is-checked-where-a-value-enters.json:1`
- [x] `covariant` is contextual and recognised only before a parameter's or a property's type. `crates/nvs-syntax/src/token.rs:84`, `crates/nvs-syntax/src/parser/expr.rs:2021`, `crates/nvs-syntax/src/parser/decl.rs:287`, `crates/nvs-syntax/src/parser/decl.rs:1379`
- [x] `Param` and `PropertyMember` carry a `covariant` flag, and a parser test reads each form: plain, `inout`, variadic, promoted, property, and `covariant` as a method name. `crates/nvs-syntax/src/ast.rs:666`, `crates/nvs-syntax/src/ast.rs:1817`
- [x] `nvs fmt` writes `covariant` after every other modifier. `crates/nvs-fmt/src/modifiers.rs:87`
- [x] Both grammars highlight it where the modifiers are. `editors/vscode/syntaxes/nvs.tmLanguage.json:169`, `website/config/novis.tmLanguage.json:268`

### 2. the checker takes a covariant parameter, and refuses every other narrowing

**Does:** a covariant parameter narrower than the topmost declaration compiles, and every misuse is a
compile error where it is written.

**Split:** at stage start, an `Explore` subagent lists every place a parameter's signature is printed
(hover, `nvs agent show`, a diagnostic), so the flag is printed everywhere at once.

- [x] `check_method_overrides` accepts a narrower parameter type with `covariant`, measured against the topmost declaration, and keeps the `tainted` and `secret` check. `crates/nvs-types/src/conformance.rs:905`, `crates/nvs-types/src/conformance.rs:1042`
- [x] `E0404`'s help names `covariant` where the parameter is narrower. `crates/nvs-types/src/conformance.rs:799`
- [x] Three new codes at the end of the E08 band: a misplaced `covariant`, a type `is` cannot test in constant time, and a topmost default the narrower type refuses. `crates/nvs-diagnostics/src/code/types_e08.rs:1068`
- [x] Every signature printer writes `covariant`. `crates/nvs-cli/src/meta.rs:1161`, `crates/nvs-lsp/src/completion.rs:3354`, `crates/nvs-lsp/src/hover.rs:764`
- [x] Reject cases: each misplaced form, an array, a shape and a callable, a default that fails, a grandchild that drops the keyword, and a qualifier dropped. `tests/conformance/reject/a-covariant-parameter-is-written-only-where-it-narrows.nvst:1`
- [x] Rust tests beside the override check. `crates/nvs-types/tests/covariant.rs:1`

### 3. a covariant parameter is checked when the method is entered

**Does:** a call that reaches a covariant parameter with a value of a wider type throws a `LogicError`
before the body runs, through a class, an interface, a static method and `compareTo`.

- [x] `lower_method` emits one type test per covariant parameter before the body, each element of a variadic one included. `crates/nvs-ir/src/lower/mod.rs:1476`, `crates/nvs-ir/src/lower/mod.rs:1317`, `crates/nvs-ir/src/lower/expr.rs:6927`
- [x] A failure throws a `LogicError` that names the method, the parameter, the type and the class it got. `crates/nvs-runtime/src/helpers.rs:193`
- [x] A case through a parent, an interface, a static method, `compareTo`, a grandchild, an `inout` and a variadic parameter, and a first-class callable of the method. The types include `int` under `int|string`, `Dog|Cat` under `Animal`, and `Dog` under `?Animal`, where a `null` throws. `tests/conformance/class/a-covariant-parameter-is-checked-when-the-method-is-entered.nvst:1`

### 4. a covariant property is checked by every typed write

**Gate:** `docs/rules/classes/a-redeclared-property-keeps-its-parents-type.md` exists.

**Does:** a subclass redeclares an instance property at a narrower type with `covariant`, and a write
through a wider type that would store another class throws.

- [x] The redeclaration check from stage 16 accepts `covariant` with a narrower type, and its help names `covariant`. `crates/nvs-types/src/conformance.rs:1`
- [x] The class descriptor carries the narrowed type for each narrowed slot, and the slot stays the topmost declaration's. `crates/nvs-types/src/layout.rs:1004`
- [x] A typed write that can reach a narrowed slot with a wider value loads the receiver's type and tests the value, and every other write is unchanged. `crates/nvs-ir/src/lower/stmt.rs:1137`, `crates/nvs-ir/src/lower/mod.rs:3034`
- [x] A case for a plain property, a promoted parameter, `lateinit`, the parent's constructor writing a wider value, and a write through the subclass's own type that is not tested. The types include `int` under `int|string` and `int` under `?int`, read back through both classes. `tests/conformance/class/a-covariant-property-is-checked-by-every-write.nvst:1`

### 5. an erased, dynamic or hooked write of a covariant property is checked

**Does:** no write path stores a value of another class in a narrowed slot.

- [x] The erased write through `object` or `mixed`, the `property<T>` key write and the reflected write test the receiver's narrowed type (`rule:types/erased-member-access`, `rule:types/property-key-access`). `crates/nvs-runtime/src/object.rs:1`
- [x] A hooked property tests before its `set` hook, and the guard and observer run after the test. `crates/nvs-ir/src/lower/stmt.rs:1217`
- [x] A case for each path. `tests/conformance/class/a-covariant-property-is-checked-on-every-write-path.nvst:1`
- [x] The redeclaration matrix of compiler-holes stage 17 gains a `covariant` parameter and a `covariant` property as rows, against every relation and path it lists, and `bun nv soundness --write` writes them. `tools/nv/cmd/soundness.ts:1`

### 6. `lang:classes/covariant` has its feature proofs, and the rule is shipped

**Does:** `bun nv proofs --id 'lang:classes/covariant'` prints nothing owed, and the rule's record is
`shipped` with its cases in `guardedBy`.

**Split:** writing subagents, one after another: the examples and `about.md` first, then the attack,
then the bench. The session checks each against AGENTS.md § *Text an end user reads* and writes the
rule flip.

- [x] `about.md` says what `covariant` does, where it is written, and what a wrong value throws. `docs/examples/lang/classes/covariant/about.md:1`
- [x] Three examples: a narrower parameter, the `LogicError` a wrong value throws, and a narrower property. A fourth shows `compareTo(covariant Price $other)`. `docs/examples/lang/classes/covariant/01-a-method-that-takes-a-narrower-type.nvs:1`
- [x] The attack: a call through the parent, an interface, a callable, an `inout`, a variadic, an erased write and the parent's constructor each try to put a wider value in, and each throws (`rule:testing/hostile-case-contract`). `tests/hostile/lang/classes/covariant/01-a-wider-value-through-every-path.nvs:1`
- [x] The bench: a call through the parent of a method with a covariant parameter, with `// bench: complexity constant`. `benches/members/lang/classes/covariant.nvs:1`
- [x] `bun nv render` writes the reference section, and `nvs agent show covariant` prints it. `docs/reference/lang/50-classes.md:1011`, `crates/nvs-cli/src/agent.rs:355`
- [x] The rule record `shipped` with every case of stages 2 to 5 in `guardedBy`, and the two neighbouring rules name `covariant`. `data/rules/classes/a-covariant-type-is-checked-where-a-value-enters.json:1`, `docs/rules/classes/an-override-keeps-its-parents-signature.md:1`

### 7. the website explains `covariant` on a page of its own

**Does:** a reader who looks up why their override does not compile finds a page that shows the
problem, the keyword and what happens to a wrong value.

**Split:** a writing subagent writes the page after stage 6 has landed; the session reads it against
AGENTS.md § *Text an end user reads*.

- [x] The page: the problem a narrower parameter solves, the `E0404` without the keyword, the keyword, what a wrong value throws, a narrower property, why a return type needs no keyword, when a separate method is the better design, and a "Do and don't" list. It embeds the feature's examples with `<FeatureSection examples="lang/classes/covariant">`. `website/src/content/docs/syntax/classes/covariant.mdx:1`
- [x] The sidebar lists it after "Methods and inheritance". `website/astro.config.ts:603`
- [x] The inheritance page and the properties page each link to it in one sentence. `website/src/content/docs/syntax/classes/methods-and-inheritance.mdx:13`, `website/src/content/docs/syntax/classes/properties.mdx:1`

## Handoff

### State

Stages 1 and 2 are done. `MethodSig::covariant` carries the flag, empty for every signature that
declares none (`crates/nvs-types/src/signatures.rs:90`), and `ResolvedCall::covariant` copies it.
`check_covariant_params` (`crates/nvs-types/src/conformance.rs:905`) measures each parameter against
its topmost declarations, found by `topmost_declarations`. `parent_params_problem` skips a covariant
parameter's type, so the nearest-ancestor check never reports it twice. The codes are `E0882`
misplaced, `E0883` not testable in constant time, and `E0884` a topmost default the narrower type
rejects. The rule fragment names them. The constructor, static and `readonly` property forms are
reported in `crates/nvs-types/src/signatures.rs` by `report_misplaced_covariant`, and the anonymous
function form in `check_anon_fn`. A covariant instance property is still not checked: that is stage 4.

Stage 3 is done. `lower_method` reads a plain covariant parameter's slot at `Ty::Tagged`, because
every argument slot is a full tagged `Value` whatever the caller's representation, and
`emit_covariant_check` (`crates/nvs-ir/src/lower/mod.rs:1476`) tests it with `test_shape` after
every parameter is bound, then untags it to the narrower representation. An `inout` one is a
`RefLoad` at `Ty::Tagged` on entry. A variadic one walks the array with `emit_every_element_with`
(`crates/nvs-ir/src/lower/expr.rs:6927`), whose witness is the first element that failed. A failure
calls `Helper::CovariantRejected`, and `nvs_covariant_rejected` throws a `LogicError` from the message
and `describe_argument` (`crates/nvs-runtime/src/object.rs`). The case is
`tests/conformance/class/a-covariant-parameter-is-checked-when-the-method-is-entered.nvst`.

A covariant parameter of a generator method still asserts in lowering
(`crates/nvs-ir/src/lower/generator.rs:709`).

Stage 4 is done and verified. `rule:classes/definite-property-initialization` says that
`parent::constructor(...)` assigns a redeclared property, for the `ancestors_assigned` change below.
What it landed:

- `check_property_redeclarations` (`crates/nvs-types/src/conformance.rs:155`) measures a `covariant`
  property in `covariant_property_problem` against the topmost declaration (`topmost_property`) and
  the nearest one, with `E0882`, `E0883` and `E0866`. A plain one is exact against both, and its
  `E0866` help names `covariant` when it is narrower.
- `SignatureTable::is_narrowed` indexes every property name some class declares `covariant`.
  `check_write_target` calls `record_covariant_write` (`crates/nvs-types/src/expr/assign.rs`), which
  marks a write when a class below the resolved class declares the property at a type the resolved
  type does not fit. `record_promoted_covariant_writes` does the same for a parent's promoted
  parameter. `ExprTypeTable::is_covariant_write` carries the mark.
- `layout::flatten_fields` gives a redeclared slot the subclass's own type spelling, so each class's
  descriptor already holds its narrowed type (`field_reprs`, `field_classes`, `field_checks` were
  own-class-first already).
- `Lowering::emit_covariant_property_check` (`crates/nvs-ir/src/lower/stmt.rs`) calls
  `Helper::CovariantPropertyChecked` before the guard and the hook in both `lower_store` arms, and in
  `promoted_stores`. `nvs_runtime::object::check_covariant_write` runs `ClassDesc::check_slot` on the
  receiver's run-time class and throws the `LogicError`. A promoted covariant parameter now takes the
  stage 3 entry test, emitted before the promoted stores.
- **A behaviour change outside the feature:** `ctor_init::check_class_init` no longer requires a class
  to assign a property it declares again when an ancestor declares it with no default and not
  `lateinit` (`ancestors_assigned`). The ancestor's constructor assigns it. Without this a subclass
  that redeclares a property could not use `parent::constructor(...)` to set it.
- The Declined sentence in `docs/rules/classes/a-redeclared-property-keeps-its-parents-type.md` that
  refused a checked narrower property now names `covariant` instead.

Stage 5's run-time half is done. `write_property` (`crates/nvs-runtime/src/object.rs`) serves the
`object` and `mixed` writes, the `property<T>` key write and `Core\Reflect`'s `set`. When
`check_slot` refuses a value, `ClassDesc::ancestor_admits` asks every ancestor descriptor in
`conforms` that has the same slot. If one takes the value, the slot was narrowed, and
`covariant_rejected` throws the typed write's `LogicError`. Otherwise the old `RuntimeError` stays.
It compares types, not spellings, so `Animal|null` under `?Animal` is not narrowed, and a write that
fits never pays for the walk. A hooked property runs the same test before its guard and its `set`
hook. The case `tests/conformance/class/a-covariant-property-is-checked-on-every-write-path.nvst`
covers each path, and also a parent's `set` hook that stores a different value into the backing slot:
that write is tested too.

Hooks are resolved statically (`rule:classes/property-hooks`), so a write through the parent's type
runs the parent's `set` hook on a subclass instance. The test runs before it, so this is safe.

Stage 5 is done and verified. The soundness matrix has the rows `covariant-method-param` and
`property-covariant` (`tools/nv/cmd/soundness.ts`, `covariantVerdicts`, `WIDER`, `covariantCall`,
`covariantWrite`). A relation compiles where the child's type is narrower: `subclass`,
`int-under-int-string`, `t-under-nullable` and `member-under-union`. `same`, `superclass`,
`nullable-over-t` and `wider-union` are `E0882`. `unrelated`, `int-over-float` and `float-over-int`
are `E0404` for the parameter and `E0866` for the property. Each path except the child's own type
passes a value the parent takes and the child does not, and prints the `LogicError`. The union paths
of the property throw too. All 465 cells pass alone on the debug build. The property's parent
constructor takes the value as an argument, since a parent that stores its own wider value throws
when the child is constructed.

The nightly sweep's failures (run `39462919214`, on `6544156`) were already fixed on `main`: push
run `39506154898` on `691cbcd` has the three `proofs` jobs and the chapter job green, and `ci.yml`
has since split `miri` into the `suites` and `samples` legs.

Stage 6 is done and verified. `# \`covariant\`` follows `# Inheritance` in the chapter source, and
`bun nv proofs --id 'lang:classes/covariant'` prints nothing owed. The session wrote the files itself
and did not use the Split line's subagents, because each is short. What landed:

- `docs/examples/lang/classes/covariant/`: `about.md`, `details.md` (where the keyword is allowed,
  which types, `E0882` to `E0884`, each write path), and five examples. They are a narrower
  parameter, the `LogicError`, a narrower property, `compareTo(covariant Price $other)` under an
  `Amount implements Comparable`, and `05-…` the `E0404` without the keyword (`proof: compile-error`).
- The attack `tests/hostile/lang/classes/covariant/01-a-wider-value-through-every-path.nvs` has eight
  steps, each caught: parent, interface, callable, `inout`, variadic, a write through `mixed`, the
  parent's constructor, and 200000 wrong calls in a loop.
- The bench `benches/members/lang/classes/covariant.nvs` calls `apply(covariant Doubling $step, …)`
  through an abstract parent. The `lang:classes` group's perf is recorded again after the chapter
  edit (`data/perf/members.ndjson`).
- The five covariant conformance cases and the first test of `crates/nvs-types/tests/covariant.rs`
  carry `// covers: lang:classes/covariant`.
- The rule record is `shipped`, with the Rust test, the five cases, the four soundness files and the
  attack in `guardedBy`. `rule:classes/an-override-keeps-its-parents-signature` names `covariant`
  in its parameter bullet. The property rule already named it.
- The binary shows the section as `nvs agent show 'classes#covariant'`. A bare `covariant` lists that
  section as the nearest match, the same way every `lang:` feature works.

Stage 7 is done and verified, and so every stage is done. The page
`website/src/content/docs/syntax/classes/covariant.mdx` was written by a subagent, as the Split line
says, and read against AGENTS.md § *Text an end user reads*. The session rewrote the "separate method"
section and two Do bullets. The sidebar lists "Covariant parameters and properties" after "Methods and
inheritance" (`website/astro.config.ts:603`). The inheritance and properties pages each link to it in
one sentence. `bun nv verify --doc` is green, and `bun nv owners --closes covariant` reports nothing.

The installed `looper` has no `verify` subcommand, so the end-of-group gate was `bun nv affected
--run`, which is green.

The nightly sweep's failures on run `39462919214` are the ones already fixed on `main`, as above.
The latest push runs on `main` (`39506154898` CI and `39506154943` Website, on `691cbcd`) are green.

### Next

Nothing. The goal is reached and its file is ready to be retired. Every decision its prose held is in
`rule:classes/a-covariant-type-is-checked-where-a-value-enters`.

### Backlog

The goal's open items moved to gap records owned by `diagnostic-records`:

- `data/gaps/nvs-ir/a-covariant-parameter-of-a-generator-method-panics-in-lowering.json`
- `data/gaps/nvs-types/a-grandchild-covariant-type-unrelated-to-its-parents-compiles.json`
- `data/gaps/nvs-runtime/a-covariant-write-finds-its-slot-by-a-name-scan.json`

The note on a parent default the narrower type rejects is now the doc comment of `ancestors_assigned`
(`crates/nvs-types/src/ctor_init.rs:461`).
