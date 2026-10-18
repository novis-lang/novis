A `string` is always valid UTF-8. Its length counts graphemes (what a person counts as one
character). `"e\u{301}"` is an `e` with a combining accent: it has one grapheme and three bytes.

`$s[0]` does not compile for a `string` or for `bytes`: it is the error `E0482`. Use
`Core\Str::slice($s, $i, 1)` for a character, or `Core\Bytes::slice` for a byte position.

You cannot write a `bytes` value directly in the code. `"nvs" as bytes` gives the UTF-8 bytes of a
string. `Core\Encoding::fromHex` and `Core\Encoding::fromBase64` give any bytes you need.

`bytes` has no string form of its own. `echo $b` is the error `E0707`, so write `echo $b as string`.
If the bytes are not valid UTF-8, `$b as string` throws a `RuntimeError`. `$b as ?string` gives
`null` instead.

`==` tests whether two strings are equal. `"a" < "b"` is the error `E0715`, so use
`Core\Str::compare` to put strings in order. Text has no arithmetic: `"3" * 2` is the error
`E0716`. Join two strings with `.`, or convert a string to a number with `as int` or `as float`.
