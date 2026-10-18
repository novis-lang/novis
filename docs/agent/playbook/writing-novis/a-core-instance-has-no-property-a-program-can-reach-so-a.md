- **A `Core` instance has no property a program can reach, so a rule's `$x->thing` example is a
  surface the registry cannot express.** `CoreClass` has `methods`, `instance`, `slots` and
  `constants` and no field roster (`rule:core-api/a-lifetime-is-an-object`). Check that rule before
  believing a property spelling in a doc; the fix is a reader with parentheses, folded back into the
  doc in the same commit. See `crates/nvs-stdlib/src/registry.rs`.
  `until: gone crates/nvs-stdlib/src/registry.rs:pub struct CoreClass`
