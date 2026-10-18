`tainted` and `secret` are two words you can write before `string` or `bytes`. `tainted string` is
text from outside your program, such as a form field or a command-line argument. `secret string` is
a confidential value, such as a password or an API key.

A value is tainted or secret only where a declaration says so. A string built from it with `.` or
interpolation is tainted or secret too. The compiler checks both words before the program runs, so
they do not make the program slower.

A tainted string cannot be a regular expression, a format template or a date pattern. It cannot be
assigned to a plain `string` either. A secret value does not compile with `echo`, in an error
message, in a debug dump or in `Core\Json::encode`.

**Good to know:** a conversion such as `as int` gives a plain value, because the result is a number.
`Core\Secret::reveal` is the only method that gives a plain string from a secret one. You give it the
reason as its second argument.

Examples 04 to 07 show each kind of mistake, and none of them compiles.
