At a parameter typed `property<T>`, `propertyPath<T>` or `string<K>` — alone, not in a union with
plain `string` — a string literal argument is converted while compiling. A right one is accepted with
no `as`, and a wrong one is the unknown-member diagnostic at the literal, on the segment or hole that
names nothing. A string built at run time still needs `as`, so no program acquires a key from a value
the compiler has not seen.

This changes one sentence of `rule:types/property-key`, "a string literal is a `string` and stays one",
for this one position. Everywhere else a literal stays a `string`: in a variable, in a return, in an
array, and at a parameter typed `…|string`, which keeps its meaning — any string is accepted, the names
are offered, and nothing is checked. A literal at a key parameter is decided exactly as `"email" as
property<User>` is decided where it is written, so the check is the one that conversion already makes
and is not a second implementation.

The position is the one where a name is wanted and the literal is the whole of what the call site
says. Making the caller write `"email" as property<User>` there repeats the class the signature already
names, and the editor can offer the names only where a literal will compile
(`rule:ide/a-field-path-completes-one-segment-at-a-time`).

- **Memory:** nothing at run time. The checker keeps the roster it already records per class a key
  names.

**Declined:**
- **Converting a literal wherever a key is expected, in a variable or a return too.** A literal in a
  variable has no single expected type until it is used, so the rule would depend on inference order.
  The argument position has its type written in the signature.
- **Checking a literal at a `property<T>|string` parameter as well.** That parameter says any string is
  fine, and a warning there would be noise at every call that passes free text.
