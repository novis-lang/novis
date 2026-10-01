---
milestone: post-parity
position: last
---
# Loop goal 181 — `Core\Program` enumerates by base class, and hands back typed constructors

Two additions to `rule:programs/implementing`'s enumeration, both answered while compiling, as it is
today:

```nvs
// 1. The type argument may be a class. The result has every concrete class that is a `T`.
foreach (Core\Program::implementing<AbstractExporter>() as $exporter) { … }

// 2. A constructor per class, typed by a callable signature, so a constructor may take arguments.
foreach (Core\Program::constructors<Handler, callable(Db): Handler>() as $row) {
    if ($row["class"] === $wanted) {
        $handler = $row["make"]($db);      // Handler
    }
}
```

**The class selector.** `implementing<T>()` and `implementingWith<T, S>()` accept a class, abstract
or not, wherever they accept an interface today. The result is every non-abstract class `C` in the
program for which `$c is T` holds — so a concrete `T` is in its own list — sorted by fully-qualified
name, exactly as for an interface. An enum, a shape, a scalar and every other type are still refused
with `E0743`.

**`constructors<T, C>()`** expands to an array literal with one row per class `implementing<T>()`
would list, in the same order, of the shape `{class: string, make: C}`. `class` is the
fully-qualified name. `make` is the closure `fn(<C's parameters>): T => new Class(<the same
arguments, in order>)`, checked exactly as that closure would be if it were written at the call site —
visibility, argument types and defaults included. `C` must be a `callable(...)` type whose return type
is `T`. A class whose constructor that closure does not fit is a new code naming the class, and
`E0744` (a no-argument constructor) does not apply to this member. Nothing is built until `make` is
called.

## Why here

It needs nothing that is not built. `nvs_hir::implementors` already walks `extends` as well as
`implements` and already counts a class as matching itself
(`crates/nvs-hir/src/hierarchy.rs:@implements_interface`); the only thing standing between a class and
the enumeration is `E0743` in `crates/nvs-types/src/program.rs:@interface_of`. Typed callables
(`rule:types/callable-signature`) have shipped, and ADR 0061's *Alternatives rejected* names
constructor references as "the better shape, blocked on the same deferred typed-`callable`
signature" — the block is gone. Its *Revisiting* asks for "a second real use case" before a base-class
selector: a user asked for this one, for plugin families built on an abstract base.

It carries `position: last` because it lands behind goal `var-array-literal`, and sits in front of
goal `ci-green` because that goal proves the tree the run ends on and this one still changes it.

## Stage 0 — the catch-up

The sentences on disk this goal makes wrong, each rewritten whole by the session that lands the
behaviour and not before:

- `docs/rules/programs/implementing.md` — "`T` must be an interface type", and the paragraph that says
  why the selector is an interface. A class gives the loop body a static type as well. The record
  `data/rules/programs/implementing.json`'s title calls it "the one enumeration", which `constructors`
  ends.
- `docs/rules/programs/implementing-with.md`, where it says interface.
- `crates/nvs-diagnostics/src/lib.rs`'s entry for `E_PROGRAM_TYPE_ARG_NOT_AN_INTERFACE` (`E0743`). Its
  doc says a class "would enumerate something, but nothing could then be called on what came back",
  which is false for a class. The code keeps its number and its identifier is renamed only if a
  `grep` shows nothing outside `crates/` spells it.
