- **A non-blocking `connect` cannot be completed by asking `peer_addr`, whatever `mio`'s own example
  says.** On Windows that call answers `Ok(the target address)` for a socket whose connect never
  succeeds, and `take_error` is `Ok(None)` too until the attempt actually ends, so a connect built
  on either alone reports success and hands back a stream that is writable and dead. The pair sound
  on both platforms is `take_error` plus a zero-length write — `Ok(0)` when connected,
  `NotConnected`/`WouldBlock` in flight, the refusal itself on Linux — and
  `crates/nvs-host/src/net.rs`'s `finish_connecting` is the worked shape.
  [until: gone crates/nvs-host/src/net.rs:finish_connecting]
