- **A `#[cfg(test)] mod tests` may already hold a `type` alias for the struct you are about to add,
  and `use super::*` lets the alias win.** A real `ColumnRow` above `catalog.rs`'s placeholder `type
  ColumnRow = (String, …)` failed as `expected struct, variant or union type, found (String, …)`.
  `grep -n '<TypeName>' <file>` before naming a type after something the file's tests name; delete
  the placeholder and move its `.0`/`.1` accesses onto the fields. [until: reviewed 2026-09-06]
