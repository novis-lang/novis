- **A `Core` class only ever answered *inside an array* is attributed to no case, and its members
  read as asked by zero.** `crates/nvs-stdlib/tests/corpus/mod.rs`'s `Attribution::builds` maps a
  member to the class it builds from `CoreTy::Instance` and `InstanceAt` alone, so a row class
  reached through `array<PropertyInfo>` holds a case only where the case spells its qualified name.
  Write that name in — a typed `foreach ($roster as Core\Reflect\PropertyInfo $row)` does it.
  [until: gone crates/nvs-stdlib/tests/corpus/mod.rs:CoreTy::Instance(made)]
