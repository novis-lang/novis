- **Rewording a `Core` reference card breaks a golden in another crate, and `-p nvs-stdlib` never
  shows it.** `crates/nvs-cli/tests/meta.rs`'s `the_golden_for_str_length_matches_the_contract` pins
  `Core\Str::length`'s whole card — the one card `nvs meta --json`'s golden froze — so a `str.rs`
  edit surfaces as a `nvs-cli` failure with no hint of what moved. `grep -rn "<the card's first
  clause>" crates/nvs-cli/tests/` before rewording.
  [until: gone crates/nvs-cli/tests/meta.rs:the_golden_for_str_length_matches_the_contract]
