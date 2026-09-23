- **Taking a C dependency fails `--check-c-deps` on more crates than you took, and one of them is
  never compiled.** `tools/gen-attribution.py`'s enumeration is deliberately host- and
  target-independent, so a `links =` crate reached only under `cfg(target_arch = "wasm32")`
  (`sqlite-wasm-rs` beside `libsqlite3-sys`) is reported exactly like the real one. Write it a
  `no-native-code` row naming the target gate rather than hunting for where it got linked in, and
  run `python tools/gen-attribution.py` after, not only `--check-c-deps`: `THIRD-PARTY-LICENSES.txt`
  is a second gate with a separate failure. [until: gone tools/gen-attribution.py:--check-c-deps]
