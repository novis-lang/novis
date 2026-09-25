- **`hyper` polls its connection's read before it writes the answer it already holds, so a blocking
  transport under it deadlocks** — the symptom is a test binary that runs its whole body and never
  reports. It reads the head, dispatches, then asks for the next request, which on a blocking stream
  waits for a client waiting for that answer. Make a transport's read answer `WouldBlock` rather
  than wait, as `crate::io::Nonblocking`'s does; and kill the hung `.exe` before rebuilding, or the
  link fails `LNK1104` naming nothing. [until: gone crates/nvs-server/src/io.rs:pub struct Nonblocking]
