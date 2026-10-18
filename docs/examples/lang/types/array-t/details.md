An `array<T>` is an ordered table with `string` keys and values of the type `T`. Arrays can contain
arrays to any depth, such as `array<array<int>>`. `array<mixed>` can contain values of any type.

An `int` key is the same key as its digits written as a string, so `$a[1]` and `$a["1"]` read one
entry. The key variable of a `foreach` loop is always a `string`.

`$a[] = 60;` adds a value at the end. Its key is one more than the largest whole-number key, so after
`$a[5] = 50;` the next key is `6`.

Every value must have the type `T`. `array<int> $a = [1, "two"];` is the error `E0401`, and so is
`$a[] = "x";`. Reading a key that does not exist throws a `RuntimeError`: `undefined array key`.

Assigning an array to another variable copies it, the same as passing it to a function. A change through one variable is never seen
through the other.

An array has no string form: `echo $a` is the error `E0707`. `Core\Json::encode($a)` gives its text.
`$a + $b` with two arrays is the error `E0467`. `Core\Arr::underlay($a, $b)` combines them and keeps
the value of `$a` where both have a key.
