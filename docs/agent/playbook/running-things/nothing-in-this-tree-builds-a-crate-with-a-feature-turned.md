- **Nothing in this tree builds a crate with a feature turned off, so a `#[cfg(feature = ...)]` arm
  rots unnoticed.** `verify.py` is the workspace at its default features, so the `exporter`-less shape
  of `nvs-server` and `nvs-cli` compiles in no gate at all. `cargo check -p nvs-cli
  --no-default-features --all-targets` is what proves it — a debug `-p` on purpose, and it does write
  the second metadata copy AGENTS.md rule 5 names, which for a `check` over a warm graph is seconds
  rather than a second build — so run it after touching anything those arms reach.
  [until: exists tools/verify.py:no-default-features]
