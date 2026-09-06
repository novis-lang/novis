# `rule:types/class-reference` — A class reference is a type, and `as` is its only source

- **Status:** Accepted
- **Date:** 2026-08-31
- **Scope:** the `class<T>` type — its grammar, its one source, its widening, the three sites that accept
  it, the constructor rule a dynamic `new` is checked against, and what a qualifier does across the
  conversion. It does **not** reopen `rule:types/conversion`'s rejected list, which
  keeps `eval`, `$$var` and `settype()` refused; it does not touch `E0235`, the dynamic *member* name,
  whose reasoning lives on `E_DYNAMIC_MEMBER_NAME` in
  [`crates/nvs-diagnostics/src/lib.rs`](../../crates/nvs-diagnostics/src/lib.rs); and it says nothing
  about which *members* a class has, which is `rule:classes/no-dynamic-properties`'s exhaustive
  declared list.
- **Depends on:** [0007](0007-explicit-type-system.md)
- **Amends:** [0007](0007-explicit-type-system.md) — § 3's `atom` production gains `'class' '<' Name '>'`,
  § 2's conversion grid gains the `string`/`class<U>` → `class<T>` row, and § 7 row 14's tail no longer
  says the dynamic `instanceof` has no Novis spelling.
- **Amended by:** 0144
- **Validated by:** [crates/nvs-syntax/src/parser/tests/ty.rs](../../crates/nvs-syntax/src/parser/tests/ty.rs)

> **In short:** `class<T>` is a type, and its value is the run-time class descriptor an object is
> allocated from. **`as` is its only source** — `$name as class<Animal>` throws unless the string names
> `Animal` or a class that is one, `Dog::class as class<Animal>` is decided at compile time, and
> `Foo::class` on its own stays a `string`. `class<Dog>` widens to `class<Animal>` wherever `Dog` widens
> to `Animal`, and never back. The three sites that refuse a bare string today — `new $cls(...)`,
> `$cls::f()` and `$x instanceof $cls` — accept this value and nothing else, so `E0496` stands with its
> help now naming `as class<T>`. A `new` over `class<T>` types its arguments against `T`'s constructor
> and is refused, at the `new` site, when an implementor of `T` declares an incompatible one. Nothing
> reaches code from an unchecked string: the check is the conversion, and it is the only door.

## Context

A PHP program that picks a class at run time writes `new $cls(...)`, `$cls::make()` or
`$x instanceof $cls` over a `string`. Novis refuses all three with `E0496`, because a request-controlled
string selecting which class runs is the shape half of PHP's remote-code-execution history is written in,
and because the checker has nothing to type the call against: a bare `string` names no constructor and no
member.

That refusal is right and it is also, on its own, a dead end. The programs that need it are ordinary — a
driver chosen by configuration, a handler chosen by a discriminator field, a factory over a closed set of
subclasses — and every one of them today falls back to a hand-written `match` over string literals that
the compiler cannot check is exhaustive, or to an array of closures that erases the constructor
entirely. The language refuses the unsafe spelling and offers no safe one.

What is missing is not a permission but a **type**. `Foo::class` already exists and is a `string`; the
descriptor it names already exists at run time, because `new static(...)` allocates from exactly that
value ([`InstKind::NewDynamic`](../../crates/nvs-ir/src/ir.rs)). The two have never been connected by
anything the checker can read. `rule:types/conversion` already owns the one operator that turns an unchecked value
into a checked one, and it already throws rather than substituting. A class reference is that operator
applied to a class name.

## Decision

### 1. `class<T>` is a type, and its value is the class descriptor

`class<T>` is a type atom, written in every position a type is written. `T` names one class or one
interface; anything else — `class<int>`, an enum name — is **`E0795`** where it is written, and a `T`
that resolves to nothing at all is `E0303` as any other name would be, which is why the refusal is the
checker's and not the parser's. Its **value** is the run-time class descriptor — the same word
`new static(...)` already allocates from — carrying the class's name, its parent chain, its interface
set and its method table.

It is its own **equality domain** (`rule:expressions/disjoint-comparison-refused`):
two class references compare by descriptor identity, and nothing else is ever equal to one. An instance
is not its own class, and `$cls == "Dog"` is exactly the string-as-a-class confusion § 2 exists to keep
out. Ordering one is refused with the other unordered types.

The grammar is one line, `rule:types/grammar`'s `atom` production reads it, and
it is parsed only in type position, where a `<` is unambiguously a type-argument list and not a
comparison. `class` is already a keyword, so `class<` is two tokens of lookahead and there is no new
ambiguity: a class *declaration* is `class Name`, never `class <`.

