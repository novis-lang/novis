The memory-safety and fuzzing evidence — the interpreter-under-Miri run, the fuzz smoke runs and the
unsafe audit — runs in the nightly and at release, and not on a push. The trade is stated plainly:
anything only those would have seen is found within a day rather than within ten minutes. **Nothing
platform-specific is deferred** — a divergence between Linux, Windows and macOS is still caught by
the push that introduced it.

The fuzz corpus is cached between nightly runs and each target's budget is larger there than a push
could afford. A run that starts from what every previous night found is strictly more coverage than
a per-push run that starts from nothing, and it costs the push lane nothing at all.

The leak check runs there too: `bun nv valgrind` runs every `examples/` program under valgrind's
memcheck over the debug build, and a definite leak or an invalid access fails the job. It is the one
run that looks for leaks from end to end, since the `asan` job runs with leak detection off. A leak a
change brings in is found the next night.

The comparison's `flood` row runs there too, in its smoke form, because it builds every peer's image
and Novis's in release, which no push can afford. It fails when a stack cannot serve the row, and the
results it prints gate nothing.

The unsafe audit's counts gate nothing, so a tool install on every push bought a number no push was
blocked by. It returns to the push lane when it becomes a hard gate.
