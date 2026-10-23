- **A `Rewrite` that moves a declaration has to cover one code run, and `use Core\Str;` is not one.**
  `crates/nvs-fmt/src/print.rs` tiles a file into trivia and the code between two of them and applies
  each edit inside one of those runs, so an edit spanning the space after a keyword is never taken and
  trips the `debug_assert` in `push_code` later. Move the one token that differs, as `modifiers.rs`
  does, and ask `print::one_code_run` first. A move that spans trivia is a text pass before the
  parse, as `imports.rs`'s `sorted` is, and `format` parses its result again.
  `until: gone crates/nvs-fmt/src/print.rs:a rewritten range lies inside one code run`
