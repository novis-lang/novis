- **A `Core` member that takes a capability is *six* edits, and the sixth is in another crate.**
  `conventions.md` § *A `Core` member* lists five, all in the class's own module, but a gated member
  also owes a row in `nvs_stdlib::registry::CAPABILITIES`, and that row cannot be written until
  `nvs_config::Cap` carries the variant — the enum is closed and both of its `match self` arms are
  exhaustive. Land the grant first (the variant, its `[capabilities.<family>]` struct in
  `nvs_config::tree`, and the `ALL`/`name`/`grant`/`grant_mut` arms), then the row, whose
  `crate::<module>::NAME` is still private if that class had no rows before.
  [until: reviewed 2026-09-10]
