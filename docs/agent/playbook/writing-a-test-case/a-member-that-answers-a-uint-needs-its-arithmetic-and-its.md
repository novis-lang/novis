- **A member that answers a `uint` needs its arithmetic and its `??` fallback written as `uint` too,
  and both misses read as the member being wrong.** `$rows->count() / $perPage` is `uint|float`
  because `/` widens, and `$row->uint("n") ?? 0` is `uint|int` because the literal `0` is an `int`,
  so each is an `E0401` against a declared `uint` that names the member's call rather than the
  operator. Write the fallback as `(0 as uint)`, and divide with `Core\Math::intDiv` over values
  converted once with `as int`. [until: gone crates/nvs-stdlib/src/math.rs:Core\Math::intDiv]
