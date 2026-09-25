- **A refusal for a *name-shaped* expression fires on every `Foo::bar()` unless the class side is
  taken off the value walk first.** `Class::method()`, `Class::CONST`, `Class::$prop` and
  `Class::class` carry the class as an ordinary `Expr` of kind `ExprKind::ConstFetch`, so a walker
  that recurses into it reports "a bare name is not a value"
  there. `nvs_hir::members::walk_class_side` is the helper over those sites in `walk_expr` and
  `nvs_types::expr::check_expr`; nothing catches a miss but a conformance case reporting an extra
  error. [until: gone crates/nvs-hir/src/members.rs:ExprKind::ConstFetch]
