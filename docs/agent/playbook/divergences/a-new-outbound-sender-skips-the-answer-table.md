- **A new outbound sender beside `Core\Http\Client` must take the test answer table's branch.** The
  client's `exchanged` checks `ctx.faked_http().is_armed()` before it resolves anything, but a
  sibling such as `push_send` calls `pin` and `transport::send` directly and reaches the network
  from a test that registered an answer. It also leaves its answer class with no way to be built in
  an example or a conformance case. Start every new sender with the armed branch and its `unanswered`
  error. See `crates/nvs-stdlib/src/http.rs`. `until: gone crates/nvs-stdlib/src/http.rs:fn unanswered`
