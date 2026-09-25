- **A new `.nvs` or `.nvst` anywhere under `tests/` joins `nvs-syntax`'s `lossless` corpus, which
  only fails at the end of the session.** `lossless` demands every byte be covered by a token or a
  trivium, which the `<?nvs` a shebang file refuses was not. Expect a case exercising a recovery
  path to be what finds a hole in `rule:ide/tokens-plus-trivia-reproduce-the-file`.
  [until: gone crates/nvs-syntax/tests/lossless.rs:losslessness]
