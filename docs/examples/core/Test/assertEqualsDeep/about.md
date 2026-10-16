`Core\Test::assertEqualsDeep()` compares two values part by part. It goes into every array and every
object and compares what is inside. Two objects are equal when they have the same class and every
property is equal. The objects do not need a `compareTo` method.

When the values are different, the check throws a `Core\Test\Failure`. The message gives the path
to the first difference, written the way you write it in code, such as `$actual->lines["0"]->quantity`.
It also shows the two values at that place.

The check goes at most 64 levels deep. A value nested deeper than that throws an error, so an
object that points back to itself cannot make the check run forever.

**The examples below** show two nested arrays, the message of a failed check, and a test of an
order with its lines.
