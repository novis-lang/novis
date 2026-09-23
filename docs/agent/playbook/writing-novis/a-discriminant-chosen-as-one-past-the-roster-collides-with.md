- **A discriminant chosen as "one past the roster" collides with the next roster entry, and nothing
  in the roster's own crate says so.** `FN_PARAM_TAG_ANY`/`CLOSURE_PARAM_TAG_ANY` — the
  closure-parameter nibble meaning "no argument can be wrong for this one" — sat at the end of the
  `nvs_runtime::Tag` roster, so adding `Tag::Unset` made every `mixed` closure parameter demand a
  tag, and the failures named closure arguments rather than the tag. Both constants are `15` now,
  parked at the top of the nibble; `nvs-codegen`'s `the_any_nibble_denotes_no_tag_at_all` holds
  three crates' copies of the number together. [until: reviewed 2026-09-06]
