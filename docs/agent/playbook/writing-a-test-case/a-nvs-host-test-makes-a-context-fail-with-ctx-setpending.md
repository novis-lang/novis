- **A `nvs-host` test makes a context *fail* with `Ctx::set_pending("message")`, not with
  `Thrown::new`.** `Thrown::new` is `unsafe` and wants a `*const ClassDesc`; `set_pending` takes a
  bare message and `take_thrown` promotes it through the context's error class. A *named* failure
  needs one installed first (`ClassTable::define("Throwable", &["message", "previous", "backtrace",
  "location"], &[])` plus `set_runtime_error_class`); without one `take_thrown` answers a null
  `Thrown`. [until: gone crates/nvs-runtime/src/ctx/error.rs:set_runtime_error_class]
