Reads base64 text back into the bytes it was made from.

Base64 writes any bytes as text, using the letters `A` to `Z` in both cases, the digits `0` to `9`,
and the two symbols `+` and `/`. Programs use it wherever only text may travel: an HTTP header, an
email attachment, a small image inside a stylesheet. `Core\Encoding::fromBase64` is the other half
of `Core\Encoding::toBase64`. It reads one exact form: the alphabet above, and `=` padding where
the last group is short. A space, a newline, or a text that stops part way through a group is an
error. The error says what is wrong and at which offset.

**Good to know:** text made for a URL or a JSON Web Token uses `-` and `_` in place of `+` and `/`.
`Core\Encoding::fromBase64Url` is the member that reads that form.
