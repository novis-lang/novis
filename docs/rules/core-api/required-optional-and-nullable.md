"Does `?T` mean the key may be absent, or that `null` is a legal value?" has a better answer than choosing:
they are separate questions, and both already have a spelling.

| Declaration | Key absent | Key present as `null` |
|---|---|---|
| `T $x` | an issue: required field missing | an issue: `null` is not permitted |
| `?T $x` | an issue: required field missing | accepted, decodes to `null` |
| `T $x = <default>` | accepted, the default is used | an issue: `null` is not permitted |
| `?T $x = null` | accepted, `null` is used | accepted, decodes to `null` |

Nullability is a property of the **type** and optionality is a property of the constructor parameter's
**default**; neither borrows the other's meaning. Encoding is the plain inverse: every field is always
emitted, including a `null` one. There is no omit-when-null option, because an asymmetric encoder is a
round-trip bug that only shows up in the value that happens to be absent — and adding one later, conditioned
on the field having a default, is purely additive.

**Designed, not shipped.** `crates/nvs-stdlib/src/json.rs` records that the two default-bearing rows are
unimplemented: an absent key is always *required field missing*, because a default is evaluated into a
constant the *call site* emits and a native decoder is not a call site.
