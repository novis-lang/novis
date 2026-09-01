# ADR 0126 — A property key is a checked name, and `as` is its only source

- **Status:** Accepted
- **Date:** 2026-09-01
- **Scope:** the `property<T>` type — its grammar, the set its values range over, its one source, which way
  it widens, the one site that accepts it, how a read and a write through it are typed, and what a
  qualifier does across the conversion. It does **not** reopen
  [ADR 0014](0014-property-observer.md) § 5's rule that every property is a declared name, which is what
  makes the set finite in the first place; it does not give `$obj->$m()` a spelling, which § 6 there
  rejects as a concept; and it adds nothing to `ClassDesc`, because the access it admits is the erased one
  [ADR 0036](0036-anonymous-object-shapes.md) § 4 already lowers.
- **Depends on:** [0014](0014-property-observer.md), [0007](0007-explicit-type-system.md),
  [0036](0036-anonymous-object-shapes.md)
- **Amends:** [0014](0014-property-observer.md) — § *Revisiting*'s deferred `property<T>` bullet is decided
  here, including the one open choice it named, and § 5's computed-name refusal `E0235` moves from the
  parser to the checker, where it stays for every operand that is not a property key.
  [0007](0007-explicit-type-system.md) — § 3's `atom` production gains `'property' '<' Name '>'`, and § 2's
  conversion grid gains the `string`/`property<U>` → `property<T>` row and the total `property<T>` →
  `string` row.
- **Validated by:** [crates/nvs-syntax/src/parser/tests/ty.rs](../../crates/nvs-syntax/src/parser/tests/ty.rs)

> **In short:** `property<T>` is a type whose values are the names of `T`'s **public declared properties**,
> and **`as` is its only source** — `$name as property<User>` throws for any other name, a written-out
> `"email" as property<User>` is decided where it is written, and a plain `string` never becomes one by
> accident. `$obj->$key` is then admitted where `E0235` refuses a computed name today, and that refusal
> moves from the parser to the checker for everything else. A read through a key is typed as the **union**
> of the set's declared types; a write is [ADR 0036](0036-anonymous-object-shapes.md) § 4's checked erased
> store — never a creation — and is refused outright where the set holds a `readonly` property, because a
> key may name any of them. The set is the same roster `Core\Reflect\ClassInfo::properties` walks, hooked
> properties and all, and nothing new is lowered: applying a key *is* the erased access.

## Context

A request arrives naming a field — a sort column, a patch document's key, a form's input name — and the
program has to turn that name into a property of a class it wrote. Novis refuses the PHP spelling
`$obj->$name` with `E0235`, in the parser, over the spelling itself: a name known only when the statement
runs defeats the resolution every property access is built on, and it is the one construct that lets a
request-controlled string pick which field to read or write.

That refusal is right, and it leaves the ordinary program with three bad options: a hand-written `match`
over string literals that the compiler cannot check covers the class; an `array<string, T>` beside the
object, which gives up the declared types the class already has; or `Core\Reflect\ClassInfo::get`, which
answers `mixed` and moves every check to run time. Each of them re-derives, badly, a fact the compiler
already holds — **the class's property roster is finite, declared and known**, which is exactly what
[ADR 0014](0014-property-observer.md) § 5 bought by making an undeclared property a hard error.

What is missing is not a permission but a **type**. [ADR 0007](0007-explicit-type-system.md) § 2 already
owns the operator that turns an unchecked value into a checked one, and
[ADR 0125](0125-a-class-reference-is-a-type-and-as-is-its-only-source.md) has already spent that operator
on the sibling question — *which class* — leaving a shape to follow: one atom, one `as` row, and the
refusal moved from the parser to the checker. A property key is that operator applied to a member name.

## Decision

### 1. `property<T>` is a type, and its value is a public property's name

`property<T>` is a type atom, written in every position a type is written. Its **values** are the names of
`T`'s public declared properties — its own and its ancestors' — and nothing else. The value carried at run
time is that name, a `string` in the representation but not in the type: a key is not a string, and § 2 is
why.

`T` names one **class**. An interface, an enum, a scalar, `class<...>` — anything that is not a class — is
**`E0799`** where it is written, and so is a class that declares no public property at all, since no value
of that type could ever exist. A `T` that resolves to nothing is `E0303`, as any other name would be,
which is why both refusals are the checker's and not the parser's. The interface row is the conservative
one: a key's set has to be a roster the receiver certainly has storage for, and an interface names a
contract its implementors satisfy with properties it does not itself declare. If Novis ever admits PHP
8.4's declared interface properties, that is the row this decision reopens, and only that one.

