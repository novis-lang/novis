`K` is `property<T>`, `propertyPath<T>` or a set of strings, such as `'asc'|'desc'`. Any other `K`,
such as `string<int>`, does not compile (`E0860`).

A hole is everything between `{` and `}`, spaces included. `{ total }` is not the hole `{total}`.
An empty hole `{}`, a `{` with no `}` and a `}` with no `{` do not compile (`E0861`).

A string written directly in the code is checked while compiling, one hole at a time. This is true
for a string literal at a parameter typed `string<K>`, and for `"{total}" as string<property<Order>>`.
A hole that is not a property of the class is the error `E0405`. A hole that is not in a set of
strings is the error `E0861`. The error points at the wrong hole.

Only a parameter takes a string literal with no `as`. A local variable typed `string<K>` needs `as`
for a literal too, or the program does not compile (`E0401`).

`as` checks any other string while the program runs. If a hole is wrong, it throws a
`RuntimeError`, and the message names the hole and what is wrong with it. `as ?string<K>` gives
`null` instead.

`as string<K>` keeps `tainted`. A `tainted string` converts to a `tainted string<K>`. A parameter
typed `string<K>` does not take it (`E0401`).

`$text as string` gives the text as written, so `{{` stays `{{`. Two values are equal when their
text is equal. A string written in the code and a string converted from a variable are equal when
the text is the same.
