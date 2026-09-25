- **A request that parks makes `hyper` skip its post-response read, and what breaks is the
  keep-alive clock.** `Conn::maybe_notify` returns early under `is_read_blocked()`, which any
  earlier `Pending` from the IO adapter sets, so `crate::io`'s "a read in the `Write` phase ends the
  response" never fires and the connection parks under the write wait until the client's FIN. Call
  `cx.waker().wake_by_ref()` from the service once the answer exists, and debug with timestamps —
  the trace reads identically without them. [until: gone crates/nvs-server/src/serve.rs:wake_by_ref]
