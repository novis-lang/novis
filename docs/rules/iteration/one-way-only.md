A generator is a lazy sequence in one direction. There is no `yield from`, no `send()` into a
generator, no `throw()` into one, and no generator return value to retrieve — a `return expr;` in a
generator body is refused, and so is a keyed `yield`.

`yield from` is a second spelling of the re-yielding loop in
`rule:iteration/yield-lexical-confinement`, and it costs O(nesting depth) per element where the loop
costs O(1) — a real if small loss, recorded rather than hidden. `send()` and `throw()` make a
generator a bidirectional coroutine, which is a different feature wearing the same syntax; Novis has
coroutines already and they are not spelled `yield`.

`yield` is a statement, so it produces no value to consume.