The set is **the same roster `Core\Reflect\ClassInfo::properties` walks** — one roster, not a second one
written here. `Core\Reflect` landing first is what makes that possible, and is the sequencing
[ADR 0014](0014-property-observer.md) § *Revisiting* asked for: the visibility question is
`ClassDesc::field_slot` plus `ClassDesc::field_is_public`, already the one shared implementation
[ADR 0019](0019-reflection-and-ast-parsing-are-core-features.md) § 2 demands.

It is its own **equality domain**
([ADR 0090](0090-one-equality-operator-and-disjoint-types-do-not-compile.md) § 2): two property keys are
equal when they name the same property, and nothing else is ever equal to one. `$key == "email"` is
exactly the string-as-a-member confusion this ADR exists to keep out; ordering one is refused with the
other unordered types.

The grammar is one line of [ADR 0007](0007-explicit-type-system.md) § 3's `atom` production, parsed only
in type position, where a `<` is unambiguously a type-argument list. `property` is a new keyword only
there — it is not one anywhere else, so `$property`, a method named `property` and a class named
`Property` are all untouched.

```php
property<User> $field = Core\Request::query('sort') as property<User>;
mixed $value = $user->$field;
```

### 2. `as` is its only source

**There is no other way to obtain a `property<T>`.** A string literal is a `string` and stays one, so no
program acquires a key by accident. [ADR 0007](0007-explicit-type-system.md) § 2's grid holds three rows:

| conversion | behaviour |
|---|---|
| `string` → `property<T>` | the string must name a public declared property of `T`, or it throws |
| `property<U>` → `property<T>` | a narrowing; the name must also be one of `T`'s, checked at run time |
| `property<T>` → `string` | total — a checked name is still a name, and this is the direction that cannot go wrong |

The `string` row is the door, and being a row of that grid it inherits everything the grid already says:
it is checked in fact, it throws rather than substituting, and `as ?property<T>`
([ADR 0066](0066-nullable-conversion-operator.md)) yields `null` exactly where it would throw. A name that
names nothing and a name that names a `private` property are the same failure and throw the same way —
visibility is decided at the conversion, once, and the message names the class and the name it was given.

**A written-out operand is decided where it is written.** `"email" as property<User>` is a compile-time
yes when `User` declares a public `$email` and a compile-time refusal (`E_UNKNOWN_MEMBER`, the diagnostic
an ordinary `$user->emial` already gets) when it does not — not a throw the program has to reach. That is
the shape a hand-written key takes, so it pays nothing at run time and misspells at build time.

**A qualifier is stripped, as every checked conversion strips one.**
`tainted string as property<User>` yields an unqualified `property<User>`, under § 2's own last row and
for a reason narrower than that row's: the conversion's whole output range is the set of properties
`User` declares `public` in this program's own source. A tainted string cannot widen that set, cannot
name a field the author did not write down, and cannot reach a `private` one. What survives is a choice
among the fields the class already exposes — which is the bound that keeps ADR 0014 § 5's priority-1
reason intact, and is why this is a type rather than a permission.

### 3. The argument bounds the receiver, so widening runs the other way

A subclass *adds* properties, so `property<Animal>`'s names are all valid on a `Dog` while
`property<Dog>`'s are not all valid on an `Animal`. **`property<Animal>` therefore widens to
`property<Dog>`, and never back** — the argument is contravariant, exactly opposite to
[ADR 0125](0125-a-class-reference-is-a-type-and-as-is-its-only-source.md) § 3's `class<T>`, and for the
reason that inverts it: a class reference is *produced* against its bound, while a key is *consumed* by a
receiver. Narrowing is written, like every other narrowing, as `as property<Animal>`, and is the run-time
check § 2's second row already describes.

The rule at the site follows from this and adds nothing: `$obj->$key` requires `$obj`'s type to be a `T`,
where `T` is the key's argument. A key made against `Animal` reads a `Dog`; a key made against `Dog` does
not read an `Animal`.

### 4. One site accepts it, and `E0235` moves to the checker

