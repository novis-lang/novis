- **A hostile proof red at a step that does no recursion, with "the call stack is too deep", is a
  miscompile, not a leaked limit.** The floor-debt red on the two iteration interfaces looked like a
  stack bound left low by the step before it. It was a direct call to the program's one `Iterator`
  class, made on a generator. Cut the proof into one file per step under `.agent-tmp/` and run each
  with `nvs run`, then delete the classes the step does not use until it passes. See
  `crates/nvs-types/src/signatures.rs` `mark_sole_bodies`. `until: gone crates/nvs-types/src/signatures.rs:fn is_native`
