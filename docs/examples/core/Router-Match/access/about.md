Returns the access rule of the route that matched, as text. It is the full name of the constant in
the route's `#[Access(allow: …)]`, such as `Core\Audience::Public` or `App\Role::Admin`. The result
is text from your program and not from the request, so it is not `tainted`.

To check access, use `accessAs<E>()`. It returns the rule as a case of the enum `E`, and the
compiler checks the comparison. A comparison with the text still compiles after a case is renamed,
and then it never matches again. Use `access()` to show or log the rule, and for a rule that names
a class constant, which has no enum.

Every route in a program has an `#[Access]`, so the result is normally never `null`. If it is
`null`, treat it as "access denied".

The examples show the rule of each route, a rule that names a class constant, and the rule of the
current request in a log line.
