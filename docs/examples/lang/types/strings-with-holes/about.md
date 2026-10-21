A string with holes is a string that contains names between `{` and `}`, such as
`"{total} > 100 AND {customer.email} LIKE ?"`.

The type is written `string<K>`. It is one string, not a collection like `array<K>`. `K` says what a
hole may contain: `property<T>`, `propertyPath<T>`, or a set of strings such as `'name'|'count'`. A
string written in a call to a `string<K>` parameter is checked when the program compiles. `as` checks
any other string while the program runs, and throws an error if a hole is wrong.

**Good to know:** `{{` is one `{` and `}}` is one `}`. Only the holes are checked. A tainted string
(text from outside the program) stays tainted after `as string<K>`, so a condition from a request
cannot be the text of a database query. `Core\Str::parts` splits the string into its text and holes.

**The examples below** check the property paths in a query condition, fill a message template, check a
condition built while the program runs, and show a wrong hole that does not compile.
