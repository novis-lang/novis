`tainted` and `secret` are two words you can write before `string` or `bytes`. `tainted string` is
text from outside your program, such as a form field. `secret string` is a confidential value, such
as a password or an API key.

A string built from such a value with `.` or interpolation is tainted or secret too. Text that a
`Core` method returns is tainted, unless the method's documentation says where the text comes from.
The compiler checks both words, so they do not make the program slower.

A tainted string cannot be a regular expression, a format template or a plain `string`. The error
names the method that makes the value safe there, such as `Core\IO::within` for a path. A secret
value does not compile with `echo`, in an error message, in a debug dump or in `Core\Json::encode`.

`secret` can also go before `mixed`. `Core\Secret::reveal($value, "reason")` is the only method that
gives a plain value from a secret one.

Examples 04 to 07 show each kind of mistake, and none of them compiles.
