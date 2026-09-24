Checks whether a text is valid JSON and returns `true` or `false`. It replaces PHP's
`json_validate`.

`isValid` returns `true` exactly when `Core\Json::decode` would read the text with its default
options. It returns `false` for text that is not JSON, for a document nested deeper than 512
levels, and for a whole number too large for `int`. It never throws an error, so you do not need
a `try` block.

`isValid` does not return the value. When you need the value too, call `decode` and catch its
`ParseError`. That reads the text once, not twice.

**The examples below** check a few short texts, check the lines of a log file one by one, and
reject a request body that is not JSON before any other work.
