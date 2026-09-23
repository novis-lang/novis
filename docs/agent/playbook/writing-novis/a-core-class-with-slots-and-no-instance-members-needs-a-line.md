- **A `Core` class with slots and no instance members needs a line in `registry.rs`'s `HANDLES`, and
  the failure names your class.** `a_class_with_slots_has_instance_members_and_the_reverse` asserts
  the two rosters empty together, so an opaque handle (`Core\Db\InList`, `Core\Html\Markup`) reads
  as "you forgot the members". List it in the test's `HANDLES` const with a doc-comment clause
  saying who reads the slot, and make the class's name constant `pub(crate)` so the test can name
  it. [until: gone crates/nvs-stdlib/src/registry.rs:const HANDLES]
