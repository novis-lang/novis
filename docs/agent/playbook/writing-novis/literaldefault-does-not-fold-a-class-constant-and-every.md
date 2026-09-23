- **`literal_default` does not fold a class constant, and every sibling pass's `folded_str` copies
  that hole forward.** `defaults::literal_default`'s `ClassConstAccess` arm lives in its callers, so
  a new compile-time string read built on it folds `Foo::WHY` to `None` and silently treats a named
  constant as computed. Use `defaults::fold_const_reference` (it needs a `&Ctx<'_>`), as
  `crates/nvs-types/src/reasons.rs`'s `folded_str` does. [until: reviewed 2026-09-06]
