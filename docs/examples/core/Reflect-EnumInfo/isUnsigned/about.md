Checks whether the values of an enum's cases are `uint` or `int` numbers.

Every case of an enum has a whole number as its value. The enum declaration chooses the type of
that number: `: uint` for numbers that are never negative, or `: int` for numbers that can be. An
enum written with no type is an `int` enum. `isUnsigned()` returns `true` for a `uint` enum and
`false` for an `int` enum. An enum with no case still has its type.

**Good to know:** `valueOf` returns `int|uint`. Check `isUnsigned()` first to know which of the two
you get.
