- **A differential case's output may not carry a lone carriage return, and the failure reads as the
  member losing it.** Both sides are normalised (`crates/nvs-test/src/expect.rs`), and a `\r`
  outside a `\r\n` survives on the Novis side and not on PHP's, so a `trimStart` case padded with
  carriage returns fails. Assert the carriage return by length —
  `Core\Str::length(Core\Str::trimStart("\r\rx"))` against `strlen(ltrim("\r\rx"))`.
  [until: gone crates/nvs-test/src/expect.rs:pub fn normalize]
