Returns one value that a route captured from the path, found by its name. You write the name
without the braces: `param("id")` for `{id}`. The result is the same value that `params()` has
under that key.

The value is already percent-decoded, and it has the type of its handler parameter. A text
capture is `tainted`, because the visitor chose it.

The result is `null` when the route has no capture with that name. It is also `null` when an
optional capture such as `{page?}` is not in the path, so `??` can give a default.

The examples show how to read one capture, a default for an optional capture, and how to find the
record that the path names.
