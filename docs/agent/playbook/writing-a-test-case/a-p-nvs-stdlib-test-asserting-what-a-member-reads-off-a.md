- **A `-p nvs-stdlib` test asserting what a member reads off a throw's slot has to install an
  exception class table first.** `Ctx::pending_slot` answers `None` until `set_runtime_error_class`
  runs, so a member deciding on the `KIND_SLOT` sees nothing. `ClassTable::define` the four-slot
  `RuntimeError` root and a subclass wide enough for the slot (`Thrown::new_as` drops it silently);
  `retries_recover_an_induced_deadlock` is the shape. [until: gone crates/nvs-stdlib/src/db/transaction.rs:retries_recover_an_induced_deadlock]
