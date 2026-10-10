- **One `CAPABILITIES` row makes the whole class capability-bearing, and every sibling member then
  owes a row of its own.** Adding `(Core\Cache, "shared", NetConnect)` is one line, and it turns
  `every_capability_bearing_member_declares_its_capability`
  (`crates/nvs-stdlib/tests/capability.rs`) red on `Core\Cache::local`, a member that needs no
  grant; a member that reaches nothing declares `None` in `registry::CAPABILITIES` beside a comment saying what it
  reaches instead. The class behind the door is untouched (`Core\Cache\Store`).
  `until: gone crates/nvs-stdlib/tests/capability.rs:fn every_capability_bearing_member_declares_its_capability`
