Some places in a program ask a value a yes-or-no question without being handed a yes-or-no value.

A condition — the test in an `if`, in a `while`, in the middle of a `for`, the test of a short
`? :`, and the values either side of *and*, *or* and *not* — takes a value of any kind and decides
whether it counts as something or as nothing. Counting as nothing are: `false`, a zero, an empty
piece of text, the one-character text `"0"`, an empty list, an empty buffer, and a missing value.
Everything else counts as something, including the text `"0.0"`, a single space and every object.
It is the same table PHP uses, so a condition carried over from PHP keeps the meaning it had.

**Good to know:** those places are the only ones that ask the question for you. Storing a value in
a yes-or-no slot, or handing it to something that wants one, still needs you to write `as bool`.
`empty` asks the same question the other way round, and `isset` asks only whether a value is there.
