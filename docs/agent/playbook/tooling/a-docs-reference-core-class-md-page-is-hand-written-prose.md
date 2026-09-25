- **A docs/reference/core/<Class>.md page is hand-written prose, not a render of the class's cards,
  so a finding that says "the card" means three files.** The fact can be right in the `MethodDoc`,
  right in the `docs/reference/lang/` chapter and wrong on the class page, and nothing checks them
  against each other. Check all three and let the binary break the tie — a probe under `.agent-tmp/`
  against `target/debug/nvs.exe` costs one call and is the only copy that cannot be out of date.
  [until: gone crates/nvs-stdlib/src/registry.rs:MethodDoc]
