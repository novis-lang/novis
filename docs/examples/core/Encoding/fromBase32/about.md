Reads base32 text back into the bytes it was made from.

Base32 writes any bytes using only the letters `A` to `Z` and the digits `2` to `7`. Those symbols
survive being typed by hand, read out loud or put in a URL, which is why the shared secret in a
two-factor authentication link is written this way. `Core\Encoding::fromBase32` is the other half of
`Core\Encoding::toBase32`. It accepts upper case and lower case, and `=` padding or none at all,
because neither of those changes which bytes come out. Text that is not base32 throws a
`RuntimeError` saying what is wrong with it and where.

**Good to know:** base32 takes more room than base64 for the same bytes. Use base64 when only
programs read the text, and base32 when a person has to handle it.
