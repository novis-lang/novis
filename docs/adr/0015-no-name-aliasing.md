# ADR 0015 — No PHP-style name aliasing; `type` aliases are the disciplined exception

- **Status:** Accepted
- **Date:** 2026-08-20
- **Scope:** `class_alias()`; the `use X as Y;` import form for classes, interfaces, traits and enums; the
  `as`-rename and `as`-visibility clauses of trait composition; and, as the one thing this ADR *adds* rather
  than removes, a new `type Name = TypeExpr;` declaration
- **Amends:** [0007](0007-explicit-type-system.md) — the *Negative* bullet calling a `type` alias "the
  obvious relief… deliberately deferred" is resolved here rather than left in *Revisiting*; § 3's type
  grammar gains `type` alias names as a third kind of identifier atom, alongside `ClassName` and an enum's
  name. [0011](0011-functions-and-constants-are-class-members.md) — § 2's "a `use` alias that shadows
  anything under it" no longer describes a real construct, since import aliasing is gone; see *Decision § 4*
  for the reworded rule, and any future `use function`/`use const`-shaped shorthand for `Core` members
  named in that ADR's *Revisiting* inherits the same no-renaming rule this ADR states.
- **Relates to:** [0004](0004-memory-for-simplicity.md) (simplicity is bought here, again, by refusing a
  second name rather than by spending memory), [0006](0006-isolated-script-execution.md) (a value crossing
  an isolate boundary carries one class identity, never a locally-chosen alternate spelling of it),
  [0008](0008-static-and-global.md) and [0011](0011-functions-and-constants-are-class-members.md) (both
  already establish "exactly one declared home" for state and for behaviour; this ADR gives the same answer
  for a *name*)

> **In short:** a class, interface, trait, enum, method or constant is reachable under **exactly the name it
> was declared with** — its own short name, or a fully-qualified path to it — and under no other. PHP's three
> ways to give something a second name are all rejected: `class_alias()` does not exist in `Core` and never
> will; `use Foo\Bar as Baz;` is a diagnostic, not an import; and trait composition's `as` clause is gone
> entirely, both the renaming form and the visibility-only form, leaving `insteadof` as the sole conflict-
> resolution tool. The one alias MWL keeps is a **`type` alias** — a new, compile-time-only synonym for a
> type *expression* (`type UserId = uint;`, `type Row = array<string, int|string>;`), erased entirely by the
> type checker, never a second runtime-reachable name for a single class. To keep that from becoming the
> same loophole through a different door, a `type` alias may not name a single bare class, interface or enum
> on its own — that case is exactly `use … as …` in disguise, and *Decision § 3* rejects it the same way.

## Context

PHP gives a name three separate ways to acquire a second, locally valid spelling:

| mechanism | what it does |
|---|---|
| `class_alias('Original\Name', 'Alias')` | registers a second global class name at runtime, interchangeable with the first everywhere `instanceof`, `new`, and autoloading look |
| `use Original\Name as Alias;` | within one file, rebinds a short name the importer picked to a class/interface/trait/enum declared elsewhere |
| `use TraitA, TraitB { TraitA::foo as bar; }` / `{ TraitA::foo as protected; }` | within one class, gives a trait's method a second name, or the same name under different visibility |

Every one of these is the same shape of problem this project has already closed twice: [ADR
0008](0008-static-and-global.md) made state reachable from exactly one declared class-relative home, and
[ADR 0011](0011-functions-and-constants-are-class-members.md) did the same for behaviour. A name is supposed
to have one declared owner a reader can find by looking at the declaration. Aliasing breaks that
symmetrically: instead of "no declared home," it gives a *second* declared home for the same thing, so a
reader who has found one spelling still cannot be sure it is the only one in play.

**Simplicity (priority 4).** Two different tokens that mean the same class is pure surface, with no
behaviour it buys. A reviewer reading `Baz::method()` in one file and `Name::method()` in another must
already know both resolve to the same class before either line means anything — the alias is a second fact
to hold in the reader's head for zero semantic gain.

