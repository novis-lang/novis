Reads URL-safe base64 text back into the bytes it was made from.

URL-safe base64 writes any bytes as text, using the letters `A` to `Z` in both cases, the digits
`0` to `9`, and the two symbols `-` and `_`. All of those are safe in a web address, in a file name
and in a JSON Web Token, which is where you meet this form. `Core\Encoding::fromBase64Url` is the
other half of `Core\Encoding::toBase64Url`. It reads one exact form: the alphabet above, and no `=`
padding at all. A `+`, a `/`, an `=`, or a text that stops part way through a group is an error.
The error says what is wrong and at which offset.

**Good to know:** one member reads one form. Text written with `+`, `/` and `=` is
`Core\Encoding::fromBase64`'s to read.
