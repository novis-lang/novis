- **A `.nvst` case's program name is `case`, and that is stable enough to freeze in an
  `--EXPECT--`.** `nvs-test` writes every case as `case.nvs` and runs `nvs run case.nvs`
  (`crates/nvs-test/src/run.rs`), so anything reading `nvs_runtime::Ctx::program_name`, such as
  `Core\Command::completions`, answers `case` in a conformance case and the file's own stem
  everywhere else. It is neither the `.nvst` file's name nor `nvs`; guessing either produces a diff
  that looks like a bug in the member. [until: gone crates/nvs-test/src/run.rs:"case.nvs"]
