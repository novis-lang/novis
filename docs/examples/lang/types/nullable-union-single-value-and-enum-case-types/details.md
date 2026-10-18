`null` is a type of its own, and `null $n = null;` compiles. `int $i = null;` is the error `E0401`.
A `?int` value is not used in arithmetic before it is tested. `$maybe + 1` is the error `E0716`.
Test it with `!= null` first, or give a default with `??`.

A union may mix any types, such as `object|int`. Inside `if ($id is int)`, `$id` is an `int`.

A value written in the code is checked against a single-value type when the program compiles.
`$mode = "delete";` for a `"read"|"write"` variable is the error `E0401`. A value converted with
`as` is checked when the program runs. If it is not in the list, `as` throws a `RuntimeError`.
A single-value type can be assigned to its wider type: a `1|2|3` value to an `int`.

An enum is a list of named whole numbers. `enum Mode { Read, Write }` numbers its cases from `0`.
`enum Mode: int { Read = 1 }` gives each case its number. `$m as int` gives the number of a case.
`1 as Mode` gives the case with that number. If no case has it, `as` throws a `RuntimeError`, and
`as ?Mode` gives `null`. `Mode::Read as string` is the error `E0708`.

`iterable` and an intersection `A&B` can be written for a parameter or a return type. No value can
be assigned to either of them. `iterable $i = $xs;` is the error `E0401`.