**Security (priority 1), narrowly but really.** `class_alias()` is exactly the tool PHP autoloading exploits
and feature-flag frameworks reach for to swap which implementation a name resolves to *after the fact* —
the whole point is that code written against `Alias` cannot tell, by reading it, which class actually runs.
That is the identical problem [ADR 0014](0014-property-observer.md) closed for property access (no
`__get` fallback) and [ADR 0011](0011-functions-and-constants-are-class-members.md) closed for calls (no
bare-name resolution): a name's target should be exactly what its own declaration says, not a runtime-mutable
indirection layer sitting in front of it.

**What this ADR does not throw out.** A reference (`&$x`) binding two *variable names* to one storage slot
is untouched — [ADR 0007](0007-explicit-type-system.md) § 1 already requires both sides to declare the same
type, and that is a different concept (two local names for one value slot, scoped to a function) from what
this ADR closes (a second global name for one declared class, function, constant, or trait method, visible
across files). Nothing here touches variables at all.

**The one place a name genuinely needs a second, shorter spelling.** [ADR 0007](0007-explicit-type-system.md)
already flagged its own cost: `array<array<int|string>> $rows` at every declaration site is real verbosity,
and its *Negative* section named a `type` alias as "the obvious relief" without deciding it. That request
has not gone away just because PHP's aliasing is rejected — it is answered here, deliberately kept separate
from the rejection above, because a synonym for a *type expression*, erased before a single byte of code
runs, is not the same thing as a second name for a *runtime-reachable declaration*. The rest of this ADR
draws that line precisely enough that it cannot be used to smuggle the rejected kind back in.

## Decision

**Nothing gets a second runtime-reachable name. `class_alias`, import `as`, and trait-use `as` are all
rejected. A `type` alias is accepted as a distinct, compile-time-only mechanism, and is barred from aliasing
a single bare class, interface or enum — the one case that would reopen this decision.**

### 1. `class_alias()` does not exist

There is no `Core` function that registers a second name for a class, and there will not be one added later
under a different name — this is not a missing-stdlib-coverage gap for M8 to fill in, it is the mechanism
this ADR removes. A class is reachable exactly where it is declared (its own short name, in scope via `use`
or a fully-qualified reference) and nowhere else.

### 2. Import `as` is rejected

```php
use App\Models\User as Model;   // rejected — diagnostic naming the collision or the rule
```

`use Path\To\Name;` imports under the declared short name, full stop. Renaming an import to something else
is a diagnostic: *imports cannot be renamed; refer to `Name` by its declared short name, or use the
fully-qualified path directly.* This applies uniformly to classes, interfaces, traits and enums — PHP does
not let `use` rename a function or constant import differently from a class import, and neither does this
rule need to, since [ADR 0011](0011-functions-and-constants-are-class-members.md) already removed
free-standing functions and constants from having an import form of their own. If a future MWL version adds
a `use`-shaped shorthand for reaching a `Core` member without the `Class::` prefix — the possibility ADR
0011 names in its own *Revisiting* — it inherits this rule without needing its own ADR: no renaming, ever, on
import.

The one real cost of this — two unrelated libraries independently choosing the same short class name — is
the same cost PHP developers already pay when they choose not to alias: use the fully-qualified name at the
call site. That is strictly more to type and strictly less to keep track of, which is the trade this whole
ADR makes on purpose.

### 3. Trait composition's `as` clause is gone; only `insteadof` remains

```php
trait Greets   { public function hello(): void { /* … */ } }
trait Announces { public function hello(): void { /* … */ } }

class Greeter {
    use Greets, Announces {
        Greets::hello insteadof Announces;   // kept — picks a winner on collision
        Greets::hello as sayHello;           // rejected — renaming
        Greets::hello as protected;          // rejected — visibility-only, still an `as` clause
    }
}
```

`insteadof` is unaffected: composing two traits whose methods collide by name still requires picking exactly
one to win, and that stays exactly as PHP has it. What is rejected is every use of `as` inside a trait `use`
block, with no exception for the visibility-only form — a class needing the trait's method with a *different
name* or a *different visibility* declares its own method under the name and visibility it wants, and calls
the trait's version explicitly from inside it:

