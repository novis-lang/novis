- **Windows compiles none of a crate's `#[cfg(unix)]` half, so `verify.py` on this host is silent
  about it** — a Unix-only type can be green here and not compile at all. The check is one call,
  `wsl.exe -- bash -lc 'cd /mnt/<drive>/<repo> && CARGO_TARGET_DIR=/var/tmp/nvs-target-wsl cargo
  test -p <crate>'`, and a second for `cargo clippy -p <crate> --all-targets` because the lints are
  just as unrun. `mio::net` re-exports no address type at all, so the symmetric-looking
  `mio::net::SocketAddr` is `E0425` — invisible to every Windows leg. [until: reviewed 2026-09-06]
