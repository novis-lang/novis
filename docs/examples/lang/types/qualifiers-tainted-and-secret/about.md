`tainted` and `secret` are two words you can write before `string` or `bytes`. `tainted string` is
text from outside your program, such as a form field. `secret string` is a confidential value, such
as a password or an API key.

A string built from such a value with `.` or interpolation is tainted or secret too. The compiler
checks both words, so they do not make the program slower.

A tainted string cannot be a regular expression, a format template or a date pattern. It cannot be
assigned to a plain `string` either. A secret value does not compile with `echo`, in an error
message, in a debug dump or in `Core\Json::encode`.

`secret` can also go before `mixed`. A plain `mixed` variable cannot contain a secret value. A
secret value cannot be an array key, and `as int` does not compile with it.
`Core\Secret::reveal($value, "reason")` is the only method that gives a plain value from a secret
one.

Examples 04 to 07 show each kind of mistake, and none of them compiles.
