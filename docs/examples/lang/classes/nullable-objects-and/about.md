`?Box` is a type that holds a `Box` or `null`. A plain `Box` never holds `null`, so a value of that
type is always there.

`->` on a value that may be `null` does not compile. Novis needs you to say what should happen when
the value is missing, and there are two ways to say it.

Write `?->`. On a `null` value the whole access returns `null`. The method is not called and its
arguments are not evaluated. Put `?? "something"` after it to give the missing case a value.

Test the value first. Inside `if ($box != null)` the value is no longer nullable, so plain `->`
works there. This is the better choice when the branch does several things with the value.

**The examples below** show reading a setting that may be missing, testing a value before using it,
and a chain of accesses where any step may be absent.