```php
class Greeter {
    use Greets, Announces {
        Greets::hello insteadof Announces;
    }

    protected function hello(): void {
        Greets::hello();   // ordinary qualified call, not an alias
    }
}
```

This is not a workaround invented for this ADR — `TraitName::method()` from inside an overriding method is
already how PHP lets a class body reach a specific trait's implementation, the same shape as a `parent::`
call. Rejecting `as` costs nothing beyond writing that method explicitly, in ordinary syntax a reader already
knows, instead of a declarative rename a reader has to go looking for.

### 4. `Core`-collision wording in ADR 0011 no longer needs the alias case

[ADR 0011](0011-functions-and-constants-are-class-members.md) § 2 refused "a `namespace` declaration, a
class declaration, or a `use` alias that shadows anything under [`Core`]." The third case described a
plain import being renamed to a name that collides with a `Core` class — e.g. `use My\Custom\Thing as Str;`
making a bare `Str` inside that file resolve to an unrelated class. Since *Decision § 2* removes import
aliasing outright, that specific spoofing shape is now structurally impossible rather than merely refused:
there is no `as` left to rename anything to `Str` in the first place. The remaining, real case — a plain
`use My\Custom\Str;` whose *own* declared short name happens to collide with `Core\Str` — was already an
ordinary duplicate-import error before this ADR and needs no rule of its own here.

### 5. `type` aliases: a compile-time-only synonym for a type expression

```php
type UserId  = uint;
type Row     = array<string, int|string>;
type Result  = User|NotFoundError;
type Matrix  = array<array<float>>;
```

- **New declaration, file/namespace scope.** `type Name = TypeExpr;` sits alongside `use` and `namespace`
  declarations, not inside a class. This does not reopen
  [ADR 0011](0011-functions-and-constants-are-class-members.md)'s "every callable is a method, every constant
  a class constant" rule: that rule closed off *runtime-reachable* names with no declared class owner. A
  `type` alias has no runtime existence at all — like a `use` import or a `namespace` statement, it is fully
  resolved and discarded by the type checker before codegen ever runs, so it is not the kind of name that
  rule was written to govern.
- **`TypeExpr` is any production of [ADR 0007](0007-explicit-type-system.md) § 3's grammar**, with one
  exception (below). The alias name becomes lexically valid anywhere `ClassName` is — `atom := … | ClassName
  | TypeAliasName | '?' atom` — resolved by the same contextual lookup that already tells `self`/`static`/
  `parent`/an enum's name apart from a class's.
- **Fully transparent, never nominal.** `UserId` and `uint` are the same type everywhere, in both directions,
  with no wrapper and no runtime tag — assigning one where the other is expected needs no `as`, because
  after the checker resolves the alias there is only ever one type there, not two related ones. This is
  deliberate: a distinct, non-interchangeable "new type over an existing representation" (what other
  languages call a newtype) is a different feature, not requested here, and not this ADR's decision.
- **Zero runtime footprint.** Nothing downstream of the type checker — codegen, the value layout, `gettype()`
  / `var_dump()`, an [isolate boundary](0006-isolated-script-execution.md) crossing — ever sees the alias
  name; only the expanded type. This is what makes it categorically different from every mechanism *Decision
  §§ 1–3* reject: those all create a second name a *runtime* observer can still see (a second class identity,
  a second callable). A `type` alias creates no runtime-observable name at all.
- **Resolved eagerly, and a cycle is a diagnostic.** `type A = B; type B = A;` is rejected at check time
  (unresolvable alias cycle), not left to loop or to silently bottom out at `mixed`.
- **Non-parametric for now.** `type Rows<T> = array<array<T>>;` is out of scope until user-defined generics
  are designed — the same deferral [ADR 0007](0007-explicit-type-system.md) *Revisiting* already carries for
  generics generally; a `type` alias is not the vehicle for smuggling that decision in early.
- **Reached through ordinary `use`/FQN resolution, nothing auto-imported** — the same "nothing is global by
  default" reading every prior ADR in this project gives every other kind of name.

