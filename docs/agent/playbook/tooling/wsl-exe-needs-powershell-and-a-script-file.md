- **`wsl.exe` needs PowerShell and a script file.** An inline `bash -lc "…"` mangles, and WSL's
  default shell has no `grep`/`sed` on `PATH` from a bare `bash -c`. For the whole suite, background
  `python tools/loop.py --leg-only`, which drives `wsl.exe` for you; for one fixture,
  `tools/leak-check.sh <paths>` takes `.nvs` files only, so a `.nvst` passed to it reports a failure
  that is not a leak. [until: reviewed 2026-09-06]
