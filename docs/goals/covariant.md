---
looper: 1
title: "A covariant parameter or property takes a narrower type, and a check guards each value that enters"
summary: "A method parameter and an instance property may be redeclared at a narrower type with covariant, each value that enters is checked where it enters, every other narrowing stays a compile error, the feature lang:classes/covariant has its proofs and its website page, every designed rule this goal owns is shipped, and the checks below pass."
context: [docs/rules/classes/an-override-keeps-its-parents-signature.md, docs/rules/classes/a-covariant-type-is-checked-where-a-value-enters.md]
stages:
  - id: 1
    title: "the rule, and `covariant` parses, formats and is highlighted"
    status: open
    context: [crates/nvs-syntax/src/token.rs, crates/nvs-syntax/src/ast.rs, crates/nvs-syntax/src/parser/expr.rs, crates/nvs-syntax/src/parser/decl.rs, crates/nvs-fmt/src/modifiers.rs]
  - id: 2
    title: "the checker takes a covariant parameter, and refuses every other narrowing"
    status: open
    context: [crates/nvs-types/src/conformance.rs, crates/nvs-diagnostics/src/code/types_e08.rs]
  - id: 3
    title: "a covariant parameter is checked when the method is entered"
    status: open
    context: [crates/nvs-ir/src/lower/mod.rs, crates/nvs-ir/src/lower/expr.rs, crates/nvs-ir/src/lower/operator.rs]
  - id: 4
    title: "a covariant property is checked by every typed write"
    status: open
    context: [crates/nvs-types/src/conformance.rs, crates/nvs-types/src/layout.rs, crates/nvs-ir/src/lower/stmt.rs, crates/nvs-ir/src/lower/mod.rs, docs/rules/classes/a-redeclared-property-keeps-its-parents-type.md]
  - id: 5
    title: "an erased, dynamic or hooked write of a covariant property is checked"
    status: open
    context: [crates/nvs-runtime/src/object.rs, crates/nvs-ir/src/lower/stmt.rs, docs/rules/types/erased-member-access.md, docs/rules/types/property-key-access.md]
  - id: 6
    title: "`lang:classes/covariant` has its feature proofs, and the rule is shipped"
    status: open
    context: [docs/rules/testing/feature-proofs.md, docs/examples/lang/classes/inheritance/about.md, benches/members/lang/classes/inheritance.nvs]
  - id: 7
    title: "the website explains `covariant` on a page of its own"
    status: open
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
`docs/rules/classes/a-redeclared-property-keeps-its-parents-type.md` exists. The goal sits behind
compiler-holes on the chain, so the gate holds when it starts.

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

- [ ] The rule fragment with its reasons and Declined list, and its record `designed` with `goal: covariant`. `docs/rules/classes/a-covariant-type-is-checked-where-a-value-enters.md:1`, `data/rules/classes/a-covariant-type-is-checked-where-a-value-enters.json:1`
- [ ] `covariant` is contextual and recognised only before a parameter's or a property's type. `crates/nvs-syntax/src/token.rs:84`, `crates/nvs-syntax/src/parser/expr.rs:2021`, `crates/nvs-syntax/src/parser/decl.rs:287`, `crates/nvs-syntax/src/parser/decl.rs:1379`
- [ ] `Param` and `PropertyMember` carry a `covariant` flag, and a parser test reads each form: plain, `inout`, variadic, promoted, property, and `covariant` as a method name. `crates/nvs-syntax/src/ast.rs:666`, `crates/nvs-syntax/src/ast.rs:1817`
- [ ] `nvs fmt` writes `covariant` after every other modifier. `crates/nvs-fmt/src/modifiers.rs:87`
- [ ] Both grammars highlight it where the modifiers are. `editors/vscode/syntaxes/nvs.tmLanguage.json:169`, `website/config/novis.tmLanguage.json:268`

### 2. the checker takes a covariant parameter, and refuses every other narrowing

