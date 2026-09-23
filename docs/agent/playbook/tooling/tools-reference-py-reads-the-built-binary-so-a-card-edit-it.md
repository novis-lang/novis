- **`tools/reference.py` reads the *built* binary, so a card edit it has not been rebuilt for is
  reported as `docs/novis.md unchanged`.** The generator asks `nvs meta` for the registry rather
  than parsing `registry.rs`, and `cargo test -p nvs-stdlib` does not rebuild the binary either, so
  a card fix that passes its own gate can still ship the old sentence. After fixing a `MethodDoc`,
  `cargo build` and *then* `reference.py`. [until: gone tools/reference.py:nvs meta]
