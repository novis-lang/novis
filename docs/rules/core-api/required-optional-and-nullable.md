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
emitted, including a `null` one. A `mixed` field reads as a `?T` row: `mixed` holds `null` without
spelling it, as `rule:core-api/a-nullable-field-omits-as-the-never-written-marker` says, and the
encoder writes a `mixed` field's `null`, so a decode refusing it would break the round trip.

Every row is answered at every door that reads a derived codec, a row's columns included: the default
is evaluated while compiling into a constant that rides on the field itself
(`nvs_runtime::CodecField::default`), so a decoder fills an absent optional key without being the call
site that would otherwise emit one. A written `= null` is that constant too
(`nvs_runtime::FieldDefault::Null`), so `?T $x = null` fills an absent key with `null`. A constructor position **no field names** — a property
`skip: true` removed from the contract, which `rule:core-classes/derive-property-list` sanctions — has no
field to carry a constant and is filled by nothing, so the call that asks for an instance out of a
document is refused while compiling rather than answered with a guess: `E0820` at a document door and
`E0806` at a row's.

**A `#[Query]` parameter answers rows 1, 3 and 4.** A query string cannot carry `null`, so a key is
present with text or absent, and row 2 has nothing to accept: `?T $x` without `= null` is `E0752` there.
Row 4's `?T $x = null` gives `null` for an absent key and converts a present one as `T`
(`rule:routing/a-query-parameter-is-declared-like-a-capture`).

**Declined:**
- **An omit-when-null option.** An asymmetric encoder is a round-trip bug that shows only in the
  value that happens to be absent. One conditioned on the field having a default can be added later
  without changing anything here, once a real interoperability case needs it.
