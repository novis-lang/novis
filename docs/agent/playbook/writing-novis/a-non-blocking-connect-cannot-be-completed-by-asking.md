- **A non-blocking `connect` cannot be completed by asking `peer_addr`, whatever `mio`'s own example
  says.** On Windows it answers `Ok(the target address)` for a connect that never succeeds, and
  `take_error` is `Ok(None)` until the attempt ends. Ask a zero-length write, then `take_error` after
  any failed write: macOS answers a refused connect with `BrokenPipe` and leaves the refusal in
  `SO_ERROR`. `crates/nvs-host/src/net.rs`'s `finish_connecting` is the worked shape.
  `until: gone crates/nvs-host/src/net.rs:finish_connecting`
