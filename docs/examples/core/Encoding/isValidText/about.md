Tells you whether bytes are valid text, in the character encoding you name.

Bytes that arrive from a file, an upload or a network message are not always text.
`Core\Encoding::isValidText` reads them in the encoding you name. It returns `true` when every byte
sequence is one that encoding allows, and `false` when one of them is not. It never throws an error,
so you can put it in an `if` and decide yourself what a bad input should do.

This is the same test `Core\Encoding::decodeText` makes before it converts. Use `isValidText` when
your program has something to do with a bad input, such as skipping a row or trying another
encoding. Use `decodeText` inside a `try` when a bad input is an error.

**Good to know:** the answer is always `true` for `Core\Charset::Latin1`. That encoding gives all
256 bytes a meaning, so no bytes are invalid in it.
