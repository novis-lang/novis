- **A served request is an isolate, and an isolate used to be armed no memory ceiling at all** — a
  `[limits] memory` case written against `nvs run` passes while the same program served over a
  socket runs to its end. `Ctx::new` displaces the thread's threshold with `Armed::NONE`, and a
  child whose own `memory_limit` stayed `0` answered `false` to `over_memory_limit` at every poll,
  so neither half of the ceiling could fire. Assert a ceiling on the path a request takes — through
  `Isolate::run` or the accept loop — never on a root `Ctx`, the one shape that was already right.
  [until: gone crates/nvs-runtime/src/ctx/isolate.rs:isolate.set_memory_limit(remaining]
