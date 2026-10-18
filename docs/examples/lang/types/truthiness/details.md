Every enum case is truthy, also a case whose value is `0`. A value of type `mixed` is tested by the
type of the value it has while the program runs. `-0.0` is falsy, like `0.0`. An array that
contains one `0` is truthy, because it is not empty.

`x as bool` gives `false` for every falsy value and `true` for every other value.

`empty($x)` gives `true` for a falsy value, so `empty("0")` is `true`. `isset($x)` tests only for
`null`, so `isset("0")` is `true`.

A label of `match (true)` is compared with `true` using `==`. It is not tested for truthiness. A
label of type `int` does not compile, because an `int` is never equal to `true` (`E0466`).

A value that is not a `bool` does not compile in a `bool` variable, a `bool` parameter or a `bool`
return value (`E0401` or `E0403`). Write `as bool` there.
