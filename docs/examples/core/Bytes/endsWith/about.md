Tells you whether a `bytes` value ends with some other bytes.

The result is `true` when the last bytes of the value are exactly the bytes you gave, and `false`
when they are not. The comparison reads the bytes one after the other. There is no option to ignore
the difference between upper case and lower case, because a `bytes` value carries no language and no
character set.

This is the method a program writes when it reads from a file or a network connection and needs to
know whether what has arrived so far ends on a complete record. Use `Core\Bytes::startsWith` for the
other end of the value, and `Core\Bytes::indexOf` when the bytes may be anywhere in it.

**Good to know:** an empty value ends every value, so bytes with nothing in them always return
`true`.
