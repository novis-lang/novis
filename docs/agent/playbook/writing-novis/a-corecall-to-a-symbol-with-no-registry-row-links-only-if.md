- **A `CoreCall` to a symbol with no registry row links only if `nvs_stdlib::symbols()` chains it
  in** — an arm in the module's own `address()` is not enough, and the failure is a Cranelift panic
  at run time reading `can't resolve symbol nvs_core_script_spawn`, naming no Novis file.
  `symbols()` walks `registry::CLASSES` and `CONSTRUCTORS`, so a member with a row is found for
  free; a rowless symbol needs its
  own `.chain([...])` there beside `address`'s arm, plus a term in
  `every_registered_member_has_an_implementation_address`' arithmetic. The `.chain` entry has no compile-time gate.
  `until: gone crates/nvs-stdlib/src/lib.rs:fn symbols`
