Returns the SID as text, such as `S-1-5-32-544`.

Use it to show a SID to a person, to write it to a log, or to use it as a key in an array.
`Sid::parse` reads the text back as the same SID.

**Good to know:** the authority is written as a decimal number. Only an authority of `4294967296`
or more is written in hex, with `0x`, as Windows does.
