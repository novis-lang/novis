- **A dossier proof timed on `target/debug/nvs.exe` reads about fifteen times its real cost, and
  the sweep runs the release binary.** `tests/hostile/core/Debug/render/01-a-pile-of-renderings-kept-at-once.nvs`
  took 43s on the debug binary and 2.9s on the release one `dossier.py` builds for itself, so counts
  tuned against the debug reading are tuned to a number nothing measures. Time a hostile case or a
  bench with `target/release/nvs.exe` before choosing its counts, and keep the debug binary for
  whether the program runs at all. [until: reviewed 2026-09-22]
