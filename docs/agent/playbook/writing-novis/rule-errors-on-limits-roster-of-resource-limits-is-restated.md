- **`rule:errors/on-limit`'s roster of resource limits is restated in four places, and three of them
  are prose no test reads.** The rule's own list (`docs/rules/errors/on-limit.md`),
  `nvs_runtime::Limit`'s enum doc and `Core\Fatal::onLimit`'s reference card `short` in
  `crates/nvs-stdlib/src/fatal.rs` each enumerate them, and only the enum's `name()` arm is
  load-bearing, so nothing fails when the other three drift. `grep -rn -i "cpu.time" docs/rules/errors
  crates/nvs-stdlib/src crates/nvs-runtime/src` finds all of them in one call.
  `until: gone crates/nvs-stdlib/src/fatal.rs:Core\Fatal::onLimit`
