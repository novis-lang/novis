Reads hexadecimal text back into the bytes it was made from.

Hexadecimal writes one byte as two digits, from `00` to `ff`. It is the form you see wherever a
person has to read bytes: a checksum beside a download, a colour in a stylesheet, a key in a
configuration file. `Core\Encoding::fromHex` is the other half of `Core\Encoding::toHex`. It reads
the digits `0` to `9` and the letters `a` to `f` in either case, and nothing else. No `0x` in
front, and no space, colon or newline between the pairs. A text with an odd number of digits is an
error, because one byte takes two digits. The error says which of the two things is wrong.

**Good to know:** `Core\Encoding::toHex` always writes lower case, and both cases read back to the
same bytes. Compare the bytes rather than the text, and the case stops mattering.
