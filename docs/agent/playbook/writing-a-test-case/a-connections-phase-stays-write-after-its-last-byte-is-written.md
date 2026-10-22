- **A connection's `Phase` stays `Write` after `hyper` writes a response's last byte.** `hyper` does
  not read again until the next request's bytes arrive, so nothing moves the phase to `KeepAlive` on
  the poll that finishes the write. A check keyed on the phase held a place until the next request.
  Ask `write_parked` in `crates/nvs-server/src/io.rs`: a `Pending` poll with no write parked has nothing left to write.
  On Windows a loopback test cannot make the server wait: the kernel took 32 MiB in one write.
  `until: gone crates/nvs-server/src/io.rs:pub fn write_parked`
