The historical leg runs under `valgrind --tool=callgrind`, which has no native Windows build, so it
runs on Linux or WSL and nowhere else. `valgrind` is part of the one-time WSL setup this repository
already documents for its fuzzing tools — the same shape of dependency, added for the same reason.

Windows and macOS machines and CI legs never run that leg. They keep the wall-clock guards, which
need no Linux-only tooling and run everywhere.

Per-function attribution inside JIT-compiled code is unavailable through callgrind, which sees no
symbols for frames the backend registered none for; the aggregate total is unaffected, being a raw
instruction count either way, and root-causing falls back to platform-native profiling on whichever
machine reproduces the regression.
