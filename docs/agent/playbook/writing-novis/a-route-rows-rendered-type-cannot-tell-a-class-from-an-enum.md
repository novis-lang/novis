- **A route row's rendered type cannot tell a class from an enum, so a reader that must branch on the
  *kind* of type needs the row to say which.** `TypeInterner::describe` renders `Ty::Class(q, [])` and
  `Ty::Enum(q, _)` as the same bare qualified name, and `closed_set` answers `None` for both, so
  neither thing `RouteParam` carried discriminated them. Compute the answer where the interner is
  still alive and carry it on the row; `crates/nvs-types/src/routes.rs`'s `RouteParam::parses` is the
  shape. [until: reviewed 2026-09-08]
