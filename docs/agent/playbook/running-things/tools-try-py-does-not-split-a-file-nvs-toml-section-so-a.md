- **`tools/try.py` does not split a `--FILE nvs.toml--` section, so a `.nvst` case carrying its own
  configuration compiles that TOML as Novis.** What comes back is a dozen `E0105`/`E0319`
  diagnostics pointing at `[capabilities.db]`, which reads as the case being wrong rather than as
  the runner not knowing that section. Run a case that carries a config with `target/debug/nvs.exe
  test <case.nvst>` — it runs one file and prints `1 passed` — and keep `try.py` for the cases whose
  only section is `--FILE--`. [until: reviewed 2026-09-21]
