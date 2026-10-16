`Core\Test::assertCount()` checks that an array has the number of entries you expect. The first
argument is the array, and the second is the number. The number cannot be negative.

The check works only on arrays. To check the length of a string, compare `Core\Str::length` with
the number in `Core\Test::assertEquals`. That way the reader sees that you count characters.

When the number is different, the check throws a `Core\Test\Failure`. The message shows the number of
entries and the number you expected. The option `message` adds your own text in front of it.

**The examples below** show a check of a function's result, the message of a failed check, and a
test of a list split into pages.
