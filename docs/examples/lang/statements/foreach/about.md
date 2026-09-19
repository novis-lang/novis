`foreach` visits every element of an array, a generator or any object you can iterate. Write
`foreach ($subject as int $value)`, and the body runs once for each element.

Every binding declares its type. `foreach ($subject as $value)` without one does not compile. Over an
array you may bind the key as well, with `foreach ($subject as string $key => int $value)`. An array
key is always a `string`, so that is the type the key binding takes.

The value binding is a copy, so writing to it leaves the array alone. Write `inout` in front of it,
as in `foreach ($rows as inout int $row)`, and each element goes back into the array when the body
ends. PHP's `&$value` does not compile.

A plain loop walks a copy of the array. Adding or removing elements inside the body does not change
what the loop visits. An `inout` loop walks the array itself.

**The examples below** show a loop over values, then a loop over keys and values, then an `inout`
loop that corrects every row in place.
