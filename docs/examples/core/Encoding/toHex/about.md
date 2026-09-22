Writes bytes as hexadecimal text, so that a person can read them.

Hexadecimal writes one byte as two digits, from `00` to `ff`. Every byte has a spelling, so this
member never fails. The text is always twice as long as the buffer, and the empty buffer gives the
empty string. `Core\Encoding::toHex` writes the digits `0` to `9` and the letters `a` to `f` in
lower case. It writes nothing else: no `0x` in front, and no space or colon between the pairs. Add
those yourself when the form you are writing for wants them.

This is the form you meet wherever bytes have to be read by a person: a checksum beside a download,
a colour in a stylesheet, a key in a configuration file, a few bytes in a log line.

**Good to know:** `Core\Encoding::fromHex` reads the text back into bytes, and it accepts upper case
as well as lower case.
