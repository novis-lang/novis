A read is: issue the syscall; on success return; on `WouldBlock` register, suspend, and loop.
Optimistic, not pessimistic, and the order is deliberate.

The costs are not symmetric. A syscall that succeeds is one `recv` and nothing else. A syscall that
returns `WouldBlock` is one failed `recv` — tens of nanoseconds, with no ready-list work in the
kernel. Registering interest is an `epoll_ctl` or an AFD submission, which is the microsecond class
and the expensive half, and the suspension is one stack switch each way. Polling before reading would
pay the microsecond-class registration on **every** read, including the ones that would have
succeeded — and those are the common case, because the first read of an accepted connection almost
always finds the request bytes already buffered: they arrived with the connection.

A registration is kept while the task holds the stream rather than torn down per park, so a stream
that parks repeatedly pays a modification and not a creation.

**A stream keeps a readiness bit, as tokio does, so the syscall is tried only when it might
succeed.** A `WouldBlock` clears the bit. Any readiness event the reactor reports for the task the
stream is registered under sets it again. The reactor's tokens are task ids, so the bit is the
reactor's per-task event count, taken at the `WouldBlock` and compared on the next read. A poll that
finds the bit clear files its deadline and arms exactly as a `WouldBlock` would, and answers
`Pending` with no syscall. The first read after registration, a read after any change of
registration and a read off a core always try the syscall. An event on another descriptor of the
same task sets the bit too, which costs one extra failed `recv` and never a missed one.

What it spends: four bytes per registered task in the reactor, and four per stream.

**Declined:**
- **A bit set by the event for the descriptor itself.** The token would have to name the stream,
  which is a second table beside the task ids for one failed `recv` saved per shared wake.
