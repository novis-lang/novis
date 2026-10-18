One `unset` can remove several entries: `unset($a["k"], $a["j"]);`. Removing a key that is not in
the array does nothing. The entries after a removed one keep their keys.

`unset($x)` on a variable does not compile: it is the error `E0234`. `unset($o->p)` on a property
does not compile either: that is the error `E0413`.

`exit("bye")` writes the text to standard output. `exit` is also an expression, so
`int $n = $value ?? exit(4);` ends the program with status 4 when `$value` is `null`.

`yield` outside a generator does not compile: it is the error `E0445`. A method that contains `yield`
must return `Iterator<T>`, or it is the error `E0446`.

`yield from` does not exist: it is the error `E0448`. Write a `foreach` loop that yields each value
instead.
