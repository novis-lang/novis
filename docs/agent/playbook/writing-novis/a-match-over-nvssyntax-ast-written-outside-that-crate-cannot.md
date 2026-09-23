- **A `match` over `nvs_syntax::ast` written outside that crate cannot be exhaustive, and the
  compiler will not say so.** `ExprKind`, `StmtKind`, `ClassMemberKind`, `NewTarget` and
  `DestructureElement` are `#[non_exhaustive]`, so a walk in another crate needs a wildcard arm and
  silently treats every later production as a leaf. Put any per-variant table over the AST in
  `nvs-syntax` itself, as `crates/nvs-syntax/src/walk.rs` does, where a new variant is a build
  error. [until: gone crates/nvs-syntax/src/ast.rs:#[non_exhaustive]
