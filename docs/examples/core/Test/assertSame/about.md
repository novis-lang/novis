`Core\Test::assertSame()` checks that a result is exactly the value you expect. You pass the result
first and the expected value second. Both must have the same type.

Two numbers, strings or arrays are the same when their values are the same. Two objects are the
same only when they are one object. Two objects with equal properties are not the same. Use
`Core\Test::assertEquals` to compare them by their properties.

When the values are not the same, the check throws a `Core\Test\Failure`. The message shows both
values. The option `message` adds your own text in front of it.

**The examples below** show two checks that pass, the message of a failed check, and a test that a
cache returns one object every time.
