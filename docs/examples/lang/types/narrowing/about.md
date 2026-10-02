A test that proves what a value is lets you use it as that thing, in the code the test guards.

The shortest form is one expression: `$user != null ? $user->name : "guest"`. The right side of `&&`
runs only when the left side is true, so `$user != null && $user->active` works too. `||` works the
same way when its left side is false. An `if` block or a `match (true)` arm does the same for more
code. You can test for `null`, test the kind of object with `is`, or compare with a fixed word. When
you are sure what a value is, `as` converts it and throws an error if you are wrong.

**Good to know:** a test works only for a variable, and only until you write to it. Copy a field
such as `$job->mode` to a variable first.

**The examples below** use a value that might be `null` in one expression and then in an `if`
block. Then they check the kind of an object, with `as` as the strict form, and act on a job's
settings.
