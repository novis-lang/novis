The memory-safety and fuzzing evidence — the interpreter-under-Miri run, the fuzz smoke runs and the
unsafe audit — runs in the nightly and at release, and not on a push. The trade is stated plainly:
anything only those would have seen is found within a day rather than within ten minutes. **Nothing
platform-specific is deferred** — a divergence between Linux, Windows and macOS is still caught by
the push that introduced it.

The fuzz corpus is cached between nightly runs and each target's budget is larger there than a push
could afford. A run that starts from what every previous night found is strictly more coverage than
a per-push run that starts from nothing, and it costs the push lane nothing at all.

The unsafe audit's counts gate nothing, so a tool install on every push bought a number no push was
blocked by. It returns to the push lane when it becomes a hard gate.
