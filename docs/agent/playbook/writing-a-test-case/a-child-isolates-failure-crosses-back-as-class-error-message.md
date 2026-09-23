- **A child isolate's `Failure` crosses back as `class: "Error", message: ""` when the parent fixture
  has no error class.** `isolate::finish` renders it through `Ctx::take_thrown`, which reads the
  *parent's* `set_runtime_error_class` — `Ctx::isolate` copies it down — so a case built on a bare
  `Ctx::new` asserts against an empty failure and reads exactly like a boundary that dropped the
  child's breach. Give the parent a one-class `ClassTable` first, as
  `crates/nvs-host/src/isolate.rs`'s `parent` does. [until: reviewed 2026-09-14]
