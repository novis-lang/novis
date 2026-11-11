`foreach` visits every element of an array, a generator or any object you can iterate. Write
`foreach ($subject as var $value)`, and the body runs once for each element.

`var` gives the variable the type of the subject's values. Over an array you may get the key too,
with `var $key => var $value`. The key is always a `string`. You can write the type in place of
`var`, as in `string $key => int $value`. A variable with neither does not compile.

The value variable is a copy. With `inout`, as in `foreach ($rows as inout var $row)`, each write to
the variable goes back into the array. The syntax `&$value` does not compile. A loop visits the
elements the array had when it began, so adding or removing elements does not change what it visits.
If the body removes an element and then writes its `inout` variable, the write throws a `LogicError`.

**The examples below** show values, then keys with values, then `inout`, each with `var` first and
a written type after. Then come a loop over what a function returns, a generator, a wider type, and
two loops that do not compile.