```php
class<Animal> $cls = Core\Config::get('driver') as class<Animal>;
Animal $a = new $cls("Rex");
```

`class<T>` is a value type in the sense that matters here — a descriptor is immortal and process-wide, so
holding one costs a word and frees nothing.

### 2. `as` is its only source

**There is no other way to obtain a `class<T>`.** `Foo::class` is a `string` and stays one, so nothing
about the existing spelling changes and no program acquires a class reference by accident. Two
conversions produce one, and `rule:types/conversion`'s grid holds both:

| conversion | behaviour |
|---|---|
| `string` → `class<T>` | the string must name `T` or a class that is a `T`, or it throws |
| `class<U>` → `class<T>` | a narrowing; `U` must be a `T`, checked at run time against the descriptor |
| `class<T>` → `string` | total — the descriptor's own fully qualified name ([0144](0144-class-answers-the-class-a-value-is-so-static-class-and-obj.md) § 3) |

The `string` row is the door, and being a row of § 2's grid it inherits everything that grid already
says: it is checked in fact, it throws rather than substituting, and `as ?class<T>`
(`rule:expressions/nullable-conversion`) yields `null` exactly where it would throw. A name
that resolves to nothing, and a name that resolves to a class outside `T`'s hierarchy, are the same
failure and throw the same way; the class named is in the message.

**A `Foo::class` operand is decided at compile time.** `Dog::class as class<Animal>` is a compile-time
yes when `Dog` is an `Animal` and a compile-time *refusal* when it is not — not a throw the program has to
reach, since both sides are written out. This is the ordinary shape a factory takes, so the common case
pays nothing at run time.

**A qualifier is stripped, as every checked conversion strips one.** `tainted string as class<Animal>`
yields an unqualified `class<Animal>`, under § 2's own last row and for a reason narrower than that
row's: the conversion's *whole output range* is the set of classes declared to be `Animal`s in this
program's own source. A tainted string cannot widen that set, cannot name a class the source does not
declare, and cannot reach a class outside the hierarchy the author wrote down. What survives the
conversion is a choice among the author's own subclasses — which is what a discriminator field is for,
and is why this feature exists rather than a second `Core` member that takes a string.

### 3. Widening follows the argument, and only upward

`class<Dog>` widens to `class<Animal>` wherever `Dog` widens to `Animal` — a parameter, a return, an
assignment to a wider binding — and **never back**. A narrowing is written, like every other narrowing,
as `as class<Dog>`, and is checked against the descriptor at run time.

This is covariance, and it is sound here for the reason it is unsound for a mutable container: a
descriptor has no write side. There is nothing to put into a `class<T>`, so the argument's position is
purely an output and the usual variance trap has nothing to catch.

### 4. Three sites accept the value, and `E0496` still refuses everything else

The three spellings [`reject_dynamic_class_name`](../../crates/nvs-types/src/expr/members.rs) refuses
today accept a `class<T>` operand and nothing else:

| site | resolves against | lowers to |
|---|---|---|
| `new $cls(...)` | `T`'s constructor | `InstKind::NewDynamic`, the instruction `new static` already uses |
| `$cls::f(...)` | `T`'s static or instance member roster | `InstKind::CallVirtual` |
| `$x instanceof $cls` | nothing; the descriptor is the test | `InstanceOf`, in a descriptor-valued form |

Every other operand type keeps `E0496`, with its help now naming `as class<T>` rather than only naming
the written-out form. A bare `string` is therefore still refused at all three sites, one refusal, with a
fix the author can take.

`$obj->$name` is untouched *by this ADR*: a class reference answers "which class", which the checker can
use, and never "which member", which it cannot — so no `class<T>` operand ever admits one. The member-name
door is a different type's to open, and
`rule:types/property-key-access` opens it for
`property<T>` alone.

### 5. A `new` over `class<T>` is checked against `T`'s constructor, and a divergent implementor is refused

`new $cls(...)` types its arguments against **`T`'s** constructor, exactly as `new static(...)` types
them against the current class's. That is the only signature the site can see, and the value may be any
implementor of `T`.

So the site is refused, **at the `new`**, when any implementor of `T` declares a constructor incompatible
with `T`'s — the same compatibility test the override check already makes. The error names that subclass:

