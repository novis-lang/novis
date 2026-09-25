- **A new compiler rule that every existing fixture violates is one helper edit and some line
  numbers, not N rewrites.** `crates/nvs-types/tests/routes.rs` builds fixtures through one
  `route_src` helper, so `rule:attributes/access-is-a-required-sibling` landed there
  (`with_access`). A `.nvst` is out of a helper's reach: `--EXPECTF-ERROR--` quotes *source line
  numbers*, so a line inserted into `--FILE--` shifts every one below it, and a `reject/` case must
  still fail for its *own* reason. [until: gone crates/nvs-types/tests/routes.rs:with_access]