**Does:** a covariant parameter narrower than the topmost declaration compiles, and every misuse is a
compile error where it is written.

**Split:** at stage start, an `Explore` subagent lists every place a parameter's signature is printed
(hover, `nvs agent show`, a diagnostic), so the flag is printed everywhere at once.

- [ ] `check_method_overrides` accepts a narrower parameter type with `covariant`, measured against the topmost declaration, and keeps the `tainted` and `secret` check. `crates/nvs-types/src/conformance.rs:484`, `crates/nvs-types/src/conformance.rs:706`
- [ ] `E0404`'s help names `covariant` where the parameter is narrower. `crates/nvs-types/src/conformance.rs:712`
- [ ] Three new codes at the end of the E08 band: a misplaced `covariant`, a type `is` cannot test in constant time, and a topmost default the narrower type refuses. `crates/nvs-diagnostics/src/code/types_e08.rs:913`
- [ ] Every signature printer writes `covariant`. `crates/nvs-types/src/conformance.rs:712`
- [ ] Reject cases: each misplaced form, an array, a shape and a callable, a default that fails, a grandchild that drops the keyword, and a qualifier dropped. `tests/conformance/reject/a-covariant-parameter-is-written-only-where-it-narrows.nvst:1`
- [ ] Rust tests beside the override check. `crates/nvs-types/tests/covariant.rs:1`

### 3. a covariant parameter is checked when the method is entered

**Does:** a call that reaches a covariant parameter with a value of a wider type throws a `LogicError`
before the body runs, through a class, an interface, a static method and `compareTo`.

- [ ] `lower_method` emits one type test per covariant parameter before the body, each element of a variadic one included. `crates/nvs-ir/src/lower/mod.rs:1246`, `crates/nvs-ir/src/lower/mod.rs:1297`, `crates/nvs-ir/src/lower/expr.rs:6569`
- [ ] A failure throws a `LogicError` that names the method, the parameter, the type and the class it got. `crates/nvs-ir/src/lower/operator.rs:256`
- [ ] A case through a parent, an interface, a static method, `compareTo`, a grandchild, an `inout` and a variadic parameter, and a first-class callable of the method. `tests/conformance/class/a-covariant-parameter-is-checked-when-the-method-is-entered.nvst:1`

### 4. a covariant property is checked by every typed write

**Gate:** `docs/rules/classes/a-redeclared-property-keeps-its-parents-type.md` exists.

**Does:** a subclass redeclares an instance property at a narrower type with `covariant`, and a write
through a wider type that would store another class throws.

- [ ] The redeclaration check from stage 16 accepts `covariant` with a narrower type, and its help names `covariant`. `crates/nvs-types/src/conformance.rs:1`
- [ ] The class descriptor carries the narrowed type for each narrowed slot, and the slot stays the topmost declaration's. `crates/nvs-types/src/layout.rs:1004`
- [ ] A typed write that can reach a narrowed slot with a wider value loads the receiver's type and tests the value, and every other write is unchanged. `crates/nvs-ir/src/lower/stmt.rs:1137`, `crates/nvs-ir/src/lower/mod.rs:3034`
- [ ] A case for a plain property, a promoted parameter, `lateinit`, the parent's constructor writing a wider value, and a write through the subclass's own type that is not tested. `tests/conformance/class/a-covariant-property-is-checked-by-every-write.nvst:1`

### 5. an erased, dynamic or hooked write of a covariant property is checked

**Does:** no write path stores a value of another class in a narrowed slot.

- [ ] The erased write through `object` or `mixed`, the `property<T>` key write and the reflected write test the receiver's narrowed type (`rule:types/erased-member-access`, `rule:types/property-key-access`). `crates/nvs-runtime/src/object.rs:1`
- [ ] A hooked property tests before its `set` hook, and the guard and observer run after the test. `crates/nvs-ir/src/lower/stmt.rs:1217`
- [ ] A case for each path. `tests/conformance/class/a-covariant-property-is-checked-on-every-write-path.nvst:1`

