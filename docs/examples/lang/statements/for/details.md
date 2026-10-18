The first part of the header declares one variable at most. `for (int $a = 0, int $b = 0; …)` is
the error `E0125`. Declare the second variable above the loop.

The first part is either one declaration or a list of expressions. It cannot have both. `for (var $i
= 0, $j = 1; …)` is the error `E0124`.

The second part can be a list separated by commas. Before each test, the loop runs every expression
in the list. The last expression decides if the loop runs again. The expressions before it also run
on the test that ends the loop.

Two loops in one function cannot declare the same counter. The second declaration is the error
`E0406`.
