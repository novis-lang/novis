- **`tools/try.py` runs the `--FILE--` body and the `--FILE nvs.toml--` body as one program**, so
  any `.nvst` case carrying a capability grant fails there with invented diagnostics —
  `error[E0319]: read is not a constant that exists` pointing at `read = ["."]`, which reads exactly
  like a name-resolution bug in the case you just wrote. Every `Core\IO` case has that section, so
  nothing is wrong with the case. `target/debug/nvs.exe test <path.nvst>` understands the format,
  takes a single file, and is the same harness `verify.py` uses; keep `try.py` for a snippet with no
  config block. [until: exists tools/try.py:nvs.toml]
