Writes a value as JSON text and returns it as a `string`.

A list becomes a JSON array, and an array with string keys or a shape becomes a JSON object.
Numbers, strings, `true`, `false` and `null` stay what they are. An instance of a class with the
`#[Core\Json\Derive]` attribute becomes an object with one key for each public property.

A `Core\Time\Date`, `TimeOfDay`, `DateTime`, `Instant`, `Duration`, `Zone` or `Core\Uuid` becomes a
JSON string. A date or a time is its ISO text, such as `"2026-05-14"`. A `Duration` is text such as
`"1h30m"`, and a `Zone` is its name, such as `"Europe/Vienna"`. `Core\Json::decodeAs` reads each
text back.

The text is on one line. The option `pretty: true` puts each value on its own line. The option
`escapeUnicode: true` writes each character outside ASCII as a `\u` code, for a reader that only
accepts ASCII. The characters `<`, `>`, `&`, `'`, U+2028 and U+2029 are always written as `\u`
codes, so the text is safe inside an HTML page.

`encode` throws a `LogicError` for a value JSON cannot write: a `float` that is NaN or infinite, or
an object that contains itself.

**The examples below** write a list and an array, write readable JSON, and build the answer of an
API request from a class. The last one writes a date, a duration and a time zone.

related: Core\Json::decode, Core\Json::isValid
