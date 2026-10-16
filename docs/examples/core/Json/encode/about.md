Writes a value as JSON text and returns it as a `string`.

A list becomes a JSON array, and an array with string keys or a shape becomes a JSON object.
Numbers, strings, `true`, `false` and `null` stay what they are. An instance of a class with the
`#[Core\Json\Derive]` attribute becomes an object with one key for each public field.

The text is on one line. The option `pretty: true` puts each value on its own line. The option
`escapeUnicode: true` writes each character outside ASCII as a `\u` code, for a reader that only
accepts ASCII.

`encode` throws a `LogicError` for a value JSON cannot write: a `float` that is NaN or infinite, or
an object that contains itself.

**The examples below** write a list and an array, write readable JSON, and build the answer of an
API request from a class.

related: Core\Json::decode, Core\Json::isValid
