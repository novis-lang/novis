Adds one or more values to the end of an array.

`Core\Arr::append` never changes the array you give it. It returns a new array: first every entry of
your array, under the key it already had, then each value you added.

Every key in Novis is a string. An added value gets the next free number as its key. Novis looks at
the keys that are written as a number, takes the largest one, and counts on from there. A list of
three entries therefore gets the key `"3"`, then `"4"`. An array with no number key at all starts at
`"0"`.

You can pass as many values as you like in one call. They are added in the order you write them. If
you pass no value, the result is your array, entry for entry.
