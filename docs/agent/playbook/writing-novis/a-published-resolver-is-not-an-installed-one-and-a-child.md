- **A published resolver is not an installed one, and a child staying `on: "here"` needs the
  second.** `nvs_runtime::script::publish` fills a process-wide slot a worker core reads as it
  starts, while the publishing thread still compiles through its own thread-local — so a `spawn
  script` path that falls through to the calling core answers `ResolveError::NoResolver` while the
  identical placed one succeeds. Wrap the caller in `SharedResolver::scoped`, as `nvs-cli` does for
  the core it booted on. [until: gone crates/nvs-runtime/src/script.rs:pub fn publish]