```console
E0794: `new` over `class<Animal>` cannot check its arguments
       Dog::constructor is not compatible with Animal::constructor
       a class reference may hold any implementor, so every one of them must accept
       the arguments written here
       fix: give Dog a constructor compatible with Animal's, or narrow this to
            `as class<Dog>` and instantiate that
```

This is stricter than PHP, which discovers the mismatch when the wrong subclass arrives, and it is never
*different* from PHP: every program this accepts, PHP runs the same way. It is deliberately checked at
the `new` and not at the class declaration — a subclass that is never instantiated through a class
reference is nobody's problem, and refusing it at the declaration would make an unrelated file's `new`
the reason a class cannot be written.

### 6. What it costs

`new $cls(...)` pays the name-keyed constructor lookup that `new static(...)` already pays, and nothing
more: the descriptor is in hand, so there is no resolution step a static `new` avoids. `new Dog()`, the
overwhelmingly common spelling, is untouched and pays **nothing** — no new branch, no new field, no
widened value.

The conversion itself is a hierarchy walk, `ClassTable::id_of` then `ClassDesc::conforms_to`, on a chain
whose depth is the program's own inheritance depth. It happens once per conversion, at the boundary
where the string arrives, and the compile-time-folded `Foo::class` form skips it entirely.

## Consequences

- **Security.** The class a request can select is bounded by the declared subtypes of `T` in this
  program's source, and the bound is checked at the one place the value is created. This is strictly
  narrower than what a `match` over string literals gives, because the `match` is written by hand and the
  compiler cannot tell whether one arm was mistyped.
- **Correctness.** The dynamic `new` and the dynamic static call become *typed* call sites: arguments,
  arity and return type are checked. Today's alternative — a closure table — erases all three.
- **Latency.** Nothing on the static path changes. The dynamic path costs one hierarchy walk per
  conversion and the constructor lookup a `new static` already pays.
- **Memory.** One word per class reference held, pointing at a descriptor that already exists for the
  lifetime of the process. Nothing per request, nothing per instance.
- **Simplicity, for the language's user.** One new atom, one new `as` row, and three refusals that turn
  into a fix. The cost is that `class<T>` and `Foo::class` are visibly different things, and a reader has
  to know the second is a `string` — which is exactly the distinction the safety rests on, so it is not
  a wart to file down later.
- **Simplicity, for the implementation.** A new `TypeAtom`, a new `Ty`, two conversion rows, one runtime
  helper and a descriptor-valued `InstanceOf`. `NewDynamic` and the descriptor constant already exist.

## Alternatives rejected

- **Keep the refusal and offer nothing.** The status quo. It is safe and it makes an ordinary program
  unwritable, which pushes authors to the closure table — a construct with *less* checking than the
  spelling being refused.
- **A `Core` member — `Core\Class::of(string $name, string $base)` — instead of a type.** It moves the
  check to run time and gives back an `object`-shaped nothing the checker cannot use, so the `new` site
  is still untyped. A type is the only shape that reaches the call site.
- **Accept a bare `string` at the three sites when the declared base is written out.**
  `new $cls(...) as Animal` and friends. It puts the check at the *use* rather than at the source, so a
  string flows through the program unchecked and every use re-checks it — the opposite of what § 2's `as`
  is for, and one forgotten annotation from PHP's behaviour.
- **`class<T>` invariant, with no widening.** Simpler to state, and it makes the ordinary
  `class<Dog>` → `class<Animal>` parameter pass impossible without a redundant run-time-checked
  narrowing. Covariance is sound here (§ 3) and it costs one rule.
- **Check the constructor rule at the class declaration instead of at the `new`.** It would make a
  subclass illegal because of an unrelated file, and would refuse programs that never instantiate through
  a class reference at all.

## Verification

- The parser: `class<Animal>` is one type atom with one class argument, in every declaration slot; the
  argument closes through the same `>`-splitting close `array<T>` uses, so `array<class<Animal>>` parses;
  `class Foo {}` is still a class declaration.
- The checker: `class<Dog>` passes where `class<Animal>` is wanted and not the reverse; a `string`
  reaches `class<T>` only through `as`; `Dog::class as class<Cat>` is a compile-time refusal;
  `tainted string as class<T>` yields an unqualified reference; a `new` over `class<T>` whose hierarchy
  holds a divergent constructor is `E0794` naming that subclass.
- The run time: a name outside the hierarchy throws at the conversion and not at the use; a class
  reference instantiates the subclass it names, carries a static call and answers `instanceof`; a
  `Foo::class` operand lowers to a descriptor constant with no walk.
