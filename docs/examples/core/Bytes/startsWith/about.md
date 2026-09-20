Tells you whether a `bytes` value begins with some other bytes.

The result is `true` when the first bytes of the value are exactly the bytes you gave, and `false`
when they are not. The comparison reads the bytes one after the other. There is no option to ignore
the difference between upper case and lower case, because a `bytes` value carries no language and no
character set.

This is the method a program writes when it looks at the first few bytes of a file to decide what
kind of file it is. Those first bytes are called a magic number, and every common format has one.
Use `Core\Bytes::indexOf` instead when the bytes may be anywhere in the value.

**Good to know:** an empty value begins every value, so bytes with nothing in them always return
`true`.
