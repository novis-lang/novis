Reads bytes as text, in the character encoding you name.

A file, a network message or a database column arrives as bytes. `Core\Encoding::decodeText` turns
those bytes into a `string`, using the encoding you say they are written in: `Core\Charset::Utf8`,
`Core\Charset::Windows1252`, `Core\Charset::ShiftJis`, and every other encoding the web standard
names. The conversion is exact, so the result is always the text that was sent. If the bytes are not
valid in that encoding, the method throws a `RuntimeError` that names the position of the first byte
it could not read.

**Good to know:** `Core\Encoding::isValidText` tests the same thing and returns `true` or `false`.
Use it when you want to choose what to do instead of catching an error.
