- **A `TcpStream` accepted from a non-blocking `TcpListener` is non-blocking on Windows and blocking
  on Linux.** Windows' accept inherits the listening socket's mode and Linux's does not, so a
  handler that reads `WouldBlock` as the end of a connection closes it in the client's face and the
  case fails with a reset it never asked for. Say `stream.set_nonblocking(false)` on every accepted
  connection before reading it, as `transport.rs`'s `answer` does.
  [until: gone crates/nvs-stdlib/src/http/transport.rs:set_nonblocking]
