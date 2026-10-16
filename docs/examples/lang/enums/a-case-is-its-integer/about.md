Turns an enum case into the whole number behind it.

Every case of an enum has a number, and `as int` gives you that number. This is the only conversion
out of an enum. A case has no text form, so `echo` on a case, a case inside a `"..."` string, and
`as string` do not compile. To show a case to a person, convert it to its number first, or write
your own function that returns a name for each case.

An enum declared as `enum E: uint` converts with `as uint` instead, and only with `as uint`.

**Good to know:** the number is the value of the case, which is not always its position in the list.
The first case is `0`, and each later case counts on from the one before it. A case may also set its
own value.

**The examples below** show reading the number, printing a name for a case, and saving a choice as a
number.