### 6. The one thing a `type` alias may not do: alias a single bare class

```php
type Id = SomeClass;   // rejected — diagnostic naming this ADR
```

A `TypeExpr` that is nothing but one bare `ClassName`, `EnumName`, `self`, `static` or `parent` atom — with
no union, intersection, array wrapper, or `?` sugar around it — is refused. Allowing it would be
`use SomeClass as Id;` wearing the type grammar as a disguise: a second name a *runtime* observer would
plausibly expect to mean the same thing `SomeClass` does everywhere, which is precisely the case *Decision §
2* just closed. A `type` alias exists to give a short name to a *shape* — a union, an intersection, a
parameterised array — never to a single already-named class. `type Ids = array<SomeClass>;` and `type Result
= SomeClass|NotFoundError;` are both fine; `type Id = SomeClass;` on its own is not.

### 7. Diagnostics

Each rejection names its replacement, in the style [ADR 0011](0011-functions-and-constants-are-class-members.md)
§ 4 already sets:

- `class_alias(...)` anywhere → *`class_alias` does not exist; a class has exactly one name*
- `use Path\To\Name as Other;` → *imports cannot be renamed; use `Name`, or the fully-qualified path*
- `TraitA::method as newName;` inside a trait `use` block → *trait methods cannot be renamed; give the class
  its own method named `newName` that calls `TraitA::method()`*
- `TraitA::method as protected;` (or `public`/`private`) inside a trait `use` block → *trait method
  visibility cannot be changed by `as`; override `method` in the class with the visibility you want*
- `type Id = SomeClass;` (a single bare class/interface/enum atom, nothing else) → *a `type` alias cannot
  name a single class on its own; refer to `SomeClass` directly, or alias a shape that includes it (e.g.
  `SomeClass|null`, `array<SomeClass>`)*
- `type A = B; type B = A;` (or any longer cycle) → *`type` alias cycle: `A` and `B` each resolve to the
  other and never bottom out in a concrete type*

## Consequences

**Positive**

- One more name kind joins the list [ADR 0008](0008-static-and-global.md) and
  [ADR 0011](0011-functions-and-constants-are-class-members.md) already closed: a class, interface, trait,
  enum, method or constant is reachable under exactly the name it declared, full stop — no runtime
  indirection layer sitting in front of any of them.
- `class_alias`'s specific footgun — code written against one name that a *different* request or a later
  call can silently repoint to a different class — cannot exist in MWL at all, structurally rather than by
  convention.
- The genuine verbosity cost [ADR 0007](0007-explicit-type-system.md) flagged in its own *Negative* section
  gets a real answer, on a mechanism narrow enough that it cannot be turned into the aliasing this ADR
  otherwise removes.
- `mwl check`'s symbol table keeps exactly one entry per declared class/interface/trait/enum/method/constant
  name, and a separate, structurally distinct table for `type` alias expansion — no code path anywhere needs
  to ask "is this name resolved directly, or through an alias someone declared."

**Negative**

- **A structural break from PHP**, joining the divergence list [ADR 0007](0007-explicit-type-system.md) § 7
  already carries forward, alongside [ADR 0010](0010-enums-are-a-value-type.md),
  [ADR 0011](0011-functions-and-constants-are-class-members.md) and
  [ADR 0012](0012-no-superglobals.md): PHP source calling `class_alias()`, importing with `as`, or composing
  traits with an `as` clause does not convert unconverted. `mwl convert` ([M11](../implementation-plan.md))
  can mechanically rewrite an import alias (replace every use of the local alias with the real short name or
  the FQN) and a trait rename (synthesize the delegating override method from *Decision § 3*), but
  `class_alias()` calls that compute the alias name dynamically, or that exist purely so two unrelated
  libraries can address the same class under different names, need a human decision about which name the
  converted code should actually use.
- Two unrelated libraries that happen to declare a class with the same short name can no longer paper over
  the collision with a local rename; the fully-qualified name is the only way to disambiguate at the call
  site. This is more typing, not a missing feature — see *Alternatives rejected*.
