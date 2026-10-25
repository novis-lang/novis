- **A count that differs cold and warm only under a ceiling is a thread-local made lazily after
  `memory_base` was read.** A compile already made it on the thread, while a warm run makes it inside the
  run, so the context holds its bytes and the ceiling is crossed sooner. Print `ctx.memory_used()` right
  after `Ctx::stdout()`: a difference there is a first use in `Ctx::new`, such as `rand::rng()`. See
  `crates/nvs-runtime/src/ctx/wiring.rs`. `until: gone crates/nvs-runtime/src/ctx/wiring.rs:let trace_context`
