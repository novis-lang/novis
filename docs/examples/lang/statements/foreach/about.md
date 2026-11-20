`foreach` visits every element of an array, a generator or any object you can iterate. Write
`foreach ($subject as var $value)`, and the body runs once for each element.

`var` gives the variable the type of the subject's values. You may get the key too, with
`var $key => var $value`. An array's key is a `string`, and a `Core\ObjectMap`'s key has the map's
key type. You can write the type in place of `var`, as in `string $key => int $value`. A loop variable without `var` and without a type does not
compile.

The value variable is a copy. With `inout`, as in `foreach ($rows as inout var $row)`, each write to
the variable goes back into the array. The syntax `&$value` does not compile. A loop visits the
elements the array had when it began, so adding or removing elements does not change what it visits.
If the body removes an element and then writes its `inout` variable, the write throws a `LogicError`.

**The examples below** show values, keys with values and `inout`. Then come a loop over what a
function returns, a generator, a wider type, and two loops that do not compile.
