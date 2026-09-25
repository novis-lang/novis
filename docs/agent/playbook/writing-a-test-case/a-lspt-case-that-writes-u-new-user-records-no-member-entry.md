- **A `.lspt` case that writes `$u = new User();` records no member entry at all.** `var` is what
  declares a variable, so a bare assignment is `E0301` and the receiver has no type — the checker
  still records `ExprInfo::New` for the `new`, and nothing for `$u->name`, so a `definition` or
  `hover` case on the member answers `none` while the same case on the class name passes. Write
  `var $u = new User();` in any case whose cursor is on a member.
  [until: gone crates/nvs-diagnostics/src/lib.rs:E0301]
