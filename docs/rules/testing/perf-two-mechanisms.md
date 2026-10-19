Two performance measurements exist and they are never one.

The **per-PR regression guards** are self-relative wall-clock ratios and slopes — a deep chain
against a shallow one, a throw against a return — and the counts beside them. They run on every
platform, and on a push when the diff touched a crate whose cost they measure. Each comparison
happens on one machine in one run, so it needs no cross-machine comparability, but load on that
machine still moves it. So a timing has two bounds: missing the **soft** one prints a warning into
the CI log and fails nothing, and only the **hard** one, an order of magnitude further or just short
of the change of kind it guards, fails, after re-measuring. A count has one bound and fails at once.
They answer "did this commit regress".

A timing past its hard bound is measured again, up to three attempts, and only the last attempt is
judged: a burst of load that lands on one attempt passes on the next, and a change of kind is past
the bound on every attempt. The soft-bound warning is written straight to standard output, because
libtest holds back what a passing test prints; on GitHub Actions it is a `::warning` command, which
lists it among the run's annotations, and a line in the file `GITHUB_STEP_SUMMARY` names. Each
attempt sits between two readings of a yardstick, a fixed scalar loop, and is labelled `noisy` when
they differ by more than 25% of the faster one and `steady` otherwise; every printed time gives its
ratio to the yardstick. Neither label nor ratio is judged: they are what a reader of a warning has
for telling a busy machine from a slower program. Each test binary carries its own `judge`, so a
change to how a timing is judged is made in `perf_guards.rs`, in the LSP's `latency.rs` and in
`nvs-cli`'s `testing.rs`, whose `judge_margin` judges that binary's `…_by_the_margin_this_test_names`
ratios.

Allocations, emitted calls and executed statements read the same on any machine however loaded, so
where a guard's question can be asked as a count, the count is the check that fails and a timing
beside it is a sanity bound. A slowdown between the two bounds no longer fails CI, and one with no
count behind it, such as a layout that misses the cache more often, shows only as a warning until it
reaches the hard bound; catching it is the job of a same-machine comparison and of the counts. A
warning is answered by measuring the commit before it on the machine that warned.

**Memory:** none per request or per task; this is test code. A test run pays two yardstick readings
per attempt, and up to two further attempts when a figure is past its hard bound.

The **historical dashboard** compiles a fixed workload on a dedicated, non-shared Linux or WSL
runner and records the aggregate retired-instruction count under callgrind. It answers "is Novis's
own implementation getting faster, in a sense comparable across whoever's machine produced each
point". The instruction count is the headline because it is bit-for-bit reproducible regardless of
CPU generation, thermal state or scheduler; a timer is not, and no cross-machine claim is ever made
from one.

Benchmarking an Novis **program's** own code is neither of these — that is `rule:testing/bench-counters`.

The dashboard's runner is one more piece of infrastructure to keep alive. That cost is accepted,
because the guards cannot answer its question: a self-relative ratio says whether a commit regressed
against itself, never whether Novis got faster in an absolute sense.

**Declined:**
- **One bound per timing.** A bound chosen on one machine is enforced on every other, and load moves
  a figure inside one run, so a single bound is either loose enough to pass the change of kind the
  guard exists for or tight enough to fail with no change behind it.
- **Counting VM opcodes or dispatch-loop iterations.** There is no interpreter tier to count.
- **The hardware's instructions-retired counter** (`perf`, ETW, Instruments). It is locked to one
  architecture and is a different tool on each OS, so it compares across machines of one
  architecture and never across operating systems. Callgrind's emulated instruction stream has
  neither limit.
- **A fixed reference workload, such as a SHA-256 or matrix loop, timed on every machine.** The
  guards' same-run ratios already normalize more simply for regressions, and the instruction count
  is more precise for the trend with no extra reference binary to keep.
- **Callgrind as the gate on every pull request.** Its emulation is one to two orders of magnitude
  slower, which every pull request would pay, and it runs on only one of the three CI platforms.
