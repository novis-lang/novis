`throw $e;` may also throw an error that was caught earlier and saved in a variable or a property.
Each throw moves `location` to the line of that `throw`. The object stays the same, so `==` with the
saved variable is `true`.

A clause that catches a wide class, such as `catch (Throwable $e)`, does not change the class of the
error. After `throw $e;`, a `catch (TimeoutError $t)` further out still catches a `TimeoutError`.

After a rethrow, `backtrace` still starts at the function that threw first. For the function that
rethrew, the line in `backtrace` is the line of `throw $e;`.

The `finally` block of the `try` that rethrows runs before the error leaves that `try`.

When nothing catches the rethrown error, it ends the program like any uncaught throw. The report on
the error output names the function that threw first.
