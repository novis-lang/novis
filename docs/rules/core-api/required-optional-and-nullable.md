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
emitted, including a `null` one.

Every row is answered at every door that reads a derived codec, a row's columns included: the default
is evaluated while compiling into a constant that rides on the field itself
(`nvs_runtime::CodecField::default`), so a decoder fills an absent optional key without being the call
site that would otherwise emit one. A written `= null` is that constant too
(`nvs_runtime::FieldDefault::Null`), so `?T $x = null` fills an absent key with `null`. A constructor position **no field names** — a property
`skip: true` removed from the contract, which `rule:core-classes/derive-field-list` sanctions — has no
field to carry a constant and is filled by nothing, so the call that asks for an instance out of a
document is refused while compiling rather than answered with a guess: `E0820` at a document door and
`E0806` at a row's.

**Declined:**
- **An omit-when-null option.** An asymmetric encoder is a round-trip bug that shows only in the
  value that happens to be absent. One conditioned on the field having a default can be added later
  without changing anything here, once a real interoperability case needs it.
