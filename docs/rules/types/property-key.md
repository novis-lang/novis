`property<T>` is a type atom, written in every position a type is written, whose **values** are the
names of `T`'s public declared properties — its own and its ancestors' — and nothing else. What is
carried at run time is that name, a string in the representation but **not** in the type.

`T` names one **class**. An interface, an enum, a scalar, a `class<...>` — anything that is not a
class — is `E0799` where it is written, and so is a class declaring no public property at all, since
no value of that type could exist. The interface row is the conservative one: a key's set has to be a
roster the receiver certainly has storage for, and an interface names a contract its implementors
satisfy with properties it does not itself declare. The set is the same roster reflection walks; the
visibility question has one shared implementation and this is not a second one.

**`as` is its only source.** A string literal is a `string` and stays one, so no program acquires a key
by accident. Three rows sit in the one conversion grid (`rule:types/conversion`): a `string` must name
a public declared property of `T`, or it throws; a `property<U>` narrows at run time; and
`property<T> → string` is total. A name that names nothing and a name that names a `private` property
are the same failure and throw the same way — visibility is decided at the conversion, once. A
written-out operand is decided **where it is written**: `"email" as property<User>` is a compile-time
yes, or the same unknown-member diagnostic an ordinary `$user->emial` gets. `as ?property<T>` yields
`null` where the checked form throws.

A qualifier is stripped for a reason narrower than the general row's: the conversion's whole output
range is the set of properties `User` declares `public` in this program's own source. A tainted string
cannot widen that set, name a field the author did not write down, or reach a `private` one.

A parameter typed **`property<T>|string`** takes a key and any string. A string literal written there
stays a `string`, and the body converts the value with `as ?property<T>` to find out whether it names one
of `T`'s properties. The editor offers `T`'s names inside such a literal
(`rule:ide/completion-offers-only-what-the-compiler-derived`), so a call site that may pass any string is
still shown the names. A parameter typed `property<T>` alone takes a key and nothing else.
`rule:types/a-literal-at-a-key-parameter-is-checked-where-it-is-written`, designed, changes this for
one position: a string literal at a parameter typed `property<T>` alone will be checked where it is
written and accepted with no `as`.

It is its own **equality domain**: two keys are equal when they name the same property, nothing else is
ever equal to one, and ordering one is refused (`rule:types/ordering`). `property` is a keyword only
in type position — `$property`, a method named `property` and a class named `Property` are untouched.

The type exists because the refusal of `$obj->$name` alone leaves three bad options for a request that
names a field — a sort column, a patch document's key, a form input. A hand-written `match` the
compiler cannot check covers the class; an `array<string, T>` beside the object gives up the declared
types; and reflection returns `mixed` and moves every check to run time. Each re-derives a fact the
compiler already has: the class's property roster is finite, declared and known
(`rule:classes/no-dynamic-properties`). The bound a key sets is narrower than the `match`, because the
compiler cannot tell whether one arm of the `match` was mistyped. A key is not a replacement for
`array<string, mixed>`: a program that wants arbitrary keys still wants an array, and a key is for
fields that are the class's own when the choice among them is made late.

The run-time conversion is one field lookup and one visibility test, the pair reflection already
performs, once where the string arrives. A written-out operand skips it. The value is a name and not a
slot index: an index would bind the key to one class's field layout, while a key is applied to
receivers of every subclass (`rule:types/property-key-variance`), and the lookup an index would save
is one the erased access performs anyway.

- **Memory:** a key is one string value, the property's name. Nothing is added to the class
  descriptor, and nothing is spent per request or per instance.

**Declined:**
- **Keep the refusal and offer nothing.** It pushes the ordinary program onto a `match` the compiler
  cannot check or an array that erases the declared types, both with less checking than the syntax it
  replaces.
- **Hooked and `readonly` properties left out of the set.** "The public properties" would become two
  rosters, this one and reflection's. Adding a hook to a property later would also turn a working `as`
  into a run-time throw in a file that did not change. The `readonly` half is refused at the write
  instead (`rule:types/property-key-access`).
- **A `Core` function that takes a property name and a class as two strings.** It checks only at run
  time and gives back a value the checker cannot type, so the read is `mixed` and the write is
  unchecked. A type is the only shape that reaches the access site.
- **A bare `string` accepted at `$obj->$name` when the class is written out.** The check moves from the
  source to every use, so an unchecked string flows through the program.
- **A key that may name a `private` property, with visibility checked at the access.** The conversion
  would succeed on a name the caller may not use, and the throw would land at a site that looks
  correct. Visibility belongs to the set, so it is checked where the set is entered.
