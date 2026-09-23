- **A `PgRows` holds its `&mut Ctx` borrow to the end of the scope because it has a `Drop`, and
  `E0499` names neither.** NLL ends a borrow at its last use only for a type with no destructor, so
  a second `ctx` call after draining a row stream fails with "first borrow might be used here, when
  `answered` is dropped". Write an explicit `drop(answered)` before touching `ctx` again; the same
  holds for anything handed out of `&mut Ctx` that releases on drop, a held connection included.
  [until: gone crates/nvs-db/src/pg.rs:Drop for PgRows]
