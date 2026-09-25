- **`Core\Config::get` answers nothing for a key only the run mode derives, so a directive page
  prints `(nothing)` for a value that is genuinely in force.** `get` reads the request overlay, the
  secrets and the written table and no fourth thing (`crates/nvs-config/src/request.rs:88-97`),
  while `mode::DERIVED` is applied by whoever reads the key — so `debug.inline` answers nothing in
  a checkout whose mode has it `false`. Give such an example a `?? '(nothing, so the mode decides)'`
  fallback, or write the key in `nvs.toml` the way `[control] socket` and `[debug] keep_temporary`
  are written. [until: gone crates/nvs-config/src/mode.rs:DERIVED]
