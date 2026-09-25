- **`hyper`'s `with_upgrades()` cannot drive a connection this server accepts; the way back to the
  socket is `http1::Connection::into_parts`.** `with_upgrades` is bounded `I: Send + 'static` and
  `crate::io::ConnectionIo` holds an `Rc` and a core-registered socket, while `into_parts` is a
  plain `self` method giving back `io` and `read_buf` after the plain `Future for Connection` ends
  at a `101` without shutting the socket. Drive the connection through `Pin::new(&mut conn)` in a
  `poll_fn` rather than moving it into `block_on`; `Connection` is `Unpin`.
  [until: gone crates/nvs-cli/src/serve.rs:into_parts]
