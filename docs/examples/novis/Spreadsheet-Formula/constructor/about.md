Makes a `Formula` from its two values: the formula text and the result the file has for it.

`new Formula` takes `text` and `cached`, in that order, and each becomes a read-only property.
`text` is the formula without `=` at the start. `cached` is a string, a number or `null`. `text` is
`tainted`, because `Reader` also makes a `Formula` when it reads a formula from a file that a user
sent.

**Good to know:** `Formula::of` makes a `Formula` for you and removes a `=` at the start. Use
`Formula::of` in your programs.
