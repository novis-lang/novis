Tests a one-time code that a user typed for two-factor login. The result is a step number when the
code is right, and `null` when it is not.

Time is divided into steps of 30 seconds, and each step has its own code. A clock on a phone is
often a few seconds wrong, so `check` also accepts the code of the step just before and just after
the current one.

A code must never work twice. When `check` accepts a code, store the step number it returns with
the account. Pass that number as `$after` on the next login. Then codes of that step and of earlier
steps return `null`. A text that is not six digits also returns `null`, and does not throw an error.

The examples show a right code, the same code used twice, and a login form that stores the step
number with the account.
