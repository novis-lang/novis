- **A new `ExprKind` variant breaks no build outside `nvs-syntax`, so nothing tells you which passes
  still owe it an arm.** `ExprKind` is `#[non_exhaustive]` (`crates/nvs-syntax/src/ast.rs:770`), so
  every match on it in another crate already carries a wildcard — the checker's is
  `_ => env.interner.mixed()` and lowering's is a panic, both of which a new node reaches silently.
  Grep for the sibling variant you copied (`ExprKind::Conversion`) across `crates/` and add an arm
  everywhere it appears, rather than trusting `cargo check` to list them.
  [until: reviewed 2026-09-10]
