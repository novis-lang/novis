Uses the cases of an enum as the keys of an array.

You write the type as `array<Status, int>`. The first type argument is the enum, and the second is
the type of the values. Every key of the array is a case of `Status`. A key that is a number, a
string or a case of another enum does not compile. To use a number as a key, convert it first with
`as Status`.

When you loop over the array with `foreach`, the key has the type `Status`. So you can pass it to a
`match` or to a method that needs a `Status`. A class constant can be such an array. Use one for a
label or a limit for each case.

**Good to know:** you cannot add a value with `$array[] = $value`, because there is no next key.
Write the case as the key: `$array[Status::Paid] = $value`.

**The examples below** show a constant table that gives a label for each case, then a count of
orders for each status, then a JSON object read into an array keyed by an enum and filtered.
