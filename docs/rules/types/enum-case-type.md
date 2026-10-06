`EnumName::CaseName`, written where a type is expected, does **not** resolve to its backing integer. It
names a new checker-only type: a subtype of `EnumName` inhabited by exactly that one case.

```nvs
enum Mode { Read, Write, Admin }

function grant(Mode::Read|Mode::Write $m) { … }   // accepts only those two cases
```

Folding it to the cases' backing integers would let a caller satisfy the parameter with a bare `int`,
which is exactly the hole a checked `int → Mode` conversion closes
(`rule:enums/closed-integer-type`). An enum-case type is therefore its own atom kind, never unified by
canonicalisation with an `int` single-value type that happens to share a case's value, because the two carry
different runtime tags.

A case-subset union may name cases of more than one enum, or mix case atoms with unrelated atoms,
exactly as any other heterogeneous union may. Widening a case-subset union to its enum is total and
free; going the other way is checked and throws unless the value's case is one of the named ones — a
further-restricted form of the existing enum conversion, not a new kind
(`rule:types/conversion`). `E0470` names the accepted cases.

A binding is narrowed to a case-subset type **through `as` and nowhere else**: `$m == Mode::Read` does
not narrow `$m` in the branch it guards (`rule:types/narrowing`). Like a single-value type, this costs
nothing at runtime — it shares the enum's existing zero-byte representation
(`rule:enums/representation`).
