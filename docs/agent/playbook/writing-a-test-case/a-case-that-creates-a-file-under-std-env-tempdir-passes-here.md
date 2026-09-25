- **A case that creates a file under `std::env::temp_dir()` passes here and fails on Linux**, where
  `/tmp` is mode 1777 and `nvs_config::trust::check` walks the parents of what it is handed. Windows
  has no such parent and `nv verify` here never runs the Unix half, so it first fails in the WSL
  leg. Use a directory beside the test binary, as `crates/nvs-server/src/control.rs`'s `scratch`
  does. [until: gone crates/nvs-server/src/control.rs:fn scratch]
