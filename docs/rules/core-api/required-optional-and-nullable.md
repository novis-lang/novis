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

**One row is designed, not shipped.** `?T $x = null` needs a written `= null` parameter default, which
the checker still refuses — `crates/nvs-types/src/defaults.rs`'s own gap, not this table's. The other
three are answered at every door that reads a derived codec, a row's columns included: the default is
evaluated while compiling into a constant that rides on the field itself
(`nvs_runtime::CodecField::default`), so a decoder fills an absent optional key without being the call
site that would otherwise emit one. What stays unfilled is a constructor position **no field names** —
a property `skip: true` removed from the contract — which `crates/nvs-stdlib/src/json.rs` records as
its own gap.
