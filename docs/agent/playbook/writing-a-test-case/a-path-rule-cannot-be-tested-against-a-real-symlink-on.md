- **A path rule cannot be tested against a real symlink on Windows; `nvs-config`'s `Files` trait is
  the seam instead.** Creating a symlink needs a privilege CI does not have, so
  `a_path_reaching_a_granted_root_through_dotdot_or_a_symlink_does_not_match` runs its `..` half
  against `resolve::Disk` and the real filesystem, and its symlink half against a fake whose
  `canonical` maps one path to another. `crates/nvs-stdlib/tests/capability.rs`'s `Fake` is the
  shape; the methods it must never reach are `unreachable!()` with a sentence saying why.
  [until: reviewed 2026-09-06]