- A `type` alias to a union that includes a class is one more kind of atom the resolver must tell apart from
  a plain class reference when producing a diagnostic, so an error message naming a type must now say whether
  it is reporting the alias name or its expansion — a small, contained cost against priority 4's
  *implementation* simplicity, paid to keep priority 4's *language surface* simplicity (the actual verbosity
  relief) intact.

## Alternatives rejected

- **Keep `class_alias()` for a narrow, blessed use** (e.g. deprecation shims, gradual class renames during a
  migration). Rejected: a "blessed" runtime aliasing mechanism is still the exact indirection *Context*
  argues against, and a migration that wants to rename a class can do so as an ordinary `mwl convert` rewrite
  across call sites instead of leaving both names live at runtime.
- **Keep import `as` for genuine short-name collisions between two unrelated libraries.** The single real
  case this ADR's rejection costs something for. Rejected anyway, because the alternative (the
  fully-qualified name at the call site) is strictly more explicit for strictly more typing, and because a
  narrow "aliasing is fine, just only for collisions" carve-out is exactly the kind of exception that is hard
  to keep narrow once it exists.
- **Keep the visibility-only trait `as` form**, since it introduces no new *name* and so is arguably not
  "aliasing" at all. Considered directly, and rejected: keeping half the `as` clause still leaves trait
  composition with two disambiguation mechanisms (`insteadof` and `as`) instead of one, and the replacement —
  overriding the method and calling `TraitName::method()` inside it — is ordinary OOP a reader already has to
  know, not new surface.
- **Let a `type` alias name a single bare class, as a readability shorthand for a long FQN.** Rejected in
  *Decision § 6*: `use LongVendor\Namespace\ClassName;` (importing under the real short name) already solves
  the long-FQN problem without introducing a second name, which is the entire point of *Decision § 2*.
  Allowing it back in through `type` would be the same aliasing wearing different syntax.
- **A nominal ("newtype") form of `type` alias**, distinct from the aliased representation and requiring an
  explicit conversion between them. A different, legitimate feature — but a different decision, with its own
  cost/benefit around where the checked-conversion boundary should sit; not what was asked for here, and not
  smuggled in under this ADR's name.

## Revisiting

- **Parametric `type` aliases** (`type Rows<T> = array<array<T>>;`), once user-defined generics are designed
  — [ADR 0007](0007-explicit-type-system.md) *Revisiting* already defers the prerequisite.
  - **Whether `mwl convert`'s automatic rewrites for import `as` and trait-rename `as` (*Consequences,
  Negative*) are good enough**, or need a `--check`-only mode that just flags the site instead of rewriting
  it, is an M11 UX question this ADR does not resolve — following the precedent
  [ADR 0011](0011-functions-and-constants-are-class-members.md) *Revisiting* already set for its own
  converter rewrites.
- **A nominal/newtype form of `type` alias**, if real code turns out to want a distinct type over an existing
  representation rather than a transparent synonym — see *Alternatives rejected* for why this ADR does not
  decide that question.

Verification, in the order it becomes possible:

- **M1**: the parser rejects `use Path\To\Name as Other;` and every `as` clause inside a trait `use` block
  (both the rename and the visibility-only forms) with a diagnostic naming the replacement in *7*;
  `insteadof` alone still parses and is unaffected; `type Name = TypeExpr;` parses at file/namespace scope
  using the full grammar of [ADR 0007](0007-explicit-type-system.md) § 3.
- **M2**: name resolution has no alias table for classes/interfaces/traits/enums/methods/constants — a name
  resolves to exactly the declaration it names, or fails; a `type` alias resolves and is substituted away
  before the checker looks at anything downstream of it; a `type` alias whose expression is a single bare
  class/interface/enum atom is a diagnostic at the declaration site (*6*); an alias cycle is a diagnostic
  naming every name in the cycle.
- **M11**: `mwl convert`'s mechanical rewrites for import `as` and trait-rename `as` exercised on a corpus
  containing both; `class_alias()` calls flagged as a `TODO` for human review, per *Consequences, Negative*.
