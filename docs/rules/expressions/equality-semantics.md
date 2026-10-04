Nothing converts. Each type has one row, and where PHP's two operators disagreed the row takes the
strict reading:

| type | two values are equal when |
|---|---|
| `null` | always — one value |
| `bool` | they are the same |
| `int`, `uint`, `float`, `decimal` | mathematically equal across the whole domain: `1 == 1.0` is true, and `decimal` ignores scale, so `1.10 == 1.1000` is true |
| `float` edges | IEEE 754: `NAN == NAN` is **false**, `0.0 == -0.0` is **true** |
| `string` | the same sequence of Unicode scalar values — never numeric, so `"1" == "01"` and `"1e3" == "1000"` are both false, and there is no normalization |
| `bytes` | the same bytes |
| `array<T>` | the same length, the same keys in the same order, and every value equal by this table, recursively |
| class instance | the same object (`rule:expressions/object-identity-equality`) |
| enum case | the underlying integers are equal |
| `callable` | the same callable — two `fn` literals with identical bodies are two callables |
| `mixed`, a union | resolved at run time (`rule:expressions/mixed-equality`) |

The array recursion terminates: an array is a copy-on-write value rather than a reference, so it
cannot contain itself. An array holding objects compares those by identity, which bounds that walk
too.

The natural ordering used for sorting is a separate question and answers differently for `float`; the
two are not the same table.
