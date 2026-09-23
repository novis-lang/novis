- **Nothing this repository runs on Windows compiles a `#[cfg(unix)]` arm, `verify.py` included.**
  `cargo check -p nvs-db --all-targets` was green over a MariaDB socket arm that read a private field
  of `Greeting` and named a type its test module does not import, because that arm is not in the
  Windows build and `verify.py` has no WSL leg. Compile every new `cfg(unix)` arm before you commit
  it: `wsl.exe -- bash -lc "cd /mnt/d/mwl && CARGO_TARGET_DIR=/var/tmp/nvs-target-wsl cargo test -p
  <crate> -- <test names>"` builds and runs it in seconds warm, over the leg's own target directory.
  [until: reviewed 2026-09-09]
