- **A `static` a test handler records into is shared by every test in the binary, and cargo runs
  them on separate threads.** A second test registering a handler that writes the same slot turns a
  `take()` from either reader into a `None` for the other, in a case that passes alone under
  `--test-threads=1`. Compiled code is called through a bare `extern "C"` pointer that captures
  nothing, so the fix is a slot and a recorder per test (`crates/nvs-host/tests/limits.rs`'s
  `SEEN_LIMIT` and its twin), not a lock held longer. [until: gone crates/nvs-host/tests/limits.rs:SEEN_LIMIT]
