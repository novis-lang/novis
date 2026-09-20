Makes a new array in which every value is stored under a key that a function returns for it. The
values stay as they are, and only the keys change.

You give `Core\Arr::mapKeys` an array and a function. The function receives the value and the key,
and returns the new key. A function that needs only the value declares only that one parameter. A
new key is a whole number or a text.

The entries keep the order they had. When two entries get the same new key, they become one entry,
and the last of them wins. A whole number and its text are the same key, so `1` and `"1"` name one
entry.

**Good to know:** to change the values instead of the keys, use `Core\Arr::map`. To collect entries
into groups under a shared key, use `Core\Arr::groupBy`.
