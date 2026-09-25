- **`python`, not `python3`; `gen` is reserved in Rust 2024; a renamed snapshot test needs its old
  `.snap` deleted.** `cargo test --release -p nvs-abi-probe` takes over two minutes.
  [until: gone Cargo.toml:edition = "2024"]
