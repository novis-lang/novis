Each field of a `Core\Task::all` argument binds its type from **a written `fn` literal's declared
return type**. A field whose value is a `callable`-typed variable is a compile error naming the
field, and so is an argument that is a variable rather than a shape literal written at the call.

There is nothing to bind from otherwise: a `callable` carries no signature, so a variable at that
position leaves the answer's field with no type to be. Reporting it at the field is what keeps the
diagnostic actionable — the program is told which field to write out, not that the call is wrong.

This is a real restriction and it is visible. A framework that stores closures in a variable and
runs them cannot use `all`; it uses `rule:concurrency/map-preserves-keys-and-order` over an
`array<callable>` and accepts a `mixed` result. Typed `callable` signatures are what would remove
the restriction, and until they land this is the honest cost of not answering `array<mixed>`.
