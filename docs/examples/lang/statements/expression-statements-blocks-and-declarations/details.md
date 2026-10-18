A variable is declared once in a function. A second declaration of the same name does not compile: it
is the error `E0406`. This is also true inside a block, and for the variable of a second `for` loop.

A declaration inside a loop body is one statement. It runs on every round of the loop, and it gives the
variable its first value again each time.

A variable declared inside one branch of an `if` has no value on the other paths. Reading it after the
`if` does not compile: it is the error `E0301`.
