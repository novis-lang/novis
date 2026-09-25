- **`Core` is two rosters, not one, and the second is the exception tree.** Narrowing a `Core` name
  from the `is_core()` spelling test to `nvs_stdlib::registry` looks total — the registry calls
  `CLASSES` the whole roster — and refuses `new Core\Test\Failure(...)`, which lives in
  `nvs_hir::errors::TREE`; `QName::is_reserved_global_class`'s doc says so, on the predicate you are
  not editing. Any check reading `is_core()` as "in the registry" owes `errors::is_exception_class`
  beside it. [until: gone crates/nvs-hir/src/errors.rs:is_reserved_global_class]
