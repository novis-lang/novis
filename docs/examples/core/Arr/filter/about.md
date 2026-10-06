Keeps the entries of an array that a function accepts, and leaves the others out.

You give `Core\Arr::filter` an array and a function. The function receives the value and the key, and
returns `true` for an entry you want to keep. A function that needs only the value declares only that
one parameter.

The keys stay as they are. An entry kept from position 3 still has the key `3`, so a filtered list
can have gaps in its numbering. Use `Core\Arr::values` when you want the result numbered from `0`
again.

**Good to know:** the function runs once for every entry, in the order the array holds them.
