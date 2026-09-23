- **A new `[limits]` key needs four edits, and the one that is easy to miss makes the other three
  read as a silent default.** `Request::get` resolves a bare name to `limits.<name>` only when
  `nvs_config::value::unit_of` knows the leaf, so a key with its `Limits` field
  (`crates/nvs-config/src/tree.rs`), its `DIRECTIVES` row (`crates/nvs-config/src/directive.rs`) and
  its `Ctx` reader but no `unit_of` arm never reaches `[limits]` and answers its default — nothing
  refuses, only the number is wrong. The roster is the `Limits` field, the `DIRECTIVES` row, the
  `unit_of` arm, and the reader; nothing fails to compile without the third.
  [until: gone crates/nvs-config/src/value.rs:fn unit_of]
