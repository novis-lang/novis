Declares a new type whose values are a fixed set of named numbers.

Write the cases between `{` and `}`, separated by commas. Every case has a whole number behind it.
You can write that number with `=`, or leave it out. The first case with no number written is `0`,
and every later case with no number written is one more than the case before it. Two cases may
have the same number.

`enum Level: int` and `enum Level: uint` name the type of number behind the cases. With nothing
written it is `int`.

The body may also declare methods and constants, but no properties. Inside a method, `$this` is the
case the method is called on, as in `Level::Info->label()`.

**The examples below** show an enum with no numbers written, then an enum whose numbers you choose,
then permissions where each case is one bit. Example 04 counts on from a written number. Examples 05
to 07 show three enums that do not compile. Example 08 shows a constant and a method.
