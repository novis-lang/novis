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
