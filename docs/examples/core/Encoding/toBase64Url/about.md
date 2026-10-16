Writes bytes as base64 text that is safe in a web address.

This form uses the letters `A` to `Z` in both cases, the digits `0` to `9`, and the two symbols `-`
and `_`. It writes no `=` padding at all. Every one of those characters is safe in a URL, in a
query string and in a file name. A JSON Web Token is written this way too, and that is where most
people meet the form. Every byte has a spelling, so this member never fails, and the empty buffer
gives the empty string.

The text is still a third longer than the buffer, and it still grows three bytes at a time. Without
the padding its length can be any number except one more than a multiple of four.

**Good to know:** `Core\Encoding::fromBase64Url` reads this text back. It reads this form only, so
text written by `Core\Encoding::toBase64` has to go back through `Core\Encoding::fromBase64`.
