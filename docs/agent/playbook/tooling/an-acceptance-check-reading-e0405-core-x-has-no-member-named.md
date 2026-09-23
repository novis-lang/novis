- **An acceptance check reading `E0405: Core\X has no member named y` can be a stage nobody has
  started, not a regression.** `nvs_hir::qname::is_reserved_global_class` knows the *name* while no
  module registers a member, so every member of an unwritten class fails with that sentence, and
  because the program legs stop at the first failure, a fixture going green moves the line to the
  next `exact` check, however far ahead. One `ls crates/nvs-stdlib/src/<x>.rs` separates "a row went
  missing" from "the stage is unstarted"; everything behind that fixture is dark until it lands.
  [until: reviewed 2026-09-06]
