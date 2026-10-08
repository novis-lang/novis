There are exactly two ways to obtain a value of a different type: `expr as T`, and a second binding
declared at the type you want. `as` is **total in intent and checked in fact** — it produces a value
of the target type or it throws. It never rounds, truncates, or substitutes a default. `expr as ?T` is
the same operator over a nullable target and yields `null` exactly where `expr as T` would throw.

This is the whole conversion surface:

| conversion | behaviour |
|---|---|
| `int` ↔ `uint` | exact, or throws — a negative into `uint`, or above `i64::MAX` into `int` |
| `int` / `uint` → `float` | exact, or throws above 2^53, where `f64` stops representing every integer |
| `float` → `int` / `uint` | integral and in range, or throws. Rounding is `Core\Math::floor`/`ceil`/`round`, said out loud |
| `string` → `int` / `uint` / `float` | the whole string must be an exact numeric literal, or throws. No leading-garbage rule, no `0` |
| anything → `string` | total for scalars; an object needs `Stringable`, or it throws. Out of `mixed` the result is `tainted string` (`rule:security/taint-propagation`) |
| `array<T>` → `array<U>` | every element must satisfy `U`, or be an `int` or `uint` where `U` is `float`, at any depth; an O(n) walk, one tag test per element. Where every element already satisfies `U`, the result shares the one copy-on-write buffer. Where an `int` or `uint` element meets a `float`, the result is a new array of the operand's size with that element converted, exact or throwing above 2^53. An element type naming a class, an enum, a single-value type or a union is refused where it is written, `array<mixed>` being the way round it |
| `int` / `uint` → `decimal` | always exact — both fit in 96 bits |
| `decimal` → `int` / `uint` | integral and in range, or throws. Rounding is `Core\Decimal::floor`/`ceil`/`round` |
| `float` → `decimal` | the shortest decimal that round-trips to that `float` — `0.1 as decimal` is `0.1` |
| `decimal` → `float` | nearest `f64`, lossy, and written like every other `as` |
| `string` → `decimal` | the whole string must be an exact decimal literal, or throws |
| `decimal` → `string` | total, and preserves scale: `19.90` renders `"19.90"` |
| `string as bytes` | total and free — valid UTF-8 is already a valid byte sequence, so the same buffer is reinterpreted |
| `bytes as string` | checked: the buffer must be well-formed UTF-8, or it throws. Never replaces, drops or substitutes an invalid byte |
| `EnumName` → its backing `int`/`uint` | total and free — the same representation, reinterpreted |
| backing type / `mixed` → `EnumName` | checked. Throws unless the value equals some case's value |
| `EnumName` → a different `EnumName` | **rejected**, even through `as`, whatever backs them; converting is a `match` naming every case |
| base type / `mixed` → a single-value type or a set of allowed values | checked against the named set (`rule:types/single-value-types`) |
| an enum / `mixed` → a case-subset type | checked against the named cases (`rule:types/enum-case-type`) |
| `string` / `class<U>` → `class<T>` | the name must be `T` or a class that is one, or it throws. `Foo::class` is decided at compile time, and `class<T>` → `string` is total — the descriptor's own name, not the annotation's |
| `string` / `property<U>` → `property<T>` | the name must be one of `T`'s public declared properties, or it throws. A written-out name is decided at compile time, and `property<T>` → `string` is total |
| `mixed` / `object` / a union / a class → a shape | checked: the value must have every field the shape names at the named type, tested the way `$x is Shape` tests it (`rule:types/type-test`), or it throws the `RuntimeError` a failed `as ClassName` throws. An operand that already satisfies the shape converts for free; one that holds no object, or a shape with a `secret` field, is refused where it is written. From `mixed` or `object`, a text field must be written `tainted` — `as tainted {…}` — or the conversion is refused |
| any row above, under a qualifier | a successful checked conversion to a non-text type strips `tainted` and `secret`; a text result keeps both, and `as` is never a launderer for a value that keeps its type |
| any row above, out of `mixed` | the one place a qualifier is added: a text result is `tainted` wherever its type can carry it, and a non-text result is clean (`rule:security/taint-propagation`) |

A conversion the operand disproves by itself is a **compile** error rather than a run-time throw:
the target has to be a closed set and the operand has to name one value. Everything else is answered
where it runs.

`as` binds tighter than any binary operator, so `$a as int + 1` is `($a as int) + 1`. Inside a
`foreach` header the `as` belongs to `foreach`, so converting the subject takes parentheses:
`foreach (($m as array<int>) as int $v)`. Two conversions are *not* spelled with it: an `int` or
`uint` widening into a `float` position, which is implicit (`rule:types/implicit-widening`), and a
condition, which tests any type for truthiness without asking for one. The `(int)` cast syntax
is not a second spelling — it does not parse at all (`rule:types/no-legacy-cast`).
