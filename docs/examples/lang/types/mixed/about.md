`mixed` is the one type Novis does not check. It holds a value of any kind, and what an operation on
it means is worked out while the program runs, from the value that is actually there.

This is where input from outside the program lands, and deliberately so: a query string, a form post,
the fields of decoded JSON. Calling those numbers or text before anyone has looked would be a lie
written into the type, and one the program has no way to notice later. So they arrive as `mixed`, and
you turn each one into the type you want with `as`, which stops with an error when the value is not
that. `is` is the other half: it asks what the value actually is, without converting anything.

A `mixed` never becomes something narrower on its own. Handing one to a parameter that wants a
`string` is refused where it is written, rather than going wrong later.

**Good to know:** every operation on a `mixed` is decided while the program runs, which costs more
than the same operation on a declared type. Convert once, close to where the value arrives, and work
with the real type after that.
