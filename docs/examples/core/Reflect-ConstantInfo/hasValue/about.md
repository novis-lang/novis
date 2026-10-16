Tells you whether `Core\Reflect\ClassInfo::constant` can return the value of a class constant.

`hasValue()` returns `true` when the constant's value is a `string`, `int`, `bool` or `float`
written in the declaration. It returns `false` for an `array` constant. For such a constant,
`constant` throws a `LogicError`, because it has no value to return.

`hasValue()` does not check who may read the constant. It returns `true` for a private constant
with an `int` value, and `constant` can still throw a `RuntimeError` outside the class. `isPublic`
is the check for that.

**The examples below** show which kinds of constant have a value, what happens when you read one
that has none, and how to print the public settings of a class when some of them are lists.
