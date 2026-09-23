- **A config field's `[unread:]` trailer is refused the moment *any* crate names the key, and
  `nvs_config::mode::DERIVED` counts as a namer.** Three gates in
  `crates/nvs-config/tests/directives.rs` fail together, the decisive one being
  `no_key_with_a_reader_still_claims_to_be_unread`. Grep the dotted key across `crates/` first;
  where a reader exists, describe the directive plainly and put what is still owed in the
  consuming crate's own `# Known gaps`.
  [until: gone crates/nvs-config/tests/directives.rs:no_key_with_a_reader_still_claims_to_be_unread]
