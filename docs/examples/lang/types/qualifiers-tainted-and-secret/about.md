`tainted` and `secret` are two words you can write before `string` or `bytes`. `tainted string` is
text from outside your program, such as a form field or a command-line argument. `secret string` is
a confidential value, such as a password or an API key.

A value is tainted or secret only where a declaration says so. A string built from it with `.` or
interpolation is tainted or secret too. The compiler checks both words before the program runs, so
they do not make the program slower.

A tainted string cannot be a regular expression, a format template or a date pattern. It cannot be
assigned to a plain `string` either. A secret value does not compile with `echo`, in an error
message, in a debug dump or in `Core\Json::encode`.

`secret` can also go before `mixed`. A secret value can be in a `mixed` variable only
when you write `secret mixed`. A secret value cannot be an array key, and `as int` does not compile
with it. `Core\Secret::reveal` is the only method that gives a plain value from a secret one. You
give it the reason as its second argument.

Examples 04 to 07 show each kind of mistake, and none of them compiles.
