Writes text as bytes, in the character encoding you name.

A file, a network message or an older system expects its text in one particular encoding.
`Core\Encoding::encodeText` turns a `string` into the bytes of that encoding: `Core\Charset::Utf8`,
`Core\Charset::Windows1252`, `Core\Charset::ShiftJis`, and every other encoding the web standard
names. The conversion is exact, so the bytes always spell the text you gave. Most encodings have no
spelling for most characters. If your text holds one of them, the method throws a `RuntimeError` that
names the character and where it sits, and you can then pick a different encoding or a different
character.

**Good to know:** `Core\Encoding::decodeText` is the other direction. UTF-8 and UTF-16 can write
every character there is; every other encoding covers only part of them.
