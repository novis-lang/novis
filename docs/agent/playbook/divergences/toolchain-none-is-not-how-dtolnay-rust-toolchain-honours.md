- **`toolchain: none` is not how `dtolnay/rust-toolchain` honours `rust-toolchain.toml` — the action
  has no such case.** It hands the literal word to `rustup toolchain install`, which answers
  *"invalid value 'none' for '[TOOLCHAIN]...'"* and takes every Rust job on every platform down in
  seconds. A bare `rustup toolchain install --no-self-update` is the honest form: the argument
  defaults to the active toolchain, so the file stays the one home for version, components and
  targets. [until: reviewed 2026-09-06]
