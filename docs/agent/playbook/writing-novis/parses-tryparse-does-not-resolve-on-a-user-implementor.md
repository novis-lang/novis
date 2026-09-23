- **`Parses::tryParse` does not resolve on a user implementor, though the interface declares it with
  a body.** `crates/nvs-types/src/iter_lib.rs:123` seeds `tryParse` with `has_body = true`, so it
  reads as available, but `Slug::tryParse("x")` on a class implementing `Parses` is
  `E0309: 'Slug' has no method named 'tryParse'` — only `parse` is reachable. Write `parse` with a
  `catch` where you wanted the pair, and never put `tryParse` on a user class in a `docs/reference/`
  example, because `tools/reference.py` compiles every one of them.
  [until: reviewed 2026-09-08]
