The trailing options bag and the fixed-key shape parameter are two registry spellings that intern into
**one** checked type — a per-field required flag over an ordered field list — so the exact-key check, the
flatten and the diagnostics are written once and every existing behaviour of the bag is the all-optional
special case of the general one.

The variant is named for the general use rather than the narrower one, because a name taken from half of
what a type does is wrong in the other half. The reference card needs no widening either: it is already one
entry per key, and for a union it lists the merged key set in ABI order with each key's description saying
which arm it belongs to (`rule:core-api/reference-card`).

The cost was a rename touching the interner, the checker, the lowering and the metadata command, and that
was the whole of the churn.
