- **`nvs_config::Request::set` is not a narrowing check for a `[limits]` ceiling.** Its module doc
  refuses "a `RuntimeTighten` one that does not narrow", but `capabilities` is the only such row in
  `crates/nvs-config/src/directive.rs`, so `[limits] memory` is plain `Runtime` and a request may
  set its own ceiling wider or narrower up to `[limits.hard]`. All `set` proves about a ceiling
  written at a call site is that the hard bound admits it, so read the directive row before writing
  "narrowed and never widened" about anything but a grant. [until: reviewed 2026-09-15]
