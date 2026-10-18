A default must be a constant of the parameter's type: a value written in the code, a class constant,
an enum case or `null`. A call such as `Core\Math::abs(-1)` is not allowed (`E0451`). Only the last
parameter can be variadic, with `...`. Inside the method, a variadic `int ...$rest` is an
`array<int>`, and it is empty when the call gives no more arguments.

A call with too few or too many arguments does not compile (`E0402`). `...$list` at the call passes
the values of an array as separate arguments.

A named argument must come after every argument without a name. A positional argument after a
`name:` or `...` argument does not compile (`E0488`). A name that no parameter has is `E0486`, and a
parameter given two arguments is `E0487`.

An `inout` argument must be a variable or a property of an object, such as `$box->count`. An array
element and a static property do not compile (`E0439`). A property with hooks, a property named by
a key and `$obj?->count` do not compile either. Neither does the result of a call or a value written
in the code. The argument's type
must be exactly the parameter's type. An `int` variable at an `inout float` parameter does not
compile (`E0440`). A call that leaves out `inout` before the argument does not compile (`E0713`).

`&` before a parameter does not compile (`E0237`). An anonymous function written with `fn` cannot
have an `inout` parameter (`E0493`).
