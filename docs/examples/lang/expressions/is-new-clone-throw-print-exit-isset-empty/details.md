`is` works with every type a value can have: a class, an interface, `int`, `?int`, `null`, a union
such as `int|string` and an array type such as `array<int>`. When the declared type of the variable
already gives the answer, the test is fixed while compiling. For an `int` variable, `$n is int` is
always `true` and `$n is string` is always `false`.

The right side of `is` is a type. The only variable allowed there is a `class<T>` variable, as in
`$v is $cls`. A `string` variable is the error `E0496`. Convert the name first with `as class<T>`.

`new P` without brackets is the same as `new P()`. The arguments work as in a method call, and named
arguments work too. You can call a method on the new object directly: `new P()->hi()`. Inside a class,
`new self()` builds the class itself, and `new static()` builds the class the method was called on.

These forms of `new` do not compile:

- `new` with an interface or an `abstract` class is the error `E0785`.
- `new $c()` with a `string` variable is the error `E0496`. The class must be written in the code.
- `new P(...)` is the error `E0740`. A constructor is not a method, so use an anonymous function.

`clone` copies every property, and a `readonly` property too. The copy is a new object, so
`$p == $q` is `false` after `$q = clone $p`. A class cannot change what `clone` does. A method named
`clone` is an ordinary method, and `clone` does not call it. `clone` on an `int` or another type that
is not an object is the error `E0781`. On a `mixed` value that is not an object, it throws a
`LogicError` when it runs.

`throw` takes a `Throwable`. `throw "oops"` is the error `E0780`. A `throw` can be the right side of
`??`, `?:`, `||` and `&&`, or one result of a ternary.

`print` takes one value. `print "a", "b"` does not compile, and `print [1]` is the error `E0707`.
`print` gives `1`, so `$n + print "x"` prints `x` and gives `$n + 1`.

`isset` takes one or more operands, and is `true` only when every one has a value that is not `null`.
Each operand is a variable, a property or an array element. A call or a literal is the error `E0498`.
To test a call, write `A::f() != null`. A variable that was never declared is the error `E0301`, for
`empty` too. An operand of a type without `null`, such as `int`, is always `true`.

A missing key at any level gives `false`: `isset($a["q"]["y"])` is `false` when `"q"` is missing. A
nullable object on the way needs `?->`. `isset($p->next->v)` is the error `E0459` when `next` is a
`?P`, and `isset($p->next?->v)` is `false` when `next` is `null`.

`empty` takes one operand, and that operand may be any expression, a call included. `"0"` and an empty
array are empty, as the truth table says.
