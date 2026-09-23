- **A named `const` holding a `Cell` is `clippy::declare_interior_mutable_const`, which is denied
  here.** The obvious way to build a `thread_local!` array — `const EMPTY: Class = …;` then `[EMPTY;
  N]` — is refused, because a constant is copied at each use rather than referenced. The spelling
  that works is an inline const block in the repeat, `[const { … }; N]`, which is also a
  const-repeat of a non-`Copy` type and so still `const`-initializes the thread local.
  [until: reviewed 2026-09-06]
