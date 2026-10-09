Returns the one value of an attribute as a whole number, such as a person's `logonCount` or the
domain's `minPwdLength`.

If the entry has no value for the attribute, the result is `null`. A directory does not send an
attribute that has no value, so `??` is a simple way to give a default.

`int` reads an attribute with one value. For an attribute with several values, it throws a
`LogicError`. It also throws a `LogicError` when the value is not a whole number, for example a
name, or a number too large for an `int`. The message names the attribute.

**Good to know:** some numbers in Active Directory are sets of flags, such as `userAccountControl`.
`int` returns the raw number. `accountFlags` and `groupType` return the same number with one
`bool` for each flag.
