- **`python tools/try.py` cannot run a `.nvst` case that carries a `--FILE nvs.toml--` section.**
  It inlines the `--FILE--` block into one scratch `.nvs` program and leaves every later section
  standing in it as source, so a case that needs a `[db.<name>]` block reports `E0105` and `E0319`
  on the section header rather than anything about the case. Run that case with
  `./target/debug/nvs.exe test <path>.nvst`, which is what `verify.py`'s conformance step drives.
  [until: reviewed 2026-09-22]
