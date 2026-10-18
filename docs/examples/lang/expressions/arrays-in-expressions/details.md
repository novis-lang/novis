`var $a = [];` is the error `E0414`, because an empty array has no values to take a type from.
`var $a = [1, "x"];` is `E0414` too. Its help names the type to write: `array<int|string>`.

A key is an `int`, a `uint` or a `string`. A `float`, `bool` or `null` key is the error `E0434`.
`"08"` and `"8"` are two different keys.

`$a[] = 1` uses the largest whole-number key plus one, and a negative key counts too. In an empty
array, after `$a[-5] = 1;` the next key is `-4`. `unset` does not number the keys again, so after
`[10, 20, 30]` loses key `1`, the next `$a[]` gets key `3`. `$a[]` can only be written to. Reading
it is the error `E0481`.

`[...$a, 30]` copies the entries of `$a` into the new array. A whole-number key, such as `"5"` or
`"-3"`, gets the next number of the new array. Every other key is kept. When two entries have the same key, the later
one wins: `["k" => 5, ...$a, "k" => 7]` has one `k`, and it is `7`. The values of `$a` must fit the
new array's type. An `array<int>` spread into an `array<string>` is the error `E0401`.

A write into a key that is not there creates every inner array on the way:
`$g["a"]["b"]["c"] = 1;` works on an empty `$g`. A read does not. `$g["a"]["b"]` throws a
`RuntimeError` for the first key that is missing: `undefined array key `a``. `??` and `isset` test
every level, so `isset($g["a"]["b"])` is `false`. `isset` is also `false` for a key whose value is
`null`.

`unset($a["k"])` does nothing when the key is not there. It also works on an inner array:
`unset($g["r"]["0"])`.

A function result is not stored anywhere, so `A::f()["k"] = 1;` is the error `E0700`. Assign the
result to a variable first.

Destructuring reads by key, not by order. `[int $a] = $p;` reads key `"0"`, wherever it is in `$p`.
Entries that no variable names are skipped, and a missing key throws `undefined array key`. A
positional variable and a `"key" =>` variable can be mixed, and a key can be a variable:
`[$k => int $v] = $row;`.

Each variable in a destructuring needs its type. `[$a, $b] = $p;` is the error `E0101`, and
`[var $a] = $p;` is the error `E0102`. Each type must match the array's value type, or it is the
error `E0401`. The right side must be an `array<T>`. A `mixed` or a `?array<int>` value is the error
`E0482`. A `foreach` variable cannot be a destructuring pattern, which is the error `E0101`.
`list($a, $b) = $p;` is the error `E0230`.
