Each history entry carries that same run's wall-clock time and, where a runnable equivalent exists,
the same-host same-run ratio against the pinned PHP the benchmarks already run. Both are
**secondary**: useful to whoever operated the runner, and not comparable across machines.

The PHP ratio reuses the engine the benchmarks already keep rather than inventing a second
reference program, on the reasoning that both runtimes meet identical hardware and OS
conditions during one measurement — which normalizes machine differences at least as well as an
unrelated synthetic baseline would, with no new moving part.

An instruction count is a proxy, and it can in principle improve while real latency worsens. These
two figures exist to catch that divergence, so the dashboard is read together with them and never
from the count alone.
