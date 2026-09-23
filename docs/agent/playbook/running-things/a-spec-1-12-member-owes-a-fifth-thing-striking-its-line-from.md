- **A spec §§ 1-12 member owes a *fifth* thing: striking its line from
  `crates/nvs-stdlib/tests/spec-members-outstanding.txt`.** That file is the ratchet
  `crates/nvs-stdlib/tests/spec_registry_coverage.rs` reads, and it fails on a *stale* line naming a
  member that is registered now exactly as loudly as on an unregistered member it does not list — so
  the failure after landing a member is the list saying it did not shrink. Keys are `§<section> <the
  spec's own Member-cell spelling>`, so `§1 chunk` and `§2 chunk` are two lines.
  [until: gone crates/nvs-stdlib/tests/spec-members-outstanding.txt]
