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

**Designed, not shipped.** `crates/nvs-stdlib/src/registry.rs` still holds the old invariant, under the
guards `a_shape_field_is_never_nullable` and `a_union_option_excludes_null`, so no bag field is nullable
today and `Core\Uri::with` still has no clearing spelling.
