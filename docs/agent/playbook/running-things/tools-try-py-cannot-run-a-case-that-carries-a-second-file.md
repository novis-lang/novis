- **`tools/try.py` cannot run a case that carries a second `--FILE <name>--` section, and what it
  prints reads as if the case itself were broken.** It hands everything after `--FILE--` to the
  compiler, so a `--FILE nvs.toml--` capability grant arrives as Novis source and answers with parse
  errors on `[capabilities.net]`. `target/debug/nvs.exe test <case.nvst>` runs one case the way the
  suite does and honours the extra sections; for a single-file case `try.py` is still the cheap way
  to capture an `--EXPECTF-ERROR--` block — paste only the `error[...]` line and the `-->` line
  under it. [until: gone tools/try.py:--FILE--]
