`Core\Test::assertMatchesInline()` checks a value against a text you write in the test. This text is
called a snapshot. The check turns the value into text with `Core\Debug::render`, and then compares
the two texts. They must be equal character by character.

This is useful for a value with many parts, such as an array or an object. You write the whole
expected result once, and you see it in the test.

When the texts are different, the check throws a `Core\Test\Failure`. The message shows the text of
the value and the snapshot. An empty snapshot `""` is also a failure, so you can copy the text from
the message. A `secret` property is shown as `[redacted]`, so its value is never in a snapshot.

**The examples below** show a check of simple values, the message of a failed check, and a test of
an object with a `secret` property.
