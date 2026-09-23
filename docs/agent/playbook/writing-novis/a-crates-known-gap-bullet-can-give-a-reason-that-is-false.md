- **A crate's known-gap bullet can give a reason that is false about the driver it names.** A gap
  bullet naming *another* crate's mechanism is a claim about that crate as it was when the bullet
  was written — `crates/nvs-stdlib/src/db/mod.rs`'s `stream` `{chunk?}` gap argued from a portal
  walk that `nvs_db::pg`'s `open_portal` does not do. Grep the named mechanism before implementing
  the gap or restating its reason. [until: reviewed 2026-09-06]
