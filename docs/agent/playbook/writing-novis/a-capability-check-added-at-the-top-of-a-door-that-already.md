- **A capability check added at the *top* of a door that already refuses on a missing directive
  re-orders refusals the `.nvst` corpus pins exactly.** Two cases assert `open_configured`'s
  "no shared store is configured" over a tree that grants nothing, so a `require` above the
  directive read turns both into a capability denial. Ask the grant after the directive, which is
  `rule:security/capability-costs-nothing-unasked`'s shape, and grep the corpus for the refusal's
  own words first. [until: gone crates/nvs-stdlib/src/cache.rs:open_configured]
