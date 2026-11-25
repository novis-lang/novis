Writes a value as JSON text and returns it as a `string`.

A list becomes a JSON array. An array with string keys or a shape becomes a JSON object.
An instance of a class with `#[Core\Json\Derive]` becomes an object with one key for each public property.

A time value or a `Core\Uuid` becomes a JSON string. A date or a time is its ISO text, such as
`"2026-05-14"`. A `Duration` is text such as `"1h30m"`, and a `Zone` is its name.
`Core\Json::decodeAs` reads each one back.

The text is on one line, and `pretty: true` puts each value on its own line. `escapeUnicode: true`
writes each character outside ASCII as a `\u` code. The characters `<`, `>`, `&` and `'` are always
`\u` codes, so the text is safe inside an HTML page.

`encode` throws a `LogicError` for a `float` that is NaN or infinite, or an object that contains
itself.

**The examples below** write a list, readable JSON, an API response from a class, and time values.

related: Core\Json::decode, Core\Json::isValid
