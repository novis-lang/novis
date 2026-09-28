`Core\Test::assertContains()` checks that an array contains a value. The first argument is the array,
and the second is the value you look for. The value must have the type of the array's entries. The
position of the entry does not matter.

The check uses the same comparison as `Core\Arr::contains`. Numbers, strings and `null` match when
they are the same value. An object matches only when it is the same object. Another object with the
same properties does not match.

When no entry matches, the check throws a `Core\Test\Failure`. The message shows the number of
entries and the value you looked for. The option `message` adds your own text in front of it.

**The examples below** show a successful check, the message of a failed check, and a test of a list
of error messages.
