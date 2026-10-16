Returns the one value of an attribute as `true` or `false`, such as `isCriticalSystemObject`.

A directory writes a yes-or-no value as the text `TRUE` or `FALSE`. `bool` reads that text and
returns a `bool`. If the entry has no value for the attribute, the result is `null`. Many entries
have no value for such an attribute, so decide what a missing value means, for example with
`?? false`.

`bool` reads an attribute with one value. For an attribute with several values, it throws a
`LogicError`. It also throws a `LogicError` when the value is not `TRUE` or `FALSE`, and the message
names the attribute.