`$obj->$key` and `$obj->{$expr}` are admitted, and only when the operand's type is a `property<T>` whose
argument the receiver satisfies. Every other operand keeps **`E0235`** — with its help now naming
`as property<T>` rather than only naming the written-out form — and the refusal **moves from the parser to
the checker**, because the operand's *type* is now the question and the parser cannot see one. The
spelling is no longer what is rejected; the missing check is.

Three neighbours stay exactly as they are, and each for its own reason:

| spelling | verdict |
|---|---|
| `$obj->$m(...)` | `E0235` forever. [ADR 0014](0014-property-observer.md) § 6 rejects computed *dispatch* as a concept, not as a spelling, and a key is not a method name |
| `$obj->$key` on a `mixed` or shape-typed receiver | `E0235`. There is no `T` to check the key's bound against, so the one thing the key promises cannot be verified |
| `unset($obj->$key)` | `E0234`, as `unset` of any property already is |

### 5. A read is the union; a write is the checked erased store, and `readonly` is refused at the write

A **read** through a key is typed as the **union of the set's declared types** — `int|string|?Address` for
a `User` declaring those three — which widens into `mixed` or any covering union without an `as`, and
narrows the way every union narrows. That is the most a site can know, and it is strictly more than the
`mixed` today's three workarounds hand back.

A **write** is [ADR 0036](0036-anonymous-object-shapes.md) § 4's checked erased store: it writes an
existing property, **never creates one**, and the incoming value is checked at run time against what the
class declares that property to hold. Statically the value must satisfy at least one member of the union —
a value no property of `T` could accept is refused where it is written, since the store could only ever
throw. Both directions lower to the erased access `nvs_runtime` already performs for ADR 0036 § 4's
receiver, so per-property hooks and a declared `PropertyObserver` behave exactly as they do there. That
is one implementation and not a fourth: the erased store's known gap — it reaches storage past a
per-property `set` hook, recorded on `nvs_runtime::write_erased_property` and in
`crates/nvs-stdlib/src/reflect.rs`' *Known gaps* — is owned there and closes for all of its callers at
once, rather than being answered a second way here.

**In the IR that is one instruction per direction and not a chain**, which is the choice this paragraph
exists to record. A key is a name, so what the access needs is the by-name search on the receiver's
concrete descriptor that the erased access already does in one call: `nvs_ir`'s `InstKind::KeyGet` and
`InstKind::KeySet` are `SlotGet`/`SlotSet` with the name arriving as a value, over
`nvs_runtime::nvs_object_key_get` and `::nvs_object_key_set`. The alternative was a closed-set chain over
the roster the conversion already tests — one equality test and one ordinary resolved read per public
property, joined by a `Phi` — which needs no new instruction and no codegen arm, and spends a comparison
and a basic block *per property at every access* to reach the same slot. It would also have to tag each
arm's read into the union's representation before the join, so the statically-typed read it looks like it
buys is not one; and the hook behaviour it would incidentally fix is the erased path's gap above, which is
closing for every caller at once rather than for this one caller early.

**A write through a key is refused, at the write, where `T`'s public set holds a `readonly` property**,
naming it, as `E0782` — the code an ordinary write to that property after construction already gets,
because it is the same rule of [ADR 0038](0038-lateinit-property-modifier.md) § 1 being broken:

```console
E0782: `User::$id` is `readonly`, so only `User`'s constructor writes it
       a `property<User>` may name any of User's public properties, and this
       write cannot know which one it holds
       fix: write the property out to name a different one, or drop the
            modifier if the property is meant to be assignable
```

This is deliberately the *write* and not the conversion: reading `$id` through a key is fine, and a class
whose fields are all assignable is unaffected. It is also deliberately compile-time. The alternative is a
`readonly` flag on `ClassDesc` and a throw the program has to reach, which buys a later report of the same
mistake and spends a descriptor field on it — and where a request-controlled name selects a field to
write, refusing at build time is the direction priority 1 points in.

### 6. What it costs

The conversion is one `ClassDesc::field_slot` lookup plus one `field_is_public`, the pair
`Core\Reflect\ClassInfo::get` already performs, once where the string arrives; the written-out form skips
it entirely. Applying a key costs exactly what ADR 0036 § 4's erased access costs today and not one
instruction more, because it *is* that access — no new IR, no new calling convention, no new descriptor
field. A written-out `$user->email` is untouched and pays nothing.

The value is a name and not a slot index. An index would bind the key to one class's field layout while
§ 3 has it applied to receivers of every subclass, and what it would save is a hash lookup the erased path
performs anyway.

