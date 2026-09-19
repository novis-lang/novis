Declares a new type whose values are a fixed set of named numbers.

Write the cases between `{` and `}`, separated by commas. Every case has a whole number behind it.
You can write that number with `=`, or leave it out. The first case with no number written is `0`,
and every later case with no number written is one more than the case before it. A number may be
negative or zero, and two cases may carry the same number.

`enum Level: int` and `enum Level: uint` name the type of number behind the cases. With nothing
written it is `int`. No other type backs an enum. An enum body holds cases and nothing else: a
method, a constant or an `implements` clause does not compile. Put the behaviour in a class that
takes the enum.

**The examples below** show an enum with no numbers written, then an enum whose numbers you choose,
then a set of permissions where each case is one bit of a number.
