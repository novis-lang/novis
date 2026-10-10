- **A `Core` member that answers `Iterable<T>` owes a named class, and `CoreTy::Iterated` is not
  it.** A member returning a sequence answers a
  `CoreTy::Instance` of a class carrying the list, plus a `registry::ITERABLES` row for its element
  type and an `iterate()` on `instance::DISPATCH_ROSTER`. Such a class has one slot and no member of
  its own, which fails `a_class_with_slots_has_instance_members_and_the_reverse` until it is added
  to that test's `HANDLES` list; `Core\IO\Lines` is the worked example. See `crates/nvs-stdlib/src/registry.rs`. `until: gone crates/nvs-stdlib/src/registry.rs:a_class_with_slots_has_instance_members_and_the_reverse`
