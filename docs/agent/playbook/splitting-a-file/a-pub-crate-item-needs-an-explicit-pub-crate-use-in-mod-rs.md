- **A `pub(crate)` item needs an explicit `pub(crate) use` in `mod.rs`** or `crate::thing::name`
  stops resolving for the rest of the crate. A glob `use self::child::*;` covers the in-directory
  names; the re-export list covers the crate-facing ones, and the two coexist.
  [until: reviewed 2026-09-06]
