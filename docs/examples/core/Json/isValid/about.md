Checks whether a text is valid JSON and returns `true` or `false`.

`isValid` returns `true` when the text is a valid JSON document at the default depth of 512. It
returns `false` for text that is not JSON and for a document nested deeper than 512 levels. It also
returns `false` for a whole number too large or too small for `int`. It never throws an error, so you do not need a
`try` block.

`isValid` reads the text and builds no values. It is faster than `decode` and uses less memory.
Each request and each script has a memory limit. A valid document can be too large for `decode`
to read within that limit. `isValid` still returns `true` for it.

`isValid` does not return the value. When you need the value too, call only `Core\Json::decode`.
It throws a `ParseError` for text that is not valid JSON. This way the text is read once.

**The examples below** check a few short texts and check the lines of a log file one by one. The
last one rejects a request body that is not JSON before the program does anything else with it.
