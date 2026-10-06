A condition is decided by one truthy table over Novis's type set:

| type | falsy | truthy |
|---|---|---|
| `null` | always | never |
| `bool` | `false` | `true` |
| `int` / `uint` | `0` | anything else |
| `float` | `0.0`, including `-0.0`; `NAN` is truthy | anything else |
| `string` | `""` and the one-character `"0"` | every other string, `"0.0"` and `"false"` included |
| `bytes` | empty, and **only** empty | every non-empty buffer, the one-octet `"0"` included |
| `array<T>` | empty, for any `T` | one or more elements, whatever they hold |
| class instance, `callable` | never | always |
| enum case | never | always (`rule:enums/truthiness`) |
| `mixed`, a union | dispatched on the runtime tag, one row per tag | — |

**The `bytes` row drops the `"0"` case deliberately.** That exception belongs to text that may be read
as a number, and `bytes` is the type that never converts to one. A `bytes` reaching a condition through
a `mixed` takes the same row, so the two
spellings of one buffer never disagree.

Where the static type is a scalar, array, class, `callable` or enum, the compiler knows the row and
lowers to the matching native test with no dispatch. Only a `mixed` or union condition pays for a
runtime helper.
