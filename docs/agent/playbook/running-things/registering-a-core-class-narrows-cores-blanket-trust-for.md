- **Registering a `Core` class narrows `Core`'s blanket trust for that name.** An unregistered
  `Core\X::y()` is waved through by `nvs_hir::members`; once `X` is in `registry::CLASSES`, an
  unknown member on it is a diagnostic. Adding a class can turn a fixture that "compiled" into one
  that reports — which is the point, but check the fixtures that name it. See `crates/nvs-hir/src/members.rs`.
  [until: gone crates/nvs-hir/src/members.rs:nvs_stdlib::registry]
