- **A `Core` class holding slots and registering no instance member fails a registry unit test until
  it is named in that test's own roster.** `a_class_with_slots_has_instance_members_and_the_reverse`
  (`crates/nvs-stdlib/src/registry.rs:5110`) asserts the two are empty together, so a class storing
  state nothing reads back — `Jwe\Key`, `Jwt\KeySet` — fails on that assertion rather than on
  anything in its own file. Add its `NAME`, made `pub(crate)`, to the test's `HANDLES` list in the
  slice that registers the class.
  [until: gone crates/nvs-stdlib/src/registry.rs:const HANDLES]
