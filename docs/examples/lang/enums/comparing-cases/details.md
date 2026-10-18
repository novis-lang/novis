A case compared with `==` or `!=` to a number, a string or a case of another enum is the error `E0466`.
A case compared to `null` is `E0466` too, unless the variable has a type such as `?Level` that allows
`null`. A `?Level` variable that contains a case compares with that case as usual.

`<`, `>`, `<=`, `>=` and `<=>` on two cases are the error `E0715`. Convert both sides with `as int`.

`==` compares numbers. When both sides have the type `Level`, only cases of `Level` are compared. In
a `mixed` variable, a case does not keep which enum it belongs to. There, `==` is `true` between a case
and a plain number with the same value, and between cases of two enums with the same number.
`Core\Arr::contains` compares the same way.
