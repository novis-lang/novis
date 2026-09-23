- **A stray `nvs.exe` locks `target/debug/nvs.exe`, and the rebuild that cannot replace it still
  looks like it worked.** A language server left running by a test, a probe or a killed editor holds
  the binary open, so the linker fails with `Zugriff verweigert (os error 5)` — a line in the middle
  of otherwise ordinary output — and the *old* binary stays on disk, so the next run tests the code
  you just changed away. `tasklist //FI "IMAGENAME eq nvs.exe"` before trusting a rebuild that
  disagrees with your edit, `taskkill //F //IM nvs.exe` to clear it, and check the mtime rather than
  the exit status. [until: reviewed 2026-09-08]
