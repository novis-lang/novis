`echo` on a case, a case in a `"..."` string and a case joined with `.` are the error `E0707`. A case
as an array key is the error `E0434`. Write `$case as int` for the key.

`as float`, `as decimal` and `as string` on a case are the error `E0708`. So is `as int` on a `uint`
enum, and `as uint` on an `int` enum. Convert to the backing type first, then to the type you want:
`(Level::High as int) as string`.

`as bool` on a case compiles, and the result is always `true`. Every case is `true` in a condition,
also a case whose number is `0`.

A case of one enum does not convert to another enum. `Mode::Read as Access` is the error `E0416`, also
when both enums have a case with the same number. Write a `match` that names every case.

`+`, `-` and the other arithmetic operators on a case are the error `E0415`. Convert the case with
`as int` first. This also applies to `-1 as Level`: `as` runs before the `-`, so write `(-1) as Level`.

`Core\Json::encode` and `Core\Debug::dump` write a case as its number.

In a `mixed` variable, a case knows that it is an enum case, but not which enum. `$value is int` is
`false` for it. `$value is Level` is `true` when its number belongs to a case of `Level`, also when it
came from another enum.