## Consequences

- **Security.** The fields a request can select are bounded by what the class declares `public` in this
  program's own source, checked at the one place the value is created, and a `readonly` field cannot be
  selected for writing at all. That is narrower than the hand-written `match` this replaces, because the
  `match` is written by hand and the compiler cannot tell whether an arm was mistyped.
- **Correctness.** A read through a key is typed as the union of real declared types rather than `mixed`,
  so the site downstream is checked. A write is checked twice — statically against the union, at run time
  against the property named.
- **Latency.** Nothing on the static path changes. The dynamic path pays one descriptor lookup per
  conversion and then the erased access's own cost.
- **Memory.** One word per key held, pointing at a name the class descriptor already owns. Nothing per
  request, nothing per instance, nothing added to `ClassDesc`.
- **Simplicity, for the language's user.** One new atom, one new `as` row, and a refusal that turns into a
  fix. The cost is that a key and a `string` are visibly different things and the conversion has to be
  written — which is exactly the distinction the safety rests on.
- **Simplicity, for the implementation.** A new `TypeAtom`, a new `Ty`, three conversion rows, and moving
  one existing refusal down a layer. The access itself already exists.
- **The negative.** `property<T>` is not `array<string, mixed>`: a program that genuinely wants arbitrary
  keys still wants an array, and reaching for a key there produces a class with a public property per
  field, which is worse. The type is for the case where the fields are the class's own and the *choice*
  among them is late.

## Alternatives rejected

- **Keep `E0235` and offer nothing.** The status quo, and it is safe. It also pushes the ordinary program
  onto a `match` the compiler cannot check or an `array<string, T>` that erases the declared types — both
  with *less* checking than the spelling being refused.
- **Exclude hooked and `readonly` properties from the set**, [ADR 0014](0014-property-observer.md)
  § *Revisiting*'s first option. It makes "the public properties" two rosters — this one and
  `Core\Reflect`'s — and it means adding a hook to a property later turns a working `as` into a run-time
  throw in a file that did not change. § 5 takes the `readonly` half of the problem at the write instead,
  where it is a build error rather than a surprise, and the hook half does not arise once the access is
  the erased one.
- **Carry a hook entry and a `readonly` flag per field on `ClassDesc`**, that section's second option. The
  hook entry buys nothing here, because the erased access already dispatches hooks and its one gap is
  owned where every caller shares it; the `readonly` flag buys a run-time throw where § 5 already has a
  compile-time refusal.
- **A `Core` member — `Core\Reflect::key(string $name, string $class)` — instead of a type.** It moves the
  check to run time and hands back something the checker cannot use, so the read is `mixed` and the write
  is unchecked. A type is the only shape that reaches the access site.
- **Accept a bare `string` at `$obj->$name` when the class is written out.** It puts the check at the use
  rather than at the source, so an unchecked string flows through the program and every use re-checks it —
  the opposite of what `as` is for.
- **`property<T>` covariant, like `class<T>`.** It reads as the obvious rule and it is unsound: a
  `property<Dog>` passed where `property<Animal>` is wanted lets `$breed` reach an `Animal` receiver, which
  has no such storage.
- **Let the key name `private` properties, with visibility checked at the access.** It would make the
  conversion succeed on a name the caller may not use, and the throw would land at a site that looks
  correct. Visibility is a property of the *set*, so it belongs where the set is entered.

## Verification

- The parser: `property<User>` is one type atom with one class argument, in every declaration slot; the
  argument closes through the same `>`-splitting close `array<T>` uses, so `array<property<User>>` parses;
  `$property`, `->property()` and a class named `Property` are unaffected.
- The checker: the set is `T`'s public declared properties and nothing else — not `private`, not a method,
  not an undeclared name; a read types as the union of that set; `property<Animal>` passes where
  `property<Dog>` is wanted and not the reverse; a `string` reaches `property<T>` only through `as`;
  `"nope" as property<User>` is a compile-time refusal; `tainted string as property<T>` yields an
  unqualified key; a computed member name with no key operand is still `E0235`, now from the checker; a
  write through a key over a class with a `readonly` property is `E0782` naming it.
- The run time: a name outside the set throws at the conversion and not at the access; a key reads and
  writes the field it names through the erased access, running that path's hook and observer steps; a key
  made against a base class reads an instance of a subclass.
