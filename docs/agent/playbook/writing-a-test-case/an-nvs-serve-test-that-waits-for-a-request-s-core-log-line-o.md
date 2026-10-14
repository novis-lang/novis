- **An `nvs serve` test that waits for a request's `Core\Log` line on the server's stderr or stdout
  waits forever, though `nvs run` prints the same line.** A served request's log line never reached
  either pipe in `crates/nvs-cli/tests/serve_isolation.rs`, even seconds after the answer; why is
  *not checked*. Observe a served program through a file it writes under a `[capabilities.fs] write`
  folder instead, as `a_looping_guest_reaches_on_limit_and_the_next_request_is_served` does with
  `marks/limit.txt`. `until: gone crates/nvs-cli/tests/serve_isolation.rs:marks/limit.txt`
