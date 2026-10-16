A double records the calls made to it, and the test asserts on that record afterwards with ordinary
assertions, in the order the test reads: `Core\Test::assertCalled($mailer, Mailer::send, {times: 1,
with: [...]})` and `assertNeverCalled`.

The method named is a compile-checked reference, so renaming it updates or breaks the test and can
never leave one silently passing against a method that no longer exists. It is written bare —
`Mailer::send`, not a callable and not a string — and it is admitted at these argument positions and
nowhere else: everywhere else that spelling reads a class constant and goes on reading one, so no
"method as a value" expression enters the language. What it folds to where it is written is the
method's own name.

There is no `expects()`. An expectation declared before the exercise reads backwards and reports its
failure from a line that is no longer where the problem is.