### 6. `lang:classes/covariant` has its feature proofs, and the rule is shipped

**Does:** `bun nv proofs --id 'lang:classes/covariant'` prints nothing owed, and the rule's record is
`shipped` with its cases in `guardedBy`.

**Split:** writing subagents, one after another: the examples and `about.md` first, then the attack,
then the bench. The session checks each against AGENTS.md § *Text an end user reads* and writes the
rule flip.

- [ ] `about.md` says what `covariant` does, where it is written, and what a wrong value throws. `docs/examples/lang/classes/covariant/about.md:1`
- [ ] Three examples: a narrower parameter, the `LogicError` a wrong value throws, and a narrower property. A fourth shows `compareTo(covariant Price $other)`. `docs/examples/lang/classes/covariant/01-a-method-that-takes-a-narrower-type.nvs:1`
- [ ] The attack: a call through the parent, an interface, a callable, an `inout`, a variadic, an erased write and the parent's constructor each try to put a wider value in, and each throws (`rule:testing/hostile-case-contract`). `tests/hostile/lang/classes/covariant/01-a-wider-value-through-every-path.nvs:1`
- [ ] The bench: a call through the parent of a method with a covariant parameter, with `// bench: complexity constant`. `benches/members/lang/classes/covariant.nvs:1`
- [ ] `bun nv render` writes the reference section, and `nvs agent show covariant` prints it. `docs/reference/lang/50-classes.md:1011`, `crates/nvs-cli/src/agent.rs:355`
- [ ] The rule record `shipped` with every case of stages 2 to 5 in `guardedBy`, and the two neighbouring rules name `covariant`. `data/rules/classes/a-covariant-type-is-checked-where-a-value-enters.json:1`, `docs/rules/classes/an-override-keeps-its-parents-signature.md:1`

### 7. the website explains `covariant` on a page of its own

**Does:** a reader who looks up why their override does not compile finds a page that shows the
problem, the keyword and what happens to a wrong value.

**Split:** a writing subagent writes the page after stage 6 has landed; the session reads it against
AGENTS.md § *Text an end user reads*.

- [ ] The page: the problem a narrower parameter solves, the `E0404` without the keyword, the keyword, what a wrong value throws, a narrower property, why a return type needs no keyword, when a separate method is the better design, and a "Do and don't" list. It embeds the feature's examples with `<FeatureSection examples="lang/classes/covariant">`. `website/src/content/docs/syntax/classes/covariant.mdx:1`
- [ ] The sidebar lists it after "Methods and inheritance". `website/astro.config.ts:605`
- [ ] The inheritance page and the properties page each link to it in one sentence. `website/src/content/docs/syntax/classes/methods-and-inheritance.mdx:13`, `website/src/content/docs/syntax/classes/properties.mdx:1`

## Handoff

### State

The goal is new and nothing is built. Compiler-holes stage 16 makes a redeclared property keep its
parent's exact type first, and this goal relaxes that with `covariant`.

### Next

**Stage 1, one group**, on the syntax crate and the formatter. No gate.

- [ ] The rule fragment and its `designed` record. `docs/rules/classes/a-covariant-type-is-checked-where-a-value-enters.md:1`, `data/rules/classes/a-covariant-type-is-checked-where-a-value-enters.json:1`
- [ ] The contextual keyword, the parser and the two AST flags. `crates/nvs-syntax/src/token.rs:84`, `crates/nvs-syntax/src/parser/expr.rs:2021`, `crates/nvs-syntax/src/parser/decl.rs:287`, `crates/nvs-syntax/src/ast.rs:666`, `crates/nvs-syntax/src/ast.rs:1817`
- [ ] The formatter and the two grammars. `crates/nvs-fmt/src/modifiers.rs:87`, `editors/vscode/syntaxes/nvs.tmLanguage.json:169`, `website/config/novis.tmLanguage.json:268`

### Backlog
