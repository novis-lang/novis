A change that does provably less work for the same result is an optimization Novis takes, whether
or not a timing shows that it wins time. Less work is fewer of something a count can see: allocations,
copies, thread-local lookups, locks, system calls, passes over the input, writes to cache lines every
core shares, emitted calls, executed statements. The proof is a count, never a clock: a test that
counts the work before and after, or code whose structure shows it, such as one lookup where there
were two on the same path.

A timing cannot decline such a change, for three reasons. A saving smaller than a machine's
run-to-run spread is real and invisible to a timing, so "no measurable gain" says nothing about the
gain. A count reads the same on every machine however loaded, while a timing moves with the CPU model,
which is why `testing/perf-two-mechanisms` makes the count the check that fails. And small savings
on a path every request takes add up across the paths, where no single one of them would show.

The change is still held to everything above latency, and to behaviour:

- **Security and request isolation are not traded for it.** A saving that weakens a check, a cap or
  an isolation boundary is not less work, it is less safety, and `programs/memory-priority`'s
  ordering declines it. New `unsafe` code a saving needs is weighed as a security cost first.
- **What a program observably does does not change.** The rules and the conformance cases pin it.
- **Simplicity is spent to buy it,** as the ordering allows: more code or a coupling between two
  modules is an acceptable price for less work on the request path. Memory is spent too, and stated
  as `programs/memory-priority` requires.
- **The count is of the whole path, not one step of it.** Moving work from one place to another, or
  saving a step by adding a more expensive one beside it, is not less work.

A timing taken around such a change has one job: to catch a change that got slower, such as a worse
memory layout, a lost inline or a cache miss no count sees. A slowdown past the noise means the proof
missed something, and it is found and fixed or the change is reverted. A gain that is too small to
time is never a reason to revert.

The size of a saving may rank it against other work for a session, by `programs/slice-ranking`. It
never decides whether the saving is taken.

**Memory:** none of its own. A saving that spends memory states the amount, per request or per task,
as `programs/memory-priority` says.

**Declined:**
- **Keeping a change only if a timing shows a gain past a threshold.** A gain under the runners'
  spread would be reverted although it is real, and a bound set on one machine fails on another.
- **A minimum share of a request's CPU before a provable saving is taken.** The share decides when the
  work is scheduled, not whether it is worth doing.
