Text is a `string`. Data that may not be text is `bytes`. They are two separate types, and you
convert between them with `as`.

A `string` is always valid text. Its length is the number of characters a person counts. An accented
letter, an emoji or a flag is one character, even when it needs several bytes. There is no
`$name[0]` syntax. `Core\Str::at` returns one character, and `Core\Str::slice` returns several.

`bytes` is for the content of a file, a reply from a network connection or a hash. Its length is the
number of bytes. `Core\Bytes::slice` takes a part of it by byte position.

**Good to know:** `$text as bytes` always works. `$data as string` checks the data first. If the
data is not valid text, it throws a `RuntimeError`.

**The examples below** show a name measured in characters and in bytes, the first character and a
short preview of a title, and uploads sorted into pictures and notes. The last one converts text to
bytes and back, and shows the error for data that is not text.
