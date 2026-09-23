- **A `cargo-named` check's name can describe an assertion whose only copy is a `.nvst` case, and
  renaming closes nothing.** Stage 7's `webcrypto_jwe_vectors_decrypt_to_their_payloads` read as one
  more rename beside three JWE tests that all *write* tokens, while the decrypt direction lived only
  in `tests/conformance/core/jwe-opens-every-token-webcrypto-sealed.nvst`, which no `cargo test`
  output ever names. Read the *direction* a check's name states against what the crate's own tests
  assert before sizing the item: a Rust-side proof that is absent gets written, not spelled
  differently. [until: reviewed 2026-09-12]
