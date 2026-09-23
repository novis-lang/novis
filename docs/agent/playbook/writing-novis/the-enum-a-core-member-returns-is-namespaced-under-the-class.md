- **The enum a `Core` member returns is namespaced under the class, so `Env\Mode::Development` does
  not resolve and `Core\Env\Mode::Development` does.** The rules and `crates/nvs-config/src/tree.rs`
  write the type as `Env\Mode`, which is the language's name for it and not the path a program
  spells; the registry's own `MODE_NAME` at `crates/nvs-stdlib/src/env.rs:267` is
  `r"Core\Env\Mode"`, and `E0303 no matching declaration` is what the short spelling gets. Read the
  `*_NAME` constant beside the `CoreMethod` row before writing an enum case into an example.
  [until: gone crates/nvs-stdlib/src/env.rs:Core\Env\Mode]
