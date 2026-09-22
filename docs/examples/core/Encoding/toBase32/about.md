Writes bytes as base32 text, which a person can type, read out loud or put in a URL.

Base32 writes any bytes using only the upper-case letters `A` to `Z` and the digits `2` to `7`.
Every five bytes become eight characters. `Core\Encoding::toBase32` writes no `=` padding at the
end, so a short last group simply gives fewer characters. Every byte has a spelling, so this method
never fails, and the empty buffer gives the empty string.

The shared secret in a two-factor authentication link is written this way.
`Core\Encoding::fromBase32` reads the text back into the same bytes.

**Good to know:** some older tools expect `=` padding, so that the length divides by eight.
`Core\Str::padEnd` adds it.