- The module docs of `crates/nvs-stdlib/src/program.rs` ("What `T` names here is an interface") and
  `crates/nvs-types/src/program.rs` (`E0743`'s sentence), and `crates/nvs-hir/src/hierarchy.rs`'s doc
  on `implementors`.
- `docs/reference/lang/90-attributes.md`'s "`I` must be an interface; a class or a shape is refused",
  or the record it is generated from.
- `docs/examples/core/Program/implementing/about.md` and `implementingWith/about.md`, and the
  `Core\Program` class card and both reference cards in `crates/nvs-stdlib/src/program.rs`.
- `tests/conformance/reject/program-implementing-takes-an-interface-whose-classes-need-no-arguments.nvst`
  — its name becomes untrue. No goal record names it, so it is renamed after a `grep` of the tree for
  the old name finds no other citer.

The same search closes the stage as it opened it:
`grep -rn "must be an interface\|an interface type\|E0743\|the one enumeration" docs crates data`,
read line by line. Every hit is either true as it stands, rewritten, or under `docs/decisions/`.
`docs/novis.md`, `docs/ground-rules.md`, the `docs/rules/*.md` chapters and `website/` are generated
and are regenerated, never edited. `docs/decisions/0061.md` and `0212.md` are **not** edited; the new
record's `changes: modifies` says what it overtook.

## Stage 1 — the floor

Goal `var-array-literal`'s whole acceptance list, carried in by the goal switch. Never traded. Every
existing `implementing` and `implementingWith` case keeps its output unchanged: an interface selects
exactly what it selected before.

## Stage 2 — the class selector, the keystone

**Does:** Lets `implementing` and `implementingWith` take a class, and makes every enumeration member
opt the program into the scan.

One file set: `crates/nvs-types/src/program.rs`, `crates/nvs-hir/src/hierarchy.rs`,
`crates/nvs-hir/src/requires.rs`, `crates/nvs-diagnostics/src/lib.rs`.

- **The decision record**, written first, from § *Standing decisions*, for both halves of this goal.
  It `modifies` `programs/implementing` and `programs/implementing-with` and `adds`
  `programs/constructors`, and the fragments are rewritten whole with it.
- **`interface_of` accepts a class.** It becomes the one function that returns the selected type's
  name for an interface or a class, and reports `E0743` for everything else, with a message that says
  "an interface or a class". `implementors` needs no change unless a test shows otherwise.
- **`constructors` joins the scan opt-in.** It is added to `PROGRAM_SCAN_MEMBERS` in
  `crates/nvs-hir/src/requires.rs`, beside `implementing` and `implementingWith`, and
  `constructors_alone_opts_the_program_into_the_scan` pins it the way
  `implementing_with_alone_opts_the_program_into_the_scan` pins `implementingWith`.
- **Pinned by** the Stage 2 checks below, and
  `tests/conformance/core/program-implementing-takes-a-class-and-lists-every-concrete-subclass.nvst`
  (an abstract base, a concrete base with subclasses, and a class two levels down; prints the order),
  `tests/conformance/core/program-implementing-with-alone-finds-autoloaded-classes.nvst` (multi-file,
  in the shape of `tests/conformance/lang/a-class-named-only-by-an-attribute-is-autoloaded.nvst`), and
  the renamed reject case, which gains an enum and a shape as refused type arguments.

## Stage 3 — `constructors<T, C>()`

**Does:** Adds `Core\Program::constructors<T, C>()`, which returns one typed constructor per class
the enumeration lists.

One file set: `crates/nvs-stdlib/src/program.rs`, `crates/nvs-stdlib/src/registry.rs`,
`crates/nvs-types/src/program.rs`, `crates/nvs-diagnostics/src/lib.rs`, `crates/nvs-lsp/src/definition.rs`.

- **The registry row and its cards.** A row with no parameters and two written type arguments, and
  a return type spelling `array<{class: string, make: C}>` for the card, `nvs meta` and `nvs agent`,
  exempt from the registry's shape guard the way `implementingWith`'s row is. If `CoreTy` cannot
  spell a written callable variable, the smallest variant that can is added, and the handoff says so.
  The member card and the class card are this stage's Help proof.
- **The expansion** in `crates/nvs-types/src/program.rs`, beside `implementingWith`'s: one shape
  literal per class, each `make` an ordinary closure literal checked at the call site. An error
  inside a closure is reported on the call, naming the class, under the new code; the ordinary
  error it replaces is attached as a note.
- **Refusals.** `C` that is not a callable type with named parameters, or whose return type is not
  `T`, is a new code. A class whose constructor does not accept `C`'s parameters, or is not visible at
  the call site, is the class-naming code above. `E0744` is never reported by this member.
- **`nvs-lsp`.** Go-to-definition works on both type arguments, as it does on `implementing<User>`
  (`crates/nvs-lsp/src/definition.rs`'s type-position jump).
- **Pinned by** the Stage 3 checks below, and
  `tests/conformance/core/program-constructors-builds-each-class-with-the-arguments-you-pass.nvst` and
  `tests/conformance/reject/program-constructors-names-the-class-whose-constructor-does-not-fit.nvst`
  (a wrong argument type, a missing required parameter, a private constructor, a `C` returning
  something other than `T`; one diagnostic each).

## Stage 4 — the feature proofs and the reference

**Does:** Adds the tests, examples, attack, bench and reference text for both additions.

- **`Core\Program::constructors`** is a new feature and owes all of `rule:testing/feature-proofs`:
  `about.md` and three examples under `docs/examples/core/Program/constructors/` (a handler picked by
  name and built with a dependency, a plugin list built lazily, a command table printed without
  building anything), an attack under `tests/hostile/core/Program/constructors/` (a request tries to
  build a class whose constructor is private, and then one outside `T`, through the list; neither
  compiles), and `benches/members/core/Program/constructors.nvs` (building the list and calling one
  `make`, where a framework dispatches a request to one handler). `bun nv proofs --id` names the id.
- **`implementing` and `implementingWith` grow.** One new example in each directory selects by an
  abstract base class, and each `about.md` is rewritten whole to say an interface or a class.
- `bun nv reference` regenerates `docs/novis.md`.

## Standing decisions

- **The user's three calls:** the class selector is the existing `implementing<T>` and
  `implementingWith<T, S>` taking a class, never a new `extending` member; a concrete `T` is in its
  own list; the second addition is typed constructors, never a list of class names to feed
  `Core\Reflect::construct`.
- **Membership is `is`.** A class is listed exactly when it is non-abstract and `$c is T` holds. No
  second notion of "subclass" is written.
- **Order is fully-qualified name, for every member.** `constructors` lists the classes
  `implementing` would, in the same order, so the two can be zipped.
- **`make` is an ordinary closure.** No new calling path, no reflection, and nothing a program could
  not have written by hand. Visibility at the call site is the one `new` faces there.
- **One ADR slot**: the next free number when Stage 2 opens, checked right before it is written. It
  states the tradeoffs: nothing on the request path for the class selector, which is compile-time
  only; for `constructors`, one closure object per listed class per call, charged to the request and
  freed with the array, and no object built until `make` is called; two more members' worth of
  surface on `Core\Program` against removing the no-argument-constructor limit for frameworks that
  need dependencies.
- **Not in this goal**, and never started by a session: hierarchy readers on `Core\Reflect\ClassInfo`
  (`parent`, `interfaces`, `isAbstract`) and a namespace filter on the enumeration. A session that
  meets a need for either writes it into the handoff's backlog for the user.
- **Every comment in a new `.nvs` and every changed `about.md` follows `AGENTS.md` § *Text an end
  user reads* at the first write**, and `bun nv proofs --comments <paths>` is run over them before
  the wrap.
