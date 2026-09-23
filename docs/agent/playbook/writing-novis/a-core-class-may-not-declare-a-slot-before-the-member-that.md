- **A `Core` class may not declare a slot before the member that reads it exists, and the test that
  says so names neither.** `a_class_with_slots_has_instance_members_and_the_reverse` holds slots and
  instance members to the same emptiness, with a `HANDLES` list for the classes whose slots
  something *else* reads (`Core\Regex\Pattern`, `Core\Script\Handle`, `Core\IO\Lines`, …). A class
  registered ahead of its readers declares `slots: &[]` and gains them in the same slice as the
  members, rather than joining `HANDLES` — that list is for state read from outside the class, not
  for state nothing reads yet. [until: gone crates/nvs-stdlib/src/registry.rs:const HANDLES]
