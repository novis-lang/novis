- **A new `Core` class trips three closed lists.** Slots with no instance members fail
  `a_class_with_slots_has_instance_members_and_the_reverse` until the class joins its `HANDLES`. A
  `tainted` member fails `a_verified_signature_does_not_launder_its_claims` until it joins that roster.
  A value class fails `tests/ext_world.rs`'s `every_core_value_class_crosses_as_a_types_record_or_is_named_as_refused`
  until it has a `REFUSED` entry, which reached CI twice. Run all three by name. See `crates/nvs-stdlib/src/registry.rs`.
  `until: gone crates/nvs-stdlib/src/registry.rs:a_class_with_slots_has_instance_members_and_the_reverse`
