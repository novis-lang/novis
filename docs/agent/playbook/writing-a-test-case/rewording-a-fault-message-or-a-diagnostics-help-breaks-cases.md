- **Rewording a `Fault::` message or a diagnostic's help breaks cases pinning a substring of it;
  `cargo test` never runs them.** An `--EXPECT--` pins the line, and an invariance sweep asserts
  `Core\Str::contains($message, "at offset " . $j . " is")` or `Core\Str::startsWith($message, ...)`
  and prints `0 of 8`, so the failure waits for `nv verify`. Before changing it, `grep -rn` the corpus
  under `tests/` for the head, a distinctive interior phrase, and `startsWith`. [until: gone crates/nvs-test/src/run.rs:standard output does not match]
