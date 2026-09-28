`Core\Test::assertEquals()` checks that a value is equal to the value you expect. The first argument
is the result of your code, and the second is the expected value. Both must have the same type.

Numbers, strings, `null` and arrays are equal when they are the same value. Two objects are equal
when `compareTo` returns `0`, so the class must implement `Comparable`. An object without
`compareTo` throws an error. Use `Core\Test::assertEqualsDeep` to compare its properties instead.

When the values are different, the check throws a `Core\Test\Failure`. The message shows both
values. The option `message` adds your own text in front of it.

**The examples below** show a check of a function's result, the message of a failed check, and a
test of a money object.
