`current()` throws a `LogicError` when it is called before the first `advance()`, and when it is
called after an `advance()` returned `false`. It answers only between those two points:

```
current() outside the iteration protocol: it answers only after advance() returned true
```

There is no sentinel for "outside the sequence" and no third member to ask first — `advance()`'s
`bool` is the whole of the liveness answer (`rule:iteration/two-interfaces`), so a consumer that
ignores it is asking for an element the cursor does not have. Both points need the guard for the
same reason and neither is the harmless one: before the first `advance()` the slot holds the element
type's null payload, and after the last it still holds the final element, which looks like an answer.

A `foreach` advances and reads in lockstep, so it never stands outside the protocol. A generator's
cursor is guarded at the same two points, and a generator that yields nothing makes them one point.
