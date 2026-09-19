A method says what it needs by listing its parameters, and the call says which value fills which.

Each parameter is a type and a name. It may carry a default, so a caller who is happy with it leaves
it out; the last one may be variadic, collecting everything left over into a list. At the call, an
argument may be written with its parameter's name in front of it — `book(minutes: 60)` — which lets
you skip over the defaults you do not care about and makes a call with several bare numbers in it
readable. Plain arguments all come before the first named one, and the arguments run in the order
you wrote them.

A parameter written `inout` is the one that changes the caller's own variable: the method writes
back into it when it returns. The call has to write `inout` in front of that argument as well, so a
method reaching into your variable is never something you have to look up.

**Good to know:** parameter names are part of what a method promises, so a caller may rely on them.
