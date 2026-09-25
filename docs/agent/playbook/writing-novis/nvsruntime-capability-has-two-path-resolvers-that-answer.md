- **`nvs_runtime::capability` has two path resolvers that answer different spellings of one file —
  on Windows a `\\?\C:\…` against a plain `C:\…`.** `capability::canonicalize` goes through
  `nvs_config::capability::resolved`, which pins a not-yet-existing path's deepest existing ancestor
  for `Core\IO::within`, while `std::fs::canonicalize` returns the verbatim form. A door that
  resolves goes through `nvs_config::capability::resolved` and asks any other question, existence
  included, separately. [until: gone crates/nvs-runtime/src/capability.rs:nvs_config::capability::resolved]
