Two performance measurements exist and they are never one.

The **per-PR regression guards** are self-relative wall-clock ratios and slopes — a deep chain
against a shallow one, a throw against a return — and the counts beside them. They run on every
platform, and on a push when the diff touched a crate whose cost they measure. Each comparison
happens on one machine in one run, so it needs no cross-machine comparability, but load on that
machine still moves it. So a timing has two bounds: missing the **soft** one prints a warning into
the CI log and fails nothing, and only the **hard** one, an order of magnitude further or just short
of the change of kind it guards, fails, after re-measuring. A count has one bound and fails at once.
They answer "did this commit regress".

The **historical dashboard** compiles a fixed workload on a dedicated, non-shared Linux or WSL
runner and records the aggregate retired-instruction count under callgrind. It answers "is Novis's
own implementation getting faster, in a sense comparable across whoever's machine produced each
point". The instruction count is the headline because it is bit-for-bit reproducible regardless of
CPU generation, thermal state or scheduler; a timer is not, and no cross-machine claim is ever made
from one.

Benchmarking an Novis **program's** own code is neither of these — that is `rule:testing/bench-counters`.
