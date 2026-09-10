A call site's two ways of not giving an options-bag field a value are two different requests. `{}` leaves a
component alone; `{fragment: null}` removes it (`rule:core-api/a-written-null-removes`). Before this, an
omitted field arrived at the helper as a null and a written null arrived as the same null, so a field that
could itself hold one was unusable and every bag field had to be non-nullable.

The mechanism is a constant rather than a sentinel: an omitted **nullable** field materializes a
never-written marker instead of a null (`rule:core-api/a-nullable-field-omits-as-the-never-written-marker`),
so the helper reads three states out of one argument and the flattening is untouched
(`rule:core-api/the-bag-abi-is-unchanged`). An in-band marker — `""`, a magic string, a reserved constant —
is a value the field's own type admits, so user data can arrive as one by accident; that is the bug class
this removes rather than relocates.

`crates/nvs-stdlib/src/registry.rs`'s `Const::NeverWritten` is the declaration and its two guards,
`a_nullable_option_omits_as_the_never_written_marker` and
`a_nullable_shape_field_omits_as_the_never_written_marker`, hold the pairing over every registered row.
`Core\Uri::with` is the first member to spend it.
