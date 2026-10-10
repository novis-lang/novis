- **A test that drives an accept loop must loop on `report.parked` rather than call
  `nvs_host::run_until_idle` once.** That call returns the moment one blocking poll wakes nothing,
  and readiness collected for a task that has already ended is the ordinary way that happens — so a
  core that is still accepting stops at whichever stale wake came first, leaving the client with no
  answer and no end of file. It fails on windows-x86_64
  and passes on the other legs; drive with `serve.rs`'s
  `run_the_core`, which is `nvs-cli`'s worker loop with a deadline on it.
  `until: gone crates/nvs-server/src/serve.rs:fn run_the_core`
