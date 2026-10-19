A test that proves what a value is lets you use it as that thing, in the code the test guards.

The shortest form is one expression: `$user != null ? $user->name : "guest"`. The right side of `&&`
runs only when the left side is true, so `$user != null && $user->active` works too. An `if` block
or a `match (true)` arm does the same for more code. You can test for `null`, test the kind of
object with `is`, or compare with a fixed word.

**Good to know:** a test works only for a variable or a `readonly` property, and only until you
write to the variable. Copy any other property, such as `$job->mode`, to a variable first.

**The examples below** use a value that might be `null`, check the kind of an object, and act on a
job's settings. The last two check two values in one test, and show a method call that does not
compile.
