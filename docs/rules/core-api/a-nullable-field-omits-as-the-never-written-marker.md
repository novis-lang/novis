A field of an options bag or of a shape arm carries a default, and that default is what an omitting call
site materializes. Three declarations, three behaviours:

| Declaration | Key absent | Key present as `null` |
|---|---|---|
| no default | a compile error: required key missing | a compile error: `null` is not of the field's type |
| a default, non-nullable type | the default is materialized | a compile error: `null` is not of the field's type |
| a default, **nullable** type | the never-written marker is materialized | `null` is materialized |

Only the third row is new, and the rule that holds it is a **pairing**: a field admitting `null` — a
nullable, a union with a null arm, or a `mixed`, which admits one without spelling it — is admitted exactly
when its default is the never-written marker, and a field not admitting `null` is admitted exactly when its
default is a null or a literal. A registry row getting the pairing wrong fails the build.

The pairing is what makes "filled" readable at all: the constant standing for *omitted* has to be one no
written value can also be, or the two states collapse again.

The two guards in `crates/nvs-stdlib/src/registry.rs` —
`a_nullable_option_omits_as_the_never_written_marker` and
`a_nullable_shape_field_omits_as_the_never_written_marker` — assert exactly this pairing, one per
spelling, over every registered row.
