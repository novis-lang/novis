`foreach` visits every element of an array, a generator or any object you can iterate. Write
`foreach ($subject as var $value)`, and the body runs once for each element.

`var` gives the variable the type of the subject's values. Over an array you may get the key too,
with `var $key => var $value`. The key is always a `string`. You can write the type in place of
`var`, as in `string $key => int $value`, and the compiler checks it. A variable with neither does
not compile.

The value variable is a copy. With `inout`, as in `foreach ($rows as inout var $row)`, each element
goes back into the array when the body ends. The syntax `&$value` does not compile. A plain
loop walks a copy of the array, so adding or removing elements in the body does not change what
it visits.

**The examples below** show values, then keys with values, then `inout`, each with `var` first and
a written type after. The last one loops over what a function returns.
