- **A bare-message failure with no runtime error class installed loses its message at
  `Ctx::take_thrown`, and answers a null `Thrown`.** `set_pending` files a `Pending::Message` and
  the promotion into an object needs a class descriptor — `Thrown::new_as` returns `Thrown::none()`
  for a null one, whose `message()` is `""` — so a Rust-side test that built `Ctx::new(...)` by hand
  reads `left: ""` against the message it just set, with nothing pointing at the missing class.
  `crates/nvs-host/src/group.rs`'s `ctx_with_error_class` is the four-line fixture
  (`Ctx::set_runtime_error_class`); asserting on `matches!(.., Threw(_))` instead is a weaker test
  for no reason. [until: reviewed 2026-09-06]
