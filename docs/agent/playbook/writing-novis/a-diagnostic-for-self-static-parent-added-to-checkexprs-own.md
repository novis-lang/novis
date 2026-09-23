- **A diagnostic for `self`/`static`/`parent` added to `check_expr`'s own arm reports twice.**
  `nvs_hir::members` splits the three keywords by position — a bare one where a value is expected is
  its `E0321`, a class side goes through `walk_class_side` unreported — and both walk through
  `check_expr`, so an arm there doubles up on `mixed $x = self;`. Put the report at the four call
  sites that resolve a class side (`infer_class_const`, `infer_static_call`, the
  `StaticPropertyAccess` arm, `check_new_target`).
  [until: gone crates/nvs-hir/src/members.rs:walk_class_side]
