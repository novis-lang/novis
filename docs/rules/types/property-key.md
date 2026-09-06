`property<T>` is a type atom, written in every position a type is written, whose **values** are the
names of `T`'s public declared properties — its own and its ancestors' — and nothing else. What is
carried at run time is that name, a string in the representation but **not** in the type.

`T` names one **class**. An interface, an enum, a scalar, a `class<...>` — anything that is not a
class — is `E0799` where it is written, and so is a class declaring no public property at all, since
no value of that type could exist. The interface row is the conservative one: a key's set has to be a
roster the receiver certainly has storage for. The set is the same roster reflection walks; the
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

It is its own **equality domain**: two keys are equal when they name the same property, nothing else is
ever equal to one, and ordering one is refused (`rule:types/ordering`). `property` is a keyword only
in type position — `$property`, a method named `property` and a class named `Property` are untouched.
