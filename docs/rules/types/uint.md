`uint` is an unsigned 64-bit integer, `0 … 2^64−1`. It is a new **tag** in the existing tagged value
whose payload is already a `u64`, so a `uint` costs **zero additional bytes per value**.
`Core\Reflect::typeOf` reports it as its own kind, and `$x is uint` asks for it directly
(`rule:types/type-test`).

Being its own tag is observable: `$id is int` is **false** for a value that arrived as a `uint` — a
`BIGINT UNSIGNED` key or a snowflake id, which is what `uint` was added for — and `$id is int|uint`
tests for any integer.

`uint` exists because web software needs the half of the 64-bit range a signed integer cannot reach: `BIGINT UNSIGNED` keys, snowflake ids, nanosecond timestamps, WIT's `u32`/`u64`. An
integer literal that does not fit `int` is legal only where a `uint` is expected, and is otherwise a
diagnostic saying exactly that.

`int` and `uint` are separate types everywhere it matters. They mix in a comparison, which has an
exact answer over the mathematical integers, and they do not mix in arithmetic, which has no
representable common type to return (`rule:types/arithmetic`). Converting between them is `as`, and it
throws rather than wrapping (`rule:types/conversion`).
