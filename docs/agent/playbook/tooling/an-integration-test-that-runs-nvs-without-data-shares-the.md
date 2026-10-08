- **An integration test that runs `nvs` without `--data` shares the `.nvsdata` beside the built
  binary with every other test.** A project command that finds no configuration writes the shipped
  `nvs.toml` there, and step 3 of `rule:config/the-root-is-config-else-nvs-toml-else-the-shipped-defaults`
  reads it in the next test whose working directory has none. Pass `--data` with an
  `nvs_repo::scratch_private` folder, and `--no-init` where the case means *no configuration*, as
  `crates/nvs-cli/tests/check.rs` does. [until: gone crates/nvs-config/src/data.rs:pub const NAME]
