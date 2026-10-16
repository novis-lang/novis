Reads a JSON text and returns the value it contains.

A JSON object becomes an array with string keys, and a JSON array becomes a list. Numbers, strings,
`true`, `false` and `null` stay what they are. The result has the type `mixed`, so you convert each
part with `as` before you use it, for example `$person["name"] as string`.

If the text is not valid JSON, `decode` throws a `ParseError`. It also throws one when the document
is nested deeper than the `maxDepth` option allows, which is 512 levels by default, or when a whole
number is too large for `int`. It never returns `null` to report an error.

**Good to know:** when you know the structure of the document in advance, `Core\Json::decodeAs`
reads it straight into a class or a shape.

**The examples below** read the fields of one object, show three texts that throw a `ParseError`,
and add up the total of a shopping cart.
