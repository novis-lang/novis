- **`#[cfg(unix)]` code is not compiled by anything this box runs, so a green `nv verify` says
  nothing about it.** The first Linux build of `nvs-cli`'s signal half reported
  `function_casts_as_integer` on a line that had been green here for sessions, and clippy's
  `-D warnings` on the driver's own leg would have failed on it. Build it yourself in seconds
  against the warm target dir the leg uses — `wsl.exe -- bash -lc "cd <the repo's /mnt path> &&
  CARGO_TARGET_DIR=/var/tmp/nvs-target-wsl cargo build --quiet"` — whenever a session writes or
  edits a `#[cfg(unix)]` block. [until: reviewed 2026-09-15]
