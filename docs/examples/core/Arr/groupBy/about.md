Sorts the values of an array into groups. A function names the group each value belongs to, and the
result holds one array per group.

You give `Core\Arr::groupBy` an array and a function. The function receives the value and the key,
and returns the name of the group. A function that needs only the value declares only that one
parameter. A group name is a whole number or a text.

A group appears in the result when its first value does, so the order follows the array you gave.
Inside a group, every value keeps the key it had. That tells you which entries ended up together.
Use `Core\Arr::values` on a group when you want a plain list instead.

**Good to know:** to count the entries per group instead of collecting them, use `Core\Arr::countBy`.
